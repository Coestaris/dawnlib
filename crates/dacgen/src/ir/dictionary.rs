use crate::ir::{normalize_name, PartialIR};
use crate::user::UserDictionaryAsset;
use crate::UserAssetFile;
use dawn_assets::ir::dictionary::IRDictionary;
use dawn_assets::ir::IRAsset;
use std::path::Path;

pub fn convert_dictionary(
    file: &UserAssetFile,
    _cache_dir: &Path,
    _cwd: &Path,
    user: &UserDictionaryAsset,
) -> anyhow::Result<Vec<PartialIR>> {
    Ok(vec![PartialIR {
        id: normalize_name(file.path.clone()),
        header: file.asset.header.clone(),
        ir: IRAsset::Dictionary(IRDictionary {
            entries: user.entries.clone(),
        }),
    }])
}
