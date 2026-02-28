use crate::{audioformat::AudioFormat, metadata::Metadata};

pub const DEFAULT_MAGIC: [u8; 8] = [0x43, 0x54, 0x45, 0x4E, 0x46, 0x44, 0x41, 0x4D];

pub const DEFAULT_CORE_KEY: [u8; 16] = [
    0x68, 0x7A, 0x48, 0x52, 0x41, 0x6D, 0x73, 0x6F, 0x35, 0x6B, 0x49, 0x6E, 0x62, 0x61, 0x78, 0x57,
];

pub const DEFAULT_MODIFY_KEY: [u8; 16] = [
    0x23, 0x31, 0x34, 0x6C, 0x6A, 0x6B, 0x5F, 0x21, 0x5C, 0x5D, 0x26, 0x30, 0x55, 0x3C, 0x27, 0x28,
];

pub struct NcmFile {
    pub metadata: Option<Metadata>,
    pub cover_image: Option<Vec<u8>>,
    pub format: AudioFormat,
    pub key_box: [u8; 256],
    pub audio_offset: u64,
}

impl Default for NcmFile {
    fn default() -> Self {
        Self {
            metadata: None,
            cover_image: None,
            format: Default::default(),
            key_box: [0_u8; 256],
            audio_offset: 0,
        }
    }
}
