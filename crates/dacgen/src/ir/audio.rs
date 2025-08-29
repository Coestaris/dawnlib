use crate::ir::PartialIR;
use crate::user::UserAudioAsset;
use crate::UserAssetFile;
use std::path::Path;

pub fn convert_audio(
    file: &UserAssetFile,
    cache_dir: &Path,
    cwd: &Path,
    user: &UserAudioAsset,
) -> anyhow::Result<Vec<PartialIR>> {
    todo!()
}
