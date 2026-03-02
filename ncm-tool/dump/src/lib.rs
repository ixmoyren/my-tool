use snafu::Snafu;

pub mod decode;
mod decrypt;
pub mod dump;
pub mod util;

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("Failed to obtain the metadata of the ncmfile"))]
    GetNcmFileMetadata { source: ncmformat::Error },
    #[snafu(display("Not a valid NCM file (bad magic)"))]
    InvalidMagic,
    #[snafu(display("Decryption failed, {message}"))]
    Aes128EcbDecryptUnpad { message: String },
    #[snafu(display("Decode failed, {message}"))]
    Base64Decode {
        message: String,
        source: base64::DecodeError,
    },
    #[snafu(display("{message}, in the lofty probe"))]
    Lofty {
        message: String,
        source: lofty::error::LoftyError,
    },
    #[snafu(display("{message}, in the lofty probe"))]
    LoftyNoSupport { message: String },
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
}
