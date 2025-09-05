use crate::ir::PartialIR;
use crate::user::{ShaderOrigin, ShaderSource, UserShaderAsset};
use crate::UserAssetFile;
use dawn_assets::ir::shader::IRShader;
use dawn_assets::ir::IRAsset;
use std::collections::HashMap;
use std::path::Path;

pub fn read_origin(origin: &ShaderOrigin, cache_dir: &Path, cwd: &Path) -> anyhow::Result<Vec<u8>> {
    Ok(match origin {
        ShaderOrigin::Inline { code } => code.clone().as_bytes().to_vec(),
        ShaderOrigin::External(source) => source.read(cache_dir, cwd)?,
    })
}

pub fn convert_shader(
    file: &UserAssetFile,
    cache_dir: &Path,
    cwd: &Path,
    user: &UserShaderAsset,
) -> anyhow::Result<Vec<PartialIR>> {
    let mut sources = HashMap::new();
    for source in user.sources.iter() {
        let mut source_string = read_origin(&source.origin, cache_dir, cwd)?;
        for pre in &source.pre_include {
            let pre = read_origin(pre, cache_dir, cwd)?;
            source_string.splice(0..0, pre);
        }
        for post in &source.post_include {
            let post = read_origin(post, cache_dir, cwd)?;
            source_string.splice(source_string.len()..source_string.len(), post);
        }

        sources.insert(source.kind, source_string);
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
