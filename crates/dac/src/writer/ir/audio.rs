use log::debug;
use crate::writer::ir::PartialIR;
use crate::writer::user::UserAudioAsset;
use crate::writer::UserAssetFile;

pub fn convert_audio(
    file: &UserAssetFile,
    user: &UserAudioAsset,
) -> Result<Vec<PartialIR>, String> {
    debug!("Converting audio: {:?}", file);
    
    todo!()
}
