use crate::ir::PartialIR;
use crate::user::UserAudioAsset;
use crate::UserAssetFile;
use std::path::Path;

pub fn convert_audio(
    _file: &UserAssetFile,
    _cache_dir: &Path,
    _cwd: &Path,
    _user: &UserAudioAsset,
) -> anyhow::Result<Vec<PartialIR>> {
    todo!()
}
