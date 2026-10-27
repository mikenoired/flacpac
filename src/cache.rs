use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};
use thiserror::Error;

use crate::library::Track;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("cache filesystem error: {0}")]
    Io(#[from] std::io::Error),
    #[error("cache database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("application data directory is unavailable")]
    DataDirectory,
}

fn database_path() -> Result<PathBuf, CacheError> {
    let root = dirs::data_local_dir().ok_or(CacheError::DataDirectory)?;
    Ok(root.join("flacpac-rusty").join("library.sqlite3"))
}

fn open_database() -> Result<Connection, CacheError> {
    let path = database_path()?;
    std::fs::create_dir_all(path.parent().ok_or(CacheError::DataDirectory)?)?;
    let connection = Connection::open(path)?;
    configure_database(&connection)?;
    Ok(connection)
}

fn configure_database(connection: &Connection) -> Result<(), CacheError> {
    connection.busy_timeout(std::time::Duration::from_secs(5))?;
    connection.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         CREATE TABLE IF NOT EXISTS tracks (
             library_root TEXT NOT NULL,
             path TEXT NOT NULL,
             title TEXT NOT NULL,
             artist TEXT NOT NULL,
             album TEXT NOT NULL,
             duration_ms INTEGER NOT NULL,
             modified_ms INTEGER NOT NULL,
             cover_path TEXT,
             PRIMARY KEY (library_root, path)
         );
         CREATE INDEX IF NOT EXISTS tracks_by_album ON tracks(library_root, album, artist);
         CREATE INDEX IF NOT EXISTS tracks_by_artist ON tracks(library_root, artist);",
    )?;
    let has_details = connection
        .prepare("PRAGMA table_info(tracks)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .any(|name| name == "details");
    if !has_details {
        connection.execute("ALTER TABLE tracks ADD COLUMN details TEXT", [])?;
    }
    Ok(())
}

pub fn load(root: &Path) -> Result<Vec<Track>, CacheError> {
    let connection = open_database()?;
    load_from(&connection, root)
}

fn load_from(connection: &Connection, root: &Path) -> Result<Vec<Track>, CacheError> {
    let mut statement = connection.prepare_cached(
        "SELECT path, title, artist, album, duration_ms, modified_ms, cover_path, details
         FROM tracks WHERE library_root = ?1 ORDER BY modified_ms DESC, path ASC",
    )?;
    let rows = statement.query_map([root.to_string_lossy().as_ref()], |row| {
        if let Some(details) = row.get::<_, Option<String>>(7)?
            && let Ok(track) = serde_json::from_str::<Track>(&details)
        {
            return Ok(track);
        }
        let path: String = row.get(0)?;
        let cover_path: Option<String> = row.get(6)?;
        Ok(Track {
            path: PathBuf::from(path),
            title: row.get(1)?,
            artist: row.get(2)?,
            album: row.get(3)?,
            duration_ms: row.get::<_, i64>(4)?.max(0) as u64,
            modified_unix_ms: row.get::<_, i64>(5)?.max(0) as u128,
            cover_path: cover_path.map(PathBuf::from),
            album_artist: String::new(),
            track_number: 0,
            disc_number: 1,
            year: None,
            file_size: 0,
            added_unix_ms: 0,
            artwork_version: 0,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn replace(root: &Path, tracks: &[Track]) -> Result<(), CacheError> {
    let mut connection = open_database()?;
    replace_in(&mut connection, root, tracks)
}

fn replace_in(
    connection: &mut Connection,
    root: &Path,
    tracks: &[Track],
) -> Result<(), CacheError> {
    let transaction = connection.transaction()?;
    let root = root.to_string_lossy();
    transaction.execute(
        "DELETE FROM tracks WHERE library_root = ?1",
        [root.as_ref()],
    )?;
    {
        let mut statement = transaction.prepare_cached(
            "INSERT INTO tracks
             (library_root, path, title, artist, album, duration_ms, modified_ms, cover_path, details)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )?;
        for track in tracks {
            statement.execute(params![
                root.as_ref(),
                track.path.to_string_lossy().as_ref(),
                track.title,
                track.artist,
                track.album,
                i64::try_from(track.duration_ms).unwrap_or(i64::MAX),
                i64::try_from(track.modified_unix_ms).unwrap_or(i64::MAX),
                track
                    .cover_path
                    .as_ref()
                    .map(|path| path.to_string_lossy().into_owned()),
                serde_json::to_string(track).unwrap_or_default(),
            ])?;
        }
    }
    transaction.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_replacement_rolls_back_and_keeps_other_libraries() {
        let mut connection = Connection::open_in_memory().unwrap();
        configure_database(&connection).unwrap();
        let track = Track {
            path: PathBuf::from("/music/song.flac"),
            title: "Saved song".into(),
            album_artist: "Album artist".into(),
            track_number: 4,
            disc_number: 2,
            added_unix_ms: 123,
            artwork_version: 1,
            ..Track::default()
        };
        replace_in(
            &mut connection,
            Path::new("/music"),
            std::slice::from_ref(&track),
        )
        .unwrap();
        replace_in(
            &mut connection,
            Path::new("/other"),
            std::slice::from_ref(&track),
        )
        .unwrap();
        assert!(
            replace_in(
                &mut connection,
                Path::new("/music"),
                &[track.clone(), track]
            )
            .is_err()
        );
        let saved = load_from(&connection, Path::new("/music")).unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].title, "Saved song");
        assert_eq!(saved[0].disc_number, 2);
        assert_eq!(saved[0].added_unix_ms, 123);
        assert_eq!(
            load_from(&connection, Path::new("/other")).unwrap().len(),
            1
        );
    }
    #[test]
    fn old_cache_rows_remain_available_after_schema_upgrade() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE tracks(library_root TEXT NOT NULL,path TEXT NOT NULL,title TEXT NOT NULL,artist TEXT NOT NULL,album TEXT NOT NULL,duration_ms INTEGER NOT NULL,modified_ms INTEGER NOT NULL,cover_path TEXT,PRIMARY KEY(library_root,path)); INSERT INTO tracks VALUES('/music','/music/old.flac','Old song','Artist','Album',2000,12,NULL);").unwrap();
        configure_database(&connection).unwrap();
        let tracks = load_from(&connection, Path::new("/music")).unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].title, "Old song");
        assert_eq!(tracks[0].file_size, 0);
    }
}
