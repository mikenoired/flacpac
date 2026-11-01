mod helpers;
mod library;
mod library_state;
mod playback;
mod root_view;
mod rows;
mod scanner;
mod session;
mod sidebar_state;
mod state;
#[cfg(test)]
mod tests;

pub(crate) use crate::components::visual::{
    Palette, Tip, artist_cover, artwork, duration, icon, label,
};
pub(crate) use helpers::{SliderValue, format_added, reveal_track, unix_now_ms};
pub(crate) use library::{LibrarySnapshot, scan_and_cache};
pub(crate) use rows::{
    ColumnResize, TrackRowState, album_grid_row, album_row, artist_grid_row, artist_row,
    empty_state, queue_sidebar_row, track_row,
};
pub(crate) use state::TrackMenuAction;
pub use state::{AppView, NavigationEntry, Page, SortMode};
