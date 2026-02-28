use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub enum AudioFormat {
    #[serde(rename = "mp3")]
    #[default]
    Mp3,
    #[serde(rename = "flac")]
    Flac,
}

impl AudioFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Flac => "flac",
        }
    }
}
