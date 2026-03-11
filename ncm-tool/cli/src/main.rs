use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use env_logger::{Builder, Target};
use log::{error, info, warn};
use ncmdump::util::convert;
use snafu::{ErrorCompat, OptionExt, ResultExt, Whatever, whatever};
use walkdir::WalkDir;

#[derive(Parser)]
#[command(
    name = "ncmtool",
    version,
    about = "NCM decryptor & Netease Cloud Music CLI"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Decrypt NCM files to MP3/FLAC
    Dump {
        /// NCM files to convert
        files: Vec<PathBuf>,
        /// Process all NCM files in directory
        #[arg(short, long, value_name = "PATH")]
        directory: Option<PathBuf>,
        /// Recursive directory traversal
        #[arg(short, long)]
        recursive: bool,
        /// Output directory
        #[arg(short, long, value_name = "PATH")]
        output: Option<PathBuf>,
        /// Remove source file after successful conversion
        #[arg(short = 'm', long = "remove")]
        remove: bool,
    },
    /// Set login cookie (`MUSIC_U`)
    Login {
        /// `MUSIC_U` cookie value
        #[arg(required_unless_present = "check")]
        music_u: Option<String>,
        /// Check current login status
        #[arg(long)]
        check: bool,
    },
    /// Clear saved session
    Logout,
    /// Search for tracks, albums, artists, or playlists
    Search {
        /// Search keyword
        keyword: String,
        /// Search type
        #[arg(short = 't', long, default_value = "track")]
        r#type: SearchKind,
        /// Max results
        #[arg(short, long, default_value = "20")]
        limit: u64,
    },
    /// Show track details
    Info {
        /// Track ID
        track_id: u64,
    },
    /// Get track lyrics
    Lyric {
        /// Track ID
        track_id: u64,
    },
    /// Download a track
    Download {
        /// Track ID
        track_id: u64,
        /// Audio quality
        #[arg(short, long, default_value = "exhigh")]
        quality: QualityArg,
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Show playlist details
    Playlist {
        /// Playlist ID
        playlist_id: u64,
    },
    /// Show current user info
    Me,
}

#[derive(Clone, ValueEnum)]
enum SearchKind {
    Track,
    Album,
    Artist,
    Playlist,
}

#[derive(Clone, ValueEnum)]
enum QualityArg {
    Standard,
    Higher,
    Exhigh,
    Lossless,
}

#[derive(Clone, ValueEnum)]
enum BiliFormatArg {
    Mp3,
    Flac,
}

impl From<SearchKind> for ncmapi::types::SearchType {
    fn from(k: SearchKind) -> Self {
        match k {
            SearchKind::Track => Self::Track,
            SearchKind::Album => Self::Album,
            SearchKind::Artist => Self::Artist,
            SearchKind::Playlist => Self::Playlist,
        }
    }
}

impl From<QualityArg> for ncmapi::types::Quality {
    fn from(q: QualityArg) -> Self {
        match q {
            QualityArg::Standard => Self::Standard,
            QualityArg::Higher => Self::Higher,
            QualityArg::Exhigh => Self::Exhigh,
            QualityArg::Lossless => Self::Lossless,
        }
    }
}

fn main() {
    let mut builder = Builder::from_default_env();
    builder.target(Target::Stdout);
    builder.init();

    let cli = Cli::parse();
    let result = match cli.command {
        Command::Dump {
            files,
            directory,
            recursive,
            output,
            remove,
        } => cmd_dump(files, directory, recursive, output, remove),
        Command::Login { music_u, check } => cmd_login(music_u, check),
        Command::Logout => cmd_logout(),
        Command::Search {
            keyword,
            r#type,
            limit,
        } => cmd_search(&keyword, r#type, limit),
        Command::Info { track_id } => cmd_info(track_id),
        Command::Lyric { track_id } => cmd_lyric(track_id),
        Command::Download {
            track_id,
            quality,
            output,
        } => cmd_download(track_id, quality, output),
        Command::Playlist { playlist_id } => cmd_playlist(playlist_id),
        Command::Me => cmd_me(),
    };
    if let Err(error) = result {
        error!("{error}");
        if let Some(bt) = ErrorCompat::backtrace(&error) {
            error!("{bt}");
        }
    }
}

fn cmd_dump(
    mut files: Vec<PathBuf>,
    directory: Option<PathBuf>,
    recursive: bool,
    output: Option<PathBuf>,
    remove: bool,
) -> Result<(), Whatever> {
    if let Some(dir) = directory {
        let walk_dir = if recursive {
            WalkDir::new(dir)
        } else {
            WalkDir::new(dir).max_depth(1)
        };
        for entry in walk_dir.into_iter().filter_map(Result::ok) {
            if entry.path().extension().is_some_and(|e| e == "ncm") {
                files.push(entry.into_path());
            }
        }
    }

    if files.is_empty() {
        whatever!("No NCM files specified. Use --help for usage.");
    }

    let output = output.as_ref();

    for file in &files {
        match convert(file, output) {
            Ok(out) => {
                info!("{} -> {}", file.display(), out.display());
                if remove && let Err(e) = std::fs::remove_file(file) {
                    warn!(
                        "Failed to remove {} while dumping successfully: {e}",
                        file.display()
                    );
                }
            }
            Err(e) => {
                error!("Couldn't dump file({}): {e}", file.display());
                if let Some(bt) = ErrorCompat::backtrace(&e) {
                    error!("{bt}");
                }
            }
        }
    }
    Ok(())
}

fn cmd_login(music_u: Option<String>, check: bool) -> Result<(), Whatever> {
    use ncmapi::auth::Session;

    if check {
        let session = Session::load().with_whatever_context(|_| "Couldn't load the session")?;
        if session.is_logged_in() {
            let client = ncmapi::client::Client::with_session(session)
                .with_whatever_context(|_| "Couldn't create a client when login")?;
            match client.user_info() {
                Ok(ncmapi::types::UserProfile { nickname, id, .. }) => {
                    info!("Logged in as: {nickname} (id={id})")
                }
                Err(e) => info!("Session exists but validation failed: {e}"),
            }
        } else {
            info!("Not logged in.");
        }
        return Ok(());
    }

    let music_u = music_u.whatever_context("MUSIC_U value required")?;
    let session = Session {
        music_u: Some(music_u),
    };
    session
        .save()
        .with_whatever_context(|_| "Couldn't save the session")?;
    info!("Session saved.");
    Ok(())
}

fn cmd_logout() -> Result<(), Whatever> {
    ncmapi::auth::Session::clear()
        .with_whatever_context(|_| "Couldn't create a client when logout")?;
    info!("Session cleared.");
    Ok(())
}

fn cmd_search(keyword: &str, kind: SearchKind, limit: u64) -> Result<(), Whatever> {
    let client = ncmapi::client::Client::new()
        .with_whatever_context(|_| "Couldn't create a client when search")?;
    let search_type = kind.into();
    let result = client
        .search(keyword, search_type, limit, 0)
        .with_whatever_context(|_| "Couldn't search")?;

    info!("Total: {}\n", result.total);

    if let Some(tracks) = &result.tracks {
        for track in tracks {
            let artists: Vec<&str> = track
                .artists
                .iter()
                .map(|artist| artist.name.as_str())
                .collect();
            info!(
                "  [{}] {} - {} ({})",
                track.id,
                artists.join(", "),
                track.name,
                track.album.name,
            );
        }
    }
    if let Some(albums) = &result.albums {
        for album in albums {
            info!("  [{}] {}", album.id, album.name);
        }
    }
    if let Some(artists) = &result.artists {
        for artist in artists {
            info!("  [{}] {}", artist.id, artist.name);
        }
    }
    if let Some(playlists) = &result.playlists {
        for playlist in playlists {
            info!(
                "  [{}] {} ({} tracks)",
                playlist.id, playlist.name, playlist.track_count
            );
        }
    }
    Ok(())
}

fn cmd_info(track_id: u64) -> Result<(), Whatever> {
    let client = ncmapi::client::Client::new()
        .with_whatever_context(|_| "Couldn't create a client when get music info")?;
    let ncmapi::types::Track {
        artists,
        name,
        id,
        album,
        duration,
        ..
    } = client
        .track_detail(track_id)
        .with_whatever_context(|_| "Couldn't get the music info")?;
    let artists: Vec<&str> = artists.iter().map(|artist| artist.name.as_str()).collect();
    info!("Track:    {name} (id={id})");
    info!("Artists:  {}", artists.join(" / "));
    info!("Album:    {} (id={})", album.name, album.id);
    info!(
        "Duration: {}:{:02}",
        duration / 60000,
        (duration / 1000) % 60
    );
    Ok(())
}

fn cmd_lyric(track_id: u64) -> Result<(), Whatever> {
    let client = ncmapi::client::Client::new()
        .with_whatever_context(|_| "Couldn't create a client when get lyric")?;
    let lyric = client
        .track_lyric(track_id)
        .with_whatever_context(|_| "Couldn't get lyric")?;
    if let Some(lrc) = &lyric.lrc {
        info!("{lrc}");
    }
    if let Some(tlyric) = &lyric.tlyric {
        info!("\n--- Translation ---\n{tlyric}");
    }
    if lyric.lrc.is_none() && lyric.tlyric.is_none() {
        info!("No lyrics available.");
    }
    Ok(())
}

fn cmd_download(
    track_id: u64,
    quality: QualityArg,
    output: Option<PathBuf>,
) -> Result<(), Whatever> {
    let client = ncmapi::client::Client::new()
        .with_whatever_context(|_| "Couldn't create a client when download ncm")?;
    let quality = quality.into();

    let dest = if let Some(output) = output {
        output
    } else {
        let url = client
            .track_url(track_id, quality)
            .with_whatever_context(|_| "Couldn't get track url when download ncm")?;
        let ext = if url.contains(".flac") { "flac" } else { "mp3" };
        PathBuf::from(format!("{track_id}.{ext}"))
    };

    let size = client
        .download_track(track_id, quality, &dest)
        .with_whatever_context(|_| "Couldn't download ncm")?;
    info!("Downloaded {} ({} bytes)", dest.display(), size);
    Ok(())
}

fn cmd_playlist(playlist_id: u64) -> Result<(), Whatever> {
    let client = ncmapi::client::Client::new()
        .with_whatever_context(|_| "Couldn't create a client when get playlist")?;
    let ncmapi::types::Playlist {
        name,
        id,
        track_count,
        description,
        creator,
        tracks,
        ..
    } = client
        .playlist_detail(playlist_id)
        .with_whatever_context(|_| "Couldn't get playlist")?;
    info!("Playlist: {name} (id={id})");
    info!("Tracks:   {track_count}");
    if let Some(desc) = &description {
        info!("Desc:     {desc}");
    }
    if let Some(creator) = &creator {
        info!("Creator:  {} (id={})", creator.name, creator.id);
    }
    if let Some(tracks) = &tracks {
        info!("\n");
        for track in tracks {
            let artists: Vec<&str> = track
                .artists
                .iter()
                .map(|artist| artist.name.as_str())
                .collect();
            info!("  [{}] {} - {}", track.id, artists.join(" / "), track.name);
        }
    }
    Ok(())
}

fn cmd_me() -> Result<(), Whatever> {
    let client = ncmapi::client::Client::new()
        .with_whatever_context(|_| "Couldn't create a client when get account info")?;
    let profile = client
        .user_info()
        .with_whatever_context(|_| "Could not read profile by ncm client")?;
    info!("User:   {} (id={})", profile.nickname, profile.id);
    if let Some(url) = &profile.avatar_url {
        info!("Avatar: {url}");
    }
    Ok(())
}
