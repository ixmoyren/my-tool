use crate::{
    Base64DecodeSnafu, GetNcmFileMetadataSnafu, InvalidMagicSnafu, IoOperationSnafu, Result,
    decrypt::{rc4_ksa, rc4_stream_byte},
};
use aes::Aes128Dec;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use cipher::{BlockDecryptMut, KeyInit, block_padding::Pkcs7, generic_array::GenericArray};
use ncmformat::{AudioFormat, NcmFile, NcmMetadata};
use snafu::{ResultExt, ensure};
use std::io::{Read, Seek, SeekFrom, Write};

macro_rules! get_len {
    ($ref:expr, [$init:expr; $len:expr], $len_msg:literal) => {{
        let mut len = [$init; $len];
        $ref.read_exact(&mut len).context(IoOperationSnafu {
            message: $len_msg.to_owned(),
        })?;
        u32::from_le_bytes(len)
    }};
}

macro_rules! get_data {
    ($ref:expr, [$init:expr; $len:expr], $len_msg:literal, $data_msg:literal) => {{
        let len = get_len!($ref, [$init; $len], $len_msg);
        let mut data = vec![0u8; len as usize];
        $ref.read_exact(&mut data).context(IoOperationSnafu {
            message: $data_msg.to_owned(),
        })?;
        data
    }};
    ($ref:expr, $len:expr, $data_msg:literal) => {{
        let mut data = vec![0u8; $len as usize];
        $ref.read_exact(&mut data).context(IoOperationSnafu {
            message: $data_msg.to_owned(),
        })?;
        data
    }};
}

macro_rules! aes_128_ecb_decrypt {
    ($key:expr, $data:expr, $msg:literal) => {{
        let key = GenericArray::from($key);
        Aes128Dec::new(&key)
            .decrypt_padded_vec_mut::<Pkcs7>($data.as_slice())
            .map_err(|e| {
                let msg = concat!($msg, ", error is ");
                crate::Error::Aes128EcbDecryptUnpad {
                    message: format!("{msg}{e}"),
                }
            })?
    }};
}

pub fn decode_with_magic_key<R>(
    reader: &mut R,
    magic: [u8; 8],
    core_key: [u8; 16],
    modify_key: [u8; 16],
) -> Result<NcmFile>
where
    R: Read + Seek,
{
    let ncm_file = NcmFile::default()
        .with_magic(magic)
        .with_core_key(core_key)
        .with_modify_key(modify_key);
    decode_ncm_file(reader, ncm_file)
}

pub fn decode<R>(reader: &mut R) -> Result<NcmFile>
where
    R: Read + Seek,
{
    let ncm_file = NcmFile::default().with_default_key().with_default_magic();
    decode_ncm_file(reader, ncm_file)
}

fn decode_ncm_file<R>(reader: &mut R, mut ncm_file: NcmFile) -> Result<NcmFile>
where
    R: Read + Seek,
{
    let mut magic = [0_u8; 8];
    reader.read_exact(&mut magic).context(IoOperationSnafu {
        message: "Couldn't read magic".to_owned(),
    })?;
    ensure!(magic == ncm_file.magic, InvalidMagicSnafu);

    reader
        .seek(SeekFrom::Current(2))
        .context(IoOperationSnafu {
            message: "Failed to seek 2 byte".to_owned(),
        })?;

    let mut key_data = get_data!(
        reader,
        [0_u8; 4],
        "Couldn't read key len",
        "Couldn't read key data"
    );
    for byte in &mut key_data {
        *byte ^= 0x64;
    }
    let key_decrypted =
        aes_128_ecb_decrypt!(ncm_file.core_key, key_data, "Couldn't decrypt the key");
    ncm_file.key_box = rc4_ksa(&key_decrypted[17..]);

    let meta_len = get_len!(reader, [0_u8; 4], "Couldn't read metadata len");
    if meta_len > 0 {
        let meta_data = get_data!(reader, meta_len, "Couldn't read meta data");
        let meta_data = BASE64.decode(&meta_data[22..]).context(Base64DecodeSnafu {
            message: "Couldn't decode metadata",
        })?;
        let meta_decrypted = aes_128_ecb_decrypt!(
            ncm_file.modify_key,
            meta_data,
            "Couldn't decrypt the metadata"
        );
        ncm_file.metadata =
            Some(NcmMetadata::from_slice(&meta_decrypted[6..]).context(GetNcmFileMetadataSnafu)?)
    };

    reader
        .seek(SeekFrom::Current(5))
        .context(IoOperationSnafu {
            message: "Failed to seek crc + image version".to_owned(),
        })?;

    let cover_frame_len = get_len!(reader, [0u8; 4], "Couldn't read cover frame len");
    let image_size = get_len!(reader, [0u8; 4], "Couldn't read image size");
    if image_size > 0 {
        let img = get_data!(reader, image_size, "Couldn't read image");
        let padding = cover_frame_len as i64 - image_size as i64;
        if padding > 0 {
            reader
                .seek(SeekFrom::Current(padding))
                .context(IoOperationSnafu {
                    message: "Failed to seek padding after image".to_owned(),
                })?;
        }
        ncm_file.cover_image = Some(img);
    } else {
        if cover_frame_len > 0 {
            reader
                .seek(SeekFrom::Current(cover_frame_len as i64))
                .context(IoOperationSnafu {
                    message: "Failed to seek cover frame".to_owned(),
                })?;
        }
    };

    ncm_file.audio_offset = reader.stream_position().context(IoOperationSnafu {
        message: "Failed to get audio offset".to_owned(),
    })?;

    let mut header = get_data!(reader, 3, "Couldn't read header msg");
    for (i, byte) in header.iter_mut().enumerate() {
        *byte ^= rc4_stream_byte(&ncm_file.key_box, i);
    }
    ncm_file.format = if header == [0x49, 0x44, 0x33] {
        AudioFormat::Mp3
    } else {
        AudioFormat::Flac
    };

    Ok(ncm_file)
}

trait DumpAudio {
    fn dump_audio<R, W>(&self, r: &mut R, w: &mut W) -> Result<()>
    where
        R: Read + Seek,
        W: Write;
}

impl DumpAudio for NcmFile {
    fn dump_audio<R, W>(&self, r: &mut R, w: &mut W) -> Result<()>
    where
        R: Read + Seek,
        W: Write,
    {
        r.seek(SeekFrom::Start(self.audio_offset))
            .context(IoOperationSnafu {
                message: "Failed to seek audio".to_owned(),
            })?;

        let mut buf = vec![0u8; 0x8000];
        let mut offset = 0usize;

        loop {
            let n = r.read(&mut buf).context(IoOperationSnafu {
                message: "Couldn't read the audio data to buf",
            })?;
            if n == 0 {
                break;
            }
            for (i, byte) in buf[..n].iter_mut().enumerate() {
                *byte ^= rc4_stream_byte(&self.key_box, offset + i);
            }
            w.write_all(&buf[..n]).context(IoOperationSnafu {
                message: "Couldn't write the audio data",
            })?;
            offset += n;
        }

        Ok(())
    }
}
