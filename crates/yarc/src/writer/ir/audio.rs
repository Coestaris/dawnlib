use crate::writer::user::{UserAssetHeader, UserAudioAsset};
use crate::writer::UserAssetFile;
use dawn_assets::ir::audio::IRAudio;
use std::path::Path;
use crate::writer::ir::PartialIR;

pub fn convert_audio(file: &UserAssetFile, user: &UserAudioAsset) -> Result<Vec<PartialIR>, String>  {
    todo!()
}
