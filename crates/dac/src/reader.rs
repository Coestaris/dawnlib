use crate::container::ContainerError;
use crate::Manifest;
use dawn_assets::ir::IRAsset;
use dawn_assets::AssetID;
use std::io::{Read, Seek};

pub fn read_manifest<R: Read + Seek>(reader: &mut R) -> Result<Manifest, ContainerError> {
    crate::container::reader::read_manifest(reader)
}

pub fn read_asset<R: Read + Seek>(reader: &mut R, id: AssetID) -> Result<IRAsset, ContainerError> {
    crate::container::reader::read_ir(reader, id)
}
