use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{Arc, mpsc},
    time::Duration,
};

use crate::library::{Album, Artist, Library, Track, album_artist, build_catalog};
use crate::media_controls::SystemMediaControls;
use crate::playback::{Playback, RepeatMode};
use crate::search::{SearchChanged, SearchInput};
use gpui::{prelude::*, *};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use rand::seq::SliceRandom;

type TrackMenuAction = Box<dyn Fn(&mut AppView, &Track, &mut Context<AppView>) + 'static>;

#[derive(Default)]
pub struct AppView {
    statistics: crate::statistics::Statistics,
    statistics_tick: Option<std::time::Instant>,
    artwork_modal: Option<PathBuf>,
    menu_open: bool,
    statistics_open: bool,
    settings_open: bool,
    settings_about: bool,
    sidebar_collapsed: bool,
    sidebar_width: f32,
    sidebar_animation: Option<Task<()>>,
    _activation_subscription: Option<Subscription>,
    queue_sidebar_open: bool,
    context_track: Option<Track>,
    context_menu_position: Point<Pixels>,
    seek_dragging: std::rc::Rc<std::cell::Cell<bool>>,
    volume_dragging: std::rc::Rc<std::cell::Cell<bool>>,
    root: Option<PathBuf>,
    tracks: Arc<Vec<Track>>,
    sorted_indices: Arc<Vec<usize>>,
    date_order: Arc<Vec<usize>>,
    title_order: Arc<Vec<usize>>,
    artist_order: Arc<Vec<usize>>,
    album_order: Arc<Vec<usize>>,
    duration_order: Arc<Vec<usize>>,
    filtered_track_indices: Arc<Vec<usize>>,
    albums: Arc<Vec<Album>>,
    artists: Arc<Vec<Artist>>,
    filtered_album_indices: Arc<Vec<usize>>,
    filtered_artist_indices: Arc<Vec<usize>>,
    search_query: String,
    search: Option<Entity<SearchInput>>,
    _search_subscription: Option<Subscription>,
    playback_task: Option<Task<()>>,
    generation: u64,
    light: bool,
    palette: crate::settings::PaletteName,
    album_grid: bool,
    preferences_task: Option<Task<()>>,
    position_ms: u64,
    volume: f32,
    history: Vec<Track>,
    continuation: VecDeque<Track>,
    playback_order: Vec<Track>,
    shuffle: bool,
    repeat: RepeatMode,
    folder_dialog_open: bool,
    pending_queue: Vec<PathBuf>,
    selected_track: Option<PathBuf>,
    search_terms: Arc<Vec<String>>,
    navigation: Vec<NavigationEntry>,
    root_focus: Option<FocusHandle>,
    scrolls: HashMap<String, UniformListScrollHandle>,
    pending_offsets: HashMap<String, f32>,
    _quit_subscription: Option<Subscription>,
    status: Option<String>,
    status_details: bool,
    library_error: Option<String>,
    overlay_focus: HashMap<String, FocusHandle>,
    active_overlay: Option<&'static str>,
    return_focus: Option<FocusHandle>,
    scanning: bool,
    library_unavailable: bool,
    sort: SortMode,
    sort_reversed: bool,
    table_ratios: [f32; 4],
    table_available: f32,
    playback: Playback,
    system_media: Option<SystemMediaControls>,
    current_track: Option<Track>,
    paused: bool,
    queue: VecDeque<Track>,
    page: Page,
    selected_album: Option<(String, String)>,
    selected_artist: Option<String>,
    watcher: Option<RecommendedWatcher>,
    reindex_task: Option<Task<()>>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum SortMode {
    #[default]
    Date,
    Title,
    Artist,
    Album,
    Duration,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Page {
    #[default]
    Tracks,
    Queue,
    Albums,
    Artists,
    AlbumDetail,
    ArtistDetail,
}

struct NavigationEntry {
    page: Page,
    album: Option<(String, String)>,
    artist: Option<String>,
}

impl AppView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let saved = crate::settings::load();
        let search = cx.new(SearchInput::new);
        search.update(cx, |input, _| {
            input.light = saved.light;
            input.palette = saved.palette;
        });
        let subscription = cx.subscribe(&search, |this, _, event: &SearchChanged, cx| {
            this.set_search_query(event.0.clone(), cx);
        });
        let mut view = Self {
            statistics: crate::statistics::Statistics::load(),
            root: saved.library_root,
            sidebar_collapsed: false,
            sidebar_width: 184.,
            queue_sidebar_open: saved.page == "queue",
            album_grid: saved.album_grid.unwrap_or(true),
            light: saved.light,
            palette: saved.palette,
            shuffle: saved.shuffle,
            repeat: saved.repeat,
            volume: saved.volume.unwrap_or(0.7),
            sort_reversed: saved.sort_reversed,
            table_ratios: saved
                .table_ratios
                .filter(|ratios| {
                    ratios
                        .iter()
                        .all(|value| value.is_finite() && *value >= 0.07)
                        && (ratios.iter().sum::<f32>() - 1.).abs() < 0.001
                })
                .unwrap_or([0.; 4]),
            sort: match saved.sort.as_str() {
                "title" => SortMode::Title,
                "artist" => SortMode::Artist,
                "album" => SortMode::Album,
                "duration" => SortMode::Duration,
                _ => SortMode::Date,
            },
            page: match saved.page.as_str() {
                "albums" => Page::Albums,
                "artists" => Page::Artists,
                _ => Page::Tracks,
            },
            root_focus: Some(cx.focus_handle()),
            pending_queue: saved.queue,
            pending_offsets: saved.scroll_offsets,
            _quit_subscription: Some(cx.on_app_quit(|view, _| {
                view.persist();
                view.statistics.save();
                async {}
            })),
            search: Some(search),
            _search_subscription: Some(subscription),
            system_media: SystemMediaControls::new().ok(),
            ..Self::default()
        };
        view.playback_task = Some(cx.spawn(async move |view, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(400))
                    .await;
                if view
                    .update(cx, |this, cx| {
                        let media_events = this
                            .system_media
                            .as_ref()
                            .map(|media| media.drain_events().collect::<Vec<_>>())
                            .unwrap_or_default();
                        for event in media_events {
                            match event {
                                souvlaki::MediaControlEvent::Play => {
                                    if this.paused {
                                        this.toggle_playback(cx);
                                    }
                                }
                                souvlaki::MediaControlEvent::Pause => {
                                    if !this.paused {
                                        this.toggle_playback(cx);
                                    }
                                }
                                souvlaki::MediaControlEvent::Toggle => this.toggle_playback(cx),
                                souvlaki::MediaControlEvent::Next => this.play_next(cx),
                                souvlaki::MediaControlEvent::Previous => this.play_previous(cx),
                                souvlaki::MediaControlEvent::SetPosition(position) => {
                                    if this.playback.seek(position.0) {
                                        this.position_ms = position.0.as_millis() as u64;
                                    }
                                }
                                souvlaki::MediaControlEvent::SeekBy(direction, amount) => {
                                    let current = this.playback.position();
                                    let target = match direction {
                                        souvlaki::SeekDirection::Forward => {
                                            current.saturating_add(amount)
                                        }
                                        souvlaki::SeekDirection::Backward => {
                                            current.saturating_sub(amount)
                                        }
                                    };
                                    if this.playback.seek(target) {
                                        this.position_ms = target.as_millis() as u64;
                                    }
                                }
                                souvlaki::MediaControlEvent::Stop => {
                                    if !this.paused && !this.playback.is_paused() {
                                        this.playback.toggle();
                                    }
                                    this.paused = true;
                                }
                                souvlaki::MediaControlEvent::SetVolume(volume) => {
                                    this.volume = volume.clamp(0.0, 1.0) as f32;
                                    this.playback.set_volume(this.volume);
                                    this.persist();
                                }
                                _ => {}
                            }
                        }
                        let now = std::time::Instant::now();
                        let elapsed = this.statistics_tick.replace(now).map_or(0, |last| {
                            now.duration_since(last).as_millis().min(1000) as u64
                        });
                        if !this.paused
                            && !this.playback.is_paused()
                            && let Some(track) = &this.current_track
                        {
                            this.statistics.record(track, elapsed);
                        }
                        if this.current_track.is_some() {
                            if this.playback.is_finished() {
                                this.position_ms = this
                                    .current_track
                                    .as_ref()
                                    .map_or(0, |track| track.duration_ms);
                                if !this.paused {
                                    this.finish_track(cx);
                                }
                            } else {
                                if !this.seek_dragging.get() {
                                    this.position_ms = this.playback.position().as_millis() as u64;
                                }
                                this.paused = this.playback.is_paused();
                            }
                            cx.notify();
                        }
                        if let (Some(media), Some(track)) =
                            (&mut this.system_media, &this.current_track)
                        {
                            media.sync(
                                &track.title,
                                &track.artist,
                                &track.album,
                                Duration::from_millis(track.duration_ms),
                                Duration::from_millis(this.position_ms),
                                this.paused || this.playback.is_paused(),
                            );
                        } else if let Some(media) = &mut this.system_media {
                            media.stop();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        }));
        if let Some(root) = view.root.clone() {
            view.start_scan(root, cx);
        }
        view
    }
    fn persist(&self) {
        crate::settings::save(&crate::settings::Settings {
            library_root: self.root.clone(),
            light: self.light,
            palette: self.palette,
            shuffle: self.shuffle,
            repeat: self.repeat,
            sort_reversed: self.sort_reversed,
            table_ratios: Some(if self.table_ratios == [0.; 4] {
                [0.44, 0.28, 0.20, 0.08]
            } else {
                self.table_ratios
            }),
            sort: match self.sort {
                SortMode::Date => "date",
                SortMode::Title => "title",
                SortMode::Artist => "artist",
                SortMode::Album => "album",
                SortMode::Duration => "duration",
            }
            .into(),
            queue: if self.pending_queue.is_empty() {
                self.queue.iter().map(|track| track.path.clone()).collect()
            } else {
                self.pending_queue.clone()
            },
            page: match self.page {
                Page::Albums | Page::AlbumDetail => "albums",
                Page::Artists | Page::ArtistDetail => "artists",
                Page::Queue => "queue",
                _ => "tracks",
            }
            .into(),
            volume: Some(self.volume),
            album_grid: Some(self.album_grid),
            scroll_offsets: self
                .pending_offsets
                .iter()
                .map(|(key, value)| (key.clone(), *value))
                .chain(self.scrolls.iter().map(|(key, handle)| {
                    (
                        key.clone(),
                        f32::from(handle.0.borrow().base_handle.offset().y),
                    )
                }))
                .collect(),
        });
    }
    fn save_later(&mut self, cx: &mut Context<Self>) {
        self.preferences_task = Some(cx.spawn(async move |view, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(350))
                .await;
            let _ = view.update(cx, |view, _| view.persist());
        }));
    }
    fn prepare_tracks(&mut self, snapshot: LibrarySnapshot) {
        self.tracks = snapshot.tracks;
        self.albums = snapshot.albums;
        self.artists = snapshot.artists;
        self.search_terms = snapshot.search_terms;
        self.date_order = snapshot.date_order;
        self.title_order = snapshot.title_order;
        self.artist_order = snapshot.artist_order;
        self.album_order = snapshot.album_order;
        self.duration_order = snapshot.duration_order;
        if !self.pending_queue.is_empty() {
            self.queue = self
                .pending_queue
                .iter()
                .filter_map(|path| {
                    self.tracks
                        .iter()
                        .find(|track| &track.path == path)
                        .cloned()
                })
                .collect();
            self.pending_queue.clear();
        }
        self.rebuild_order();
    }
    fn set_sort(&mut self, sort: SortMode, cx: &mut Context<Self>) {
        self.sort_reversed = if self.sort == sort {
            !self.sort_reversed
        } else {
            false
        };
        self.sort = sort;
        self.rebuild_order();
        self.persist();
        cx.notify();
    }

    fn rebuild_order(&mut self) {
        self.sorted_indices = match self.sort {
            SortMode::Date => self.date_order.clone(),
            SortMode::Title => self.title_order.clone(),
            SortMode::Artist => self.artist_order.clone(),
            SortMode::Album => self.album_order.clone(),
            SortMode::Duration => self.duration_order.clone(),
        };
        if self.sort_reversed {
            self.sorted_indices = Arc::new(self.sorted_indices.iter().rev().copied().collect());
        }
        self.rebuild_search();
    }
    fn set_search_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.context_track = None;
        self.search_query = query;
        self.pending_offsets.clear();
        for handle in self.scrolls.values() {
            handle.scroll_to_item_strict(0, ScrollStrategy::Top);
        }
        self.rebuild_search();
        cx.notify();
    }

    fn rebuild_search(&mut self) {
        let query = self.search_query.trim().to_lowercase();
        self.filtered_track_indices = if query.is_empty() {
            self.sorted_indices.clone()
        } else {
            Arc::new(
                self.sorted_indices
                    .iter()
                    .copied()
                    .filter(|index| {
                        query.is_empty()
                            || self
                                .search_terms
                                .get(*index)
                                .is_some_and(|text| text.contains(&query))
                    })
                    .collect(),
            )
        };
        self.filtered_album_indices = Arc::new(
            self.albums
                .iter()
                .enumerate()
                .filter(|(_, album)| {
                    query.is_empty()
                        || album.title.to_lowercase().contains(&query)
                        || album.artist.to_lowercase().contains(&query)
                })
                .map(|(index, _)| index)
                .collect(),
        );
        self.filtered_artist_indices = Arc::new(
            self.artists
                .iter()
                .enumerate()
                .filter(|(_, artist)| {
                    query.is_empty() || artist.name.to_lowercase().contains(&query)
                })
                .map(|(index, _)| index)
                .collect(),
        );
    }

    fn remember_page(&mut self) {
        self.context_track = None;
        self.navigation.push(NavigationEntry {
            page: self.page,
            album: self.selected_album.clone(),
            artist: self.selected_artist.clone(),
        });
    }
    fn set_page(&mut self, page: Page, cx: &mut Context<Self>) {
        if self.page != page {
            self.remember_page();
            self.page = page;
            self.persist();
        }
        cx.notify();
    }
    fn back(&mut self, cx: &mut Context<Self>) {
        if let Some(NavigationEntry {
            page,
            album,
            artist,
        }) = self.navigation.pop()
        {
            self.page = page;
            self.selected_album = album;
            self.selected_artist = artist;
            self.persist();
            cx.notify();
        }
    }
    fn open_album(&mut self, title: String, artist: String, cx: &mut Context<Self>) {
        self.remember_page();
        self.selected_album = Some((title, artist));
        self.page = Page::AlbumDetail;
        self.persist();
        cx.notify();
    }
    fn open_artist(&mut self, artist: String, cx: &mut Context<Self>) {
        self.remember_page();
        self.selected_artist = Some(artist);
        self.page = Page::ArtistDetail;
        self.persist();
        cx.notify();
    }
    fn detail_indices(&self) -> Arc<Vec<usize>> {
        let indices = match self.page {
            Page::AlbumDetail => self
                .selected_album
                .as_ref()
                .and_then(|(title, artist)| {
                    self.albums
                        .iter()
                        .find(|a| &a.title == title && &a.artist == artist)
                })
                .map(|a| a.track_indices.clone())
                .unwrap_or_default(),
            Page::ArtistDetail => self
                .selected_artist
                .as_ref()
                .and_then(|name| self.artists.iter().find(|a| &a.name == name))
                .map(|a| a.track_indices.clone())
                .unwrap_or_default(),
            _ => self.filtered_track_indices.clone(),
        };
        let query = self.search_query.trim().to_lowercase();
        if matches!(self.page, Page::AlbumDetail | Page::ArtistDetail) && !query.is_empty() {
            Arc::new(
                indices
                    .iter()
                    .copied()
                    .filter(|index| {
                        self.search_terms
                            .get(*index)
                            .is_some_and(|text| text.contains(&query))
                    })
                    .collect(),
            )
        } else {
            indices
        }
    }
    fn upcoming_tracks(&self) -> Vec<Track> {
        self.queue
            .iter()
            .chain(self.continuation.iter())
            .cloned()
            .collect()
    }
    fn remove_upcoming(&mut self, index: usize) -> bool {
        if index < self.queue.len() {
            self.queue.remove(index).is_some()
        } else {
            self.continuation.remove(index - self.queue.len()).is_some()
        }
    }
    fn remove_queued_track(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.remove_upcoming(index) {
            self.persist();
            cx.notify();
        }
    }
    fn reorder_upcoming(&mut self, from: usize, to: usize) -> bool {
        let mut upcoming = self.upcoming_tracks();
        if from >= upcoming.len() || to >= upcoming.len() || from == to {
            return false;
        }
        let explicit_count = self.queue.len();
        let track = upcoming.remove(from);
        upcoming.insert(to, track);
        self.continuation = upcoming.split_off(explicit_count).into();
        self.queue = upcoming.into();
        true
    }
    fn reorder_queue(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        if self.reorder_upcoming(from, to) {
            self.persist();
            cx.notify();
        }
    }
    fn move_queued_track(&mut self, index: usize, delta: isize, cx: &mut Context<Self>) {
        if let Some(target) = index.checked_add_signed(delta) {
            self.reorder_queue(index, target, cx);
        }
    }
    fn play_queued_track(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.queue.len() + self.continuation.len() {
            return;
        }
        let track = if index < self.queue.len() {
            self.queue.drain(..=index).next_back()
        } else {
            let context_index = index - self.queue.len();
            self.queue.clear();
            self.continuation.drain(..=context_index).next_back()
        };
        if let Some(track) = track {
            self.start_track(track, cx);
            self.persist();
        }
    }
    fn start_track(&mut self, track: Track, cx: &mut Context<Self>) -> bool {
        let started = match self.playback.play(&track.path) {
            Ok(()) => {
                if let Some(previous) = self.current_track.take()
                    && previous.path != track.path
                {
                    self.history.push(previous);
                }
                self.statistics.start_session(&track);
                self.statistics_tick = Some(std::time::Instant::now());
                self.current_track = Some(track);
                self.paused = false;
                self.position_ms = 0;
                self.playback.set_volume(self.volume);
                self.status = None;
                true
            }
            Err(error) => {
                self.status = Some(format!("Cannot play this file: {error}"));
                false
            }
        };
        cx.notify();
        started
    }
    fn play_track(&mut self, track: Track, cx: &mut Context<Self>) {
        let context: Vec<Track> = if self.page == Page::Queue {
            self.queue.iter().cloned().collect()
        } else {
            self.detail_indices()
                .iter()
                .filter_map(|index| self.tracks.get(*index).cloned())
                .collect()
        };
        if self.page == Page::Queue {
            if let Some(index) = context
                .iter()
                .position(|candidate| candidate.path == track.path)
            {
                self.queue.drain(..=index);
            }
            self.playback_order = context;
            self.continuation.clear();
        } else {
            self.playback_order = context;
            self.reset_continuation(&track.path);
        }
        self.start_track(track, cx);
        self.persist();
    }
    fn toggle_playback(&mut self, cx: &mut Context<Self>) {
        if let Some(track) = self.current_track.clone() {
            if self.playback.is_finished() {
                self.start_track(track, cx);
            } else {
                self.playback.toggle();
                self.paused = self.playback.is_paused();
            }
        } else if !self.queue.is_empty() {
            self.play_next(cx);
        } else if let Some(track) = self
            .filtered_track_indices
            .first()
            .and_then(|index| self.tracks.get(*index))
            .cloned()
        {
            self.play_track(track, cx);
        }
        cx.notify();
    }
    fn reset_continuation(&mut self, current: &std::path::Path) {
        let index = self
            .playback_order
            .iter()
            .position(|track| track.path == current);
        let mut remaining: Vec<Track> = if self.shuffle {
            self.playback_order
                .iter()
                .filter(|track| track.path != current)
                .cloned()
                .collect()
        } else {
            self.playback_order
                .iter()
                .skip(index.map_or(0, |index| index + 1))
                .cloned()
                .collect()
        };
        if self.shuffle {
            remaining.shuffle(&mut rand::rng());
        }
        self.continuation = remaining.into();
    }
    fn toggle_shuffle(&mut self, cx: &mut Context<Self>) {
        self.shuffle = !self.shuffle;
        if let Some(track) = &self.current_track {
            let path = track.path.clone();
            self.reset_continuation(&path);
        }
        self.persist();
        cx.notify();
    }
    fn cycle_repeat(&mut self, cx: &mut Context<Self>) {
        self.repeat = self.repeat.next();
        self.persist();
        cx.notify();
    }
    fn play_collection(&mut self, shuffle: bool, cx: &mut Context<Self>) {
        self.shuffle = shuffle;
        let indices = self.detail_indices();
        let index = if shuffle {
            use rand::Rng;
            (!indices.is_empty()).then(|| rand::rng().random_range(0..indices.len()))
        } else {
            (!indices.is_empty()).then_some(0)
        };
        if let Some(track) = index
            .and_then(|index| self.tracks.get(indices[index]))
            .cloned()
        {
            self.play_track(track, cx);
        }
    }
    fn finish_track(&mut self, cx: &mut Context<Self>) {
        if let Some(track) = self.next_track(true) {
            self.start_track(track, cx);
            self.persist();
        } else {
            self.paused = true;
        }
        cx.notify();
    }
    fn next_track(&mut self, automatic: bool) -> Option<Track> {
        if automatic && self.repeat == RepeatMode::One {
            return self.current_track.clone();
        }
        let mut next = self
            .queue
            .pop_front()
            .or_else(|| self.continuation.pop_front());
        if next.is_none() && self.repeat == RepeatMode::All && !self.playback_order.is_empty() {
            let mut cycle = self.playback_order.clone();
            if self.shuffle {
                cycle.shuffle(&mut rand::rng());
                // Avoid playing the boundary track twice when another track is available.
                if cycle.len() > 1
                    && self
                        .current_track
                        .as_ref()
                        .is_some_and(|track| track.path == cycle[0].path)
                {
                    cycle.swap(0, 1);
                }
            }
            self.continuation = cycle.into();
            next = self.continuation.pop_front();
        }
        next
    }
    fn play_next(&mut self, cx: &mut Context<Self>) {
        let next = self.next_track(false);
        if let Some(track) = next {
            self.start_track(track, cx);
            self.persist();
        } else {
            self.paused = true;
        }
        cx.notify();
    }
    fn play_previous(&mut self, cx: &mut Context<Self>) {
        if self.position_ms > 3000 {
            if self.playback.is_finished() {
                if let Some(track) = self.current_track.clone() {
                    self.start_track(track, cx);
                }
            } else {
                self.playback.seek(Duration::ZERO);
                self.position_ms = 0;
            }
        } else if let Some(track) = self.history.last().cloned() {
            let history_len = self.history.len();
            let current = self.current_track.clone();
            if self.start_track(track, cx) {
                self.history.truncate(history_len - 1);
                if let Some(current) = current {
                    self.continuation.push_front(current);
                }
            }
        }
        cx.notify();
    }
    fn choose_folder(&mut self, cx: &mut Context<Self>) {
        if self.folder_dialog_open {
            return;
        }
        self.folder_dialog_open = true;
        // Never run NSOpenPanel's synchronous nested event loop while GPUI's App
        // RefCell is borrowed by this listener. The async panel yields first.
        cx.spawn(async move |view, cx| {
            let selected = rfd::AsyncFileDialog::new()
                .set_title("Choose music folder")
                .pick_folder()
                .await;
            let _ = view.update(cx, |this, cx| {
                this.folder_dialog_open = false;
                let Some(selected) = selected else { return };
                let path = selected.path().to_path_buf();
                if this.root.as_ref() != Some(&path) {
                    this.prepare_tracks(LibrarySnapshot::new(Vec::new()));
                    this.queue.clear();
                    this.pending_queue.clear();
                }
                this.root = Some(path.clone());
                this.persist();
                this.start_scan(path, cx);
            });
        })
        .detach();
    }

    fn start_scan(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.generation += 1;
        let generation = self.generation;
        self.library_unavailable = false;
        self.library_error = None;
        self.status_details = false;
        self.status = Some("Loading saved library…".into());
        self.scanning = true;
        cx.notify();

        self.watch_library(path.clone(), cx);

        cx.spawn(async move |view, cx| {
            let cache_root = path.clone();
            let cached_task = cx
                .background_executor()
                .spawn(async move { crate::cache::load(&cache_root).map(LibrarySnapshot::new) });
            if let Ok(cached_tracks) = cached_task.await
                && !cached_tracks.tracks.is_empty()
            {
                let _ = view.update(cx, |view, cx| {
                    if view.generation != generation {
                        return;
                    }
                    view.prepare_tracks(cached_tracks);
                    view.status = Some("Refreshing library in background…".into());
                    cx.notify();
                });
            }

            let scan_root = path.clone();
            let task = cx
                .background_executor()
                .spawn(async move { scan_and_cache(scan_root) });
            let result = task.await;
            let _ = view.update(cx, |view, cx| {
                if view.generation != generation {
                    return;
                }
                view.scanning = false;
                match result {
                    Ok((library, cache_result)) => {
                        view.library_unavailable = false;
                        view.library_error = None;
                        let skipped=library.skipped_files;
                        view.prepare_tracks(library);
                        view.status = cache_result.err().map(|error| {
                            format!("Library loaded; cache could not be saved: {error}")
                        }).or_else(||(skipped>0).then(||format!("{skipped} files could not be read. Check their format or file contents.")));
                    }
                    Err(error) => {
                        view.library_unavailable = true;
                        view.library_error = Some(error.to_string());
                        view.status = None;
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn watch_library(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.reindex_task.take();
        self.watcher.take();
        let (sender, receiver) = mpsc::channel();
        let mut watcher =
            match notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                if event.is_ok() {
                    let _ = sender.send(());
                }
            }) {
                Ok(watcher) => watcher,
                Err(error) => {
                    self.status = Some(format!("Could not watch library folder: {error}"));
                    return;
                }
            };
        if let Err(error) = watcher.watch(&path, RecursiveMode::Recursive) {
            self.status = Some(format!("Could not watch library folder: {error}"));
            return;
        }
        self.watcher = Some(watcher);

        let generation = self.generation;
        self.reindex_task = Some(cx.spawn(async move |view, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(750))
                    .await;
                match view.update(cx, |view, _| view.scanning) {
                    Ok(true) => continue,
                    Err(_) => break,
                    _ => {}
                }
                if receiver.try_recv().is_err() {
                    continue;
                }
                while receiver.try_recv().is_ok() {}

                let _ = view.update(cx, |view, cx| {
                    view.scanning = true;
                    cx.notify();
                });
                let scan_path = path.clone();
                let task = cx
                    .background_executor()
                    .spawn(async move { scan_and_cache(scan_path) });
                let result = task.await;
                if view
                    .update(cx, |view, cx| {
                        if view.generation != generation {
                            return;
                        }
                        view.scanning = false;
                        match result {
                            Ok((library, cache_result)) => {
                                view.library_unavailable = false;
                                view.library_error = None;
                                let skipped=library.skipped_files;
                                view.prepare_tracks(library);
                                view.status = cache_result.err().map(|error| {
                                    format!("Library refreshed; cache could not be saved: {error}")
                                }).or_else(||(skipped>0).then(||format!("{skipped} files could not be read. Check their format or file contents.")));
                            }
                            Err(error) => {
                                view.library_unavailable = true;
                                view.library_error = Some(error.to_string());
                                view.status = None;
                            }
                        }
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        }));
    }
}

#[derive(Clone, Copy)]
struct Palette {
    bg: u32,
    panel: u32,
    raised: u32,
    line: u32,
    text: u32,
    muted: u32,
    accent: u32,
    accent_bg: u32,
}
impl Palette {
    fn from_name(name: crate::settings::PaletteName, light: bool) -> Self {
        let d = name.definition(light);
        Self {
            bg: d.bg,
            panel: d.panel,
            raised: d.raised,
            line: d.line,
            text: d.text,
            muted: d.muted,
            accent: d.accent,
            accent_bg: d.accent_bg,
        }
    }
}
fn icon(name: &'static str, size: f32, color: u32) -> impl IntoElement {
    svg()
        .path(name)
        .size(px(size))
        .flex_shrink_0()
        .text_color(rgb(color))
}
struct Tip(String, Palette);
impl Render for Tip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_3()
            .py_2()
            .rounded_md()
            .text_xs()
            .bg(rgb(self.1.raised))
            .text_color(rgb(self.1.text))
            .border_1()
            .border_color(rgb(self.1.line))
            .child(self.0.clone())
    }
}
fn icon_button(
    id: impl Into<String>,
    name: &'static str,
    title: &'static str,
    p: Palette,
    enabled: bool,
    primary: bool,
    listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let id = id.into();
    div()
        .id(id.clone())
        .debug_selector(move || id.clone())
        .role(Role::Button)
        .aria_label(title)
        .border_1()
        .border_color(rgba(0x00000000))
        .size(px(if primary && title == "Play / pause · Space" {
            38.
        } else {
            32.
        }))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .text_color(rgb(if !enabled {
            p.muted
        } else if primary {
            p.bg
        } else {
            p.text
        }))
        .bg(rgb(if primary && enabled {
            p.accent
        } else {
            p.panel
        }))
        .opacity(if enabled { 1. } else { 0.35 })
        .when(enabled, |el| {
            el.tab_index(0)
                .focus_visible(move |style| style.border_color(rgb(p.accent)))
                .cursor_pointer()
                .hover(move |s| s.bg(rgb(if primary { p.text } else { p.raised })))
                .active(move |s| s.bg(rgb(p.accent_bg)))
                .on_click(listener)
        })
        .tooltip(move |_, cx| cx.new(|_| Tip(title.into(), p)).into())
        .child(icon(
            name,
            if primary { 19. } else { 17. },
            if primary && enabled { p.bg } else { p.text },
        ))
}
fn icon_toggle_button(
    id: &'static str,
    name: &'static str,
    title: &'static str,
    p: Palette,
    enabled: bool,
    active: bool,
    listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(title)
        .aria_toggled(if active {
            Toggled::True
        } else {
            Toggled::False
        })
        .border_1()
        .border_color(rgba(0x00000000))
        .size(px(32.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_color(rgb(if active { p.accent } else { p.text }))
        .bg(rgb(p.panel))
        .opacity(if enabled { 1. } else { 0.35 })
        .when(enabled, |el| {
            el.tab_index(0)
                .focus_visible(move |s| s.border_color(rgb(p.accent)))
                .cursor_pointer()
                .hover(move |s| s.text_color(rgb(p.accent)))
                .on_click(listener)
        })
        .tooltip(move |_, cx| cx.new(|_| Tip(title.into(), p)).into())
        .child(icon(name, 17., if active { p.accent } else { p.text }))
}
fn text_button(
    id: impl Into<String>,
    name: &'static str,
    label: impl Into<String>,
    p: Palette,
    primary: bool,
    listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id.into())
        .role(Role::Button)
        .tab_index(0)
        .border_1()
        .border_color(rgba(0x00000000))
        .focus_visible(move |style| style.border_color(rgb(p.accent)))
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .h(px(32.))
        .px_3()
        .rounded_md()
        .cursor_pointer()
        .text_size(px(12.))
        .font_weight(FontWeight::MEDIUM)
        .bg(rgb(if primary { p.accent } else { p.raised }))
        .text_color(rgb(if primary { p.bg } else { p.text }))
        .hover(move |s| s.bg(rgb(if primary { p.text } else { p.accent_bg })))
        .active(move |s| s.opacity(0.7))
        .on_click(listener)
        .child(icon(name, 14., if primary { p.bg } else { p.text }))
        .child(label.into())
}
fn artwork(path: Option<PathBuf>, label: String, size: f32, p: Palette) -> AnyElement {
    match path {
        Some(path) => img(path)
            .size(px(size))
            .flex_shrink_0()
            .rounded_md()
            .object_fit(ObjectFit::Cover)
            .with_loading(move || placeholder(size, p).into_any_element())
            .with_fallback(move || placeholder(size, p).into_any_element())
            .into_any_element(),
        None => {
            let _ = label;
            placeholder(size, p).into_any_element()
        }
    }
}
fn placeholder(size: f32, p: Palette) -> impl IntoElement {
    div()
        .size(px(size))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .border_1()
        .border_color(rgb(p.line))
        .bg(rgb(p.raised))
        .text_color(rgb(p.muted))
        .child(icon("album", size * 0.35, p.muted))
}
fn label(text: impl Into<String>, p: Palette) -> impl IntoElement {
    div()
        .text_size(px(12.))
        .text_color(rgb(p.muted))
        .child(text.into())
}
fn duration(ms: u64) -> String {
    let seconds = ms / 1000;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
fn reveal_track(path: &PathBuf) {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut c = std::process::Command::new("open");
        c.arg("-R").arg(path);
        c
    };
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut c = std::process::Command::new("explorer");
        c.arg("/select,").arg(path);
        c
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(path.parent().unwrap_or(path));
        c
    };
    let _ = command.spawn();
}
struct SliderValue {
    fraction: f32,
    maximum: f32,
    step: f32,
}

fn rail(
    id: &'static str,
    value: SliderValue,
    enabled: bool,
    p: Palette,
    tooltip_text: Option<String>,
    dragging: std::rc::Rc<std::cell::Cell<bool>>,
    callback: impl Fn(f32, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    use std::{cell::Cell, rc::Rc};
    let SliderValue {
        fraction: value,
        maximum,
        step,
    } = value;
    let fraction_step = if maximum > 0. { step / maximum } else { 0. };
    let bounds = Rc::new(Cell::new(Bounds::<Pixels>::default()));
    let painting = bounds.clone();
    let callback = Rc::new(callback);
    let click_callback = callback.clone();
    let click_dragging = dragging.clone();
    let is_dragging = dragging.get();
    let key_callback = callback.clone();
    div()
        .id(id)
        .debug_selector(move || format!("rail-{id}"))
        .role(Role::Slider)
        .aria_label(if id == "seek" {
            "Playback position"
        } else {
            "Volume"
        })
        .aria_min_numeric_value(0.)
        .aria_max_numeric_value(maximum as f64)
        .aria_numeric_value((value.clamp(0., 1.) * maximum) as f64)
        .aria_numeric_value_step(step as f64)
        .aria_orientation(Orientation::Horizontal)
        .aria_value(
            tooltip_text
                .clone()
                .unwrap_or_else(|| format!("{:.0}%", value * 100.)),
        )
        .relative()
        .group(format!("rail-{id}"))
        .h(px(16.))
        .w_full()
        .when(enabled, |el| {
            el.tab_index(0)
                .focus_visible(move |s| s.bg(rgb(p.raised)))
                .on_key_down(move |event: &KeyDownEvent, window, cx| {
                    let target = match event.keystroke.key.as_str() {
                        "left" | "down" => value - fraction_step,
                        "right" | "up" => value + fraction_step,
                        "home" => 0.,
                        "end" => 1.,
                        _ => return,
                    };
                    key_callback(target.clamp(0., 1.), window, cx);
                    cx.stop_propagation();
                })
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                    let b = bounds.get();
                    click_dragging.set(true);
                    if b.size.width > px(0.) {
                        click_callback(
                            ((event.position.x - b.left()) / b.size.width).clamp(0., 1.),
                            window,
                            cx,
                        );
                    }
                    cx.stop_propagation();
                })
        })
        .child(
            canvas(
                move |b, _, _| {
                    painting.set(b);
                },
                move |b, _, window, _| {
                    let y = b.top() + b.size.height / 2. - px(1.5);
                    window.paint_quad(fill(
                        Bounds::new(point(b.left(), y), size(b.size.width, px(3.))),
                        rgb(p.line),
                    ));
                    if enabled && value > 0. {
                        window.paint_quad(fill(
                            Bounds::new(
                                point(b.left(), y),
                                size(b.size.width * value.clamp(0., 1.), px(3.)),
                            ),
                            rgb(p.accent),
                        ));
                    }
                    if enabled {
                        let moving = dragging.clone();
                        let moving_callback = callback.clone();
                        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                            if phase == DispatchPhase::Bubble
                                && moving.get()
                                && b.size.width > px(0.)
                            {
                                moving_callback(
                                    ((event.position.x - b.left()) / b.size.width).clamp(0., 1.),
                                    window,
                                    cx,
                                );
                            }
                        });
                        let released = dragging.clone();
                        let release_callback = callback.clone();
                        window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                            if phase == DispatchPhase::Bubble
                                && event.button == MouseButton::Left
                                && released.replace(false)
                                && b.size.width > px(0.)
                            {
                                release_callback(
                                    ((event.position.x - b.left()) / b.size.width).clamp(0., 1.),
                                    window,
                                    cx,
                                );
                                window.refresh();
                            }
                        });
                    }
                },
            )
            .size_full(),
        )
        .when(id == "seek" && enabled, |el| {
            el.child(
                div()
                    .absolute()
                    .left(relative(value.clamp(0., 1.)))
                    .ml(px(-5.))
                    .top(px(3.))
                    .size(px(10.))
                    .group("seek-thumb")
                    .rounded_full()
                    .bg(rgb(p.text))
                    .opacity(if is_dragging { 1. } else { 0. })
                    .group_hover(format!("rail-{id}"), |s| s.opacity(1.))
                    .when_some(tooltip_text, |el, text| {
                        el.child(
                            div()
                                .absolute()
                                .debug_selector(|| "seek-time-tooltip".into())
                                .w(px(54.))
                                .left(px(-22.))
                                .bottom(px(18.))
                                .py_1()
                                .rounded_md()
                                .text_center()
                                .whitespace_nowrap()
                                .bg(rgb(p.raised))
                                .text_color(rgb(p.text))
                                .text_size(px(11.))
                                .opacity(if is_dragging { 1. } else { 0. })
                                .group_hover("seek-thumb", |s| s.opacity(1.))
                                .child(text),
                        )
                    }),
            )
        })
}

impl AppView {
    fn current_album(&self) -> Option<&Album> {
        if self.page != Page::AlbumDetail {
            return None;
        }
        self.selected_album.as_ref().and_then(|(title, artist)| {
            self.albums
                .iter()
                .find(|album| &album.title == title && &album.artist == artist)
        })
    }
    fn title(&self) -> String {
        match self.page {
            Page::Tracks => "All tracks".into(),
            Page::Albums => "Albums".into(),
            Page::Artists => "Artists".into(),
            Page::Queue => "Play queue".into(),
            Page::AlbumDetail => self
                .selected_album
                .as_ref()
                .map(|a| a.0.clone())
                .unwrap_or_default(),
            Page::ArtistDetail => self.selected_artist.clone().unwrap_or_default(),
        }
    }
    fn page_key(&self) -> String {
        match self.page {
            Page::Tracks => "tracks".into(),
            Page::Queue => "queue".into(),
            Page::Artists => "artists".into(),
            Page::Albums => format!("albums-{}", self.album_grid),
            Page::AlbumDetail => format!("album-{:?}", self.selected_album),
            Page::ArtistDetail => format!("artist-{:?}", self.selected_artist),
        }
    }
    fn overlay(&self) -> Option<&'static str> {
        if self.artwork_modal.is_some() {
            Some("artwork")
        } else if self.settings_open {
            Some("settings")
        } else if self.statistics_open {
            Some("statistics")
        } else if self.menu_open {
            Some("menu")
        } else {
            None
        }
    }

    fn overlay_stops(&self) -> Vec<String> {
        match self.overlay() {
            Some("menu") => vec!["menu-statistics".into(), "menu-settings".into()],
            Some("settings") => {
                let mut stops = vec![
                    "settings-tab-false".into(),
                    "settings-tab-true".into(),
                    "close-settings".into(),
                ];
                if !self.settings_about {
                    stops.extend(["settings-mode-false".into(), "settings-mode-true".into()]);
                    stops.extend(
                        crate::settings::PALETTES
                            .iter()
                            .map(|p| format!("palette-{}", p.as_str())),
                    );
                }
                stops
            }
            Some("statistics") => vec!["close-statistics".into()],
            Some("artwork") => vec!["close-artwork".into()],
            _ => vec![],
        }
    }

    fn empty_content(&self) -> (&'static str, &'static str, Option<&'static str>) {
        if self.page == Page::Queue {
            return (
                "Your queue is empty",
                "Use a track’s menu to add it to the queue.",
                None,
            );
        }
        if self.tracks.is_empty() {
            if self.scanning {
                return (
                    "Loading your library",
                    "Indexing continues in the background.",
                    None,
                );
            }
            if self.library_unavailable {
                return (
                    "Music folder unavailable",
                    "Reconnect the drive or choose another music folder.",
                    Some("Choose folder"),
                );
            }
            if self.root.is_none() {
                return (
                    "Your library starts here",
                    "Choose a music folder to start listening.",
                    Some("Choose folder"),
                );
            }
        }
        if !self.search_query.trim().is_empty() {
            return (
                "No results",
                "Try another title, artist or album.",
                Some("Clear search"),
            );
        }
        (
            "No music found",
            "Choose a folder containing supported audio files.",
            Some("Choose folder"),
        )
    }

    fn status_banner(&self, p: Palette, cx: &mut Context<Self>) -> Div {
        div()
            .debug_selector(|| "status-banner".into())
            .px_6()
            .py_2()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(div().flex_1().min_w_0().text_size(px(12.)).child(
                        if self.library_unavailable {
                            if self.tracks.is_empty() {
                                "Music folder unavailable.".into()
                            } else {
                                "Music folder unavailable. Saved library retained.".into()
                            }
                        } else {
                            self.status.clone().unwrap_or_default()
                        },
                    ))
                    .when(self.library_unavailable, |el| {
                        el.child(text_button(
                            "recover-folder",
                            "folder",
                            "Choose folder",
                            p,
                            false,
                            cx.listener(|this, _, _, cx| this.choose_folder(cx)),
                        ))
                    })
                    .when(
                        self.library_unavailable && self.library_error.is_some(),
                        |el| {
                            el.child(text_button(
                                "status-details",
                                "menu",
                                if self.status_details {
                                    "Hide details"
                                } else {
                                    "Details"
                                },
                                p,
                                false,
                                cx.listener(|this, _, _, cx| {
                                    this.status_details = !this.status_details;
                                    cx.notify();
                                }),
                            ))
                        },
                    ),
            )
            .when(self.library_unavailable && self.status_details, |el| {
                el.child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(p.muted))
                        .child(self.library_error.clone().unwrap_or_default()),
                )
            })
            .when(self.library_unavailable && self.status.is_some(), |el| {
                el.child(
                    div()
                        .text_size(px(12.))
                        .child(self.status.clone().unwrap_or_default()),
                )
            })
    }
    fn key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;
        if key.key == "escape"
            && (self.artwork_modal.is_some()
                || self.menu_open
                || self.statistics_open
                || self.settings_open
                || self.context_track.is_some())
        {
            self.artwork_modal = None;
            self.menu_open = false;
            self.statistics_open = false;
            self.settings_open = false;
            self.context_track = None;
            cx.notify();
            cx.stop_propagation();
            return;
        }
        if self.overlay().is_some() {
            if key.key == "tab" {
                let stops = self.overlay_stops();
                if !stops.is_empty() {
                    let current = stops
                        .iter()
                        .position(|id| self.overlay_focus[id].is_focused(window));
                    let index = match current {
                        Some(i) if key.modifiers.shift => (i + stops.len() - 1) % stops.len(),
                        Some(i) => (i + 1) % stops.len(),
                        None if key.modifiers.shift => stops.len() - 1,
                        None => 0,
                    };
                    window.focus(&self.overlay_focus[&stops[index]], cx);
                }
                cx.stop_propagation();
            }
            return;
        }
        if self.context_track.take().is_some() {
            cx.notify();
        }
        if self.artwork_modal.is_some() || self.statistics_open || self.settings_open {
            return;
        }
        if (key.modifiers.platform || key.modifiers.control) && key.key == "f" {
            if let Some(search) = &self.search {
                let focus = search.read(cx).focus.clone();
                window.focus(&focus, cx);
            }
            cx.stop_propagation();
            return;
        }
        if key.key == "tab" {
            if key.modifiers.shift {
                window.focus_prev(cx);
            } else {
                window.focus_next(cx);
            }
            cx.stop_propagation();
            return;
        }
        if matches!(key.key.as_str(), "space" | "enter")
            && self
                .root_focus
                .as_ref()
                .is_some_and(|focus| !focus.is_focused(window))
        {
            return;
        }
        if self
            .search
            .as_ref()
            .is_some_and(|search| search.read(cx).focus.is_focused(window))
        {
            return;
        }
        match key.key.as_str() {
            "space" => self.toggle_playback(cx),
            "enter" => {
                if let Some(path) = &self.selected_track
                    && let Some(track) = self
                        .tracks
                        .iter()
                        .find(|track| &track.path == path)
                        .cloned()
                {
                    self.play_track(track, cx);
                }
            }
            "up" | "down" if !matches!(self.page, Page::Albums | Page::Artists | Page::Queue) => {
                let indices = self.detail_indices();
                if indices.is_empty() {
                    return;
                }
                let current = indices.iter().position(|index| {
                    self.selected_track.as_ref() == Some(&self.tracks[*index].path)
                });
                let index = match (current, key.key.as_str()) {
                    (Some(index), "up") => index.saturating_sub(1),
                    (Some(index), _) => (index + 1).min(indices.len() - 1),
                    _ => 0,
                };
                self.selected_track = Some(self.tracks[indices[index]].path.clone());
                if let Some(handle) = self.scrolls.get(&self.page_key()) {
                    handle.scroll_to_item(index, ScrollStrategy::Nearest);
                }
                cx.notify();
            }
            _ => return,
        }
        cx.stop_propagation();
    }
    fn hero(&self, p: Palette, cx: &mut Context<Self>) -> AnyElement {
        let indices = self.detail_indices();
        let first = indices.first().and_then(|index| self.tracks.get(*index));
        let selected_artist = (self.page == Page::ArtistDetail)
            .then_some(self.selected_artist.as_ref())
            .flatten()
            .and_then(|name| self.artists.iter().find(|artist| &artist.name == name));
        let total = self
            .current_album()
            .map(|album| album.duration_ms)
            .or_else(|| selected_artist.map(|artist| artist.duration_ms))
            .unwrap_or_default();
        let title = self.title();
        let artist = self.current_album().map(|album| album.artist.clone());
        let artwork_path = first.and_then(|track| track.cover_path.clone());
        let year = self.current_album().and_then(|album| album.year);
        let play = indices
            .first()
            .and_then(|index| self.tracks.get(*index))
            .cloned();
        let mut hero = div()
            .flex()
            .items_center()
            .gap_5()
            .py_4()
            .flex_shrink_0()
            .child(
                div()
                    .id("hero-artwork")
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.artwork_modal = artwork_path.clone();
                        cx.notify();
                    }))
                    .child(artwork(
                        first.and_then(|t| t.cover_path.clone()),
                        title.clone(),
                        168.,
                        p,
                    )),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(label(
                        if self.page == Page::AlbumDetail {
                            "ALBUM"
                        } else {
                            "ARTIST"
                        },
                        p,
                    ))
                    .child(
                        div()
                            .text_size(px(26.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .truncate()
                            .child(title),
                    )
                    .when_some(artist, |el, credit| {
                        el.child(div().flex().items_center().gap_2().children(
                            credit.split(" · ").enumerate().map(|(index, name)| {
                                let name = name.to_owned();
                                let target = name.clone();
                                div()
                                    .id(format!("album-artist-{index}"))
                                    .cursor_pointer()
                                    .text_size(px(13.))
                                    .text_color(rgb(p.accent))
                                    .hover(move |style| style.text_color(rgb(p.text)))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.open_artist(target.clone(), cx)
                                    }))
                                    .child(name)
                            }),
                        ))
                    })
                    .child(label(
                        format!(
                            "{}{} tracks · {} min",
                            year.map_or(String::new(), |year| format!("{year} · ")),
                            indices.len(),
                            total / 60000
                        ),
                        p,
                    ))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(text_button(
                                "play-collection",
                                "play",
                                "Play",
                                p,
                                true,
                                cx.listener(move |this, _, _, cx| {
                                    if let Some(track) = play.clone() {
                                        this.play_track(track, cx);
                                    }
                                }),
                            ))
                            .child(text_button(
                                "queue-collection",
                                "plus",
                                "Add to queue",
                                p,
                                false,
                                cx.listener(move |this, _, _, cx| {
                                    let queued: Vec<Track> = this
                                        .detail_indices()
                                        .iter()
                                        .map(|index| this.tracks[*index].clone())
                                        .collect();
                                    this.queue.extend(queued);
                                    this.persist();
                                    cx.notify();
                                }),
                            )),
                    ),
            );
        if self.page == Page::ArtistDetail {
            let albums: Vec<Album> = selected_artist
                .map(|artist| {
                    artist
                        .album_indices
                        .iter()
                        .take(4)
                        .filter_map(|index| self.albums.get(*index))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            hero = hero.child(
                div()
                    .w(px(240.))
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(label("ALBUMS", p))
                    .children(albums.into_iter().map(|album| {
                        let title = album.title.clone();
                        let artist = album.artist.clone();
                        div()
                            .id(format!("artist-album-{}", album.title))
                            .flex()
                            .items_center()
                            .gap_2()
                            .cursor_pointer()
                            .rounded_md()
                            .hover(move |s| s.bg(rgb(p.raised)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.open_album(title.clone(), artist.clone(), cx)
                            }))
                            .child(artwork(album.cover_path, album.title.clone(), 28., p))
                            .child(div().text_size(px(12.)).truncate().child(album.title))
                    })),
            );
        }
        hero.into_any_element()
    }
    fn player(&self, p: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let available =
            self.current_track.is_some() || !self.tracks.is_empty() || !self.queue.is_empty();
        let total = self
            .current_track
            .as_ref()
            .map_or(0, |track| track.duration_ms);
        let next = !self.queue.is_empty()
            || !self.continuation.is_empty()
            || (self.repeat == RepeatMode::All && !self.playback_order.is_empty());
        let seek_view = cx.weak_entity();
        let volume_view = cx.weak_entity();
        div()
            .debug_selector(|| "player-bar".into())
            .h(px(94.))
            .flex_shrink_0()
            .border_t_1()
            .border_color(rgb(p.line))
            .bg(rgb(p.panel))
            .px_5()
            .flex()
            .items_center()
            .gap_5()
            .child(
                div()
                    .w(px(0.))
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .id("player-artwork")
                            .cursor_pointer()
                            .hover(move |s| s.opacity(0.8))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.artwork_modal = this
                                    .current_track
                                    .as_ref()
                                    .and_then(|t| t.cover_path.clone());
                                cx.notify();
                            }))
                            .child(artwork(
                                self.current_track
                                    .as_ref()
                                    .and_then(|t| t.cover_path.clone()),
                                String::new(),
                                52.,
                                p,
                            )),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .id("player-title")
                                    .cursor_pointer()
                                    .hover(move |s| s.text_color(rgb(p.accent)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if let Some(t) = this.current_track.clone() {
                                            this.open_album(t.album.clone(), album_artist(&t), cx);
                                        }
                                    }))
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .truncate()
                                    .child(
                                        self.current_track
                                            .as_ref()
                                            .map_or("Nothing playing".into(), |t| t.title.clone()),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .when(self.current_track.is_none(), |el| {
                                        el.child(label("Choose a track from your library", p))
                                    })
                                    .children(
                                        self.current_track
                                            .as_ref()
                                            .map(|track| {
                                                crate::library::split_artist_credits(&track.artist)
                                            })
                                            .unwrap_or_default()
                                            .into_iter()
                                            .enumerate()
                                            .map(|(index, name)| {
                                                let target = name.clone();
                                                div()
                                                    .id(format!("player-artist-{index}"))
                                                    .flex_shrink_0()
                                                    .cursor_pointer()
                                                    .text_size(px(12.))
                                                    .text_color(rgb(p.muted))
                                                    .hover(move |s| s.text_color(rgb(p.accent)))
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        this.open_artist(target.clone(), cx)
                                                    }))
                                                    .child(if index == 0 {
                                                        name
                                                    } else {
                                                        format!("· {name}")
                                                    })
                                            }),
                                    ),
                            ),
                    ),
            )
            .child(
                div()
                    .w(px(0.))
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(icon_toggle_button(
                                "shuffle-mode",
                                "shuffle",
                                "Shuffle",
                                p,
                                available,
                                self.shuffle,
                                cx.listener(|this, _, _, cx| this.toggle_shuffle(cx)),
                            ))
                            .child(icon_button(
                                "previous",
                                "previous",
                                "Previous track",
                                p,
                                self.current_track.is_some(),
                                false,
                                cx.listener(|this, _, _, cx| this.play_previous(cx)),
                            ))
                            .child(icon_button(
                                "toggle-playback",
                                if self.current_track.is_none() || self.paused {
                                    "play"
                                } else {
                                    "pause"
                                },
                                "Play / pause · Space",
                                p,
                                available,
                                true,
                                cx.listener(|this, _, _, cx| this.toggle_playback(cx)),
                            ))
                            .child(icon_button(
                                "next",
                                "next",
                                "Next track",
                                p,
                                next,
                                false,
                                cx.listener(|this, _, _, cx| this.play_next(cx)),
                            ))
                            .child(icon_toggle_button(
                                "repeat-mode",
                                if self.repeat == RepeatMode::One {
                                    "repeat-one"
                                } else {
                                    "repeat"
                                },
                                match self.repeat {
                                    RepeatMode::Off => "Repeat: off",
                                    RepeatMode::All => "Repeat: all",
                                    RepeatMode::One => "Repeat: one",
                                },
                                p,
                                available,
                                self.repeat != RepeatMode::Off,
                                cx.listener(|this, _, _, cx| this.cycle_repeat(cx)),
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .w_full()
                            .text_size(px(10.))
                            .text_color(rgb(p.muted))
                            .child(
                                div()
                                    .w(px(34.))
                                    .text_right()
                                    .child(duration(self.position_ms.min(total))),
                            )
                            .child(div().flex_1().child(rail(
                                "seek",
                                SliderValue {
                                    fraction: if total > 0 {
                                        self.position_ms as f32 / total as f32
                                    } else {
                                        0.
                                    },
                                    maximum: total as f32 / 1000.,
                                    step: 5.,
                                },
                                total > 0,
                                p,
                                Some(duration(self.position_ms.min(total))),
                                self.seek_dragging.clone(),
                                move |fraction, _, cx| {
                                    let _ = seek_view.update(cx, |this, cx| {
                                        let target =
                                            Duration::from_millis((fraction * total as f32) as u64);
                                        if this.playback.seek(target) {
                                            this.position_ms = target.as_millis() as u64;
                                        } else {
                                            this.status =
                                                Some("Seeking is unavailable for this file".into());
                                        }
                                        cx.notify();
                                    });
                                },
                            )))
                            .child(div().w(px(34.)).child(duration(total))),
                    ),
            )
            .child(
                div()
                    .w(px(0.))
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_3()
                    .child(icon("volume", 16., p.muted))
                    .child(div().w(px(80.)).child(rail(
                        "volume",
                        SliderValue {
                            fraction: self.volume,
                            maximum: 100.,
                            step: 5.,
                        },
                        true,
                        p,
                        None,
                        self.volume_dragging.clone(),
                        move |value, _, cx| {
                            let _ = volume_view.update(cx, |this, cx| {
                                this.volume = value;
                                this.playback.set_volume(value);
                                this.save_later(cx);
                                cx.notify();
                            });
                        },
                    )))
                    .child(div().w(px(1.)).h(px(22.)).bg(rgb(p.line)))
                    .child(icon_button(
                        "open-queue",
                        "queue",
                        "Open play queue",
                        p,
                        true,
                        false,
                        cx.listener(|this, _, _, cx| {
                            this.queue_sidebar_open = !this.queue_sidebar_open;
                            cx.notify();
                        }),
                    )),
            )
    }
}
impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Palette::from_name(self.palette, self.light);
        if self._activation_subscription.is_none() {
            self._activation_subscription =
                Some(cx.observe_window_activation(window, |this, window, cx| {
                    if !window.is_window_active() {
                        this.context_track = None;
                        this.menu_open = false;
                        this.seek_dragging.set(false);
                        this.volume_dragging.set(false);
                        cx.notify();
                    }
                }));
        }
        self.context_menu_position.x = self.context_menu_position.x.clamp(
            px(8.),
            (window.viewport_size().width - px(228.)).max(px(8.)),
        );
        self.context_menu_position.y = self.context_menu_position.y.clamp(
            px(8.),
            (window.viewport_size().height - px(200.)).max(px(8.)),
        );

        self.overlay_focus
            .entry("app-menu".into())
            .or_insert_with(|| cx.focus_handle());
        let overlay = self.overlay();
        let stops = self.overlay_stops();
        for id in &stops {
            self.overlay_focus
                .entry(id.clone())
                .or_insert_with(|| cx.focus_handle());
        }
        if overlay != self.active_overlay {
            if self.active_overlay.is_none() && overlay.is_some() {
                self.return_focus = window.focused(cx);
            }
            if let Some(first) = stops.first() {
                window.focus(&self.overlay_focus[first], cx);
            } else if let Some(focus) = self.return_focus.take() {
                window.focus(&focus, cx);
            }
            self.active_overlay = overlay;
        }
        if window.focused(cx).is_none()
            && let Some(focus) = &self.root_focus
        {
            window.focus(focus, cx);
        }
        let page = self.page;
        let key = self.page_key();

        let indices = self.detail_indices();
        let tracks = self.tracks.clone();
        let albums = self.albums.clone();
        let artists = self.artists.clone();
        let album_indices = self.filtered_album_indices.clone();
        let artist_indices = self.filtered_artist_indices.clone();
        let count = match page {
            Page::Albums => album_indices.len(),
            Page::Artists => artist_indices.len(),
            Page::Queue => self.queue.len(),
            _ => indices.len(),
        };
        let scroll = self.scrolls.entry(key.clone()).or_default().clone();
        if count > 0
            && let Some(offset) = self.pending_offsets.remove(&key)
        {
            scroll
                .0
                .borrow()
                .base_handle
                .set_offset(point(px(0.), px(offset)));
        }
        let view = cx.weak_entity();
        let current = self.current_track.as_ref().map(|track| track.path.clone());
        let paused = self.paused;
        let selected = self.selected_track.clone();
        let sidebar_width = if self.sidebar_width > 0. {
            self.sidebar_width
        } else if self.sidebar_collapsed {
            64.
        } else {
            184.
        };
        let content_width = (f32::from(window.viewport_size().width)
            - sidebar_width
            - if self.queue_sidebar_open { 300. } else { 0. }
            - 48.)
            .max(180.);
        let columns = (content_width / 180.).floor().max(1.) as usize;
        let card_width = (content_width - 16. * (columns - 1) as f32) / columns as f32;
        self.table_available =
            (content_width - 24. - 34. - if page == Page::Queue { 108. } else { 32. }).max(100.);
        let ratios = if self.table_ratios == [0.; 4] {
            [0.44, 0.28, 0.20, 0.08]
        } else {
            self.table_ratios
        };
        let widths = ratios.map(|ratio| ratio * self.table_available);
        let artist_grid = page == Page::Artists;
        let grid = (page == Page::Albums && self.album_grid) || artist_grid;
        let row_count = if grid { count.div_ceil(columns) } else { count };
        let rows = uniform_list(
            key,
            row_count,
            cx.processor(move |this, range: std::ops::Range<usize>, _, _| {
                range
                    .filter_map(|index| match page {
                        Page::Albums if grid => Some(album_grid_row(
                            album_indices
                                .iter()
                                .skip(index * columns)
                                .take(columns)
                                .filter_map(|index| albums.get(*index))
                                .cloned()
                                .collect(),
                            index,
                            card_width,
                            p,
                            view.clone(),
                        )),
                        Page::Albums => album_indices
                            .get(index)
                            .and_then(|index| albums.get(*index))
                            .cloned()
                            .map(|album| album_row(album, index, p, view.clone())),
                        Page::Artists if artist_grid => Some(artist_grid_row(
                            artist_indices
                                .iter()
                                .skip(index * columns)
                                .take(columns)
                                .filter_map(|index| artists.get(*index))
                                .cloned()
                                .collect(),
                            index,
                            card_width,
                            p,
                            view.clone(),
                        )),
                        Page::Artists => artist_indices
                            .get(index)
                            .and_then(|index| artists.get(*index))
                            .cloned()
                            .map(|artist| artist_row(artist, index, p, view.clone())),
                        Page::Queue => this.queue.get(index).cloned().map(|track| {
                            track_row(
                                track,
                                TrackRowState {
                                    index,
                                    widths,
                                    numbered: true,
                                    show_artwork: true,
                                    album_detail: false,
                                    queued: true,
                                    queue_len: this.queue.len(),
                                    current: current.clone(),
                                    paused,
                                    selected: selected.clone(),
                                },
                                p,
                                view.clone(),
                            )
                        }),
                        _ => indices
                            .get(index)
                            .and_then(|index| tracks.get(*index))
                            .cloned()
                            .map(|track| {
                                track_row(
                                    track,
                                    TrackRowState {
                                        index,
                                        widths,
                                        numbered: matches!(page, Page::Tracks | Page::AlbumDetail),
                                        show_artwork: page != Page::AlbumDetail,
                                        album_detail: page == Page::AlbumDetail,
                                        queued: false,
                                        queue_len: 0,
                                        current: current.clone(),
                                        paused,
                                        selected: selected.clone(),
                                    },
                                    p,
                                    view.clone(),
                                )
                            }),
                    })
                    .collect()
            }),
        )
        .track_scroll(&scroll)
        .size_full();
        let body = div()
            .flex_1()
            .min_h_0()
            .flex()
            .child(self.sidebar(p, cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(self.header(p, cx))
                    .when(self.library_unavailable || self.status.is_some(), |el| {
                        el.child(self.status_banner(p, cx))
                    })
                    .child(self.library_content(rows, count, p, cx)),
            )
            .when(self.queue_sidebar_open, |el| {
                el.child(self.queue_sidebar(p, cx))
            });
        div()
            .relative()
            .size_full()
            .bg(rgb(p.bg))
            .text_color(rgb(p.text))
            .font_family("Helvetica Neue")
            .text_size(px(13.))
            .flex()
            .flex_col()
            .when_some(self.root_focus.as_ref(), |el, focus| el.track_focus(focus))
            .tab_group()
            .on_key_down(cx.listener(Self::key))
            .on_scroll_wheel(cx.listener(|this, _, _, cx| {
                if this.context_track.take().is_some() {
                    cx.notify();
                }
            }))
            .child(body)
            .child(self.player(p, cx))
            .when(self.menu_open, |el| el.child(self.application_menu(p, cx)))
            .when(self.statistics_open, |el| {
                el.child(self.statistics_panel(p, cx))
            })
            .when(self.settings_open, |el| {
                el.child(self.settings_modal(p, cx))
            })
            .when_some(self.context_track.clone(), |el, track| {
                el.child(self.track_context_menu(track, p, cx))
            })
            .when_some(self.artwork_modal.clone(), |el, path| {
                el.child(self.cover_modal(path, window, p, cx))
            })
    }
}
fn empty_state(title: &'static str, description: &'static str, p: Palette) -> Div {
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_3()
        .text_center()
        .child(
            div()
                .text_color(rgb(p.muted))
                .child(icon("music", 32., p.muted)),
        )
        .child(
            div()
                .text_size(px(18.))
                .font_weight(FontWeight::MEDIUM)
                .child(title),
        )
        .child(label(description, p))
}
#[derive(Clone)]
struct ColumnResize {
    column: usize,
    ratios: [f32; 4],
    available: f32,
}
impl Render for ColumnResize {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}
impl AppView {
    fn table_header(&self, p: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        if self.page == Page::AlbumDetail {
            return div()
                .id("track-table-header")
                .h(px(32.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .px_3()
                .border_b_1()
                .border_color(rgb(p.line))
                .text_size(px(10.))
                .text_color(rgb(p.muted))
                .child(div().w(px(34.)).flex_shrink_0().child("#"))
                .child(div().flex_1().min_w_0().px_2().child("TITLE / ARTIST"))
                .child(
                    div()
                        .w(px(88.))
                        .flex_shrink_0()
                        .px_2()
                        .text_right()
                        .child("LENGTH"),
                )
                .into_any_element();
        }
        let queued = self.page == Page::Queue;
        let ratios = if self.table_ratios == [0.; 4] {
            [0.44, 0.28, 0.20, 0.08]
        } else {
            self.table_ratios
        };
        let available = self.table_available.max(300.);
        let mut header = div()
            .id("track-table-header")
            .h(px(32.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .px_3()
            .border_b_1()
            .border_color(rgb(p.line))
            .text_size(px(10.))
            .text_color(rgb(p.muted))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.persist()),
            )
            .on_drag_move(
                cx.listener(|this, event: &DragMoveEvent<ColumnResize>, _, cx| {
                    let drag = event.drag(cx);
                    let boundary = f32::from(event.bounds.origin.x)
                        + 46.
                        + drag.ratios[..=drag.column].iter().sum::<f32>() * drag.available;
                    let delta = (f32::from(event.event.position.x) - boundary) / drag.available;
                    let col = drag.column;
                    let delta = delta.clamp(0.07 - drag.ratios[col], drag.ratios[col + 1] - 0.07);
                    this.table_ratios = drag.ratios;
                    this.table_ratios[col] += delta;
                    this.table_ratios[col + 1] -= delta;
                    cx.notify();
                }),
            )
            .child(div().w(px(34.)).flex_shrink_0().child("#"));
        for (column, mode, title) in [
            (0, SortMode::Title, "TITLE"),
            (1, SortMode::Album, "ALBUM"),
            (2, SortMode::Date, "DATE ADDED"),
        ] {
            let active = self.sort == mode;
            let reversed = self.sort_reversed;
            header = header.child(
                div()
                    .id(format!("column-{column}"))
                    .relative()
                    .w(px(ratios[column] * available))
                    .flex_shrink_0()
                    .h_full()
                    .flex()
                    .items_center()
                    .min_w_0()
                    .child(
                        div()
                            .id(format!("sort-column-{column}"))
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .flex()
                            .items_center()
                            .gap_1()
                            .px_2()
                            .cursor_pointer()
                            .tab_index(0)
                            .role(Role::Button)
                            .aria_label(format!("Sort by {title}"))
                            .text_color(rgb(if active { p.accent } else { p.muted }))
                            .hover(move |s| s.text_color(rgb(p.text)).bg(rgb(p.raised)))
                            .on_click(cx.listener(move |this, _, _, cx| this.set_sort(mode, cx)))
                            .child(title)
                            .when(active, |el| {
                                el.child(icon(
                                    if reversed ^ (mode == SortMode::Date) {
                                        "down"
                                    } else {
                                        "up"
                                    },
                                    12.,
                                    p.accent,
                                ))
                            }),
                    )
                    .when(column < 3, |el| {
                        el.child(
                            div()
                                .id(format!("resize-column-{column}"))
                                .absolute()
                                .right(px(0.))
                                .top_0()
                                .w(px(8.))
                                .h_full()
                                .cursor(CursorStyle::ResizeLeftRight)
                                .border_r_1()
                                .border_color(rgb(p.line))
                                .hover(move |s| s.bg(rgb(p.accent_bg)).border_color(rgb(p.accent)))
                                .on_drag(
                                    ColumnResize {
                                        column,
                                        ratios,
                                        available,
                                    },
                                    move |drag, _, _, cx| {
                                        let drag = drag.clone();
                                        cx.new(|_| drag)
                                    },
                                ),
                        )
                    }),
            );
        }
        header
            .child(
                div()
                    .id("sort-duration")
                    .w(px(ratios[3] * available))
                    .flex_shrink_0()
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_1()
                    .cursor_pointer()
                    .text_color(rgb(if self.sort == SortMode::Duration {
                        p.accent
                    } else {
                        p.muted
                    }))
                    .hover(move |s| s.bg(rgb(p.raised)))
                    .on_click(cx.listener(|this, _, _, cx| this.set_sort(SortMode::Duration, cx)))
                    .child(icon(
                        "clock",
                        14.,
                        if self.sort == SortMode::Duration {
                            p.accent
                        } else {
                            p.muted
                        },
                    ))
                    .when(self.sort == SortMode::Duration, |el| {
                        el.child(icon(
                            if self.sort_reversed { "down" } else { "up" },
                            12.,
                            p.accent,
                        ))
                    }),
            )
            .when(queued, |el| el.child(div().w(px(108.)).flex_shrink_0()))
            .into_any_element()
    }
}
fn queue_sidebar_row(
    track: Track,
    index: usize,
    p: Palette,
    view: WeakEntity<AppView>,
) -> AnyElement {
    let drag_view = view.clone();
    let play_view = view.clone();
    let remove_view = view;
    div()
        .id(format!("queue-sidebar-{index}"))
        .debug_selector(move || format!("queue-row-{index}"))
        .w_full()
        .h(px(66.))
        .px_3()
        .flex()
        .items_center()
        .gap_2()
        .border_b_1()
        .border_color(rgb(p.line))
        .cursor_pointer()
        .hover(move |s| s.bg(rgb(p.raised)))
        .on_click(move |event, _, app| {
            if event.click_count() > 1 {
                let _ = play_view.update(app, |this, cx| this.play_queued_track(index, cx));
            }
        })
        .on_drag(index, |index, _, _, cx| cx.new(|_| QueueDrag(*index)))
        .on_drop(move |from: &QueueDrag, _, app| {
            let _ = drag_view.update(app, |this, cx| this.reorder_queue(from.0, index, cx));
        })
        .child(div().flex_shrink_0().child(icon("drag", 14., p.muted)))
        .child(artwork(track.cover_path, track.title.clone(), 40., p))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().truncate().child(track.title))
                .child(
                    div()
                        .text_color(rgb(p.muted))
                        .text_size(px(12.))
                        .truncate()
                        .child(track.artist),
                ),
        )
        .child(div().flex_shrink_0().child(icon_button(
            format!("remove-queued-{index}"),
            "close",
            "Remove from queue",
            p,
            true,
            false,
            move |_, _, app| {
                app.stop_propagation();
                let _ = remove_view.update(app, |this, cx| this.remove_queued_track(index, cx));
            },
        )))
        .into_any_element()
}
struct QueueDrag(usize);
impl Render for QueueDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}
struct TrackRowState {
    widths: [f32; 4],
    numbered: bool,
    show_artwork: bool,
    album_detail: bool,
    index: usize,
    queued: bool,
    queue_len: usize,
    current: Option<PathBuf>,
    paused: bool,
    selected: Option<PathBuf>,
}
fn track_row(
    track: Track,
    state: TrackRowState,
    p: Palette,
    view: WeakEntity<AppView>,
) -> AnyElement {
    let TrackRowState {
        widths,
        numbered,
        show_artwork,
        album_detail,
        index,
        queued,
        queue_len,
        current,
        paused,
        selected,
    } = state;
    let is_current = current.as_ref() == Some(&track.path);
    let is_selected = selected.as_ref() == Some(&track.path);
    let context_track_for_menu = track.clone();
    let row_path = track.path.clone();
    let row_track = track.clone();
    let row_view = view.clone();
    let up_view = view.clone();
    let down_view = view.clone();
    let add_view = view.clone();
    let album = track.album.clone();
    let album_artist = album_artist(&track);
    let album_view = view.clone();
    let artist = crate::library::split_artist_credits(&track.artist)
        .into_iter()
        .next()
        .unwrap_or_default();
    let artist_view = view.clone();
    div()
        .id(format!("track-{index}"))
        .group(format!("track-row-{index}"))
        .h(px(56.))
        .w_full()
        .flex()
        .items_center()
        .px_3()
        .border_b_1()
        .border_color(rgb(p.line))
        .bg(rgb(if is_selected {
            p.raised
        } else if is_current {
            p.accent_bg
        } else {
            p.bg
        }))
        .hover(move |s| s.bg(rgb(p.raised)))
        .on_mouse_down(MouseButton::Right, {
            let target = view.clone();
            let track = track.clone();
            move |event, _, cx| {
                let _ = target.update(cx, |this, cx| {
                    this.context_menu_position = event.position;
                    this.context_track = Some(track.clone());
                    cx.notify();
                });
                cx.stop_propagation();
            }
        })
        .on_click(move |event, window, cx| {
            let _ = row_view.update(cx, |this, cx| {
                this.selected_track = Some(row_path.clone());
                if let Some(focus) = &this.root_focus {
                    window.focus(focus, cx);
                }
                if event.click_count() > 1 {
                    this.play_track(row_track.clone(), cx);
                }
                cx.notify();
            });
        })
        .child(
            div()
                .w(px(34.))
                .flex_shrink_0()
                .text_size(px(10.))
                .text_color(rgb(if is_current { p.accent } else { p.muted }))
                .child(if is_current && !numbered {
                    icon(if paused { "pause" } else { "volume" }, 13., p.accent).into_any_element()
                } else {
                    div()
                        .child(if !numbered && track.track_number > 0 {
                            track.track_number.to_string()
                        } else {
                            (index + 1).to_string()
                        })
                        .into_any_element()
                }),
        )
        .child(
            div()
                .w(px(if album_detail {
                    widths.iter().sum::<f32>() - 88.
                } else {
                    widths[0]
                }))
                .flex_shrink_0()
                .px_2()
                .min_w_0()
                .flex()
                .flex_row()
                .items_center()
                .when(show_artwork, |el| el.gap_3())
                .when(show_artwork, |el| {
                    el.child(artwork(
                        track.cover_path.clone(),
                        track.title.clone(),
                        36.,
                        p,
                    ))
                })
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(rgb(if is_current { p.accent } else { p.text }))
                                .truncate()
                                .child(track.title),
                        )
                        .child(
                            div()
                                .id(format!("artist-link-{index}"))
                                .text_size(px(11.))
                                .text_color(rgb(p.muted))
                                .cursor_pointer()
                                .hover(move |s| s.text_color(rgb(p.accent)))
                                .truncate()
                                .on_click(move |_, _, cx| {
                                    cx.stop_propagation();
                                    let _ = artist_view.update(cx, |this, cx| {
                                        this.open_artist(artist.clone(), cx)
                                    });
                                })
                                .child(track.artist.replace(';', " ·")),
                        ),
                ),
        )
        .when(!album_detail, |el| {
            el.child(
                div()
                    .id(format!("album-link-{index}"))
                    .w(px(widths[1]))
                    .flex_shrink_0()
                    .px_2()
                    .text_size(px(11.))
                    .text_color(rgb(p.muted))
                    .cursor_pointer()
                    .hover(move |s| s.text_color(rgb(p.accent)))
                    .truncate()
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        let _ = album_view.update(cx, |this, cx| {
                            this.open_album(album.clone(), album_artist.clone(), cx)
                        });
                    })
                    .child(track.album.clone()),
            )
        })
        .when(!album_detail, |el| {
            el.child(
                div()
                    .w(px(widths[2]))
                    .flex_shrink_0()
                    .px_2()
                    .text_size(px(11.))
                    .text_color(rgb(p.muted))
                    .truncate()
                    .child(format_added(track.added_unix_ms, unix_now_ms())),
            )
        })
        .child(
            div()
                .w(px(if album_detail { 88. } else { widths[3] }))
                .flex_shrink_0()
                .px_2()
                .text_size(px(11.))
                .text_right()
                .text_color(rgb(p.muted))
                .child(duration(track.duration_ms)),
        )
        .when(!queued, |el| {
            let target = view.clone();
            let context_track = context_track_for_menu.clone();
            el.child(
                div()
                    .opacity(0.)
                    .group_hover(format!("track-row-{index}"), |s| s.opacity(1.))
                    .child(div().w(px(32.)).flex_shrink_0().flex().justify_end().child(
                        icon_button(
                            format!("track-menu-{index}"),
                            "more",
                            "Track actions",
                            p,
                            true,
                            false,
                            move |event, _, cx| {
                                cx.stop_propagation();
                                let _ = target.update(cx, |this, cx| {
                                    this.context_menu_position = event.position();
                                    this.context_track = Some(context_track.clone());
                                    cx.notify();
                                });
                            },
                        ),
                    )),
            )
        })
        .when(queued, |el| {
            el.child(
                div()
                    .w(px(108.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap_1()
                    .when(queued, |el| {
                        el.child(icon_button(
                            format!("queue-up-{index}"),
                            "up",
                            "Move earlier",
                            p,
                            index > 0,
                            false,
                            move |_, _, cx| {
                                cx.stop_propagation();
                                let _ = up_view
                                    .update(cx, |this, cx| this.move_queued_track(index, -1, cx));
                            },
                        ))
                        .child(icon_button(
                            format!("queue-down-{index}"),
                            "down",
                            "Move later",
                            p,
                            index + 1 < queue_len,
                            false,
                            move |_, _, cx| {
                                cx.stop_propagation();
                                let _ = down_view
                                    .update(cx, |this, cx| this.move_queued_track(index, 1, cx));
                            },
                        ))
                    })
                    .child(icon_button(
                        format!("row-queue-{index}"),
                        if queued { "close" } else { "plus" },
                        if queued {
                            "Remove from queue"
                        } else {
                            "Add to queue"
                        },
                        p,
                        true,
                        false,
                        move |_, _, cx| {
                            cx.stop_propagation();
                            let _ = add_view.update(cx, |this, cx| {
                                this.remove_queued_track(index, cx);
                            });
                        },
                    )),
            )
        })
        .into_any_element()
}
fn album_row(album: Album, index: usize, p: Palette, view: WeakEntity<AppView>) -> AnyElement {
    let title = album.title.clone();
    let artist = album.artist.clone();
    div()
        .id(format!("album-{index}"))
        .debug_selector(move || format!("album-row-{index}"))
        .tab_index(0)
        .role(Role::Button)
        .focus_visible(move |style| style.bg(rgb(p.raised)))
        .w_full()
        .h(px(94.))
        .flex()
        .items_center()
        .gap_4()
        .px_3()
        .border_b_1()
        .border_color(rgb(p.line))
        .cursor_pointer()
        .hover(move |s| s.bg(rgb(p.panel)))
        .on_click(move |_, _, cx| {
            let _ = view.update(cx, |this, cx| {
                this.open_album(title.clone(), artist.clone(), cx)
            });
        })
        .child(artwork(album.cover_path, album.title.clone(), 70., p))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_size(px(15.))
                        .font_weight(FontWeight::MEDIUM)
                        .truncate()
                        .child(album.title),
                )
                .child(label(album.artist, p)),
        )
        .child(label(format!("{} tracks", album.track_indices.len()), p))
        .child(icon("forward", 14., p.muted))
        .into_any_element()
}
fn artist_row(artist: Artist, index: usize, p: Palette, view: WeakEntity<AppView>) -> AnyElement {
    let name = artist.name.clone();
    div()
        .id(format!("artist-{index}"))
        .tab_index(0)
        .role(Role::Button)
        .focus_visible(move |style| style.bg(rgb(p.raised)))
        .w_full()
        .h(px(82.))
        .flex()
        .items_center()
        .gap_4()
        .px_3()
        .border_b_1()
        .border_color(rgb(p.line))
        .cursor_pointer()
        .hover(move |s| s.bg(rgb(p.panel)))
        .on_click(move |_, _, cx| {
            let _ = view.update(cx, |this, cx| this.open_artist(name.clone(), cx));
        })
        .child(artwork(artist.cover_path, artist.name.clone(), 58., p))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(15.))
                .font_weight(FontWeight::MEDIUM)
                .truncate()
                .child(artist.name),
        )
        .child(label(
            format!(
                "{} albums · {} tracks",
                artist.album_count,
                artist.track_indices.len()
            ),
            p,
        ))
        .child(icon("forward", 14., p.muted))
        .into_any_element()
}
fn artist_grid_row(
    artists: Vec<Artist>,
    row: usize,
    width: f32,
    p: Palette,
    view: WeakEntity<AppView>,
) -> AnyElement {
    div()
        .id(format!("artist-grid-row-{row}"))
        .w_full()
        .flex()
        .gap_4()
        .py_3()
        .children(artists.into_iter().enumerate().map(|(column, artist)| {
            let width = ((column + 1) as f32 * width).round() - (column as f32 * width).round();
            let name = artist.name.clone();
            let target = view.clone();
            div()
                .id(format!("artist-card-{row}-{name}"))
                .w(px(width))
                .flex_shrink_0()
                .flex()
                .flex_col()
                .items_center()
                .gap_2()
                .p_3()
                .rounded_lg()
                .cursor_pointer()
                .hover(move |s| s.bg(rgb(p.raised)))
                .on_click(move |_, _, cx| {
                    let _ = target.update(cx, |this, cx| this.open_artist(name.clone(), cx));
                })
                .child(
                    div()
                        .size(px(96.))
                        .rounded_full()
                        .overflow_hidden()
                        .child(artwork(artist.cover_path, artist.name.clone(), 96., p)),
                )
                .child(
                    div()
                        .w_full()
                        .text_center()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .truncate()
                        .child(artist.name),
                )
                .child(label(
                    format!(
                        "{} albums · {} tracks",
                        artist.album_count,
                        artist.track_indices.len()
                    ),
                    p,
                ))
        }))
        .into_any_element()
}
struct LibrarySnapshot {
    skipped_files: usize,
    tracks: Arc<Vec<Track>>,
    albums: Arc<Vec<Album>>,
    artists: Arc<Vec<Artist>>,
    search_terms: Arc<Vec<String>>,
    date_order: Arc<Vec<usize>>,
    title_order: Arc<Vec<usize>>,
    artist_order: Arc<Vec<usize>>,
    album_order: Arc<Vec<usize>>,
    duration_order: Arc<Vec<usize>>,
}
impl LibrarySnapshot {
    fn new(tracks: Vec<Track>) -> Self {
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
fn scan_and_cache(
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

#[cfg(test)]
mod tests {
    use super::*;
    fn test_tracks() -> Vec<Track> {
        (0..4)
            .map(|index| Track {
                path: PathBuf::from(format!("track-{index}")),
                ..Track::default()
            })
            .collect()
    }
    #[::core::prelude::v1::test]
    fn shuffled_context_visits_every_other_track_once() {
        let tracks = test_tracks();
        let mut view = AppView {
            shuffle: true,
            playback_order: tracks.clone(),
            ..AppView::default()
        };
        view.reset_continuation(&tracks[2].path);
        let mut actual: Vec<_> = view
            .continuation
            .iter()
            .map(|track| track.path.clone())
            .collect();
        actual.sort();
        assert_eq!(
            actual,
            vec![
                tracks[0].path.clone(),
                tracks[1].path.clone(),
                tracks[3].path.clone()
            ]
        );
        view.shuffle = false;
        view.reset_continuation(&tracks[2].path);
        assert_eq!(view.continuation.len(), 1);
        assert_eq!(view.continuation[0].path, tracks[3].path);
    }
    #[::core::prelude::v1::test]
    fn repeat_one_only_affects_automatic_completion_and_preserves_queue() {
        let tracks = test_tracks();
        let mut view = AppView {
            repeat: RepeatMode::One,
            current_track: Some(tracks[0].clone()),
            queue: vec![tracks[1].clone()].into(),
            continuation: vec![tracks[2].clone()].into(),
            ..AppView::default()
        };
        assert_eq!(view.next_track(true).unwrap().path, tracks[0].path);
        assert_eq!(view.queue.len(), 1);
        assert_eq!(view.next_track(false).unwrap().path, tracks[1].path);
        assert_eq!(view.next_track(false).unwrap().path, tracks[2].path);
        assert!(view.next_track(false).is_none());
    }
    #[::core::prelude::v1::test]
    fn repeat_all_restarts_context_and_shuffle_avoids_boundary_duplicate() {
        let tracks = test_tracks();
        let mut view = AppView {
            playback_order: tracks.clone(),
            current_track: Some(tracks[3].clone()),
            ..AppView::default()
        };
        assert!(view.next_track(true).is_none());
        view.repeat = RepeatMode::All;
        assert_eq!(view.next_track(true).unwrap().path, tracks[0].path);
        assert_eq!(view.continuation.len(), 3);
        view.continuation.clear();
        view.shuffle = true;
        let first = view.next_track(true).unwrap();
        assert_ne!(first.path, tracks[3].path);
        assert_eq!(view.continuation.len(), 3);
    }

    #[::core::prelude::v1::test]
    fn unavailable_library_never_produces_a_cache_replacement() {
        let missing = std::env::temp_dir().join(format!(
            "flacpac-missing-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert!(scan_and_cache(missing).is_err());
    }
    #[::core::prelude::v1::test]
    fn large_catalog_preparation_and_search() {
        for count in [1000, 10_000, 100_000] {
            let tracks = (0..count)
                .map(|index| Track {
                    title: format!("Track {index}"),
                    artist: format!("Artist {}", index % 100),
                    album: format!("Album {}", index / 10),
                    added_unix_ms: index as u128,
                    ..Track::default()
                })
                .collect();
            let start = std::time::Instant::now();
            let snapshot = LibrarySnapshot::new(tracks);
            let preparation = start.elapsed();
            let start = std::time::Instant::now();
            let matches = snapshot
                .search_terms
                .iter()
                .filter(|text| text.contains("track 999"))
                .count();
            let search = start.elapsed();
            assert!(matches > 0);
            assert_eq!(snapshot.date_order[0], count - 1);
            assert_eq!(snapshot.tracks.len(), count);
            eprintln!("{count} tracks: prepare {preparation:?}; search {search:?}");
        }
    }
}

fn album_grid_row(
    albums: Vec<Album>,
    row: usize,
    width: f32,
    p: Palette,
    view: WeakEntity<AppView>,
) -> AnyElement {
    div()
        .h(px(width + 88.))
        .w_full()
        .flex()
        .items_start()
        .gap_4()
        .children(albums.into_iter().enumerate().map(|(column, album)| {
            let width = ((column + 1) as f32 * width).round() - (column as f32 * width).round();
            let view = view.clone();
            let title = album.title.clone();
            let artist = album.artist.clone();
            div()
                .id(format!("album-card-{row}-{column}"))
                .debug_selector(move || format!("album-card-{row}-{column}"))
                .w(px(width))
                .flex_shrink_0()
                .rounded_md()
                .cursor_pointer()
                .tab_index(0)
                .role(Role::Button)
                .hover(move |style| style.bg(rgb(p.panel)))
                .focus_visible(move |style| style.bg(rgb(p.raised)))
                .on_click(move |_, _, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.open_album(title.clone(), artist.clone(), cx)
                    });
                })
                .child(artwork(album.cover_path, album.title.clone(), width, p))
                .child(
                    div()
                        .mt_2()
                        .px_3()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .truncate()
                        .child(album.title),
                )
                .child(
                    div()
                        .mt_1()
                        .px_3()
                        .pb_3()
                        .text_size(px(11.))
                        .text_color(rgb(p.muted))
                        .truncate()
                        .child(album.artist),
                )
        }))
        .into_any_element()
}

impl AppView {
    fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_collapsed = !self.sidebar_collapsed;
        let start = if self.sidebar_width > 0. {
            self.sidebar_width
        } else {
            184.
        };
        let target = if self.sidebar_collapsed { 64. } else { 184. };
        self.sidebar_animation = Some(cx.spawn(async move |view, cx| {
            let started = std::time::Instant::now();
            loop {
                let t = (started.elapsed().as_secs_f32() / 0.18).min(1.);
                let eased = 1. - (1. - t).powi(3);
                if view
                    .update(cx, |this, cx| {
                        this.sidebar_width = start + (target - start) * eased;
                        cx.notify();
                    })
                    .is_err()
                    || t >= 1.
                {
                    break;
                }
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
            }
        }));
        cx.notify();
    }
}

impl AppView {
    fn sidebar(&self, p: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let active = match self.page {
            Page::AlbumDetail => Page::Albums,
            Page::ArtistDetail => Page::Artists,
            page => page,
        };
        let collapsed = self.sidebar_collapsed;
        let nav = [
            (Page::Tracks, "music", "All tracks"),
            (Page::Albums, "album", "Albums"),
            (Page::Artists, "artist", "Artists"),
        ];
        div()
            .debug_selector(|| "sidebar".into())
            .group("sidebar")
            .w(px(if self.sidebar_width > 0. {
                self.sidebar_width
            } else if collapsed {
                64.
            } else {
                184.
            }))
            .overflow_hidden()
            .flex_shrink_0()
            .bg(rgb(p.panel))
            .border_r_1()
            .border_color(rgb(p.line))
            .p_3()
            .flex()
            .flex_col()
            .child(
                div()
                    .relative()
                    .h(px(42.))
                    .mb_6()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .group("sidebar-logo")
                    .child(
                        div().when(collapsed, |el| el.opacity(0.)).child(
                            svg()
                                .path("brand")
                                .w(px(36.))
                                .h(px(34.))
                                .flex_shrink_0()
                                .text_color(rgb(p.text)),
                        ),
                    )
                    .when(!collapsed, |el| {
                        el.child(
                            div()
                                .text_size(px(19.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("flacpac"),
                        )
                    })
                    .when(collapsed, |el| {
                        el.child(div().absolute().left(px(4.)).top(px(5.)).child(icon_button(
                            "expand-sidebar",
                            "expand",
                            "Expand sidebar",
                            p,
                            true,
                            false,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_sidebar(cx);
                            }),
                        )))
                    })
                    .when(!collapsed, |el| {
                        el.child(div().child(icon_button(
                            "collapse-sidebar",
                            "collapse",
                            "Collapse sidebar",
                            p,
                            true,
                            false,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_sidebar(cx);
                            }),
                        )))
                    }),
            )
            .children(nav.into_iter().map(|(target, name, title)| {
                div()
                    .id(format!("nav-{target:?}"))
                    .tab_index(0)
                    .role(Role::Button)
                    .aria_label(title)
                    .focus_visible(move |style| style.bg(rgb(p.raised)))
                    .h(px(38.))
                    .px_3()
                    .mb_1()
                    .rounded_md()
                    .flex()
                    .items_center()
                    .gap_3()
                    .cursor_pointer()
                    .bg(rgb(if active == target {
                        p.accent_bg
                    } else {
                        p.panel
                    }))
                    .text_color(rgb(if active == target { p.accent } else { p.muted }))
                    .text_size(px(13.))
                    .hover(move |style| style.bg(rgb(p.raised)).text_color(rgb(p.text)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if target == Page::Queue {
                            this.queue_sidebar_open = !this.queue_sidebar_open;
                            cx.notify();
                        } else {
                            this.set_page(target, cx);
                        }
                    }))
                    .child(icon(
                        name,
                        17.,
                        if active == target { p.accent } else { p.muted },
                    ))
                    .when(!collapsed, |el| el.child(title))
                    .tooltip(move |_, cx| cx.new(|_| Tip(title.into(), p)).into())
            }))
            .child(
                div()
                    .mt_auto()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .when(!collapsed, |el| {
                        el.child(text_button(
                            "library-folder",
                            "folder",
                            "Music folder",
                            p,
                            false,
                            cx.listener(|this, _, _, cx| this.choose_folder(cx)),
                        ))
                    })
                    .when(collapsed, |el| {
                        el.child(icon_button(
                            "library-folder",
                            "folder",
                            "Music folder",
                            p,
                            true,
                            false,
                            cx.listener(|this, _, _, cx| this.choose_folder(cx)),
                        ))
                    }),
            )
    }
    fn header(&self, p: Palette, cx: &mut Context<Self>) -> Div {
        div()
            .debug_selector(|| "app-header".into())
            .h(px(66.))
            .flex_shrink_0()
            .px_6()
            .border_b_1()
            .border_color(rgb(p.line))
            .flex()
            .items_center()
            .gap_3()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap_2()
                    .when(!self.navigation.is_empty(), |el| {
                        el.child(icon_button(
                            "back",
                            "back",
                            "Go back",
                            p,
                            true,
                            false,
                            cx.listener(|this, _, _, cx| this.back(cx)),
                        ))
                    })
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_size(px(19.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(self.title()),
                    ),
            )
            .when_some(self.search.clone(), |el, search| el.child(search))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(icon_button(
                        "add-folder",
                        "folder",
                        "Choose music folder",
                        p,
                        true,
                        false,
                        cx.listener(|this, _, _, cx| this.choose_folder(cx)),
                    ))
                    .child(
                        icon_button(
                            "app-menu",
                            "menu",
                            "Application menu",
                            p,
                            true,
                            self.menu_open,
                            cx.listener(|this, _, _, cx| {
                                this.menu_open = !this.menu_open;
                                cx.notify();
                            }),
                        )
                        .track_focus(&self.overlay_focus["app-menu"]),
                    ),
            )
    }
    fn toolbar(&self, count: usize, p: Palette, cx: &mut Context<Self>) -> Div {
        let description = match self.page {
            Page::Tracks => self
                .root
                .as_ref()
                .and_then(|path| path.file_name())
                .map(|name| format!("{} · {count} tracks", name.to_string_lossy()))
                .unwrap_or("Your local music collection".into()),
            Page::Albums => format!("{count} albums"),
            Page::Artists => format!("{count} artists"),
            _ => format!("{count} tracks up next"),
        };
        div()
            .h(px(56.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .child(label(description, p))
            .when(self.page == Page::Tracks, |el| {
                el.child(
                    div()
                        .flex()
                        .gap_2()
                        .child(text_button(
                            "play-library",
                            "play",
                            "Play",
                            p,
                            true,
                            cx.listener(|this, _, _, cx| this.play_collection(false, cx)),
                        ))
                        .child(text_button(
                            "shuffle-library",
                            "shuffle",
                            "Shuffle",
                            p,
                            false,
                            cx.listener(|this, _, _, cx| this.play_collection(true, cx)),
                        )),
                )
            })
            .when(self.page == Page::Albums, |el| {
                el.child(icon_button(
                    "album-layout",
                    if self.album_grid { "list" } else { "grid" },
                    "Switch album layout",
                    p,
                    true,
                    false,
                    cx.listener(|this, _, _, cx| {
                        this.album_grid = !this.album_grid;
                        this.persist();
                        cx.notify();
                    }),
                ))
            })
            .when(self.page == Page::Queue && !self.queue.is_empty(), |el| {
                el.child(text_button(
                    "clear-queue",
                    "close",
                    "Clear queue",
                    p,
                    false,
                    cx.listener(|this, _, _, cx| {
                        this.queue.clear();
                        this.pending_queue.clear();
                        this.persist();
                        cx.notify();
                    }),
                ))
            })
    }
    fn library_content(
        &self,
        rows: impl IntoElement,
        count: usize,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> Div {
        let detail = matches!(self.page, Page::AlbumDetail | Page::ArtistDetail);
        let (empty_title, empty_description, action) = self.empty_content();
        div()
            .flex_1()
            .min_h_0()
            .px_6()
            .flex()
            .flex_col()
            .when(detail, |el| el.child(self.hero(p, cx)))
            .when(!detail, |el| el.child(self.toolbar(count, p, cx)))
            .when(
                !matches!(self.page, Page::Albums | Page::Artists) && count > 0,
                |el| el.child(self.table_header(p, cx)),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .when(count > 0, |el| el.child(rows))
                    .when(count == 0, |el| {
                        el.child(
                            div()
                                .size_full()
                                .flex()
                                .flex_col()
                                .items_center()
                                .justify_center()
                                .gap_3()
                                .child(empty_state(empty_title, empty_description, p).size_auto())
                                .when_some(action, |el, action| {
                                    el.child(text_button(
                                        "empty-action",
                                        if action == "Clear search" {
                                            "close"
                                        } else {
                                            "folder"
                                        },
                                        action,
                                        p,
                                        true,
                                        cx.listener(move |this, _, _, cx| {
                                            if action == "Clear search" {
                                                if let Some(search) = &this.search {
                                                    search
                                                        .update(cx, |search, cx| search.clear(cx));
                                                }
                                            } else {
                                                this.choose_folder(cx);
                                            }
                                        }),
                                    ))
                                }),
                        )
                    }),
            )
    }
}

#[cfg(test)]
mod layout_tests {
    use super::{AppView, LibrarySnapshot, Page};
    use crate::{library::Track, search::SearchInput};
    use gpui::{AppContext, TestAppContext, px, size};

    #[gpui::test]
    fn compact_and_wide_layouts_keep_search_and_player_inside_the_window(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| {
            let search = cx.new(SearchInput::new);
            let mut view = AppView {
                search: Some(search),
                root_focus: Some(cx.focus_handle()),
                album_grid: true,
                ..AppView::default()
            };
            view.prepare_tracks(LibrarySnapshot::new(
                (0..100)
                    .map(|index| Track {
                        title: format!("A very long track title {index}"),
                        artist: "Artist".into(),
                        album: format!("Album {}", index / 10),
                        track_number: index % 10 + 1,
                        ..Track::default()
                    })
                    .collect(),
            ));
            view
        });
        for light in [false, true] {
            for (width, height) in [(960., 640.), (1440., 900.)] {
                cx.simulate_resize(size(px(width), px(height)));
                cx.update(|_, cx| {
                    view.update(cx, |view, cx| {
                        view.light = light;
                        view.page = Page::Albums;
                        cx.notify();
                    })
                });
                cx.run_until_parked();
                let player = cx.debug_bounds("player-bar").expect("player rendered");
                let header = cx.debug_bounds("app-header").expect("header rendered");
                let search = cx.debug_bounds("search-field").expect("search rendered");
                let sidebar = cx.debug_bounds("sidebar").expect("sidebar rendered");
                assert!(player.bottom() <= px(height) + px(1.));
                assert!((player.size.height - px(94.)).abs() < px(1.));
                assert!(search.left() >= header.left());
                assert!(search.right() <= header.right());
                assert!(
                    ((search.left() + search.right()) / 2. - (header.left() + header.right()) / 2.)
                        .abs()
                        < px(1.)
                );
                assert!((header.left() - sidebar.right()).abs() < px(1.));
                cx.update(|_, cx| {
                    view.update(cx, |view, cx| {
                        view.artwork_modal = Some(std::path::PathBuf::from("/missing-cover.png"));
                        cx.notify();
                    })
                });
                cx.run_until_parked();
                let art = cx.debug_bounds("expanded-artwork").expect("modal artwork");
                assert!(art.left() >= px(0.) && art.top() >= px(0.));
                assert!(art.right() <= px(width) && art.bottom() <= px(height));
                cx.update(|_, cx| {
                    view.update(cx, |view, cx| {
                        view.artwork_modal = None;
                        cx.notify();
                    })
                });
            }
        }
    }
}
impl AppView {
    fn application_menu(&self, p: Palette, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("application-menu")
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                this.menu_open = false;
                cx.notify();
                cx.stop_propagation();
            }))
            .absolute()
            .top(px(58.))
            .right(px(24.))
            .w(px(230.))
            .p_2()
            .bg(rgb(p.raised))
            .border_1()
            .border_color(rgb(p.line))
            .rounded_lg()
            .shadow_lg()
            .child(
                div()
                    .px_3()
                    .py_2()
                    .text_size(px(11.))
                    .text_color(rgb(p.muted))
                    .child("APPLICATION"),
            )
            .child(
                div()
                    .id("menu-statistics")
                    .track_focus(&self.overlay_focus["menu-statistics"])
                    .tab_index(0)
                    .role(Role::Button)
                    .focus_visible(move |s| s.bg(rgb(p.accent_bg)))
                    .px_3()
                    .py_3()
                    .rounded_md()
                    .cursor_pointer()
                    .hover(move |s| s.bg(rgb(p.accent_bg)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.menu_open = false;
                        this.statistics_open = true;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(icon("statistics", 16., p.text))
                            .child("Listening statistics"),
                    ),
            )
            .child(
                div()
                    .id("menu-settings")
                    .track_focus(&self.overlay_focus["menu-settings"])
                    .tab_index(0)
                    .role(Role::Button)
                    .focus_visible(move |s| s.bg(rgb(p.accent_bg)))
                    .px_3()
                    .py_3()
                    .rounded_md()
                    .cursor_pointer()
                    .hover(move |s| s.bg(rgb(p.accent_bg)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.menu_open = false;
                        this.settings_open = true;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(icon("settings", 16., p.text))
                            .child("Settings"),
                    ),
            )
            .into_any_element()
    }
    fn settings_modal(&self, p: Palette, cx: &mut Context<Self>) -> AnyElement {
        let about = self.settings_about;
        div()
            .id("settings-modal")
            .absolute()
            .inset_0()
            .occlude()
            .bg(rgba(0x000000bb))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(700.))
                    .h(px(440.))
                    .max_w(relative(0.92))
                    .flex()
                    .rounded_lg()
                    .overflow_hidden()
                    .bg(rgb(p.panel))
                    .border_1()
                    .border_color(rgb(p.line))
                    .child(
                        div()
                            .w(px(180.))
                            .flex_shrink_0()
                            .p_4()
                            .border_r_1()
                            .border_color(rgb(p.line))
                            .child(
                                div()
                                    .mb_6()
                                    .text_size(px(20.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Settings"),
                            )
                            .children([(false, "Appearance"), (true, "About")].into_iter().map(
                                |(tab, title)| {
                                    div()
                                        .id(format!("settings-tab-{tab}"))
                                        .aria_selected(about == tab)
                                        .track_focus(
                                            &self.overlay_focus[&format!("settings-tab-{tab}")],
                                        )
                                        .tab_index(0)
                                        .role(Role::Button)
                                        .focus_visible(move |s| s.bg(rgb(p.raised)))
                                        .p_3()
                                        .mb_1()
                                        .rounded_md()
                                        .bg(rgb(if about == tab { p.raised } else { p.panel }))
                                        .cursor_pointer()
                                        .hover(move |s| s.bg(rgb(p.raised)))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.settings_about = tab;
                                            cx.notify();
                                        }))
                                        .child(title)
                                },
                            )),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .p_6()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .mb_5()
                                    .child(
                                        div()
                                            .text_size(px(22.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(if about { "About" } else { "Appearance" }),
                                    )
                                    .child(
                                        icon_button(
                                            "close-settings",
                                            "close",
                                            "Close settings",
                                            p,
                                            true,
                                            false,
                                            cx.listener(|this, _, _, cx| {
                                                this.settings_open = false;
                                                cx.notify();
                                            }),
                                        )
                                        .track_focus(&self.overlay_focus["close-settings"]),
                                    ),
                            )
                            .when(about, |el| {
                                el.child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_3()
                                        .child(
                                            div()
                                                .text_size(px(20.))
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .child("Flacpac"),
                                        )
                                        .child(label(
                                            concat!("Version ", env!("CARGO_PKG_VERSION")),
                                            p,
                                        ))
                                        .child(label("Local music player", p))
                                        .child(label(
                                            format!(
                                                "Platform: {} · {}",
                                                std::env::consts::OS,
                                                std::env::consts::ARCH
                                            ),
                                            p,
                                        ))
                                        .child(label(
                                            format!("Library: {} tracks", self.tracks.len()),
                                            p,
                                        )),
                                )
                            })
                            .when(!about, |el| {
                                el.child(label("Appearance mode", p))
                                    .child(div().flex().gap_2().mt_2().mb_5().children(
                                        [(false, "Dark"), (true, "Light")].into_iter().map(
                                            |(light, title)| {
                                                div()
                                                    .id(format!("settings-mode-{light}"))
                                                    .aria_toggled(if self.light == light {
                                                        Toggled::True
                                                    } else {
                                                        Toggled::False
                                                    })
                                                    .track_focus(
                                                        &self.overlay_focus
                                                            [&format!("settings-mode-{light}")],
                                                    )
                                                    .tab_index(0)
                                                    .role(Role::Button)
                                                    .focus_visible(move |s| s.bg(rgb(p.raised)))
                                                    .flex_1()
                                                    .p_2()
                                                    .rounded_md()
                                                    .border_1()
                                                    .border_color(rgb(if self.light == light {
                                                        p.accent
                                                    } else {
                                                        p.line
                                                    }))
                                                    .cursor_pointer()
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        this.light = light;
                                                        if let Some(search) = &this.search {
                                                            search.update(cx, |search, cx| {
                                                                search.light = light;
                                                                cx.notify();
                                                            });
                                                        }
                                                        this.persist();
                                                        cx.notify();
                                                    }))
                                                    .child(title)
                                            },
                                        ),
                                    ))
                                    .child(label("Color palette", p))
                                    .child(div().mt_3().grid().grid_cols(2).gap_2().children(
                                        crate::settings::PALETTES.into_iter().map(|name| {
                                            let dark = name.definition(false);
                                            let light = name.definition(true);
                                            let current = self.palette == name;
                                            div()
                                                .id(format!("palette-{}", name.as_str()))
                                                .aria_toggled(if current {
                                                    Toggled::True
                                                } else {
                                                    Toggled::False
                                                })
                                                .track_focus(
                                                    &self.overlay_focus
                                                        [&format!("palette-{}", name.as_str())],
                                                )
                                                .tab_index(0)
                                                .role(Role::Button)
                                                .focus_visible(move |s| s.bg(rgb(p.raised)))
                                                .flex()
                                                .items_center()
                                                .gap_3()
                                                .p_2()
                                                .rounded_md()
                                                .border_1()
                                                .border_color(rgb(if current {
                                                    p.accent
                                                } else {
                                                    p.line
                                                }))
                                                .cursor_pointer()
                                                .hover(move |s| s.bg(rgb(p.raised)))
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.palette = name;
                                                    if let Some(search) = &this.search {
                                                        search.update(cx, |search, cx| {
                                                            search.palette = name;
                                                            cx.notify();
                                                        });
                                                    }
                                                    this.persist();
                                                    cx.notify();
                                                }))
                                                .child(
                                                    div()
                                                        .size(px(28.))
                                                        .flex_shrink_0()
                                                        .rounded_full()
                                                        .bg(linear_gradient(
                                                            45.,
                                                            linear_color_stop(
                                                                rgb(dark.accent),
                                                                0.5,
                                                            ),
                                                            linear_color_stop(
                                                                rgb(light.accent),
                                                                0.5,
                                                            ),
                                                        )),
                                                )
                                                .child(div().flex_1().child(dark.name))
                                                .when(current, |el| {
                                                    el.child(icon("check", 16., p.accent))
                                                })
                                        }),
                                    ))
                            }),
                    ),
            )
            .into_any_element()
    }
    fn track_context_menu(&self, track: Track, p: Palette, cx: &mut Context<Self>) -> AnyElement {
        let artist = crate::library::split_artist_credits(&track.artist)
            .into_iter()
            .next()
            .unwrap_or_default();
        let album = track.album.clone();
        let album_artist = album_artist(&track);
        let path = track.path.clone();
        let menu_item = |id: &'static str,
                         icon_name: &'static str,
                         title: &'static str,
                         action: TrackMenuAction| {
            let item_track = track.clone();
            div()
                .id(id)
                .flex()
                .items_center()
                .gap_3()
                .px_3()
                .py_2()
                .rounded_md()
                .cursor_pointer()
                .hover(move |s| s.bg(rgb(p.accent_bg)))
                .on_click(cx.listener(move |this, _, _, cx| {
                    action(this, &item_track, cx);
                    this.context_track = None;
                    cx.notify();
                }))
                .child(icon(icon_name, 16., p.text))
                .child(title)
        };
        div()
            .debug_selector(|| "track-context-menu".into())
            .absolute()
            .top(self.context_menu_position.y)
            .left(self.context_menu_position.x)
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                this.context_track = None;
                cx.notify();
            }))
            .w(px(220.))
            .p_2()
            .bg(rgb(p.raised))
            .border_1()
            .border_color(rgb(p.line))
            .rounded_lg()
            .shadow_lg()
            .child(menu_item(
                "track-open-artist",
                "artist",
                "Open artist",
                Box::new(move |this, _, cx| this.open_artist(artist.clone(), cx)),
            ))
            .child(menu_item(
                "track-open-album",
                "album",
                "Open album",
                Box::new(move |this, _, cx| {
                    this.open_album(album.clone(), album_artist.clone(), cx)
                }),
            ))
            .child(menu_item(
                "track-show-file",
                "folder",
                if cfg!(target_os = "macos") {
                    "Show in Finder"
                } else if cfg!(target_os = "windows") {
                    "Show in Explorer"
                } else {
                    "Show in file manager"
                },
                Box::new(move |_, _, _| {
                    reveal_track(&path);
                }),
            ))
            .child(menu_item(
                "track-add-queue",
                "plus",
                "Add to queue",
                Box::new(|this, track, cx| {
                    this.queue.push_back(track.clone());
                    this.persist();
                    cx.notify();
                }),
            ))
            .into_any_element()
    }
    fn queue_sidebar(&self, p: Palette, cx: &mut Context<Self>) -> AnyElement {
        let queue = self.upcoming_tracks();
        let count = queue.len();
        let view = cx.weak_entity();
        let list = uniform_list(
            "queue-sidebar-list",
            count,
            cx.processor(move |_, range: std::ops::Range<usize>, _, _| {
                range
                    .filter_map(|index| {
                        queue
                            .get(index)
                            .cloned()
                            .map(|track| queue_sidebar_row(track, index, p, view.clone()))
                    })
                    .collect()
            }),
        )
        .size_full();
        div()
            .debug_selector(|| "queue-sidebar".into())
            .w(px(300.))
            .flex_shrink_0()
            .h_full()
            .flex()
            .flex_col()
            .bg(rgb(p.panel))
            .border_l_1()
            .border_color(rgb(p.line))
            .child(
                div()
                    .h(px(54.))
                    .px_4()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().font_weight(FontWeight::SEMIBOLD).child("Play queue"))
                    .child(icon_button(
                        "close-queue-sidebar",
                        "close",
                        "Close queue",
                        p,
                        true,
                        false,
                        cx.listener(|this, _, _, cx| {
                            this.queue_sidebar_open = false;
                            cx.notify();
                        }),
                    )),
            )
            .when_some(self.current_track.as_ref(), |el, track| {
                el.child(
                    div()
                        .w_full()
                        .px_3()
                        .py_3()
                        .border_b_1()
                        .border_color(rgb(p.line))
                        .child(label("Now playing", p))
                        .child(
                            div()
                                .mt_2()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(artwork(
                                    track.cover_path.clone(),
                                    track.title.clone(),
                                    40.,
                                    p,
                                ))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            div()
                                                .truncate()
                                                .text_color(rgb(p.accent))
                                                .child(track.title.clone()),
                                        )
                                        .child(
                                            div()
                                                .truncate()
                                                .text_color(rgb(p.muted))
                                                .text_size(px(12.))
                                                .child(track.artist.clone()),
                                        ),
                                ),
                        ),
                )
            })
            .child(div().w_full().px_3().py_2().child(label("Up next", p)))
            .child(
                div()
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .when(count > 0, |el| el.child(list))
                    .when(count == 0, |el| {
                        el.child(
                            div().px_4().py_4().child("Nothing queued").child(
                                div()
                                    .mt_2()
                                    .text_color(rgb(p.muted))
                                    .child("Use a track’s menu to add it to the queue."),
                            ),
                        )
                    }),
            )
            .into_any_element()
    }

    fn cover_modal(
        &self,
        path: PathBuf,
        window: &Window,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let size = window.viewport_size();
        let side = (f32::from(size.width) - 96.)
            .min(f32::from(size.height) - 128.)
            .clamp(120., 900.);
        div()
            .absolute()
            .inset_0()
            .occlude()
            .bg(rgba(0x000000dd))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .id("artwork-modal")
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.artwork_modal = None;
                    cx.notify();
                }),
            )
            .child(
                div().w(px(side)).flex().justify_end().child(
                    icon_button(
                        "close-artwork",
                        "close",
                        "Close artwork",
                        p,
                        true,
                        false,
                        cx.listener(|this, _, _, cx| {
                            this.artwork_modal = None;
                            cx.notify();
                        }),
                    )
                    .track_focus(&self.overlay_focus["close-artwork"]),
                ),
            )
            .child(
                div()
                    .debug_selector(|| "expanded-artwork".into())
                    .size(px(side))
                    .child(
                        img(path)
                            .size_full()
                            .object_fit(ObjectFit::Contain)
                            .rounded_lg(),
                    ),
            )
            .into_any_element()
    }
    fn statistics_panel(&self, p: Palette, cx: &mut Context<Self>) -> AnyElement {
        div()
            .absolute()
            .inset_0()
            .occlude()
            .bg(rgba(0x000000bb))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(680.))
                    .max_h(relative(0.85))
                    .p_6()
                    .rounded_lg()
                    .bg(rgb(p.panel))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(22.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Listening statistics"),
                            )
                            .child(
                                icon_button(
                                    "close-statistics",
                                    "close",
                                    "Close statistics",
                                    p,
                                    true,
                                    false,
                                    cx.listener(|this, _, _, cx| {
                                        this.statistics_open = false;
                                        cx.notify();
                                    }),
                                )
                                .track_focus(&self.overlay_focus["close-statistics"]),
                            ),
                    )
                    .child(self.statistics_content(p)),
            )
            .into_any_element()
    }
}

impl AppView {
    fn statistics_content(&self, p: Palette) -> AnyElement {
        let groups = [
            ("Top artists", self.statistics.top_artists(5)),
            ("Top tracks", self.statistics.top_tracks(5)),
            ("Top albums", self.statistics.top_albums(5)),
        ];
        div()
            .flex()
            .gap_5()
            .mt_5()
            .children(groups.into_iter().map(|(title, entries)| {
                div()
                    .flex_1()
                    .min_w_0()
                    .child(div().font_weight(FontWeight::SEMIBOLD).mb_3().child(title))
                    .when(entries.is_empty(), |el| {
                        el.child(label("Play music to start collecting statistics", p))
                    })
                    .children(entries.into_iter().enumerate().map(|(index, item)| {
                        div()
                            .py_2()
                            .border_b_1()
                            .border_color(rgb(p.line))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(artwork(item.cover_path, String::new(), 28., p))
                                    .child(div().flex_1().min_w_0().truncate().child(format!(
                                        "{}. {}",
                                        index + 1,
                                        item.title
                                    ))),
                            )
                            .child(label(item.subtitle, p))
                            .child(label(
                                format!("{} min · {} plays", item.listened_ms / 60000, item.plays),
                                p,
                            ))
                    }))
            }))
            .into_any_element()
    }
}

fn unix_now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
fn format_added(added: u128, now: u128) -> String {
    if added == 0 {
        return "—".into();
    }
    if added <= now && now - added < 30 * 86_400_000 {
        let seconds = (now - added) / 1000;
        let (count, unit) = if seconds < 60 {
            (seconds, "second")
        } else if seconds < 3600 {
            (seconds / 60, "minute")
        } else if seconds < 86400 {
            (seconds / 3600, "hour")
        } else if seconds < 604800 {
            (seconds / 86400, "day")
        } else {
            (seconds / 604800, "week")
        };
        return format!("{count} {unit}{} ago", if count == 1 { "" } else { "s" });
    }
    // Gregorian civil date from days since the Unix epoch, without locale-dependent parsing.
    let z = (added / 86_400_000).min(i64::MAX as u128 - 719468) as i64 + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    let names = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    format!("{} {day}, {year}", names[(month - 1) as usize])
}

#[cfg(test)]
mod table_tests {
    use super::*;
    #[::core::prelude::v1::test]
    fn table_album_duration_orders_reverse_without_changing_tracks() {
        let mut view = AppView {
            tracks: Arc::new(vec![
                Track {
                    album: "Zulu".into(),
                    duration_ms: 100,
                    ..Track::default()
                },
                Track {
                    album: "alpha".into(),
                    duration_ms: 300,
                    ..Track::default()
                },
                Track {
                    album: "Beta".into(),
                    duration_ms: 200,
                    ..Track::default()
                },
            ]),
            sort: SortMode::Album,
            ..AppView::default()
        };
        view.prepare_tracks(LibrarySnapshot::new(view.tracks.as_ref().clone()));
        assert_eq!(*view.filtered_track_indices, vec![1, 2, 0]);
        view.sort = SortMode::Duration;
        view.sort_reversed = true;
        view.rebuild_order();
        assert_eq!(*view.filtered_track_indices, vec![1, 2, 0]);
        assert_eq!(view.tracks[0].album, "Zulu");
    }
    #[::core::prelude::v1::test]
    fn added_dates_match_relative_month_boundary() {
        let day = 86_400_000;
        let added = 1_704_412_800_000; // Jan 5, 2024, UTC.
        assert_eq!(format_added(0, added), "—");
        assert_eq!(format_added(added, added + 30_000), "30 seconds ago");
        assert_eq!(format_added(added, added + 60_000), "1 minute ago");
        assert_eq!(format_added(added, added + 3_600_000), "1 hour ago");
        assert_eq!(format_added(added, added + day), "1 day ago");
        assert_eq!(format_added(added, added + 14 * day), "2 weeks ago");
        assert_eq!(format_added(added, added + 30 * day), "Jan 5, 2024");
        assert_eq!(format_added(added, added - 1), "Jan 5, 2024");
    }
}

#[cfg(test)]
mod interaction_regressions {
    use super::{AppView, LibrarySnapshot, Page, Palette, duration, rail};
    use crate::library::Track;
    use gpui::{
        Context, IntoElement, Modifiers, MouseButton, Render, TestAppContext, Window, div, point,
        prelude::*, px, size,
    };
    use std::path::PathBuf;

    #[derive(Default)]
    struct RailHarness {
        value: f32,
        dragging: std::rc::Rc<std::cell::Cell<bool>>,
    }
    impl Render for RailHarness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let view = cx.weak_entity();
            div().w(px(300.)).h(px(100.)).p_5().child(rail(
                "seek",
                super::SliderValue {
                    fraction: self.value,
                    maximum: 100.,
                    step: 5.,
                },
                true,
                Palette::from_name(crate::settings::PaletteName::Forest, false),
                Some(duration((self.value * 100_000.) as u64)),
                self.dragging.clone(),
                move |value, _, app| {
                    let _ = view.update(app, |this, cx| {
                        this.value = value;
                        cx.notify();
                    });
                },
            ))
        }
    }

    #[gpui::test]
    fn scrubber_updates_before_release_and_captures_moves_outside_the_rail(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(|_, _| RailHarness::default());
        cx.run_until_parked();
        let bounds = cx.debug_bounds("rail-seek").unwrap();
        cx.simulate_mouse_down(bounds.center(), MouseButton::Left, Modifiers::default());
        let target = point(
            bounds.left() + bounds.size.width * 0.8,
            bounds.bottom() + px(20.),
        );
        cx.simulate_mouse_move(target, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
        cx.update(|_, cx| {
            assert!((view.read(cx).value - 0.8).abs() < 0.001);
            assert!(view.read(cx).dragging.get());
        });
        let tip = cx.debug_bounds("seek-time-tooltip").unwrap();
        assert_eq!(tip.size.width, px(54.));
        assert!(tip.size.height < px(30.));
        cx.simulate_mouse_up(target, MouseButton::Left, Modifiers::default());
        cx.update(|_, cx| assert!(!view.read(cx).dragging.get()));
    }

    #[gpui::test]
    fn album_list_and_queue_fill_the_width_after_sidebar_changes(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| {
            let mut view = AppView {
                page: Page::Albums,
                ..AppView::default()
            };
            view.prepare_tracks(LibrarySnapshot::new(
                (0..10)
                    .map(|i| Track {
                        title: format!("Track {i}"),
                        album: format!("Album {i}"),
                        artist: "Artist".into(),
                        path: PathBuf::from(format!("{i}.flac")),
                        ..Track::default()
                    })
                    .collect(),
            ));
            view.continuation = view.tracks.iter().cloned().collect();
            let _ = cx;
            view
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        for sidebar_width in [184., 64.] {
            for queue_open in [false, true] {
                cx.update(|_, cx| {
                    view.update(cx, |view, cx| {
                        view.sidebar_width = sidebar_width;
                        view.sidebar_collapsed = sidebar_width == 64.;
                        view.queue_sidebar_open = queue_open;
                        view.album_grid = false;
                        cx.notify();
                    })
                });
                cx.run_until_parked();
                let row = cx.debug_bounds("album-row-0").unwrap();
                assert!((row.left() - px(sidebar_width + 24.)).abs() < px(1.));
                assert!(
                    (row.right() - px(1200. - if queue_open { 300. } else { 0. } - 24.)).abs()
                        < px(1.)
                );
                if queue_open {
                    let panel = cx.debug_bounds("queue-sidebar").unwrap();
                    let row = cx.debug_bounds("queue-row-0").unwrap();
                    assert!(
                        (row.left() - panel.left()).abs() <= px(1.),
                        "row {row:?}, panel {panel:?}"
                    );
                    assert!(
                        (row.right() - panel.right()).abs() <= px(1.),
                        "row {row:?}, panel {panel:?}"
                    );
                }
                cx.update(|_, cx| {
                    view.update(cx, |view, cx| {
                        view.album_grid = true;
                        cx.notify();
                    })
                });
                cx.run_until_parked();
                let last_card = [
                    "album-card-0-0",
                    "album-card-0-1",
                    "album-card-0-2",
                    "album-card-0-3",
                    "album-card-0-4",
                    "album-card-0-5",
                ]
                .into_iter()
                .filter_map(|selector| cx.debug_bounds(selector))
                .last()
                .unwrap();
                assert!(
                    (last_card.right() - px(1200. - if queue_open { 300. } else { 0. } - 24.))
                        .abs()
                        < px(1.),
                    "card {last_card:?}, sidebar {sidebar_width}, queue {queue_open}"
                );
            }
        }
    }

    #[gpui::test]
    fn track_menu_uses_pointer_position_and_closes_on_outside_press(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, _| AppView {
            context_track: Some(Track::default()),
            context_menu_position: point(px(200.), px(150.)),
            ..AppView::default()
        });
        cx.run_until_parked();
        let menu = cx.debug_bounds("track-context-menu").unwrap();
        assert_eq!(menu.origin, point(px(200.), px(150.)));
        cx.simulate_mouse_down(
            point(px(20.), px(400.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.update(|_, cx| assert!(view.read(cx).context_track.is_none()));
    }

    #[::core::prelude::v1::test]
    fn artist_page_does_not_use_previously_selected_album() {
        let mut view = AppView::default();
        view.prepare_tracks(LibrarySnapshot::new(vec![Track {
            album: "Album".into(),
            artist: "Artist".into(),
            duration_ms: 120_000,
            year: Some(2020),
            ..Track::default()
        }]));
        view.selected_album = Some(("Album".into(), "Artist".into()));
        view.page = Page::AlbumDetail;
        assert_eq!(view.current_album().unwrap().year, Some(2020));
        view.selected_artist = Some("Artist".into());
        view.page = Page::ArtistDetail;
        assert!(view.current_album().is_none());
        // Back navigation can still restore the album selection.
        view.page = Page::AlbumDetail;
        assert_eq!(view.current_album().unwrap().duration_ms, 120_000);
    }

    #[gpui::test]
    fn unavailable_previous_keeps_current_track_and_history(cx: &mut TestAppContext) {
        let current = Track {
            path: "/current.flac".into(),
            ..Track::default()
        };
        let previous = Track {
            path: "/nonexistent-flacpac-review/previous.flac".into(),
            ..Track::default()
        };
        let view = cx.new(|_| AppView {
            current_track: Some(current.clone()),
            history: vec![previous.clone()],
            ..AppView::default()
        });
        view.update(cx, |view, cx| {
            view.play_previous(cx);
            assert_eq!(view.current_track.as_ref().unwrap().path, current.path);
            assert_eq!(view.history[0].path, previous.path);
            assert!(view.continuation.is_empty());
            assert!(view.status.as_ref().unwrap().contains("Cannot play"));
        });
    }

    #[::core::prelude::v1::test]
    fn editing_context_queue_changes_the_actual_playback_order() {
        let tracks: Vec<_> = (0..5)
            .map(|i| Track {
                path: PathBuf::from(format!("{i}.flac")),
                ..Track::default()
            })
            .collect();
        let mut view = AppView {
            queue: tracks[..2].to_vec().into(),
            continuation: tracks[2..].to_vec().into(),
            ..AppView::default()
        };
        assert!(view.reorder_upcoming(4, 0));
        assert!(view.remove_upcoming(2));
        let actual: Vec<_> = std::iter::from_fn(|| view.next_track(false))
            .map(|track| track.path)
            .collect();
        assert_eq!(actual, [4, 0, 2, 3].map(|i| tracks[i].path.clone()));
    }
}

#[cfg(test)]
mod design_regressions {
    use super::{AppView, Page, Palette, SliderValue, Track, rail};
    use gpui::{
        AppContext, Context, FocusHandle, IntoElement, Modifiers, MouseButton, Render,
        TestAppContext, Window, div, point, prelude::*, px,
    };
    use std::sync::Arc;

    #[::core::prelude::v1::test]
    fn empty_source_and_empty_search_results_offer_the_right_action() {
        let mut view = AppView {
            search_query: "missing".into(),
            ..AppView::default()
        };
        assert_eq!(view.empty_content().2, Some("Choose folder"));
        view.scanning = true;
        assert_eq!(view.empty_content().0, "Loading your library");
        assert_eq!(view.empty_content().2, None);
        view.scanning = false;
        view.library_unavailable = true;
        assert_eq!(view.empty_content().0, "Music folder unavailable");
        view.tracks = Arc::new(vec![Track::default()]);
        assert_eq!(view.empty_content().0, "No results");
        assert_eq!(view.empty_content().2, Some("Clear search"));
        view.page = Page::Queue;
        assert_eq!(view.empty_content().0, "Your queue is empty");
    }

    #[gpui::test]
    fn keyboard_opens_settings_traps_focus_and_returns_to_the_opener(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| AppView {
            root_focus: Some(cx.focus_handle()),
            search: Some(cx.new(crate::search::SearchInput::new)),
            ..AppView::default()
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            let focus = view.read(cx).overlay_focus["app-menu"].clone();
            window.focus(&focus, cx);
        });
        cx.run_until_parked();
        cx.simulate_keystrokes("enter");
        cx.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse("enter").unwrap(),
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert!(view.read(cx).overlay_focus["menu-statistics"].is_focused(window))
        });
        cx.simulate_keystrokes("tab");
        cx.run_until_parked();
        cx.simulate_keystrokes("enter");
        cx.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse("enter").unwrap(),
        });
        cx.run_until_parked();
        cx.read_entity(&view, |view, _| assert!(view.settings_open));
        let behind = cx.debug_bounds("search-field").unwrap().center();
        cx.simulate_click(behind, Modifiers::default());
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert!(
                !view
                    .read(cx)
                    .search
                    .as_ref()
                    .unwrap()
                    .read(cx)
                    .focus
                    .is_focused(window)
            )
        });
        for _ in 0..20 {
            cx.simulate_keystrokes("tab");
            cx.run_until_parked();
            cx.update(|window, cx| {
                let state = view.read(cx);
                assert!(
                    state
                        .overlay_stops()
                        .iter()
                        .any(|id| state.overlay_focus[id].is_focused(window))
                );
            });
        }
        cx.simulate_keystrokes("shift-tab");
        cx.update(|window, cx| {
            let focus = view.read(cx).overlay_focus["settings-tab-true"].clone();
            window.focus(&focus, cx);
        });
        cx.run_until_parked();
        cx.simulate_keystrokes("enter");
        cx.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse("enter").unwrap(),
        });
        cx.run_until_parked();
        cx.read_entity(&view, |view, _| assert!(view.settings_about));
        for _ in 0..5 {
            cx.simulate_keystrokes("tab");
            cx.run_until_parked();
        }
        cx.update(|window, cx| {
            let state = view.read(cx);
            assert!(
                state
                    .overlay_stops()
                    .iter()
                    .any(|id| state.overlay_focus[id].is_focused(window))
            );
        });
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert!(!view.read(cx).settings_open);
            assert!(view.read(cx).overlay_focus["app-menu"].is_focused(window));
        });
    }

    #[gpui::test]
    fn menu_outside_click_dismisses_without_reopening(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| AppView {
            root_focus: Some(cx.focus_handle()),
            ..AppView::default()
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            let focus = view.read(cx).overlay_focus["app-menu"].clone();
            window.focus(&focus, cx);
        });
        cx.run_until_parked();
        cx.simulate_keystrokes("enter");
        cx.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse("enter").unwrap(),
        });
        cx.run_until_parked();
        cx.read_entity(&view, |view, _| assert!(view.menu_open));
        cx.simulate_mouse_down(
            point(px(10.), px(300.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.simulate_mouse_up(
            point(px(10.), px(300.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.run_until_parked();
        cx.read_entity(&view, |view, _| assert!(!view.menu_open));
        cx.simulate_keystrokes("enter");
        cx.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse("enter").unwrap(),
        });
        cx.run_until_parked();
        cx.read_entity(&view, |view, _| assert!(view.menu_open));
        let opener = cx.debug_bounds("app-menu").unwrap().center();
        cx.simulate_click(opener, Modifiers::default());
        cx.run_until_parked();
        cx.read_entity(&view, |view, _| assert!(!view.menu_open));
    }

    #[derive(Default)]
    struct SliderHarness {
        value: f32,
        enabled: bool,
        root: Option<FocusHandle>,
        bubbled: usize,
    }
    impl Render for SliderHarness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let target = cx.weak_entity();
            div()
                .tab_group()
                .when_some(self.root.as_ref(), |el, focus| el.track_focus(focus))
                .on_key_down(cx.listener(|this, _, _, _| this.bubbled += 1))
                .w(px(300.))
                .h(px(100.))
                .child(rail(
                    "volume",
                    SliderValue {
                        fraction: self.value,
                        maximum: 100.,
                        step: 5.,
                    },
                    self.enabled,
                    Palette::from_name(Default::default(), false),
                    None,
                    Default::default(),
                    move |value, _, cx| {
                        let _ = target.update(cx, |this, cx| {
                            this.value = value;
                            cx.notify();
                        });
                    },
                ))
        }
    }

    #[gpui::test]
    fn slider_keyboard_steps_clamps_and_stops_arrow_propagation(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| SliderHarness {
            value: 0.5,
            enabled: true,
            root: Some(cx.focus_handle()),
            ..Default::default()
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            let focus = view.read(cx).root.as_ref().unwrap().clone();
            window.focus(&focus, cx);
            window.focus_next(cx);
        });
        cx.simulate_keystrokes("right");
        cx.read_entity(&view, |view, _| {
            assert!((view.value - 0.55).abs() < 0.001);
            assert_eq!(view.bubbled, 0);
        });
        cx.simulate_keystrokes("end right");
        cx.read_entity(&view, |view, _| assert_eq!(view.value, 1.));
        cx.simulate_keystrokes("home left");
        cx.read_entity(&view, |view, _| assert_eq!(view.value, 0.));
        cx.update(|_, cx| {
            view.update(cx, |view, cx| {
                view.enabled = false;
                cx.notify();
            })
        });
        cx.run_until_parked();
        cx.simulate_keystrokes("right");
        cx.read_entity(&view, |view, _| assert_eq!(view.value, 0.));
    }
    #[gpui::test]
    fn compact_window_contains_search_recovery_banner_and_open_queue(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| AppView {
            search: Some(cx.new(crate::search::SearchInput::new)),
            root_focus: Some(cx.focus_handle()),
            queue_sidebar_open: true,
            sidebar_width: 184.,
            library_unavailable: true,
            library_error: Some("A long technical error kept in details".into()),
            ..Default::default()
        });
        cx.simulate_resize(gpui::size(px(960.), px(640.)));
        for light in [false, true] {
            cx.update(|_, cx| {
                view.update(cx, |view, cx| {
                    view.light = light;
                    cx.notify();
                })
            });
            cx.run_until_parked();
            let header = cx.debug_bounds("app-header").unwrap();
            let search = cx.debug_bounds("search-field").unwrap();
            let banner = cx.debug_bounds("status-banner").unwrap();
            let queue = cx.debug_bounds("queue-sidebar").unwrap();
            assert!(search.left() >= header.left() && search.right() <= header.right());
            assert!(banner.left() >= header.left() && banner.right() <= queue.left());
            assert!(banner.bottom() < cx.debug_bounds("player-bar").unwrap().top());
            assert!(queue.right() <= px(960.));
        }
    }
}
