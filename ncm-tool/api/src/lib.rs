use snafu::Snafu;

mod auth;
mod client;
mod crypto;
mod playlist;
mod search;
mod track;
mod types;
mod user;

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("API error (code {code}): {message}"))]
    Api { code: i64, message: String },
    #[snafu(display("Missing songs"))]
    MissSong,
    #[snafu(display("Not logged in"))]
    NotLoggedIn,
    #[snafu(display("Track not found: {id}"))]
    TrackNotFound { id: u64 },
    #[snafu(display("Track unavailable (no copyright or VIP required)"))]
    TrackUnavailable,
    #[snafu(display("Couldn't determine cache directory"))]
    GetCacheDir,
    #[snafu(display("{message}"))]
    IoOperation {
        message: String,
        source: std::io::Error,
    },
    #[snafu(display("{message}"))]
    RequestOperation {
        message: String,
        source: reqwest::Error,
    },
    #[snafu(display("{message}"))]
    SerdeJsonOperation {
        message: String,
        source: serde_json::Error,
    },
}
