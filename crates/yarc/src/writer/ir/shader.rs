use crate::writer::user::{UserAssetHeader, UserShaderAsset};
use dawn_assets::ir::shader::IRShader;
use std::collections::HashMap;
use std::path::Path;
use crate::writer::ir::PartialIR;
use crate::writer::UserAssetFile;

pub fn convert_shader(
    file: &UserAssetFile,
    user: &UserShaderAsset,
) -> Result<Vec<PartialIR>, String> {
    let mut sources = HashMap::new();
    for (source_type, path_part) in user.files.iter() {
        // Try to find the file in the same directory as the shader
        let directory = asset_path.parent().unwrap();
        let path = directory.join(path_part);

        let content = std::fs::read(path.clone()).map_err(|e| {
            format!(
                "Failed to read shader source file '{}': {}",
                path.to_string_lossy(),
                e
            )
        })?;
        sources.insert(source_type.clone(), content);
    }

    Ok(IRShader {
        sources,
        compile_options: user.compile_options.clone(),
    })
}
