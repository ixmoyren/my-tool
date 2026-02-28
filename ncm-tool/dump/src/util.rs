use crate::{IoOperationSnafu, Result, decode::decode, dump::DumpAudio};
use snafu::ResultExt;
use std::{
    env,
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

pub fn convert(input: impl AsRef<Path>, output: Option<impl AsRef<Path>>) -> Result<PathBuf> {
    convert_with_extension(input, output, "ncm")
}

pub fn convert_with_extension(
    input: impl AsRef<Path>,
    output: Option<impl AsRef<Path>>,
    extension: impl AsRef<str>,
) -> Result<PathBuf> {
    let input = input.as_ref();
    if input.is_file() {
        covert_ncm_file(input, output)
    } else {
        let output = if let Some(output) = output {
            let output = output.as_ref();
            if output.is_dir() {
                output.to_path_buf()
            } else {
                output
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or(env::current_dir().context(IoOperationSnafu {
                        message: "Failed to get current dir when deal with output",
                    })?)
            }
        } else {
            input.to_path_buf()
        };
        WalkDir::new(input)
            .into_iter()
            .filter_map(Result::ok)
            .for_each(|e| {
                if e.path().is_file() {
                    if let Some(ext) = e.path().extension()
                        && ext != extension.as_ref()
                    {
                        return;
                    }
                    match covert_ncm_file(e.path(), Some(&output)) {
                        Ok(path) => {
                            log::debug!("{}", path.display())
                        }
                        Err(err) => {
                            log::error!("{err}");
                        }
                    }
                }
            });

        Ok(output)
    }
}

fn covert_ncm_file(input: &Path, output: Option<impl AsRef<Path>>) -> Result<PathBuf> {
    let input_file = File::open(input).context(IoOperationSnafu {
        message: format!("Failed to open the file({})", input.display()),
    })?;
    let mut reader = BufReader::new(input_file);
    let ncm_file = decode(&mut reader)?;
    let stem = input.file_stem().unwrap_or_default();
    let ext = ncm_file.format.extension();
    let (out_file, output) = if let Some(output) = output {
        let output = output.as_ref();
        if output.is_dir() {
            let output = output.join(format!("{}.{ext}", stem.to_string_lossy()));
            (
                File::create(&output).context(IoOperationSnafu {
                    message: format!("Failed to create a file({})", output.display()),
                })?,
                output,
            )
        } else {
            let out_file = if output.exists() {
                OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(output)
                    .context(IoOperationSnafu {
                        message: format!("Failed to open a file({})", output.display()),
                    })?
            } else {
                File::create(output).context(IoOperationSnafu {
                    message: format!("Failed to create a file({})", output.display()),
                })?
            };
            (out_file, output.to_path_buf())
        }
    } else {
        let out = input
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or(env::current_dir().context(IoOperationSnafu {
                message: "Failed to get current dir",
            })?);
        let output = out.join(format!("{}.{ext}", stem.to_string_lossy()));
        (
            File::create(&output).context(IoOperationSnafu {
                message: format!("Failed to create a file({})", output.display()),
            })?,
            output,
        )
    };

    let mut writer = BufWriter::new(out_file);
    ncm_file.dump_audio(&mut reader, &mut writer)?;
    ncm_file.write_tag(&output)?;

    Ok(output)
}
