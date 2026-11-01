use super::*;
use crate::media_controls::SystemMediaControls;
use crate::search::{SearchChanged, SearchInput};
use gpui::prelude::*;
use std::time::Duration;

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
    pub fn persist(&self) {
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
}
