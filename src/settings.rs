use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

/// Color families are independent of the light/dark appearance mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PaletteName {
    #[default]
    #[serde(alias = "forest-light")]
    Forest,
    Midnight,
    Ocean,
    Violet,
    Ember,
    Rose,
    Sand,
    Slate,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaletteDefinition {
    pub id: PaletteName,
    pub name: &'static str,
    pub bg: u32,
    pub panel: u32,
    pub raised: u32,
    pub line: u32,
    pub text: u32,
    pub muted: u32,
    pub accent: u32,
    pub accent_bg: u32,
}
pub const PALETTES: [PaletteName; 8] = [
    PaletteName::Forest,
    PaletteName::Midnight,
    PaletteName::Ocean,
    PaletteName::Violet,
    PaletteName::Ember,
    PaletteName::Rose,
    PaletteName::Sand,
    PaletteName::Slate,
];
impl PaletteName {
    pub fn definition(self, light: bool) -> PaletteDefinition {
        // Keep the original application colors for the established Forest and Ocean modes.
        let original = match (self, light) {
            (Self::Forest, false) => Some((
                0x121610, 0x181e16, 0x232d20, 0x2a3326, 0xe8edde, 0x939e8a, 0xb2d77c, 0x2b3a22,
            )),
            (Self::Forest, true) => Some((
                0xf7f8f4, 0xeff2eb, 0xe4eadd, 0xdce2d5, 0x20291f, 0x65715e, 0x486c2d, 0xdceacb,
            )),
            (Self::Ocean, false) => Some((
                0x10191c, 0x152226, 0x203238, 0x2b4147, 0xe0f0f1, 0x8ea9ad, 0x70d4df, 0x21444a,
            )),
            _ => None,
        };
        if let Some((bg, panel, raised, line, text, muted, accent, accent_bg)) = original {
            return PaletteDefinition {
                id: self,
                name: if self == Self::Forest {
                    "Forest"
                } else {
                    "Ocean"
                },
                bg,
                panel,
                raised,
                line,
                text,
                muted,
                accent,
                accent_bg,
            };
        }
        let (name, dark_accent, light_accent, dark_bg, dark_panel, light_bg, light_panel) =
            match self {
                Self::Forest => (
                    "Forest", 0xb2d77c, 0x486c2d, 0x121612, 0x1d2520, 0xfafbf9, 0xf0f3ee,
                ),
                Self::Midnight => (
                    "Midnight", 0x9daeff, 0x4354ae, 0x11141a, 0x1b212a, 0xf9fafd, 0xf0f1f9,
                ),
                Self::Ocean => (
                    "Ocean", 0x70d4df, 0x147783, 0x0e1618, 0x182428, 0xf8fbfb, 0xecf4f5,
                ),
                Self::Violet => (
                    "Violet", 0xd0a4fa, 0x8045b3, 0x141319, 0x1f202a, 0xfbf9fd, 0xf5f0f9,
                ),
                Self::Ember => (
                    "Ember", 0xffad72, 0xb45320, 0x171411, 0x23211f, 0xfdfaf8, 0xf9f1ed,
                ),
                Self::Rose => (
                    "Rose", 0xf09abb, 0xa63e68, 0x161316, 0x222025, 0xfcf9fa, 0xf8f0f3,
                ),
                Self::Sand => (
                    "Sand", 0xe6c58a, 0x8b6330, 0x151513, 0x212321, 0xfcfaf9, 0xf6f3ee,
                ),
                Self::Slate => (
                    "Slate", 0xb7c8d9, 0x4d657d, 0x131517, 0x1d2327, 0xfafafb, 0xf1f3f5,
                ),
            };
        if light {
            PaletteDefinition {
                id: self,
                name,
                bg: light_bg,
                panel: light_panel,
                raised: 0xe4e8e7,
                line: 0xd2d9d7,
                text: 0x202a29,
                muted: 0x61706d,
                accent: light_accent,
                accent_bg: light_panel,
            }
        } else {
            PaletteDefinition {
                id: self,
                name,
                bg: dark_bg,
                panel: dark_panel,
                raised: 0x243135,
                line: 0x334346,
                text: 0xe5eeec,
                muted: 0x94aaa5,
                accent: dark_accent,
                accent_bg: dark_panel,
            }
        }
    }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Forest => "forest",
            Self::Midnight => "midnight",
            Self::Ocean => "ocean",
            Self::Violet => "violet",
            Self::Ember => "ember",
            Self::Rose => "rose",
            Self::Sand => "sand",
            Self::Slate => "slate",
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub library_root: Option<PathBuf>,
    pub light: bool,
    #[serde(default)]
    pub palette: PaletteName,
    pub shuffle: bool,
    pub repeat: crate::playback::RepeatMode,
    pub sort: String,
    pub sort_reversed: bool,
    pub table_ratios: Option<[f32; 4]>,
    pub queue: Vec<PathBuf>,
    pub page: String,
    pub volume: Option<f32>,
    pub album_grid: Option<bool>,
    pub scroll_offsets: HashMap<String, f32>,
}
fn settings_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|base| base.join("flacpac-rusty").join("settings.json"))
}
pub fn load() -> Settings {
    settings_path()
        .and_then(|path| std::fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}
pub fn save(settings: &Settings) {
    let Some(path) = settings_path() else { return };
    let Some(parent) = path.parent() else { return };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    let Ok(bytes) = serde_json::to_vec_pretty(settings) else {
        return;
    };
    let temporary = path.with_extension("tmp");
    if std::fs::write(&temporary, bytes).is_ok() {
        let _ = std::fs::rename(temporary, path);
    }
}
