use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};

use super::Track;

#[derive(Clone, Debug)]
pub struct Album {
    pub title: String,
    pub artist: String,
    pub cover_path: Option<std::path::PathBuf>,
    pub track_indices: Arc<Vec<usize>>,
    pub duration_ms: u64,
    pub year: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct Artist {
    pub name: String,
    pub cover_path: Option<std::path::PathBuf>,
    pub album_count: usize,
    pub track_indices: Arc<Vec<usize>>,
    pub album_indices: Arc<Vec<usize>>,
    pub duration_ms: u64,
}

pub fn build_catalog(tracks: &[Track]) -> (Vec<Album>, Vec<Artist>) {
    let mut album_tracks: HashMap<(String, String), Vec<usize>> = HashMap::new();
    let mut artist_tracks: HashMap<String, Vec<usize>> = HashMap::new();

    for (index, track) in tracks.iter().enumerate() {
        let lead_artist = album_artist(track);
        album_tracks
            .entry((track.album.clone(), lead_artist))
            .or_default()
            .push(index);
        for artist in split_artist_credits(&track.artist)
            .into_iter()
            .chain(split_artist_credits(&track.album_artist))
        {
            let indices = artist_tracks.entry(artist).or_default();
            if indices.last() != Some(&index) {
                indices.push(index);
            }
        }
    }

    let mut albums: Vec<Album> = album_tracks
        .into_iter()
        .map(|((title, artist), mut track_indices)| {
            track_indices
                .sort_by_key(|index| (tracks[*index].disc_number, tracks[*index].track_number));
            Album {
                cover_path: track_indices
                    .iter()
                    .find_map(|index| tracks[*index].cover_path.clone()),
                duration_ms: track_indices
                    .iter()
                    .map(|index| tracks[*index].duration_ms)
                    .sum(),
                year: track_indices.iter().find_map(|index| tracks[*index].year),
                title,
                artist,
                track_indices: Arc::new(track_indices),
            }
        })
        .collect();
    albums.sort_by_cached_key(|album| (album.title.to_lowercase(), album.artist.to_lowercase()));

    let track_albums: HashMap<usize, usize> = albums
        .iter()
        .enumerate()
        .flat_map(|(album_index, album)| {
            album
                .track_indices
                .iter()
                .map(move |index| (*index, album_index))
        })
        .collect();
    let mut artists: Vec<Artist> = artist_tracks
        .into_iter()
        .map(|(name, mut track_indices)| {
            track_indices.sort_by_key(|index| {
                (
                    tracks[*index].album.to_lowercase(),
                    tracks[*index].disc_number,
                    tracks[*index].track_number,
                )
            });
            let album_indices: Vec<usize> = track_indices
                .iter()
                .filter_map(|index| track_albums.get(index).copied())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let album_count = album_indices.len();
            Artist {
                cover_path: track_indices
                    .iter()
                    .find_map(|index| tracks[*index].cover_path.clone()),
                duration_ms: track_indices
                    .iter()
                    .map(|index| tracks[*index].duration_ms)
                    .sum(),
                album_indices: Arc::new(album_indices),
                name,
                album_count,
                track_indices: Arc::new(track_indices),
            }
        })
        .collect();
    artists.sort_by_cached_key(|artist| artist.name.to_lowercase());

    (albums, artists)
}

pub fn album_artist(track: &Track) -> String {
    if track.album_artist.trim().is_empty() {
        split_artist_credits(&track.artist)
            .into_iter()
            .next()
            .unwrap_or_else(|| "Unknown artist".into())
    } else {
        split_artist_credits(&track.album_artist).join(" · ")
    }
}

/// Splits common multi-artist credits into individual catalog entries while
/// leaving the original credit untouched on each track.
pub(crate) fn split_artist_credits(credit: &str) -> Vec<String> {
    credit
        .split(';')
        .flat_map(|part| {
            let lower = part.to_ascii_lowercase();
            let mut result = Vec::new();
            let mut start = 0;
            let mut cursor = 0;
            while cursor < lower.len() {
                let rest = &lower[cursor..];
                let marker = [" feat. ", " feat ", " featuring ", " ft. ", " ft "]
                    .into_iter()
                    .find(|marker| rest.starts_with(marker));
                if let Some(marker) = marker {
                    push_artist(&mut result, &part[start..cursor]);
                    cursor += marker.len();
                    start = cursor;
                } else {
                    cursor += lower[cursor..].chars().next().unwrap().len_utf8();
                }
            }
            push_artist(&mut result, &part[start..]);
            result
        })
        .collect()
}

fn push_artist(artists: &mut Vec<String>, value: &str) {
    let name = value.trim();
    if !name.is_empty()
        && !artists
            .iter()
            .any(|artist| artist.eq_ignore_ascii_case(name))
    {
        artists.push(name.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn track(artist: &str, album_artist: &str, number: u32, disc: u32) -> Track {
        Track {
            artist: artist.into(),
            album_artist: album_artist.into(),
            album: "BALLADS 1".into(),
            track_number: number,
            disc_number: disc,
            ..Track::default()
        }
    }
    #[test]
    fn guests_share_an_album_and_have_individual_artist_pages() {
        let tracks = vec![
            track("Joji", "Joji", 3, 1),
            track("Joji; Clams Casino", "Joji", 1, 1),
            track("Joji; Shlohmo; D33J", "Joji", 2, 1),
        ];
        let (albums, artists) = build_catalog(&tracks);
        assert_eq!(albums.len(), 1);
        assert_eq!(albums[0].track_indices.as_slice(), &[1, 2, 0]);
        let joji = artists.iter().find(|artist| artist.name == "Joji").unwrap();
        assert_eq!(joji.track_indices.len(), 3);
        assert_eq!(joji.album_count, 1);
        assert!(artists.iter().any(|artist| artist.name == "Clams Casino"));
        assert!(!artists.iter().any(|artist| artist.name.contains(';')));
    }
    #[test]
    fn album_artist_distinguishes_releases_and_disc_order_is_preserved() {
        let tracks = vec![
            track("Guest A", "Various Artists", 1, 2),
            track("Guest B", "Various Artists", 7, 1),
            track("Someone else", "Other artist", 1, 1),
        ];
        let (albums, _) = build_catalog(&tracks);
        assert_eq!(albums.len(), 2);
        let compilation = albums
            .iter()
            .find(|album| album.artist == "Various Artists")
            .unwrap();
        assert_eq!(compilation.track_indices.as_slice(), &[1, 0]);
    }
    #[test]
    fn feature_credits_preserve_unicode_and_band_names() {
        assert_eq!(
            split_artist_credits("Björk feat. Arca; Sigur Rós"),
            vec!["Björk", "Arca", "Sigur Rós"]
        );
        assert_eq!(
            split_artist_credits("Earth, Wind & Fire"),
            vec!["Earth, Wind & Fire"]
        );
    }
}
