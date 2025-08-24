use log::debug;
use crate::ir::PartialIR;
use crate::user::UserAudioAsset;
use crate::UserAssetFile;

pub fn convert_audio(
    file: &UserAssetFile,
    user: &UserAudioAsset,
) -> Result<Vec<PartialIR>, String> {
    debug!("Converting audio: {:?}", file);
    
    todo!()
}
