use crate::ir::{normalize_name, PartialIR};
use crate::user::{UserAssetHeader, UserFontAsset};
use crate::UserAssetFile;
use dawn_assets::ir::font::{IRFont, IRGlyph, IRGlyphVertex};
use dawn_assets::ir::mesh::IRTopology;
use dawn_assets::ir::texture::{IRPixelFormat, IRTexture, IRTextureType};
use dawn_assets::ir::IRAsset;
use dawn_assets::{AssetID, AssetType};
use glam::Vec2;
use image::DynamicImage;
use rusttype::{point, Font, Scale};
use std::collections::HashMap;
use std::path::Path;

const VERTICAL_SPACING: f32 = 4.0;
const HORIZONTAL_SPACING: f32 = 4.0;

fn convert_texture(
    font_id: AssetID,
    w: u32,
    h: u32,
    data: Vec<u8>,
) -> anyhow::Result<(Vec<PartialIR>, AssetID)> {
    let texture_id = AssetID::from(format!("{}_atlas", font_id.as_str()));

    Ok((
        vec![PartialIR {
            id: texture_id.clone(),
            header: UserAssetHeader {
                asset_type: AssetType::Texture,
                dependencies: Default::default(),
                tags: vec![],
                author: Some("Auto-generated".to_string()),
                license: None,
            },
            ir: IRAsset::Texture(IRTexture {
                data,
                texture_type: IRTextureType::Texture2D {
                    width: w,
                    height: h,
                },
                pixel_format: IRPixelFormat::R8,
                use_mipmaps: false,
                min_filter: Default::default(),
                mag_filter: Default::default(),
                wrap_s: Default::default(),
                wrap_t: Default::default(),
                wrap_r: Default::default(),
            }),
        }],
        texture_id,
    ))
}

pub fn convert_font(
    file: &UserAssetFile,
    cache_dir: &Path,
    cwd: &Path,
    user: &UserFontAsset,
) -> anyhow::Result<Vec<PartialIR>> {
    let data = user.source.read(cache_dir, cwd)?;
    let font = Font::try_from_bytes(&data)
        .ok_or_else(|| anyhow::anyhow!("Failed to load font from file"))?;

    let font_id = normalize_name(file.path.clone());
    let chars = user.charset.to_chars();
    let mut text = chars.into_iter().collect::<Vec<_>>();
    text.sort();
    text.dedup();
    let text = text.into_iter().collect::<String>();

    let scale = Scale::uniform(user.size as f32);

    let v_metrics = font.v_metrics(scale);
    let glyphs: Vec<_> = font
        .layout(
            text.as_str(),
            scale,
            point(HORIZONTAL_SPACING, VERTICAL_SPACING + v_metrics.ascent),
        )
        .collect();

    // work out the layout size
    let glyphs_height = (v_metrics.ascent - v_metrics.descent).ceil() as u32;
    let glyphs_width = {
        let min_x = glyphs
            .first()
            .map(|g| g.pixel_bounding_box().unwrap().min.x)
            .unwrap();
        let max_x = glyphs
            .last()
            .map(|g| g.pixel_bounding_box().unwrap().max.x)
            .unwrap();
        (max_x - min_x) as u32
    };
    let width = glyphs_width + 2 * HORIZONTAL_SPACING as u32;
    let height = glyphs_height + 2 * VERTICAL_SPACING as u32;

    let mut ir_glyphs = HashMap::new();
    let mut vertices = Vec::with_capacity(glyphs.len() * 6 * size_of::<IRGlyphVertex>());
    let mut current_vertex = 0;
    for (char, positioned) in text.chars().zip(&glyphs) {
        let bounding_box = positioned.pixel_bounding_box().unwrap();
        let unpositioned = positioned.unpositioned();

        let w = (bounding_box.max.x - bounding_box.min.x) as f32;
        let h = (bounding_box.max.y - bounding_box.min.y) as f32;
        let x = bounding_box.min.x as f32 - HORIZONTAL_SPACING;
        let y = bounding_box.min.y as f32 - VERTICAL_SPACING;

        let a = Vec2::new(x, y);
        let b = Vec2::new(x + w, y);
        let c = Vec2::new(x, y + h);
        let d = Vec2::new(x + w, y + h);

        // Insert two triangles
        // (A, B, C)
        vertices.extend_from_slice(IRGlyphVertex::new(a).into_bytes());
        vertices.extend_from_slice(IRGlyphVertex::new(b).into_bytes());
        vertices.extend_from_slice(IRGlyphVertex::new(c).into_bytes());

        // (A, C, D)
        vertices.extend_from_slice(IRGlyphVertex::new(a).into_bytes());
        vertices.extend_from_slice(IRGlyphVertex::new(d).into_bytes());
        vertices.extend_from_slice(IRGlyphVertex::new(c).into_bytes());

        ir_glyphs.insert(
            char,
            IRGlyph {
                vertex_offset: current_vertex,
                vertex_count: 6,
                x_advance: unpositioned.h_metrics().advance_width,
                y_offset: 0.0,
                x_offset: unpositioned.h_metrics().left_side_bearing,
            },
        );

        current_vertex += 6;
    }

    // Render the glyphs into a buffer
    let mut raw = vec![0; (width * height) as usize];
    for glyph in glyphs {
        if let Some(bounding_box) = glyph.pixel_bounding_box() {
            // Draw the glyph into the image per-pixel by using the draw closure
            glyph.draw(|x, y, v| {
                // Offset the position by the glyph bounding box
                let x = x + bounding_box.min.x as u32;
                let y = y + bounding_box.min.y as u32;
                // Turn the coverage into an alpha value
                let color = (v * 255.0) as u8;
                let idx = (x + y * width) as usize;
                if idx < raw.len() {
                    raw[idx] = color;
                } else {
                    panic!("Index out of bounds: {} >= {}", idx, raw.len());
                }
            });
        }
    }

    // Test the image
    // let image = DynamicImage::ImageRgb8(image::ImageBuffer::from_fn(width, height, |x, y| {
    //     let alpha = raw[(x + y * width) as usize];
    //     image::Rgb([alpha, alpha, alpha])
    // }));
    // image.save("/tmp/output_font.png")?;

    let (mut irs, atlas) = convert_texture(font_id.clone(), width, height, raw)?;
    let mut header = file.asset.header.clone();
    header.dependencies.insert(atlas.clone());
    irs.push(PartialIR {
        id: font_id,
        header,
        ir: IRAsset::Font(IRFont {
            glyphs: ir_glyphs,
            y_advance: 0.0,
            atlas,
            vertices,
            topology: IRTopology::Triangles,
        }),
    });

    Ok(irs)
}
