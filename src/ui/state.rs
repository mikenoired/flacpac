use crate::library::{Album, Artist, Track};
use crate::media_controls::SystemMediaControls;
use crate::playback::{Playback, RepeatMode};
use crate::search::SearchInput;
use gpui::*;
use notify::RecommendedWatcher;
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::Arc,
};

pub(crate) type TrackMenuAction =
    Box<dyn Fn(&mut AppView, &Track, &mut Context<AppView>) + 'static>;

#[derive(Default)]
pub struct AppView {
    pub statistics: crate::statistics::Statistics,
    pub statistics_tick: Option<std::time::Instant>,
    pub artwork_modal: Option<PathBuf>,
    pub menu_open: bool,
    pub statistics_open: bool,
    pub settings_open: bool,
    pub settings_about: bool,
    pub sidebar_collapsed: bool,
    pub sidebar_width: f32,
    pub sidebar_animation: Option<Task<()>>,
    pub _activation_subscription: Option<Subscription>,
    pub queue_sidebar_open: bool,
    pub context_track: Option<Track>,
    pub context_menu_position: Point<Pixels>,
    pub seek_dragging: std::rc::Rc<std::cell::Cell<bool>>,
    pub volume_dragging: std::rc::Rc<std::cell::Cell<bool>>,
    pub root: Option<PathBuf>,
    pub tracks: Arc<Vec<Track>>,
    pub sorted_indices: Arc<Vec<usize>>,
    pub date_order: Arc<Vec<usize>>,
    pub title_order: Arc<Vec<usize>>,
    pub artist_order: Arc<Vec<usize>>,
    pub album_order: Arc<Vec<usize>>,
    pub duration_order: Arc<Vec<usize>>,
    pub filtered_track_indices: Arc<Vec<usize>>,
    pub albums: Arc<Vec<Album>>,
    pub artists: Arc<Vec<Artist>>,
    pub filtered_album_indices: Arc<Vec<usize>>,
    pub filtered_artist_indices: Arc<Vec<usize>>,
    pub search_query: String,
    pub search: Option<Entity<SearchInput>>,
    pub _search_subscription: Option<Subscription>,
    pub playback_task: Option<Task<()>>,
    pub generation: u64,
    pub light: bool,
    pub palette: crate::settings::PaletteName,
    pub album_grid: bool,
    pub preferences_task: Option<Task<()>>,
    pub position_ms: u64,
    pub volume: f32,
    pub history: Vec<Track>,
    pub continuation: VecDeque<Track>,
    pub playback_order: Vec<Track>,
    pub shuffle: bool,
    pub repeat: RepeatMode,
    pub folder_dialog_open: bool,
    pub pending_queue: Vec<PathBuf>,
    pub selected_track: Option<PathBuf>,
    pub search_terms: Arc<Vec<String>>,
    pub navigation: Vec<NavigationEntry>,
    pub root_focus: Option<FocusHandle>,
    pub scrolls: HashMap<String, UniformListScrollHandle>,
    pub pending_offsets: HashMap<String, f32>,
    pub _quit_subscription: Option<Subscription>,
    pub status: Option<String>,
    pub status_details: bool,
    pub library_error: Option<String>,
    pub overlay_focus: HashMap<String, FocusHandle>,
    pub active_overlay: Option<&'static str>,
    pub return_focus: Option<FocusHandle>,
    pub scanning: bool,
    pub library_unavailable: bool,
    pub sort: SortMode,
    pub sort_reversed: bool,
    pub table_ratios: [f32; 4],
    pub table_available: f32,
    pub playback: Playback,
    pub system_media: Option<SystemMediaControls>,
    pub current_track: Option<Track>,
    pub paused: bool,
    pub queue: VecDeque<Track>,
    pub page: Page,
    pub selected_album: Option<(String, String)>,
    pub selected_artist: Option<String>,
    pub watcher: Option<RecommendedWatcher>,
    pub reindex_task: Option<Task<()>>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    #[default]
    Date,
    Title,
    Artist,
    Album,
    Duration,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    #[default]
    Tracks,
    Queue,
    Albums,
    Artists,
    AlbumDetail,
    ArtistDetail,
}

pub struct NavigationEntry {
    pub(crate) page: Page,
    pub(crate) album: Option<(String, String)>,
    pub(crate) artist: Option<String>,
}
