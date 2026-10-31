use crate::AppView;
use crate::ui::{Palette, artwork, label};
use gpui::{prelude::*, *};

impl AppView {
    pub fn statistics_content(&self, p: Palette) -> AnyElement {
        let groups = [
            ("Top artists", self.statistics.top_artists(5)),
            ("Top tracks", self.statistics.top_tracks(5)),
            ("Top albums", self.statistics.top_albums(5)),
        ];
        div()
            .flex()
            .gap_5()
            .mt_5()
            .children(groups.into_iter().map(|(title, entries)| {
                div()
                    .flex_1()
                    .min_w_0()
                    .child(div().font_weight(FontWeight::SEMIBOLD).mb_3().child(title))
                    .when(entries.is_empty(), |el| {
                        el.child(label("Play music to start collecting statistics", p))
                    })
                    .children(entries.into_iter().enumerate().map(|(index, item)| {
                        div()
                            .py_2()
                            .border_b_1()
                            .border_color(rgb(p.line))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(artwork(item.cover_path, String::new(), 28., p))
                                    .child(div().flex_1().min_w_0().truncate().child(format!(
                                        "{}. {}",
                                        index + 1,
                                        item.title
                                    ))),
                            )
                            .child(label(item.subtitle, p))
                            .child(label(
                                format!("{} min · {} plays", item.listened_ms / 60000, item.plays),
                                p,
                            ))
                    }))
            }))
            .into_any_element()
    }
}
