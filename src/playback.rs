use std::{fs::File, path::Path, time::Duration};

use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink};
use thiserror::Error;

#[derive(Default)]
pub struct Playback {
    output: Option<OutputStream>,
    sink: Option<Sink>,
}

#[derive(Debug, Error)]
pub enum PlaybackError {
    #[error("cannot open audio output: {0}")]
    Output(#[from] rodio::StreamError),
    #[error("cannot open audio file: {0}")]
    File(#[from] std::io::Error),
    #[error("cannot decode this audio file: {0}")]
    Decode(#[from] rodio::decoder::DecoderError),
}

impl Playback {
    pub fn play(&mut self, path: &Path) -> Result<(), PlaybackError> {
        let file = File::open(path)?;
        let source = Decoder::try_from(file)?;
        if self.output.is_none() {
            self.output = Some(OutputStreamBuilder::open_default_stream()?);
        }
        let output = self.output.as_ref().expect("audio output initialized");
        let sink = self
            .sink
            .get_or_insert_with(|| Sink::connect_new(output.mixer()));
        sink.stop();
        self.sink = Some(Sink::connect_new(output.mixer()));
        self.sink.as_ref().expect("sink initialized").append(source);
        Ok(())
    }

    pub fn position(&self) -> Duration {
        self.sink.as_ref().map_or(Duration::ZERO, Sink::get_pos)
    }
    pub fn is_paused(&self) -> bool {
        self.sink.as_ref().is_none_or(Sink::is_paused)
    }
    pub fn is_finished(&self) -> bool {
        self.sink.as_ref().is_some_and(Sink::empty)
    }
    pub fn seek(&self, position: Duration) -> bool {
        self.sink
            .as_ref()
            .is_some_and(|sink| sink.try_seek(position).is_ok())
    }
    pub fn set_volume(&self, volume: f32) {
        if let Some(sink) = &self.sink {
            sink.set_volume(volume);
        }
    }

    pub fn toggle(&self) {
        if let Some(sink) = &self.sink {
            if sink.is_paused() {
                sink.play();
            } else {
                sink.pause();
            }
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepeatMode {
    #[default]
    Off,
    All,
    One,
}
impl RepeatMode {
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::All,
            Self::All => Self::One,
            Self::One => Self::Off,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::RepeatMode;
    #[test]
    fn repeat_cycles_through_both_modes_and_off() {
        assert_eq!(RepeatMode::Off.next(), RepeatMode::All);
        assert_eq!(RepeatMode::All.next(), RepeatMode::One);
        assert_eq!(RepeatMode::One.next(), RepeatMode::Off);
        assert_eq!(serde_json::to_string(&RepeatMode::One).unwrap(), "\"one\"");
    }
}

#[cfg(test)]
mod library_validation {
    #[test]
    #[ignore = "set FLACPAC_TEST_LIBRARY to validate a local library without audio output"]
    fn local_library_decodes() {
        let root = std::env::var_os("FLACPAC_TEST_LIBRARY").expect("FLACPAC_TEST_LIBRARY");
        let mut count = 0;
        let mut failures = Vec::new();
        for entry in walkdir::WalkDir::new(root) {
            let entry = entry.unwrap();
            let path = entry.path();
            if !entry.file_type().is_file()
                || entry.file_name().to_string_lossy().starts_with("._")
                || !path.extension().is_some_and(|ext| {
                    ["flac", "mp3", "m4a", "aiff", "aif"]
                        .iter()
                        .any(|supported| ext.eq_ignore_ascii_case(supported))
                })
            {
                continue;
            }
            count += 1;
            match rodio::Decoder::try_from(std::fs::File::open(path).unwrap()) {
                Ok(mut decoder) => {
                    assert!(decoder.next().is_some(), "empty audio: {}", path.display());
                }
                Err(error) => failures.push(format!("{}: {error}", path.display())),
            }
        }
        eprintln!("Validated {count} audio files");
        assert!(count > 0);
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
}
