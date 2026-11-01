use gpui::{AssetSource, SharedString};
use std::borrow::Cow;

pub struct Icons;

static PHOSPHOR_INDEX: &[(&str, usize, usize)] = include!(concat!(env!("OUT_DIR"), "/phosphor_index.rs"));
static PHOSPHOR_DATA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/phosphor.bin"));

fn legacy_icon(name: &str) -> Option<&'static str> {
    Some(match name {
        "clock" => "regular/clock",
        "menu" => "regular/list",
        "more" => "regular/dots-three",
        "collapse" => "regular/sidebar-simple",
        "expand" => "regular/sidebar",
        "settings" => "regular/gear",
        "statistics" => "regular/chart-bar",
        "check" => "regular/check",
        "drag" => "regular/dots-six-vertical",
        "shuffle" => "regular/shuffle",
        "repeat" => "regular/repeat",
        "repeat-one" => "regular/repeat-once",
        "grid" => "regular/squares-four",
        "list" => "regular/list-bullets",
        "play" => "fill/play",
        "pause" => "regular/pause",
        "next" => "regular/skip-forward",
        "previous" => "regular/skip-back",
        "search" => "regular/magnifying-glass",
        "plus" => "regular/plus",
        "close" => "regular/x",
        "forward" => "regular/caret-right",
        "back" => "regular/caret-left",
        "music" => "regular/music-note",
        "album" => "regular/vinyl-record",
        "artist" => "regular/microphone-stage",
        "queue" => "regular/queue",
        "folder" => "regular/folder",
        "volume" => "regular/speaker-high",
        "sun" => "regular/sun",
        "moon" => "regular/moon",
        "up" => "regular/caret-up",
        "down" => "regular/caret-down",
        _ => return None,
    })
}

impl AssetSource for Icons {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        if path == "brand" {
            return Ok(Some(Cow::Borrowed(include_bytes!(
                "../assets/flacpac-face.svg"
            ))));
        }

        let Some(key) = path.strip_prefix("phosphor/").or_else(|| legacy_icon(path)) else {
            return Ok(None);
        };
        let Some((_, offset, len)) = PHOSPHOR_INDEX.iter().find(|(name, _, _)| *name == key) else {
            return Ok(None);
        };
        Ok(Some(Cow::Owned(PHOSPHOR_DATA[*offset..*offset + *len].to_vec())))
    }

    fn list(&self, _: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}
