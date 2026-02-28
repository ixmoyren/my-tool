use snafu::Snafu;

mod audioformat;
mod metadata;
mod ncmfile;

pub use audioformat::AudioFormat;
pub use metadata::Metadata as NcmMetadata;
pub use ncmfile::NcmFile;

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
