use crate::ir::PartialIR;
use crate::user::{ShaderOrigin, UserShaderAsset};
use crate::UserAssetFile;
use dawn_assets::ir::shader::IRShader;
use dawn_assets::ir::IRAsset;
use std::collections::HashMap;
use std::path::Path;

pub fn convert_shader(
    file: &UserAssetFile,
    cache_dir: &Path,
    cwd: &Path,
    user: &UserShaderAsset,
) -> anyhow::Result<Vec<PartialIR>> {
    let mut sources = HashMap::new();
    for source in user.sources.iter() {
        sources.insert(
            source.kind,
            match &source.origin {
                ShaderOrigin::Inline { code } => code.clone().as_bytes().to_vec(),
                ShaderOrigin::External(source) => source.read(cache_dir, cwd)?,
            },
        );
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
