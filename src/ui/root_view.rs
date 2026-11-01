use super::*;
use gpui::{prelude::*, *};

impl Render for AppView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Palette::from_name(self.palette, self.light);
        if self._activation_subscription.is_none() {
            self._activation_subscription =
                Some(cx.observe_window_activation(window, |this, window, cx| {
                    if !window.is_window_active() {
                        this.context_track = None;
                        this.menu_open = false;
                        this.seek_dragging.set(false);
                        this.volume_dragging.set(false);
                        cx.notify();
                    }
                }));
        }
        self.context_menu_position.x = self.context_menu_position.x.clamp(
            px(8.),
            (window.viewport_size().width - px(228.)).max(px(8.)),
        );
        self.context_menu_position.y = self.context_menu_position.y.clamp(
            px(8.),
            (window.viewport_size().height - px(200.)).max(px(8.)),
        );

        self.overlay_focus
            .entry("app-menu".into())
            .or_insert_with(|| cx.focus_handle());
        let overlay = self.overlay();
        let stops = self.overlay_stops();
        for id in &stops {
            self.overlay_focus
                .entry(id.clone())
                .or_insert_with(|| cx.focus_handle());
        }
        if overlay != self.active_overlay {
            if self.active_overlay.is_none() && overlay.is_some() {
                self.return_focus = window.focused(cx);
            }
            if let Some(first) = stops.first() {
                window.focus(&self.overlay_focus[first], cx);
            } else if let Some(focus) = self.return_focus.take() {
                window.focus(&focus, cx);
            }
            self.active_overlay = overlay;
        }
        if window.focused(cx).is_none()
            && let Some(focus) = &self.root_focus
        {
            window.focus(focus, cx);
        }
        let page = self.page;
        let key = self.page_key();

        let indices = self.detail_indices();
        let tracks = self.tracks.clone();
        let albums = self.albums.clone();
        let artists = self.artists.clone();
        let album_indices = self.filtered_album_indices.clone();
        let artist_indices = self.filtered_artist_indices.clone();
        let count = match page {
            Page::Albums => album_indices.len(),
            Page::Artists => artist_indices.len(),
            Page::Queue => self.queue.len(),
            _ => indices.len(),
        };
        let scroll = self.scrolls.entry(key.clone()).or_default().clone();
        if count > 0
            && let Some(offset) = self.pending_offsets.remove(&key)
        {
            scroll
                .0
                .borrow()
                .base_handle
                .set_offset(point(px(0.), px(offset)));
        }
        let view = cx.weak_entity();
        let current = self.current_track.as_ref().map(|track| track.path.clone());
        let paused = self.paused;
        let selected = self.selected_track.clone();
        let sidebar_width = if self.sidebar_width > 0. {
            self.sidebar_width
        } else if self.sidebar_collapsed {
            64.
        } else {
            184.
        };
        let content_width = (f32::from(window.viewport_size().width)
            - sidebar_width
            - if self.queue_sidebar_open { 300. } else { 0. }
            - 48.)
            .max(180.);
        let columns = (content_width / 180.).floor().max(1.) as usize;
        let card_width = (content_width - 16. * (columns - 1) as f32) / columns as f32;
        self.table_available =
            (content_width - 24. - 34. - if page == Page::Queue { 108. } else { 32. }).max(100.);
        let ratios = if self.table_ratios == [0.; 4] {
            [0.44, 0.28, 0.20, 0.08]
        } else {
            self.table_ratios
        };
        let widths = ratios.map(|ratio| ratio * self.table_available);
        let artist_grid = page == Page::Artists;
        let grid = (page == Page::Albums && self.album_grid) || artist_grid;
        let row_count = if grid { count.div_ceil(columns) } else { count };
        let rows = uniform_list(
            key,
            row_count,
            cx.processor(move |this, range: std::ops::Range<usize>, _, _| {
                range
                    .filter_map(|index| match page {
                        Page::Albums if grid => Some(album_grid_row(
                            album_indices
                                .iter()
                                .skip(index * columns)
                                .take(columns)
                                .filter_map(|index| albums.get(*index))
                                .cloned()
                                .collect(),
                            index,
                            card_width,
                            p,
                            view.clone(),
                        )),
                        Page::Albums => album_indices
                            .get(index)
                            .and_then(|index| albums.get(*index))
                            .cloned()
                            .map(|album| album_row(album, index, p, view.clone())),
                        Page::Artists if artist_grid => Some(artist_grid_row(
                            artist_indices
                                .iter()
                                .skip(index * columns)
                                .take(columns)
                                .filter_map(|index| artists.get(*index))
                                .cloned()
                                .collect(),
                            index,
                            card_width,
                            p,
                            view.clone(),
                        )),
                        Page::Artists => artist_indices
                            .get(index)
                            .and_then(|index| artists.get(*index))
                            .cloned()
                            .map(|artist| artist_row(artist, index, p, view.clone())),
                        Page::Queue => this.queue.get(index).cloned().map(|track| {
                            track_row(
                                track,
                                TrackRowState {
                                    index,
                                    widths,
                                    numbered: true,
                                    show_artwork: true,
                                    album_detail: false,
                                    queued: true,
                                    queue_len: this.queue.len(),
                                    current: current.clone(),
                                    paused,
                                    selected: selected.clone(),
                                },
                                p,
                                view.clone(),
                            )
                        }),
                        _ => indices
                            .get(index)
                            .and_then(|index| tracks.get(*index))
                            .cloned()
                            .map(|track| {
                                track_row(
                                    track,
                                    TrackRowState {
                                        index,
                                        widths,
                                        numbered: matches!(page, Page::Tracks | Page::AlbumDetail),
                                        show_artwork: page != Page::AlbumDetail,
                                        album_detail: page == Page::AlbumDetail,
                                        queued: false,
                                        queue_len: 0,
                                        current: current.clone(),
                                        paused,
                                        selected: selected.clone(),
                                    },
                                    p,
                                    view.clone(),
                                )
                            }),
                    })
                    .collect()
            }),
        )
        .track_scroll(&scroll)
        .size_full();
        let body = div()
            .flex_1()
            .min_h_0()
            .flex()
            .child(self.sidebar(p, cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(self.header(p, cx))
                    .when(self.library_unavailable || self.status.is_some(), |el| {
                        el.child(self.status_banner(p, cx))
                    })
                    .child(self.library_content(rows, count, p, cx)),
            )
            .when(self.queue_sidebar_open, |el| {
                el.child(self.queue_sidebar(p, cx))
            });
        div()
            .relative()
            .size_full()
            .bg(rgb(p.bg))
            .text_color(rgb(p.text))
            .font_family("Helvetica Neue")
            .text_size(px(13.))
            .flex()
            .flex_col()
            .when_some(self.root_focus.as_ref(), |el, focus| el.track_focus(focus))
            .tab_group()
            .on_key_down(cx.listener(Self::key))
            .on_scroll_wheel(cx.listener(|this, _, _, cx| {
                if this.context_track.take().is_some() {
                    cx.notify();
                }
            }))
            .child(body)
            .child(self.player(p, cx))
            .when(self.menu_open, |el| el.child(self.application_menu(p, cx)))
            .when(self.statistics_open, |el| {
                el.child(self.statistics_panel(p, cx))
            })
            .when(self.settings_open, |el| {
                el.child(self.settings_modal(p, cx))
            })
            .when_some(self.context_track.clone(), |el, track| {
                el.child(self.track_context_menu(track, p, cx))
            })
            .when_some(self.artwork_modal.clone(), |el, path| {
                el.child(self.cover_modal(path, window, p, cx))
            })
    }
}
