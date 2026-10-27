use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use lofty::picture::PictureType;
use lofty::prelude::*;
use lofty::probe::Probe;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub modified_unix_ms: u128,
    pub cover_path: Option<PathBuf>,
    #[serde(default)]
    pub album_artist: String,
    #[serde(default)]
    pub track_number: u32,
    #[serde(default)]
    pub disc_number: u32,
    #[serde(default)]
    pub year: Option<u32>,
    #[serde(default)]
    pub file_size: u64,
    #[serde(default)]
    pub added_unix_ms: u128,
    #[serde(default)]
    pub artwork_version: u8,
}

#[derive(Debug, Default)]
pub struct Library {
    pub tracks: Vec<Track>,
    pub changed: bool,
    pub skipped_files: usize,
}

#[derive(Debug, Error)]
pub enum LibraryError {
    #[error("cannot read file metadata: {0}")]
    Io(#[from] std::io::Error),
    #[error("cannot access library: {0}")]
    Walk(#[from] walkdir::Error),
}

impl Library {
    pub fn scan(root: impl AsRef<Path>) -> Result<Self, LibraryError> {
        let root = root.as_ref().to_path_buf();
        let metadata = std::fs::metadata(&root)?;
        if !metadata.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotADirectory,
                "Library folder is unavailable",
            )
            .into());
        }
        std::fs::read_dir(&root)?;
        let cached: HashMap<PathBuf, Track> = crate::cache::load(&root)
            .unwrap_or_default()
            .into_iter()
            .map(|track| (track.path.clone(), track))
            .collect();
        let mut skipped_files = 0;
        let mut changed = false;
        let mut tracks = Vec::new();
        let mut folder_covers = HashMap::new();
        for entry in WalkDir::new(&root).follow_links(false).into_iter() {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.into_path();
            if !is_supported_audio(&path) {
                continue;
            }
            let file = std::fs::metadata(&path)?;
            let modified_unix_ms = file
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_millis());
            let previous = cached.get(&path);
            if let Some(track) = previous
                && track.artwork_version == 1
                && track.file_size == file.len()
                && track.modified_unix_ms == modified_unix_ms
                && track
                    .cover_path
                    .as_ref()
                    .is_none_or(|cover| cover.is_file())
            {
                let mut track = track.clone();
                if track.cover_path.is_none()
                    && let Some(directory) = path.parent()
                {
                    track.cover_path = folder_covers
                        .entry(directory.to_path_buf())
                        .or_insert_with(|| find_folder_cover(directory))
                        .clone();
                    changed |= track.cover_path.is_some();
                }
                tracks.push(track);
                continue;
            }
            let Ok(tagged) = Probe::open(&path).and_then(Probe::read) else {
                skipped_files += 1;
                continue;
            };
            changed = true;
            let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
            let embedded_cover = tag
                .and_then(|tag| {
                    tag.pictures()
                        .iter()
                        .find(|picture| picture.pic_type() == PictureType::CoverFront)
                        .or_else(|| tag.pictures().first())
                        .map(|picture| cache_artwork(picture.data()))
                })
                .flatten();
            let cover_path = embedded_cover.or_else(|| {
                let directory = path.parent()?.to_path_buf();
                folder_covers
                    .entry(directory.clone())
                    .or_insert_with(|| find_folder_cover(&directory))
                    .clone()
            });
            let properties = tagged.properties();
            let fallback = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown track")
                .to_owned();
            tracks.push(Track {
                path,
                title: tag
                    .and_then(|t| t.title())
                    .map(|s| s.to_string())
                    .unwrap_or(fallback),
                artist: tag
                    .and_then(|t| t.artist())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "Unknown artist".into()),
                album: tag
                    .and_then(|t| t.album())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "Unknown album".into()),
                duration_ms: properties.duration().as_millis() as u64,
                modified_unix_ms,
                cover_path,
                album_artist: tag
                    .and_then(|t| t.get_string(&ItemKey::AlbumArtist))
                    .unwrap_or_default()
                    .to_owned(),
                track_number: tag.and_then(|t| t.track()).unwrap_or_default(),
                disc_number: tag.and_then(|t| t.disk()).unwrap_or(1),
                year: tag.and_then(|t| t.year()),
                file_size: file.len(),
                artwork_version: 1,
                added_unix_ms: previous
                    .map(|track| track.added_unix_ms)
                    .filter(|value| *value != 0)
                    .unwrap_or_else(|| {
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis()
                    }),
            });
        }
        tracks.sort_by_key(|track| track.modified_unix_ms);
        changed |= tracks.len() != cached.len();
        Ok(Self {
            tracks,
            changed,
            skipped_files,
        })
    }
}

fn cache_artwork(bytes: &[u8]) -> Option<PathBuf> {
    use std::collections::hash_map::DefaultHasher;

    let directory = dirs::cache_dir()?.join("flacpac-rusty").join("artwork");
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let path = directory.join(format!("{:016x}-384.png", hasher.finish()));
    if path.is_file() {
        return Some(path);
    }
    std::fs::create_dir_all(&directory).ok()?;
    if let Ok(image) = image::load_from_memory(bytes) {
        let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
        image
            .thumbnail(384, 384)
            .save_with_format(&temporary, image::ImageFormat::Png)
            .ok()?;
        std::fs::rename(temporary, &path).ok()?;
    } else {
        // Preserve unusual formats supported by the UI's image loader.
        let path = directory.join(format!("{:016x}.img", hasher.finish()));
        std::fs::write(&path, bytes).ok()?;
        return Some(path);
    }
    Some(path)
}

fn find_folder_cover(directory: &Path) -> Option<PathBuf> {
    const NAMES: &[&str] = &[
        "cover.jpg",
        "cover.jpeg",
        "cover.png",
        "cover.webp",
        "folder.jpg",
        "folder.png",
        "front.jpg",
        "front.png",
        "album.jpg",
        "album.png",
    ];
    for name in NAMES {
        let image_path = directory.join(name);
        if let Ok(bytes) = std::fs::read(&image_path)
            && let Some(cached_path) = cache_artwork(&bytes)
        {
            return Some(cached_path);
        }
    }
    None
}

fn is_supported_audio(path: &Path) -> bool {
    // AppleDouble sidecars carry metadata, even when their extension matches audio.
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("._"))
    {
        return false;
    }
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "flac" | "wav" | "wave" | "mp3" | "m4a" | "mp4" | "ogg" | "oga" | "aiff" | "aif"
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn apple_double_sidecars_are_not_audio_files() {
        assert!(!is_supported_audio(Path::new("/music/._song.flac")));
        assert!(is_supported_audio(Path::new("/music/song.FLAC")));
        assert!(is_supported_audio(Path::new("/music/song.aiff")));
        assert!(is_supported_audio(Path::new("/music/song.m4a")));
        for extension in ["aac", "opus", "ape", "wv", "mpc"] {
            assert!(!is_supported_audio(Path::new(&format!("song.{extension}"))));
        }
    }
}
