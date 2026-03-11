use crate::{Error::GetCacheDir, IoOperationSnafu, Result, SerdeJsonOperationSnafu};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use std::{fs, path::PathBuf};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Session {
    #[serde(rename = "MUSIC_U")]
    pub music_u: Option<String>,
}

impl Session {
    pub fn load() -> Result<Self> {
        let session_json = Self::session_path()?.join("session.json");
        if !session_json.exists() {
            return Ok(Self::default());
        }
        let data = fs::read_to_string(&session_json).context(IoOperationSnafu {
            message: "Couldn't read the session file".to_owned(),
        })?;
        serde_json::from_str(&data).context(SerdeJsonOperationSnafu {
            message: "The json of this file is illegal".to_owned(),
        })
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::session_path()?;
        if !path.exists() {
            fs::create_dir_all(&path).context(IoOperationSnafu {
                message: "The directory for saving the session cannot be created".to_owned(),
            })?
        }
        let data = serde_json::to_string_pretty(self).context(SerdeJsonOperationSnafu {
            message: "Failed to serialize the session object into json".to_owned(),
        })?;
        let session_json = path.join("session.json");
        fs::write(&session_json, data).context(IoOperationSnafu {
            message: "The json cannot be written to session.json".to_owned(),
        })?;
        Ok(())
    }

    pub fn clear() -> Result<()> {
        let session_json = Self::session_path()?.join("session.json");
        if session_json.exists() {
            fs::remove_file(&session_json).context(IoOperationSnafu {
                message: "Failed to delete session.json".to_owned(),
            })?;
        }
        Ok(())
    }

    #[inline]
    pub fn cookie_header(&self) -> Option<String> {
        self.music_u
            .as_deref()
            .map(|music_u| format!("os=pc; __remember_me=true; MUSIC_U={music_u}"))
    }

    #[inline]
    pub fn is_logged_in(&self) -> bool {
        self.music_u
            .as_ref()
            .is_some_and(|music_u| !music_u.is_empty())
    }

    #[inline]
    fn session_path() -> Result<PathBuf> {
        let cache = dirs::cache_dir().ok_or(GetCacheDir)?;
        Ok(cache.join("ncmdump"))
    }
}
