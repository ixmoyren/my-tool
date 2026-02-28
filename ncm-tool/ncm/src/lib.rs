use snafu::Snafu;

mod audioformat;
mod metadata;
mod ncmfile;

pub use audioformat::AudioFormat;
pub use metadata::Metadata as NcmMetadata;
pub use ncmfile::{DEFAULT_CORE_KEY, DEFAULT_MAGIC, DEFAULT_MODIFY_KEY, NcmFile};

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("{message}"))]
    SerdeJsonOperation {
        message: String,
        source: serde_json::Error,
    },
}
