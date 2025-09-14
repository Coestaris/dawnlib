use crate::ir::PartialIR;
use crate::user::UserMaterialAsset;
use crate::UserAssetFile;
use std::path::Path;

pub fn convert_material(
    _file: &UserAssetFile,
    _cache_dir: &Path,
    _cwd: &Path,
    _user: &UserMaterialAsset,
) -> anyhow::Result<Vec<PartialIR>> {
    todo!();

    // TODO: Read iamges from disk
    // convert_material_from_memory(
    //     normalize_name(file.path.clone()),
    //     file.asset.header.clone(),
    //     UserMaterialAssetInner {
    //         base_color_factor: user.base_color_factor.clone(),
    //         base_color_texture: None,
    //         metallic_texture: None,
    //         metallic_factor: 0.0,
    //         roughness_texture: None,
    //         roughness_factor: 0.0,
    //     },
    // )
}
