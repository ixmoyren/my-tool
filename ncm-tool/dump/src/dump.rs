use crate::{Error::LoftyNoSupport, IoOperationSnafu, LoftySnafu, decrypt::rc4_stream_byte};
use lofty::{
    config::WriteOptions,
    file::TaggedFileExt,
    picture::{MimeType, Picture, PictureType},
    prelude::{Accessor, TagExt},
    probe::Probe,
    tag::Tag,
};
use ncmformat::{NcmFile, NcmMetadata};
use snafu::ResultExt;
use std::{
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

pub trait DumpAudio {
    fn dump_audio<R, W>(&self, r: &mut R, w: &mut W) -> crate::Result<()>
    where
        R: Read + Seek,
        W: Write;

    fn write_tag(self, path: &Path) -> crate::Result<()>;
}

impl DumpAudio for NcmFile {
    fn dump_audio<R, W>(&self, r: &mut R, w: &mut W) -> crate::Result<()>
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

    fn write_tag(self, path: &Path) -> crate::Result<()> {
        let Some(metadata) = self.metadata else {
            return Ok(());
        };
        let artist = metadata.artist_names();
        let NcmMetadata {
            music_name: title,
            album,
            ..
        } = metadata;
        let mut tagged_file = Probe::open(path)
            .context(LoftySnafu {
                message: format!("Failed to open the audio file({})", path.display()),
            })?
            .read()
            .context(LoftySnafu {
                message: format!("Failed to read the audio file({})", path.display()),
            })?;

        let tag = if let Some(tag) = tagged_file.primary_tag_mut() {
            tag
        } else if let Some(tag) = tagged_file.first_tag_mut() {
            tag
        } else {
            let tag_type = tagged_file.primary_tag_type();
            tagged_file.insert_tag(Tag::new(tag_type));
            tagged_file.primary_tag_mut().ok_or(LoftyNoSupport {
                message: "Failed to insert tag by lofty".to_owned(),
            })?
        };

        tag.set_title(title);
        tag.set_artist(artist);
        tag.set_album(album);

        if let Some(img_data) = self.cover_image {
            let mime = if img_data.starts_with(&PNG_MAGIC) {
                MimeType::Png
            } else {
                MimeType::Jpeg
            };
            let pic = Picture::unchecked(img_data.to_vec())
                .pic_type(PictureType::CoverFront)
                .mime_type(mime)
                .build();
            tag.push_picture(pic);
        } // Todo 音乐图片可以通过网络下载
        tag.save_to_path(path, WriteOptions::default())
            .context(LoftySnafu {
                message: format!("Failed to save tag to file({})", path.display()),
            })?;

        Ok(())
    }
}
