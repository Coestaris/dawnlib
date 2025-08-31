use crate::ir::{normalize_name, PartialIR};
use crate::user::{UserAssetHeader, UserFontAsset};
use crate::UserAssetFile;
use dawn_assets::ir::font::{IRFont, IRGlyph, IRGlyphVertex};
use dawn_assets::ir::mesh::{IRIndexType, IRTopology};
use dawn_assets::ir::texture::{
    IRPixelFormat, IRTexture, IRTextureFilter, IRTextureType, IRTextureWrap,
};
use dawn_assets::ir::IRAsset;
use dawn_assets::{AssetID, AssetType};
use glam::Vec2;
use image::DynamicImage;
use rusttype::{point, Font, Point, Scale};
use std::collections::HashMap;
use std::path::Path;

const VERTICAL_SPACING: f32 = 4.0;
const HORIZONTAL_SPACING: f32 = 4.0;

// Add some padding to avoid sampling issues
// when rendering the font atlas.
// This is especially important for fonts with very thin glyphs
// like "I" or "l".
const PAD: u32 = 5;

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
                min_filter: IRTextureFilter::Linear,
                mag_filter: IRTextureFilter::Linear,
                wrap_s: IRTextureWrap::ClampToEdge,
                wrap_t: IRTextureWrap::ClampToEdge,
                wrap_r: IRTextureWrap::ClampToEdge,
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

    fn layout_glyphs<'a>(
        start: Point<f32>,
        font: &'a Font,
        scale: Scale,
        text: &str,
    ) -> Vec<rusttype::PositionedGlyph<'a>> {
        let mut caret = 0.0;
        let mut last_glyph: Option<rusttype::GlyphId> = None;
        let mut glyphs = Vec::with_capacity(text.len());

        for c in text.chars() {
            let g = font.glyph(c).scaled(scale);
            if let Some(last) = last_glyph {
                caret += font.pair_kerning(scale, last, g.id());
            }
            let g = g.positioned(point(start.x + caret, start.y));
            let metrics = g.unpositioned().h_metrics();
            caret += metrics.advance_width;
            caret += PAD as f32;
            last_glyph = Some(g.id());
            glyphs.push(g);
        }

        glyphs
    }

    let glyphs: Vec<_> = layout_glyphs(
        point(HORIZONTAL_SPACING, VERTICAL_SPACING + v_metrics.ascent),
        &font,
        scale,
        &text,
    );

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

    let width = glyphs_width as f32 + 2.0 * HORIZONTAL_SPACING;
    let height = glyphs_height as f32 + 2.0 * VERTICAL_SPACING;
    let width_u32 = width.ceil() as u32;
    let height_u32 = height.ceil() as u32;

    let mut ir_glyphs = HashMap::new();

    let vertex_count = glyphs.len() * 4;
    let index_count = glyphs.len() * 6;
    if vertex_count > u16::MAX as usize {
        return Err(anyhow::anyhow!(
            "Font has too many glyphs to fit in a single atlas (max {} glyphs)",
            u16::MAX as usize / 6
        ));
    }

    let mut vertices = Vec::with_capacity(vertex_count * size_of::<IRGlyphVertex>());
    let mut indices = Vec::with_capacity(index_count * size_of::<u16>());
    let mut current_vertex = 0;
    let mut current_index = 0;

    for (char, positioned) in text.chars().zip(&glyphs) {
        let bounding_box = positioned.pixel_bounding_box().unwrap();
        let unpositioned = positioned.unpositioned();

        let w = (bounding_box.max.x - bounding_box.min.x) as f32;
        let h = (bounding_box.max.y - bounding_box.min.y) as f32;
        let x = bounding_box.min.x as f32;
        let y = bounding_box.min.y as f32;

        // Texture coordinates
        let inv_w = 1.0 / width_u32 as f32;
        let inv_h = 1.0 / height_u32 as f32;

        let tc_a = Vec2::new((x) * inv_w, (y) * inv_h);
        let tc_b = Vec2::new((x + w) * inv_w, (y) * inv_h);
        let tc_c = Vec2::new((x) * inv_w, (y + h) * inv_h);
        let tc_d = Vec2::new((x + w) * inv_w, (y + h) * inv_h);

        // Vertex positions
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(w, 0.0);
        let c = Vec2::new(0.0, h);
        let d = Vec2::new(w, h);

        // A  ---- B
        // |    /  |
        // |  /    |
        // C  ---- D
        // Since we're using default CCW winding, we need to
        // make sure the triangles are defined in that order.
        // A, C, B and B, C, D

        // Insert four vertices
        vertices.extend_from_slice(IRGlyphVertex::new(a, tc_a).into_bytes());
        vertices.extend_from_slice(IRGlyphVertex::new(b, tc_b).into_bytes());
        vertices.extend_from_slice(IRGlyphVertex::new(c, tc_c).into_bytes());
        vertices.extend_from_slice(IRGlyphVertex::new(d, tc_d).into_bytes());

        // Insert (A, C, B) triangle
        indices.extend_from_slice((current_vertex as u16).to_le_bytes().as_slice());
        indices.extend_from_slice((current_vertex as u16 + 2).to_le_bytes().as_slice());
        indices.extend_from_slice((current_vertex as u16 + 1).to_le_bytes().as_slice());

        // Insert (B, C, D) triangle
        indices.extend_from_slice((current_vertex as u16 + 1).to_le_bytes().as_slice());
        indices.extend_from_slice((current_vertex as u16 + 2).to_le_bytes().as_slice());
        indices.extend_from_slice((current_vertex as u16 + 3).to_le_bytes().as_slice());

        ir_glyphs.insert(
            char,
            IRGlyph {
                index_offset: current_index,
                index_count: 6,
                x_advance: unpositioned.h_metrics().advance_width,
                y_offset: (bounding_box.min.y) as f32,
                x_offset: unpositioned.h_metrics().left_side_bearing,
            },
        );

        current_vertex += 4;
        current_index += 6;
    }

    // Offset all y offsets by the minimum y offset to make sure
    // that all glyphs are positioned correctly relative to each other.
    let min = ir_glyphs
        .values()
        .map(|g| g.y_offset)
        .fold(f32::INFINITY, |a, b| a.min(b));
    for g in ir_glyphs.values_mut() {
        g.y_offset -= min;
    }

    // Render the glyphs into a buffer
    let mut raw = vec![0; (width_u32 * height_u32) as usize];
    for glyph in glyphs {
        if let Some(bounding_box) = glyph.pixel_bounding_box() {
            // Draw the glyph into the image per-pixel by using the draw closure
            glyph.draw(|x, y, v| {
                // Offset the position by the glyph bounding box
                let x = x + bounding_box.min.x as u32;
                let y = y + bounding_box.min.y as u32;
                // Turn the coverage into an alpha value
                let color = (v * 255.0) as u8;
                let idx = (x + y * width_u32) as usize;
                if idx < raw.len() {
                    raw[idx] = color;
                } else {
                    panic!("Index out of bounds: {} >= {}", idx, raw.len());
                }
            });
        }
    }

    // Test the image
    // let image = DynamicImage::ImageRgb8(image::ImageBuffer::from_fn(
    //     width_u32,
    //     height_u32,
    //     |x, y| {
    //         for tc in &tcs {
    //             let a_x = (tc.a.x * width_u32 as f32).round() as u32;
    //             let a_y = (tc.a.y * height_u32 as f32).round() as u32;
    //             let b_x = (tc.b.x * width_u32 as f32).round() as u32;
    //             let b_y = (tc.b.y * height_u32 as f32).round() as u32;
    //             let c_x = (tc.c.x * width_u32 as f32).round() as u32;
    //             let c_y = (tc.c.y * height_u32 as f32).round() as u32;
    //             let d_x = (tc.d.x * width_u32 as f32).round() as u32;
    //             let d_y = (tc.d.y * height_u32 as f32).round() as u32;
    //
    //             if (x == a_x && y == a_y) {
    //                 return image::Rgb([255, 0, 0]);
    //             }
    //             if (x == b_x && y == b_y) {
    //                 return image::Rgb([0, 255, 0]);
    //             }
    //             if (x == c_x && y == c_y) {
    //                 return image::Rgb([0, 0, 255]);
    //             }
    //             if (x == d_x && y == d_y) {
    //                 return image::Rgb([255, 255, 0]);
    //             }
    //         }
    //
    //         let alpha = raw[(x + y * width_u32) as usize];
    //         image::Rgb([alpha, alpha, alpha])
    //     },
    // ));
    // image.save(r"c:\Users\user\AppData\Local\dawn_cache\image.png")?;

    let (mut irs, atlas) = convert_texture(font_id.clone(), width_u32, height_u32, raw)?;
    let mut header = file.asset.header.clone();
    header.dependencies.insert(atlas.clone());
    irs.push(PartialIR {
        id: font_id,
        header,
        ir: IRAsset::Font(IRFont {
            glyphs: ir_glyphs,
            y_advance: (v_metrics.ascent - v_metrics.descent).ceil(),
            space_advance: font.glyph(' ').scaled(scale).h_metrics().advance_width,
            atlas,
            vertices,
            topology: IRTopology::Triangles,
            indices,
            index_type: IRIndexType::U16,
        }),
    });

    Ok(irs)
}
