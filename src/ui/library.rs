use crate::library::{Album, Artist, Library, Track, build_catalog};
use std::{path::PathBuf, sync::Arc};

pub struct LibrarySnapshot {
    pub(crate) skipped_files: usize,
    pub(crate) tracks: Arc<Vec<Track>>,
    pub(crate) albums: Arc<Vec<Album>>,
    pub(crate) artists: Arc<Vec<Artist>>,
    pub(crate) search_terms: Arc<Vec<String>>,
    pub(crate) date_order: Arc<Vec<usize>>,
    pub(crate) title_order: Arc<Vec<usize>>,
    pub(crate) artist_order: Arc<Vec<usize>>,
    pub(crate) album_order: Arc<Vec<usize>>,
    pub(crate) duration_order: Arc<Vec<usize>>,
}
impl LibrarySnapshot {
    pub fn new(tracks: Vec<Track>) -> Self {
        let (albums, artists) = build_catalog(&tracks);
        let search_terms = tracks
            .iter()
            .map(|track| {
                format!(
                    "{} {} {} {}",
                    track.title, track.artist, track.album, track.album_artist
                )
                .to_lowercase()
            })
            .collect();
        let mut date: Vec<usize> = (0..tracks.len()).collect();
        date.sort_by_key(|index| std::cmp::Reverse(tracks[*index].added_unix_ms));
        let mut title: Vec<usize> = (0..tracks.len()).collect();
        title.sort_by_cached_key(|index| tracks[*index].title.to_lowercase());
        let mut artist: Vec<usize> = (0..tracks.len()).collect();
        artist.sort_by_cached_key(|index| tracks[*index].artist.to_lowercase());
        let mut album: Vec<usize> = (0..tracks.len()).collect();
        album.sort_by_cached_key(|index| tracks[*index].album.to_lowercase());
        let mut duration: Vec<usize> = (0..tracks.len()).collect();
        duration.sort_by_key(|index| tracks[*index].duration_ms);
        Self {
            skipped_files: 0,
            tracks: Arc::new(tracks),
            albums: Arc::new(albums),
            artists: Arc::new(artists),
            search_terms: Arc::new(search_terms),
            date_order: Arc::new(date),
            title_order: Arc::new(title),
            artist_order: Arc::new(artist),
            album_order: Arc::new(album),
            duration_order: Arc::new(duration),
        }
    }
}
pub(crate) fn scan_and_cache(
    root: PathBuf,
) -> Result<(LibrarySnapshot, Result<(), crate::cache::CacheError>), crate::library::LibraryError> {
    let library = Library::scan(&root)?;
    let saved = if library.changed {
        crate::cache::replace(&root, &library.tracks)
    } else {
        Ok(())
    };
    let mut snapshot = LibrarySnapshot::new(library.tracks);
    snapshot.skipped_files = library.skipped_files;
    Ok((snapshot, saved))
}
