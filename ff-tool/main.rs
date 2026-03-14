use clap::{Parser, ValueEnum};
use env_logger::{Builder, Target};
use indicatif::{ProgressBar, ProgressStyle};
use lzma_rust2::XzReader;
use reqwest::blocking::Client;
use snafu::{ResultExt, Whatever, ensure_whatever};
use std::{
    fs, io,
    io::Read,
    path::{Path, PathBuf},
};
use tar::Archive;

macro_rules! print_then_return {
    ($error: expr) => {{
        let err = $error;
        ::log::error!("{err}");
        if let Some(bt) = ::snafu::ErrorCompat::backtrace(&err) {
            ::log::error!("{bt}");
        }
        return;
    }};
}

#[derive(Parser)]
#[command(
    name = "fftool",
    version,
    about = "A small tool for updating the firefox browser"
)]
struct Cli {
    #[arg(value_enum)]
    browser_type: BrowserType,
    #[arg(short, long, value_name = "/opt/mozilla")]
    install: PathBuf,
    #[arg(short, long, default_value_t = false)]
    backup: bool,
    #[arg(long, default_value_t = false)]
    clean_backup: bool,
}

#[derive(Copy, Clone, PartialEq, Default, ValueEnum)]
enum BrowserType {
    #[default]
    Firefox,
    Zen,
}

impl BrowserType {
    pub fn url(&self) -> &'static str {
        match self {
            Self::Firefox => {
                "https://download.mozilla.org/?product=firefox-nightly-latest-ssl&os=linux64&lang=en-US"
            }
            Self::Zen => {
                "https://github.com/zen-browser/desktop/releases/download/twilight-1/zen.linux-x86_64.tar.xz"
            }
        }
    }

    pub fn install_dir(&self) -> &'static str {
        match self {
            Self::Firefox => "firefox",
            Self::Zen => "zen",
        }
    }

    pub fn back_dir(&self) -> &'static str {
        match self {
            Self::Firefox => "firefox-back",
            Self::Zen => "zen-back",
        }
    }
}
pub struct ProgressReader<R> {
    inner: R,
    progress: ProgressBar,
}

impl<R> ProgressReader<R>
where
    R: Read,
{
    pub fn new(inner: R, progress: ProgressBar) -> Self {
        Self { inner, progress }
    }
}

impl<R> Read for ProgressReader<R>
where
    R: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        if n > 0 {
            self.progress.inc(n as u64);
        }
        Ok(n)
    }
}
fn main() {
    let mut builder = Builder::from_default_env();
    builder.target(Target::Stdout);
    builder.init();

    let Cli {
        browser_type: browser,
        install,
        backup: is_backup,
        clean_backup: is_clean_backup,
    } = Cli::parse();

    let (install, back) = match get_install_back(&browser, install) {
        Ok(t) => t,
        Err(error) => print_then_return!(error),
    };

    if is_clean_backup {
        if let Err(error) = clean_backup(&back) {
            print_then_return!(error);
        }
    } else {
        if let Err(error) = if is_backup {
            backup(&install, &back)
        } else {
            update(&browser, &install, &back)
        } {
            print_then_return!(error);
        }
    }
}

fn clean_backup(back: &Path) -> Result<(), Whatever> {
    if back.exists() {
        fs::remove_dir_all(back).with_whatever_context(|_| {
            format!("Couldn't remove the backup dir({})", back.display())
        })?;
    }
    Ok(())
}

fn backup(install: &Path, back: &Path) -> Result<(), Whatever> {
    if install.exists() && back.exists() {
        fs::remove_dir_all(install).with_whatever_context(|_| {
            format!("Couldn't remove the backup dir({})", back.display())
        })?;
        fs::rename(back, install).with_whatever_context(|_| {
            format!("Couldn't backup the install dir({})", install.display())
        })?;
    } else if !install.exists() && back.exists() {
        fs::rename(back, install).with_whatever_context(|_| {
            format!("Couldn't backup the install dir({})", install.display())
        })?;
    }
    Ok(())
}

fn update(browser: &BrowserType, install: &Path, back: &Path) -> Result<(), Whatever> {
    if back.exists() {
        fs::remove_dir_all(back).with_whatever_context(|_| {
            format!("Couldn't remove the backup dir({})", back.display())
        })?;
    }

    if install.exists() {
        fs::rename(install, back).with_whatever_context(|_| {
            format!("Couldn't backup the install dir({})", install.display())
        })?;
    }

    let install = if let Some(parent) = install.parent() {
        parent
    } else {
        install
    };

    if !install.exists() {
        fs::create_dir_all(install).with_whatever_context(|_| {
            format!("Couldn't create the install dir({})", install.display())
        })?;
    }

    let client = Client::new();
    let response = client
        .get(browser.url())
        .send()
        .with_whatever_context(|_| format!("Couldn't get the browser({})", browser.url()))?;

    ensure_whatever!(
        response.status().is_success(),
        "Failed to download the browser, status code is {}",
        response.status()
    );

    let total_size = response
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok());

    let pb = if let Some(size) = total_size {
        ProgressBar::new(size)
    } else {
        ProgressBar::new_spinner()
    };
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message(format!("Downloading & extracting {}", browser.url()));

    let lzma_reader = XzReader::new(response, true);

    let reader = ProgressReader::new(lzma_reader, pb.clone());

    let mut archive = Archive::new(reader);
    archive
        .unpack(install)
        .with_whatever_context(|_| "Couldn't unpack the archive file")?;

    Ok(())
}

fn get_install_back(
    browser: &BrowserType,
    install: PathBuf,
) -> Result<(PathBuf, PathBuf), Whatever> {
    ensure_whatever!(
        !install.exists() || install.is_dir(),
        "The install path is not allowed to be a file"
    );

    if !install.ends_with(browser.install_dir()) {
        Ok((
            install.join(browser.install_dir()),
            install.join(browser.back_dir()),
        ))
    } else {
        let back = if let Some(install_parent) = install.parent() {
            install_parent.join(browser.back_dir())
        } else {
            install.join(browser.back_dir())
        };
        Ok((install, back))
    }
}
