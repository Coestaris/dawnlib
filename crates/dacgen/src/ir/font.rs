use crate::ir::{normalize_name, PartialIR};
use crate::user::{UserAssetHeader, UserFontAsset};
use crate::UserAssetFile;
use dawn_assets::ir::font::{IRFont, IRGlyph};
use dawn_assets::ir::texture::{IRPixelFormat, IRTexture, IRTextureType};
use dawn_assets::ir::IRAsset;
use dawn_assets::{AssetID, AssetType};
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

    let mut ir_glyphs = HashMap::new();
    for c in text.chars() {
        let g = font.glyph(c).scaled(scale);
        let h_metrics = g.h_metrics();
        let advance_width = h_metrics.advance_width;
        let left_side_bearing = h_metrics.left_side_bearing;
        let bounding_box = g
            .exact_bounding_box()
            .map(|bb| (bb.min.x, bb.min.y, bb.max.x - bb.min.x, bb.max.y - bb.min.y));

        ir_glyphs.insert(
            c,
            IRGlyph {
                width: 0,
                height: 0,
                x: 0,
                y: 0,
                x_advance: 0,
                y_offset: 0,
                x_offset: 0,
                page: 0,
            },
        );
    }

    // Render the glyphs into a buffer
    let width = glyphs_width + 2 * HORIZONTAL_SPACING as u32;
    let height = glyphs_height + 2 * VERTICAL_SPACING as u32;
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
    let image = DynamicImage::ImageRgb8(image::ImageBuffer::from_fn(width, height, |x, y| {
        let alpha = raw[(x + y * width) as usize];
        image::Rgb([alpha, alpha, alpha])
    }));
    image.save("/tmp/output_font.png")?;

    let (mut irs, texture_id) = convert_texture(font_id.clone(), width, height, raw)?;
    let mut header = file.asset.header.clone();
    header.dependencies.insert(texture_id.clone());
    irs.push(PartialIR {
        id: font_id,
        header,
        ir: IRAsset::Font(IRFont {
            glyphs: ir_glyphs,
            atlases: vec![texture_id],
        }),
    });

    Ok(irs)
}
