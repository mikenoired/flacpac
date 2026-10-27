//! Local listening history. Playback supplies elapsed *playing* time, never seek offsets.
use crate::library::{Track, album_artist, build_catalog};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::mpsc, thread, time::Duration};

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Entry {
    title: String,
    artist: String,
    artists: Vec<String>,
    album: String,
    album_artist: String,
    cover_path: Option<PathBuf>,
    listened_ms: u64,
    plays: u64,
}

type History = HashMap<PathBuf, Entry>;
type WriteRequest = (History, Option<mpsc::Sender<()>>);

#[derive(Clone, Debug)]
pub struct Ranking {
    pub title: String,
    pub subtitle: String,
    pub cover_path: Option<PathBuf>,
    pub listened_ms: u64,
    pub plays: u64,
}

#[derive(Default)]
pub struct Statistics {
    history: History,
    session_path: Option<PathBuf>,
    session_ms: u64,
    session_counted: bool,
    unsaved_ms: u64,
    writer: Option<mpsc::Sender<WriteRequest>>,
}

impl Statistics {
    pub fn load() -> Self {
        let path =
            dirs::data_local_dir().map(|base| base.join("flacpac-rusty/listening-history.json"));
        let history = path
            .as_ref()
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        let writer = path.map(|path| {
            let (sender, receiver) = mpsc::channel::<WriteRequest>();
            thread::spawn(move || {
                while let Ok((history, acknowledgement)) = receiver.recv() {
                    let _ = write_history(&path, &history);
                    if let Some(acknowledgement) = acknowledgement {
                        let _ = acknowledgement.send(());
                    }
                }
            });
            sender
        });
        Self {
            history,
            writer,
            session_path: None,
            session_ms: 0,
            session_counted: false,
            unsaved_ms: 0,
        }
    }

    /// Call after every successful start, including repeat of the same track.
    pub fn start_session(&mut self, track: &Track) {
        self.session_path = Some(track.path.clone());
        self.session_ms = 0;
        self.session_counted = false;
        self.history.entry(track.path.clone()).or_insert_with(|| {
            let (_, artists) = build_catalog(std::slice::from_ref(track));
            Entry {
                title: track.title.clone(),
                artist: track.artist.clone(),
                artists: artists.into_iter().map(|artist| artist.name).collect(),
                album: track.album.clone(),
                album_artist: album_artist(track),
                cover_path: track.cover_path.clone(),
                ..Entry::default()
            }
        });
    }

    /// Add wall time only while audio is playing. Seeking must not add time.
    pub fn record(&mut self, track: &Track, elapsed_ms: u64) {
        if elapsed_ms == 0 {
            return;
        }
        if self.session_path.as_ref() != Some(&track.path) {
            self.start_session(track);
        }
        self.session_ms = self.session_ms.saturating_add(elapsed_ms);
        let entry = self.history.get_mut(&track.path).expect("session entry");
        entry.listened_ms = entry.listened_ms.saturating_add(elapsed_ms);
        // Thirty seconds, or half of a shorter track, counts as one play.
        let threshold = if track.duration_ms == 0 {
            30_000
        } else {
            (track.duration_ms / 2).clamp(1, 30_000)
        };
        if !self.session_counted && self.session_ms >= threshold {
            entry.plays = entry.plays.saturating_add(1);
            self.session_counted = true;
        }
        self.unsaved_ms = self.unsaved_ms.saturating_add(elapsed_ms);
        if self.unsaved_ms >= 10_000 {
            self.flush();
        }
    }

    /// Periodic persistence runs on the writer thread, outside the UI loop.
    pub fn flush(&mut self) {
        if let Some(writer) = &self.writer {
            let _ = writer.send((self.history.clone(), None));
        }
        self.unsaved_ms = 0;
    }

    /// Use from the application quit hook to ensure the final snapshot is written.
    pub fn save(&self) {
        if let Some(writer) = &self.writer {
            let (sender, receiver) = mpsc::channel();
            if writer.send((self.history.clone(), Some(sender))).is_ok() {
                let _ = receiver.recv_timeout(Duration::from_secs(2));
            }
        }
    }

    pub fn top_tracks(&self, limit: usize) -> Vec<Ranking> {
        ranked(
            self.history
                .values()
                .map(|entry| Ranking {
                    title: entry.title.clone(),
                    subtitle: entry.artist.clone(),
                    cover_path: entry.cover_path.clone(),
                    listened_ms: entry.listened_ms,
                    plays: entry.plays,
                })
                .collect(),
            limit,
        )
    }

    pub fn top_artists(&self, limit: usize) -> Vec<Ranking> {
        self.aggregate(limit, |entry| {
            entry
                .artists
                .iter()
                .map(|artist| (artist.clone(), String::new()))
                .collect()
        })
    }

    pub fn top_albums(&self, limit: usize) -> Vec<Ranking> {
        self.aggregate(limit, |entry| {
            vec![(entry.album.clone(), entry.album_artist.clone())]
        })
    }

    fn aggregate(
        &self,
        limit: usize,
        keys: impl Fn(&Entry) -> Vec<(String, String)>,
    ) -> Vec<Ranking> {
        let mut totals: HashMap<(String, String), Ranking> = HashMap::new();
        for entry in self.history.values().filter(|entry| entry.listened_ms > 0) {
            for (title, subtitle) in keys(entry) {
                let row = totals
                    .entry((title.clone(), subtitle.clone()))
                    .or_insert_with(|| Ranking {
                        title,
                        subtitle,
                        cover_path: entry.cover_path.clone(),
                        listened_ms: 0,
                        plays: 0,
                    });
                row.listened_ms = row.listened_ms.saturating_add(entry.listened_ms);
                row.plays = row.plays.saturating_add(entry.plays);
            }
        }
        ranked(totals.into_values().collect(), limit)
    }
}

fn ranked(mut rows: Vec<Ranking>, limit: usize) -> Vec<Ranking> {
    rows.retain(|row| row.listened_ms > 0);
    rows.sort_by(|a, b| {
        b.listened_ms
            .cmp(&a.listened_ms)
            .then_with(|| b.plays.cmp(&a.plays))
            .then_with(|| a.title.cmp(&b.title))
    });
    rows.truncate(limit);
    rows
}

fn write_history(path: &std::path::Path, history: &History) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec(history)?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn statistics() -> Statistics {
        Statistics {
            history: HashMap::new(),
            writer: None,
            session_path: None,
            session_ms: 0,
            session_counted: false,
            unsaved_ms: 0,
        }
    }
    fn track(path: &str, artist: &str) -> Track {
        Track {
            path: path.into(),
            title: path.into(),
            artist: artist.into(),
            album: "Album".into(),
            album_artist: "Joji".into(),
            duration_ms: 120_000,
            ..Track::default()
        }
    }
    #[test]
    fn plays_count_once_per_session_and_repeats_count_again() {
        let mut stats = statistics();
        let track = track("one", "Joji");
        stats.record(&track, 29_000);
        assert_eq!(stats.top_tracks(10)[0].plays, 0);
        stats.record(&track, 1_000);
        stats.record(&track, 40_000);
        assert_eq!(stats.top_tracks(10)[0].plays, 1);
        stats.start_session(&track);
        stats.record(&track, 30_000);
        assert_eq!(stats.top_tracks(10)[0].plays, 2);
        assert_eq!(stats.top_tracks(10)[0].listened_ms, 100_000);
    }
    #[test]
    fn album_collaborations_aggregate_and_artists_are_individual() {
        let mut stats = statistics();
        stats.record(&track("one", "Joji; Clams Casino"), 40_000);
        stats.record(&track("two", "Joji"), 60_000);
        assert_eq!(stats.top_albums(10).len(), 1);
        assert_eq!(stats.top_albums(10)[0].listened_ms, 100_000);
        assert_eq!(stats.top_artists(1)[0].title, "Joji");
        assert_eq!(stats.top_artists(10).len(), 2);
        assert_eq!(stats.top_tracks(1)[0].title, "two");
    }
    #[test]
    fn history_roundtrips_without_losing_counters() {
        let mut stats = statistics();
        stats.record(&track("one", "Joji"), 35_000);
        let encoded = serde_json::to_vec(&stats.history).unwrap();
        let history: History = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(history[&PathBuf::from("one")].plays, 1);
        assert_eq!(history[&PathBuf::from("one")].listened_ms, 35_000);
    }
}
