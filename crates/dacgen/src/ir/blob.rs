use crate::ir::{normalize_name, PartialIR};
use crate::user::UserBlobAsset;
use crate::UserAssetFile;
use dawn_assets::ir::blob::IRBlob;
use dawn_assets::ir::IRAsset;
use std::path::Path;

pub fn convert_blob(
    file: &UserAssetFile,
    cache_dir: &Path,
    cwd: &Path,
    user: &UserBlobAsset,
) -> anyhow::Result<Vec<PartialIR>> {
    let data = user.source.read(cache_dir, cwd)?;

    Ok(vec![PartialIR {
        id: normalize_name(file.path.clone()),
        header: file.asset.header.clone(),
        ir: IRAsset::Blob(IRBlob { data }),
    }])
}
