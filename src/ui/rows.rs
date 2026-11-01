use super::*;
use crate::components::icon_button;
use crate::library::{Album, Artist, Track, album_artist};
use gpui::{prelude::*, *};
use std::path::PathBuf;

pub(crate) fn empty_state(title: &'static str, description: &'static str, p: Palette) -> Div {
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_3()
        .text_center()
        .child(
            div()
                .text_color(rgb(p.muted))
                .child(icon("music", 32., p.muted)),
        )
        .child(
            div()
                .text_size(px(18.))
                .font_weight(FontWeight::MEDIUM)
                .child(title),
        )
        .child(label(description, p))
}
#[derive(Clone)]
pub struct ColumnResize {
    pub column: usize,
    pub ratios: [f32; 4],
    pub available: f32,
}
impl Render for ColumnResize {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}
pub(crate) fn queue_sidebar_row(
    track: Track,
    index: usize,
    p: Palette,
    view: WeakEntity<AppView>,
) -> AnyElement {
    let drag_view = view.clone();
    let play_view = view.clone();
    let remove_view = view;
    div()
        .id(format!("queue-sidebar-{index}"))
        .debug_selector(move || format!("queue-row-{index}"))
        .w_full()
        .h(px(66.))
        .px_3()
        .flex()
        .items_center()
        .gap_2()
        .border_b_1()
        .border_color(rgb(p.line))
        .cursor_pointer()
        .hover(move |s| s.bg(rgb(p.raised)))
        .on_click(move |event, _, app| {
            if event.click_count() > 1 {
                let _ = play_view.update(app, |this, cx| this.play_queued_track(index, cx));
            }
        })
        .on_drag(index, |index, _, _, cx| cx.new(|_| QueueDrag(*index)))
        .on_drop(move |from: &QueueDrag, _, app| {
            let _ = drag_view.update(app, |this, cx| this.reorder_queue(from.0, index, cx));
        })
        .child(div().flex_shrink_0().child(icon("drag", 14., p.muted)))
        .child(artwork(track.cover_path, track.title.clone(), 40., p))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().truncate().child(track.title))
                .child(
                    div()
                        .text_color(rgb(p.muted))
                        .text_size(px(12.))
                        .truncate()
                        .child(track.artist),
                ),
        )
        .child(div().flex_shrink_0().child(icon_button(
            format!("remove-queued-{index}"),
            "close",
            "Remove from queue",
            p,
            true,
            false,
            move |_, _, app| {
                app.stop_propagation();
                let _ = remove_view.update(app, |this, cx| this.remove_queued_track(index, cx));
            },
        )))
        .into_any_element()
}
struct QueueDrag(usize);
impl Render for QueueDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}
pub(crate) struct TrackRowState {
    pub(crate) widths: [f32; 4],
    pub(crate) numbered: bool,
    pub(crate) show_artwork: bool,
    pub(crate) album_detail: bool,
    pub(crate) index: usize,
    pub(crate) queued: bool,
    pub(crate) queue_len: usize,
    pub(crate) current: Option<PathBuf>,
    pub(crate) paused: bool,
    pub(crate) selected: Option<PathBuf>,
}
pub(crate) fn track_row(
    track: Track,
    state: TrackRowState,
    p: Palette,
    view: WeakEntity<AppView>,
) -> AnyElement {
    let TrackRowState {
        widths,
        numbered,
        show_artwork,
        album_detail,
        index,
        queued,
        queue_len,
        current,
        paused,
        selected,
    } = state;
    let is_current = current.as_ref() == Some(&track.path);
    let is_selected = selected.as_ref() == Some(&track.path);
    let context_track_for_menu = track.clone();
    let row_path = track.path.clone();
    let row_track = track.clone();
    let row_view = view.clone();
    let up_view = view.clone();
    let down_view = view.clone();
    let add_view = view.clone();
    let album = track.album.clone();
    let album_artist = album_artist(&track);
    let album_view = view.clone();
    let artist = crate::library::split_artist_credits(&track.artist)
        .into_iter()
        .next()
        .unwrap_or_default();
    let artist_view = view.clone();
    div()
        .id(format!("track-{index}"))
        .group(format!("track-row-{index}"))
        .h(px(56.))
        .w_full()
        .flex()
        .items_center()
        .px_3()
        .border_b_1()
        .border_color(rgb(p.line))
        .bg(rgb(if is_selected {
            p.raised
        } else if is_current {
            p.accent_bg
        } else {
            p.bg
        }))
        .hover(move |s| s.bg(rgb(p.raised)))
        .on_mouse_down(MouseButton::Right, {
            let target = view.clone();
            let track = track.clone();
            move |event, _, cx| {
                let _ = target.update(cx, |this, cx| {
                    this.context_menu_position = event.position;
                    this.context_track = Some(track.clone());
                    cx.notify();
                });
                cx.stop_propagation();
            }
        })
        .on_click(move |event, window, cx| {
            let _ = row_view.update(cx, |this, cx| {
                this.selected_track = Some(row_path.clone());
                if let Some(focus) = &this.root_focus {
                    window.focus(focus, cx);
                }
                if event.click_count() > 1 {
                    this.play_track(row_track.clone(), cx);
                }
                cx.notify();
            });
        })
        .child(
            div()
                .w(px(34.))
                .flex_shrink_0()
                .text_size(px(10.))
                .text_color(rgb(if is_current { p.accent } else { p.muted }))
                .child(if is_current && !numbered {
                    icon(if paused { "pause" } else { "volume" }, 13., p.accent).into_any_element()
                } else {
                    div()
                        .child(if !numbered && track.track_number > 0 {
                            track.track_number.to_string()
                        } else {
                            (index + 1).to_string()
                        })
                        .into_any_element()
                }),
        )
        .child(
            div()
                .w(px(if album_detail {
                    widths.iter().sum::<f32>() - 88.
                } else {
                    widths[0]
                }))
                .flex_shrink_0()
                .px_2()
                .min_w_0()
                .flex()
                .flex_row()
                .items_center()
                .when(show_artwork, |el| el.gap_3())
                .when(show_artwork, |el| {
                    el.child(artwork(
                        track.cover_path.clone(),
                        track.title.clone(),
                        36.,
                        p,
                    ))
                })
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(rgb(if is_current { p.accent } else { p.text }))
                                .truncate()
                                .child(track.title),
                        )
                        .child(
                            div()
                                .id(format!("artist-link-{index}"))
                                .text_size(px(12.))
                                .text_color(rgb(p.muted))
                                .cursor_pointer()
                                .hover(move |s| s.text_color(rgb(p.accent)))
                                .truncate()
                                .on_click(move |_, _, cx| {
                                    cx.stop_propagation();
                                    let _ = artist_view.update(cx, |this, cx| {
                                        this.open_artist(artist.clone(), cx)
                                    });
                                })
                                .child(track.artist.replace(';', ",")),
                        ),
                ),
        )
        .when(!album_detail, |el| {
            el.child(
                div()
                    .id(format!("album-link-{index}"))
                    .w(px(widths[1]))
                    .flex_shrink_0()
                    .px_2()
                    .text_size(px(12.))
                    .text_color(rgb(p.muted))
                    .cursor_pointer()
                    .hover(move |s| s.text_color(rgb(p.accent)))
                    .truncate()
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        let _ = album_view.update(cx, |this, cx| {
                            this.open_album(album.clone(), album_artist.clone(), cx)
                        });
                    })
                    .child(track.album.clone()),
            )
        })
        .when(!album_detail, |el| {
            el.child(
                div()
                    .w(px(widths[2]))
                    .flex_shrink_0()
                    .px_2()
                    .text_size(px(11.))
                    .text_color(rgb(p.muted))
                    .truncate()
                    .child(format_added(track.added_unix_ms, unix_now_ms())),
            )
        })
        .child(
            div()
                .w(px(if album_detail { 88. } else { widths[3] }))
                .flex_shrink_0()
                .px_2()
                .text_size(px(11.))
                .text_right()
                .text_color(rgb(p.muted))
                .child(duration(track.duration_ms)),
        )
        .when(!queued, |el| {
            let target = view.clone();
            let context_track = context_track_for_menu.clone();
            el.child(
                div()
                    .opacity(0.)
                    .group_hover(format!("track-row-{index}"), |s| s.opacity(1.))
                    .child(div().w(px(32.)).flex_shrink_0().flex().justify_end().child(
                        icon_button(
                            format!("track-menu-{index}"),
                            "more",
                            "Track actions",
                            p,
                            true,
                            false,
                            move |event, _, cx| {
                                cx.stop_propagation();
                                let _ = target.update(cx, |this, cx| {
                                    this.context_menu_position = event.position();
                                    this.context_track = Some(context_track.clone());
                                    cx.notify();
                                });
                            },
                        ),
                    )),
            )
        })
        .when(queued, |el| {
            el.child(
                div()
                    .w(px(108.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap_1()
                    .when(queued, |el| {
                        el.child(icon_button(
                            format!("queue-up-{index}"),
                            "up",
                            "Move earlier",
                            p,
                            index > 0,
                            false,
                            move |_, _, cx| {
                                cx.stop_propagation();
                                let _ = up_view
                                    .update(cx, |this, cx| this.move_queued_track(index, -1, cx));
                            },
                        ))
                        .child(icon_button(
                            format!("queue-down-{index}"),
                            "down",
                            "Move later",
                            p,
                            index + 1 < queue_len,
                            false,
                            move |_, _, cx| {
                                cx.stop_propagation();
                                let _ = down_view
                                    .update(cx, |this, cx| this.move_queued_track(index, 1, cx));
                            },
                        ))
                    })
                    .child(icon_button(
                        format!("row-queue-{index}"),
                        if queued { "close" } else { "plus" },
                        if queued {
                            "Remove from queue"
                        } else {
                            "Add to queue"
                        },
                        p,
                        true,
                        false,
                        move |_, _, cx| {
                            cx.stop_propagation();
                            let _ = add_view.update(cx, |this, cx| {
                                this.remove_queued_track(index, cx);
                            });
                        },
                    )),
            )
        })
        .into_any_element()
}
pub(crate) fn album_row(
    album: Album,
    index: usize,
    p: Palette,
    view: WeakEntity<AppView>,
) -> AnyElement {
    let title = album.title.clone();
    let artist = album.artist.clone();
    div()
        .id(format!("album-{index}"))
        .debug_selector(move || format!("album-row-{index}"))
        .tab_index(0)
        .role(Role::Button)
        .focus_visible(move |style| style.bg(rgb(p.raised)))
        .w_full()
        .h(px(94.))
        .flex()
        .items_center()
        .gap_4()
        .px_3()
        .border_b_1()
        .border_color(rgb(p.line))
        .cursor_pointer()
        .hover(move |s| s.bg(rgb(p.panel)))
        .on_click(move |_, _, cx| {
            let _ = view.update(cx, |this, cx| {
                this.open_album(title.clone(), artist.clone(), cx)
            });
        })
        .child(artwork(album.cover_path, album.title.clone(), 70., p))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_size(px(15.))
                        .font_weight(FontWeight::MEDIUM)
                        .truncate()
                        .child(album.title),
                )
                .child(label(album.artist, p)),
        )
        .child(label(format!("{} tracks", album.track_indices.len()), p))
        .child(icon("forward", 14., p.muted))
        .into_any_element()
}
pub(crate) fn artist_row(
    artist: Artist,
    index: usize,
    p: Palette,
    view: WeakEntity<AppView>,
) -> AnyElement {
    let name = artist.name.clone();
    div()
        .id(format!("artist-{index}"))
        .tab_index(0)
        .role(Role::Button)
        .focus_visible(move |style| style.bg(rgb(p.raised)))
        .w_full()
        .h(px(82.))
        .flex()
        .items_center()
        .gap_4()
        .px_3()
        .border_b_1()
        .border_color(rgb(p.line))
        .cursor_pointer()
        .hover(move |s| s.bg(rgb(p.panel)))
        .on_click(move |_, _, cx| {
            let _ = view.update(cx, |this, cx| this.open_artist(name.clone(), cx));
        })
        .child(artist_cover(artist.cover_path, artist.name.clone(), 68., p))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(16.))
                .font_weight(FontWeight::BOLD)
                .truncate()
                .child(artist.name),
        )
        .child(label(
            format!(
                "{} albums · {} tracks",
                artist.album_count,
                artist.track_indices.len()
            ),
            p,
        ))
        .child(icon("forward", 14., p.muted))
        .into_any_element()
}
pub(crate) fn artist_grid_row(
    artists: Vec<Artist>,
    row: usize,
    width: f32,
    p: Palette,
    view: WeakEntity<AppView>,
) -> AnyElement {
    div()
        .id(format!("artist-grid-row-{row}"))
        .w_full()
        .flex()
        .gap_4()
        .py_3()
        .children(artists.into_iter().enumerate().map(|(column, artist)| {
            let width = ((column + 1) as f32 * width).round() - (column as f32 * width).round();
            let name = artist.name.clone();
            let target = view.clone();
            div()
                .id(format!("artist-card-{row}-{name}"))
                .w(px(width))
                .flex_shrink_0()
                .flex()
                .flex_col()
                .items_center()
                .gap_2()
                .p_3()
                .rounded_lg()
                .cursor_pointer()
                .hover(move |s| s.bg(rgb(p.raised)))
                .on_click(move |_, _, cx| {
                    let _ = target.update(cx, |this, cx| this.open_artist(name.clone(), cx));
                })
                .child(
                    div()
                        .size(px(124.))
                        .rounded_full()
                        .overflow_hidden()
                        .child(artist_cover(
                            artist.cover_path,
                            artist.name.clone(),
                            124.,
                            p,
                        )),
                )
                .child(
                    div()
                        .w_full()
                        .text_center()
                        .text_size(px(14.))
                        .font_weight(FontWeight::BOLD)
                        .truncate()
                        .child(artist.name),
                )
                .child(label(
                    format!(
                        "{} albums · {} tracks",
                        artist.album_count,
                        artist.track_indices.len()
                    ),
                    p,
                ))
        }))
        .into_any_element()
}
pub(crate) fn album_grid_row(
    albums: Vec<Album>,
    row: usize,
    width: f32,
    p: Palette,
    view: WeakEntity<AppView>,
) -> AnyElement {
    div()
        .h(px(width + 88.))
        .w_full()
        .flex()
        .items_start()
        .gap_4()
        .children(albums.into_iter().enumerate().map(|(column, album)| {
            let width = ((column + 1) as f32 * width).round() - (column as f32 * width).round();
            let view = view.clone();
            let title = album.title.clone();
            let artist = album.artist.clone();
            div()
                .id(format!("album-card-{row}-{column}"))
                .debug_selector(move || format!("album-card-{row}-{column}"))
                .w(px(width))
                .flex_shrink_0()
                .rounded_md()
                .cursor_pointer()
                .tab_index(0)
                .role(Role::Button)
                .hover(move |style| style.bg(rgb(p.panel)))
                .focus_visible(move |style| style.bg(rgb(p.raised)))
                .on_click(move |_, _, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.open_album(title.clone(), artist.clone(), cx)
                    });
                })
                .child(artwork(album.cover_path, album.title.clone(), width, p))
                .child(
                    div()
                        .mt_2()
                        .px_3()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .truncate()
                        .child(album.title),
                )
                .child(
                    div()
                        .mt_1()
                        .px_3()
                        .pb_3()
                        .text_size(px(11.))
                        .text_color(rgb(p.muted))
                        .truncate()
                        .child(album.artist),
                )
        }))
        .into_any_element()
}
