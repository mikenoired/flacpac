mod catalog;
mod scanner;

pub use catalog::{Album, Artist, album_artist, build_catalog};
pub use scanner::{Library, LibraryError, Track};

pub(crate) use catalog::split_artist_credits;
