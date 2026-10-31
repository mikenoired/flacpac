use crate::ui::{AppView, ColumnResize, Page, Palette, SortMode, icon};
use gpui::{prelude::*, *};

impl AppView {
    pub fn table_header(&self, p: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        if self.page == Page::AlbumDetail {
            return div()
                .id("track-table-header")
                .h(px(32.))
                .flex_shrink_0()
                .flex()
                .items_center()
                .px_3()
                .border_b_1()
                .border_color(rgb(p.line))
                .text_size(px(10.))
                .text_color(rgb(p.muted))
                .child(div().w(px(34.)).flex_shrink_0().child("#"))
                .child(div().flex_1().min_w_0().px_2().child("TITLE / ARTIST"))
                .child(
                    div()
                        .w(px(88.))
                        .flex_shrink_0()
                        .px_2()
                        .text_right()
                        .child("LENGTH"),
                )
                .into_any_element();
        }
        let queued = self.page == Page::Queue;
        let ratios = if self.table_ratios == [0.; 4] {
            [0.44, 0.28, 0.20, 0.08]
        } else {
            self.table_ratios
        };
        let available = self.table_available.max(300.);
        let mut header = div()
            .id("track-table-header")
            .h(px(32.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .px_3()
            .border_b_1()
            .border_color(rgb(p.line))
            .text_size(px(12.))
            .text_color(rgb(p.muted))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.persist()),
            )
            .on_drag_move(
                cx.listener(|this, event: &DragMoveEvent<ColumnResize>, _, cx| {
                    let drag = event.drag(cx);
                    let boundary = f32::from(event.bounds.origin.x)
                        + 46.
                        + drag.ratios[..=drag.column].iter().sum::<f32>() * drag.available;
                    let delta = (f32::from(event.event.position.x) - boundary) / drag.available;
                    let col = drag.column;
                    let delta = delta.clamp(0.07 - drag.ratios[col], drag.ratios[col + 1] - 0.07);
                    this.table_ratios = drag.ratios;
                    this.table_ratios[col] += delta;
                    this.table_ratios[col + 1] -= delta;
                    cx.notify();
                }),
            )
            .child(div().w(px(34.)).flex_shrink_0().child("#"));
        for (column, mode, title) in [
            (0, SortMode::Title, "TITLE"),
            (1, SortMode::Album, "ALBUM"),
            (2, SortMode::Date, "DATE ADDED"),
        ] {
            let active = self.sort == mode;
            let reversed = self.sort_reversed;
            header = header.child(
                div()
                    .id(format!("column-{column}"))
                    .relative()
                    .w(px(ratios[column] * available))
                    .flex_shrink_0()
                    .h_full()
                    .flex()
                    .items_center()
                    .min_w_0()
                    .child(
                        div()
                            .id(format!("sort-column-{column}"))
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .flex()
                            .items_center()
                            .gap_1()
                            .px_2()
                            .cursor_pointer()
                            .tab_index(0)
                            .role(Role::Button)
                            .aria_label(format!("Sort by {title}"))
                            .text_color(rgb(if active { p.accent } else { p.muted }))
                            .hover(move |s| s.text_color(rgb(p.text)).bg(rgb(p.raised)))
                            .on_click(cx.listener(move |this, _, _, cx| this.set_sort(mode, cx)))
                            .child(title)
                            .when(active, |el| {
                                el.child(icon(
                                    if reversed ^ (mode == SortMode::Date) {
                                        "down"
                                    } else {
                                        "up"
                                    },
                                    12.,
                                    p.accent,
                                ))
                            }),
                    )
                    .when(column < 3, |el| {
                        el.child(
                            div()
                                .id(format!("resize-column-{column}"))
                                .absolute()
                                .right(px(0.))
                                .top_0()
                                .w(px(8.))
                                .h_full()
                                .cursor(CursorStyle::ResizeLeftRight)
                                .border_r_1()
                                .border_color(rgb(p.line))
                                .hover(move |s| s.bg(rgb(p.accent_bg)).border_color(rgb(p.accent)))
                                .on_drag(
                                    ColumnResize {
                                        column,
                                        ratios,
                                        available,
                                    },
                                    move |drag, _, _, cx| {
                                        let drag = drag.clone();
                                        cx.new(|_| drag)
                                    },
                                ),
                        )
                    }),
            );
        }
        header
            .child(
                div()
                    .id("sort-duration")
                    .w(px(ratios[3] * available))
                    .flex_shrink_0()
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_1()
                    .pr_2()
                    .cursor_pointer()
                    .text_color(rgb(if self.sort == SortMode::Duration {
                        p.accent
                    } else {
                        p.muted
                    }))
                    .hover(move |s| s.bg(rgb(p.raised)))
                    .on_click(cx.listener(|this, _, _, cx| this.set_sort(SortMode::Duration, cx)))
                    .child(icon(
                        "clock",
                        14.,
                        if self.sort == SortMode::Duration {
                            p.accent
                        } else {
                            p.muted
                        },
                    ))
                    .when(self.sort == SortMode::Duration, |el| {
                        el.child(icon(
                            if self.sort_reversed { "down" } else { "up" },
                            12.,
                            p.accent,
                        ))
                    }),
            )
            .when(queued, |el| el.child(div().w(px(108.)).flex_shrink_0()))
            .into_any_element()
    }
}

#[cfg(test)]
mod table_tests {
    use super::*;
    use crate::library::Track;
    use crate::ui::{LibrarySnapshot, format_added};
    use std::sync::Arc;
    #[::core::prelude::v1::test]
    fn table_album_duration_orders_reverse_without_changing_tracks() {
        let mut view = AppView {
            tracks: Arc::new(vec![
                Track {
                    album: "Zulu".into(),
                    duration_ms: 100,
                    ..Track::default()
                },
                Track {
                    album: "alpha".into(),
                    duration_ms: 300,
                    ..Track::default()
                },
                Track {
                    album: "Beta".into(),
                    duration_ms: 200,
                    ..Track::default()
                },
            ]),
            sort: SortMode::Album,
            ..AppView::default()
        };
        view.prepare_tracks(LibrarySnapshot::new(view.tracks.as_ref().clone()));
        assert_eq!(*view.filtered_track_indices, vec![1, 2, 0]);
        view.sort = SortMode::Duration;
        view.sort_reversed = true;
        view.rebuild_order();
        assert_eq!(*view.filtered_track_indices, vec![1, 2, 0]);
        assert_eq!(view.tracks[0].album, "Zulu");
    }
    #[::core::prelude::v1::test]
    fn added_dates_match_relative_month_boundary() {
        let day = 86_400_000;
        let added = 1_704_412_800_000; // Jan 5, 2024, UTC.
        assert_eq!(format_added(0, added), "—");
        assert_eq!(format_added(added, added + 30_000), "30 seconds ago");
        assert_eq!(format_added(added, added + 60_000), "1 minute ago");
        assert_eq!(format_added(added, added + 3_600_000), "1 hour ago");
        assert_eq!(format_added(added, added + day), "1 day ago");
        assert_eq!(format_added(added, added + 14 * day), "2 weeks ago");
        assert_eq!(format_added(added, added + 30 * day), "Jan 5, 2024");
        assert_eq!(format_added(added, added - 1), "Jan 5, 2024");
    }
}
