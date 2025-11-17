#[derive(Clone, Copy, Default, Debug)]
pub struct TextureRef {
    pub width: u32,
    pub height: u32,
    pub index: u32,
}

pub enum TextureDefinition {
    FromFile {
        path: String,
    },
    #[allow(unused)]
    FromData {
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    },
}
