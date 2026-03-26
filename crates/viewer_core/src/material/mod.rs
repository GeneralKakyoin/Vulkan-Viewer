use serde::{Deserialize, Serialize};
pub mod animation;
pub use animation::TextureAnim;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextureEntry {
    pub texture_id: crate::AssetID,
    pub rgba: [f32; 4],
    pub offset_s: f32,
    pub offset_t: f32,
    pub scale_s: f32,
    pub scale_t: f32,
    pub rotation: f32,
    pub bump: u8,
    pub fullbright: bool,
    pub shiny: u8,
    pub media_flags: u16,
}

impl Eq for TextureEntry {}

impl std::hash::Hash for TextureEntry {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.texture_id.hash(state);
        for f in self.rgba {
            f.to_bits().hash(state);
        }
        self.offset_s.to_bits().hash(state);
        self.offset_t.to_bits().hash(state);
        self.scale_s.to_bits().hash(state);
        self.scale_t.to_bits().hash(state);
        self.rotation.to_bits().hash(state);
        self.bump.hash(state);
        self.fullbright.hash(state);
        self.shiny.hash(state);
        self.media_flags.hash(state);
    }
}

impl Default for TextureEntry {
    fn default() -> Self {
        Self {
            texture_id: crate::AssetID::default(),
            rgba: [1.0, 1.0, 1.0, 1.0],
            offset_s: 0.0,
            offset_t: 0.0,
            scale_s: 1.0,
            scale_t: 1.0,
            rotation: 0.0,
            bump: 0,
            fullbright: false,
            shiny: 0,
            media_flags: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PbrDescriptor {
    pub base_color_id: crate::AssetID,
    pub normal_id: crate::AssetID,
    pub metallic_roughness_id: crate::AssetID,
    pub emissive_id: crate::AssetID,
    pub base_color_tint: [f32; 4],
    pub metallic_factor: f32,
    pub roughness_factor: f32,
}

impl Eq for PbrDescriptor {}

impl std::hash::Hash for PbrDescriptor {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.base_color_id.hash(state);
        self.normal_id.hash(state);
        self.metallic_roughness_id.hash(state);
        self.emissive_id.hash(state);
        for f in self.base_color_tint {
            f.to_bits().hash(state);
        }
        self.metallic_factor.to_bits().hash(state);
        self.roughness_factor.to_bits().hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MaterialDescriptor {
    Legacy(TextureEntry),
    Pbr(PbrDescriptor),
}

impl Default for MaterialDescriptor {
    fn default() -> Self {
        Self::Legacy(TextureEntry::default())
    }
}
