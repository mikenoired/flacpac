use crate::components::icon_button;
use crate::components::player_block::rail;
use crate::library::album_artist;
use crate::playback::RepeatMode;
use crate::ui::SliderValue;
use crate::ui::duration;
use crate::ui::icon;
use crate::ui::{Palette, artwork, label};
use crate::{components::icon_toggle_button, ui::AppView};
use gpui::{prelude::*, *};
use std::time::Duration;

impl AppView {
    fn save_later(&mut self, cx: &mut Context<Self>) {
        self.preferences_task = Some(cx.spawn(async move |view, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(350))
                .await;
            let _ = view.update(cx, |view, _| view.persist());
        }));
    }
    pub fn player(&self, p: Palette, cx: &mut Context<Self>) -> impl IntoElement {
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
                                    .text_size(px(14.))
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
                                                    .text_size(px(14.))
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
                            .text_size(px(12.))
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
