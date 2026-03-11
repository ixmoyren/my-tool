use crate::{Result, SerdeJsonOperationSnafu, audioformat::AudioFormat};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use snafu::ResultExt;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Metadata {
    #[serde(rename = "musicId")]
    pub music_id: MetadataId,
    #[serde(rename = "musicName")]
    pub music_name: String,
    pub artist: Vec<Vec<Value>>,
    #[serde(rename = "albumId")]
    pub album_id: MetadataId,
    pub album: String,
    #[serde(rename = "albumPicDocId")]
    pub album_pic_doc_id: MetadataId,
    #[serde(rename = "albumPic")]
    pub album_pic: String,
    pub bitrate: u32,
    #[serde(rename = "mp3DocId", default)]
    pub mp3_doc_id: MetadataId,
    pub duration: u32,
    #[serde(rename = "mvId")]
    pub mv_id: MetadataId,
    pub alias: Vec<String>,
    #[serde(rename = "transNames")]
    pub trans_names: Vec<String>,
    pub format: AudioFormat,
    #[serde(default)]
    pub fee: u8,
    #[serde(rename = "volumeDelta", default)]
    pub volume_delta: f64,
    #[serde(default)]
    pub privilege: Privilege,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Privilege {
    pub flag: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum MetadataId {
    String(String),
    U64(u64),
}

impl Default for MetadataId {
    fn default() -> Self {
        Self::U64(0_u64)
    }
}

impl Metadata {
    pub fn from_slice(metadata: &[u8]) -> Result<Self> {
        let metadata = if metadata.starts_with(b"music:") {
            &metadata[6..]
        } else {
            metadata
        };
        serde_json::from_slice(metadata).context(SerdeJsonOperationSnafu {
            message: "Failed to obtain metadata from json".to_owned(),
        })
    }

    /// Join artist names with " / ".
    pub fn artist_names(&self) -> String {
        if self.artist.is_empty() {
            "".to_owned()
        } else {
            self.artist
                .iter()
                .filter_map(|artist| artist.first().and_then(|v| v.as_str()))
                .collect::<Vec<_>>()
                .join(" / ")
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{audioformat::AudioFormat::Flac, metadata::Metadata};

    #[test]
    fn test_parse_metadata() {
        let json = r#"{"musicId":"2604307454","musicName":"LOVE 2000","artist":[["遠野ひかる","33947223"]],"albumId":"241003755","album":"LOVE 2000","albumPicDocId":"109951169743863380","albumPic":"http://p4.music.126.net/gVjSHS4eTNYnqx73JBN7nA==/109951169743863380.jpg","bitrate":1999000,"mp3DocId":"202302cf8423b05edeb0bcf3bf301ad0","duration":263546,"mvId":"","alias":[],"transNames":["TV动画《败犬女主太多了！》片尾曲1"],"format":"flac","fee":8,"volumeDelta":-11.4337,"privilege":{"flag":1806596}}"#;
        let meta = Metadata::from_slice(json.as_bytes()).unwrap();
        assert_eq!(meta.music_name, "LOVE 2000");
        assert_eq!(meta.artist_names(), "遠野ひかる");
        assert_eq!(meta.format, Flac)
    }

    #[test]
    fn test_parse_metadata2() {
        let json = r#"{"format":"mp3","musicId":4153632,"musicName":"In The End","artist":[["Linkin Park",95439]],"album":"In The End","albumId":419897,"albumPicDocId":1719636185851311,"albumPic":"http://p2.music.126.net/kACfPVCIjo67lPo8ca0REQ==/1719636185851311.jpg","mvId":509036,"flag":4,"bitrate":320000,"duration":220000,"alias":[],"transNames":[]}
"#;
        serde_json::from_str::<Metadata>(json).unwrap();
    }
}
