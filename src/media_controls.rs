use std::{sync::mpsc, time::Duration};

use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
};

/// Cross-platform OS media session. Events are forwarded to the GPUI thread so
/// playback state remains owned by AppView.
pub struct SystemMediaControls {
    controls: MediaControls,
    events: mpsc::Receiver<MediaControlEvent>,
}

impl SystemMediaControls {
    pub fn new() -> Result<Self, souvlaki::Error> {
        let mut controls = MediaControls::new(PlatformConfig {
            display_name: "Flacpac",
            dbus_name: "com.flacpac.player",
            // Souvlaki requires a native HWND on Windows. GPUI does not expose
            // one through this startup path, so Windows currently cannot attach.
            hwnd: None,
        })?;
        let (tx, events) = mpsc::channel();
        controls.attach(move |event| {
            let _ = tx.send(event);
        })?;
        Ok(Self { controls, events })
    }

    pub fn drain_events(&self) -> impl Iterator<Item = MediaControlEvent> + '_ {
        std::iter::from_fn(|| self.events.try_recv().ok())
    }

    pub fn sync(
        &mut self,
        title: &str,
        artist: &str,
        album: &str,
        duration: Duration,
        position: Duration,
        paused: bool,
    ) {
        let _ = self.controls.set_metadata(MediaMetadata {
            title: Some(title),
            artist: Some(artist),
            album: Some(album),
            duration: Some(duration),
            ..Default::default()
        });
        let progress = Some(MediaPosition(position));
        let playback = if paused {
            MediaPlayback::Paused { progress }
        } else {
            MediaPlayback::Playing { progress }
        };
        let _ = self.controls.set_playback(playback);
    }

    pub fn stop(&mut self) {
        let _ = self.controls.set_playback(MediaPlayback::Stopped);
    }
}
