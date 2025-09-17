use crate::ir::PartialIR;
use crate::source::SourceRef;
use crate::user::{ShaderOrigin, UserShaderAsset};
use crate::UserAssetFile;
use dawn_assets::ir::shader::IRShader;
use dawn_assets::ir::IRAsset;
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

pub fn read_origin(origin: &ShaderOrigin, cache_dir: &Path, cwd: &Path) -> anyhow::Result<String> {
    Ok(match origin {
        ShaderOrigin::Inline { code } => code.clone(),
        ShaderOrigin::External(source) => {
            let bytes = source.read(cache_dir, cwd)?;
            String::from_utf8(bytes)?
        }
    })
}

pub fn preprocess_shader<'a>(
    text: &'a str,
    file_path: &'a Path,
    cache_dir: &'a Path,
    cwd: &'a Path,
) -> anyhow::Result<Cow<'a, str>> {
    // Supported directives:
    //      #include "relative/path/to/shader"
    //      #pragma user_defines
    static INCLUDE_DIRECTIVE_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r#"(?m)^\s*#include\s+"([^"]+)"\s*$"#).expect("Failed to compile regex")
    });

    let mut output = String::new();
    let mut last_end = 0;
    let mut lines = 1;
    for capture in INCLUDE_DIRECTIVE_REGEX.captures_iter(text) {
        // Add text before the match
        let m = capture.get(0).unwrap();
        let part = &text[last_end..m.start()];
        output.push_str(part);
        lines += part.matches('\n').count();
        last_end = m.end();

        // Get the included file path
        let include_path = capture.get(1).unwrap().as_str();
        let include_full_path = file_path.parent().unwrap().join(include_path);
        let include_text = read_origin(
            &ShaderOrigin::External(SourceRef::File(include_full_path.clone().into())),
            cache_dir,
            cwd,
        )
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to read included shader file '{}': {}",
                include_full_path.display(),
                e
            )
        })?;

        let preprocessed_include =
            preprocess_shader(&include_text, &include_full_path, cache_dir, cwd)?;
        output.push_str(&preprocessed_include);

        // Push a #line directive to keep line numbers correct
        output.push_str(&format!("\n#line {}\n", lines));
    }
    if !output.is_empty() {
        output.push_str(&text[last_end..]);
    }

    if output.is_empty() {
        Ok(Cow::Borrowed(text))
    } else {
        Ok(Cow::Owned(output))
    }
}

pub fn convert_shader(
    file: &UserAssetFile,
    cache_dir: &Path,
    cwd: &Path,
    user: &UserShaderAsset,
) -> anyhow::Result<Vec<PartialIR>> {
    let mut sources = HashMap::new();
    for source in user.sources.iter() {
        let text = read_origin(&source.origin, cache_dir, cwd)?;
        let text = preprocess_shader(&text, &file.path, cache_dir, cwd)?;
        sources.insert(source.kind, text.as_bytes().to_vec());
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
