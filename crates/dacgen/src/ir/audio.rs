use std::path::Path;
use log::debug;
use crate::ir::PartialIR;
use crate::user::UserAudioAsset;
use crate::UserAssetFile;

pub fn convert_audio(
    file: &UserAssetFile,
    cache_dir: &Path,
    cwd: &Path,
    user: &UserAudioAsset,
) -> Result<Vec<PartialIR>, String> {
    todo!()
}
