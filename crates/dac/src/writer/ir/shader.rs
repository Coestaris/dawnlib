use crate::writer::ir::PartialIR;
use crate::writer::user::UserShaderAsset;
use crate::writer::UserAssetFile;
use dawn_assets::ir::shader::IRShader;
use dawn_assets::ir::IRAsset;
use std::collections::HashMap;
use log::debug;

pub fn convert_shader(
    file: &UserAssetFile,
    user: &UserShaderAsset,
) -> Result<Vec<PartialIR>, String> {
    debug!("Converting shader: {:?}", file);
    
    let mut sources = HashMap::new();
    for (source_type, path_part) in user.files.iter() {
        // Try to find the file in the same directory as the shader
        let directory = file.path.parent().unwrap();
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

    Ok(vec![PartialIR::new_from_path(
        IRAsset::Shader(IRShader {
            compile_options: user.compile_options.clone(),
            sources,
        }),
        file.asset.header.clone(),
        file.path.clone(),
    )])
}
