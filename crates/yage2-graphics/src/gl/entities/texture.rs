use crate::passes::events::PassEventTrait;
use crate::renderer::RendererBackend;
use yage2_core::assets::raw::TextureAssetRaw;

pub struct Texture {}

impl Texture {
    pub(crate) fn from_raw<E: PassEventTrait>(
        raw: &TextureAssetRaw,
    ) -> Result<Self, String> {
        todo!()
    }

    fn new<E>() -> Self {
        Self {}
    }
}
