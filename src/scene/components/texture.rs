#[derive(Clone, Copy, Debug)]
pub struct TextureRef {
    pub width: u32,
    pub height: u32,
    pub index: i32,
}

impl Default for TextureRef {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            index: -1,
        }
    }
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
