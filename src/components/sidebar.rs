use crate::AppView;
use crate::components::icon_button;
use crate::ui::Page;
use crate::ui::Palette;
use crate::ui::Tip;
use crate::ui::icon;
use gpui::{prelude::*, *};

impl AppView {
    fn collapsable_button(
        &self,
        p: Palette,
        is_active: bool,
        icon_name: &'static str,
        title: &'static str,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        target: impl Into<ElementId>,
    ) -> impl IntoElement {
        let collapsed = self.sidebar_collapsed;
        div()
            .id(target)
            .tab_index(0)
            .role(Role::Button)
            .aria_label(title)
            .focus_visible(move |style| style.bg(rgb(p.raised)))
            .h(px(38.))
            .px_3()
            .mb_1()
            .rounded_md()
            .flex()
            .items_center()
            .gap_3()
            .cursor_pointer()
            .bg(rgb(if is_active { p.accent_bg } else { p.panel }))
            .text_color(rgb(if is_active { p.accent } else { p.muted }))
            .text_size(px(13.))
            .hover(move |style| style.bg(rgb(p.raised)).text_color(rgb(p.text)))
            .on_click(on_click)
            .child(icon(
                icon_name,
                17.,
                if is_active { p.accent } else { p.muted },
            ))
            .when(!collapsed, |el| el.child(title))
            .tooltip(move |_, cx| cx.new(|_| Tip(title.into(), p)).into())
    }
    pub fn sidebar(&self, p: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let active = match self.page {
            Page::AlbumDetail => Page::Albums,
            Page::ArtistDetail => Page::Artists,
            page => page,
        };
        let collapsed = self.sidebar_collapsed;
        let nav = [
            (Page::Tracks, "music", "All tracks"),
            (Page::Albums, "album", "Albums"),
            (Page::Artists, "artist", "Artists"),
        ];
        div()
            .debug_selector(|| "sidebar".into())
            .group("sidebar")
            .w(px(if self.sidebar_width > 0. {
                self.sidebar_width
            } else if collapsed {
                64.
            } else {
                184.
            }))
            .overflow_hidden()
            .flex_shrink_0()
            .bg(rgb(p.panel))
            .rounded_lg()
            .border_color(rgb(p.line))
            .p_3()
            .flex()
            .flex_col()
            .child(
                div()
                    .relative()
                    .h(px(42.))
                    .mb_6()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .group("sidebar-logo")
                    .child(
                        div().when(collapsed, |el| el.opacity(0.)).child(
                            svg()
                                .path("brand")
                                .w(px(36.))
                                .h(px(34.))
                                .flex_shrink_0()
                                .text_color(rgb(p.text)),
                        ),
                    )
                    .when(!collapsed, |el| {
                        el.child(
                            div()
                                .text_size(px(19.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("flacpac"),
                        )
                    })
                    .when(collapsed, |el| {
                        el.child(div().absolute().left(px(4.)).top(px(5.)).child(icon_button(
                            "expand-sidebar",
                            "expand",
                            "Expand sidebar",
                            p,
                            true,
                            false,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_sidebar(cx);
                            }),
                        )))
                    })
                    .when(!collapsed, |el| {
                        el.child(div().child(icon_button(
                            "collapse-sidebar",
                            "collapse",
                            "Collapse sidebar",
                            p,
                            true,
                            false,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_sidebar(cx);
                            }),
                        )))
                    }),
            )
            .children(nav.into_iter().map(|(target, name, title)| {
                self.collapsable_button(
                    p,
                    active == target,
                    name,
                    title,
                    cx.listener(move |this, _, _, cx| {
                        if target == Page::Queue {
                            this.queue_sidebar_open = !this.queue_sidebar_open;
                            cx.notify();
                        } else {
                            this.set_page(target, cx);
                        }
                    }),
                    format!("nav-{target:?}"),
                )
            }))
            .child(
                div()
                    .mt_auto()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(self.collapsable_button(
                        p,
                        false,
                        "settings",
                        "Settings",
                        cx.listener(|this, _, _, cx| {
                            this.menu_open = false;
                            this.settings_open = true;
                            cx.notify();
                        }),
                        "settings",
                    ))
                    .child(self.collapsable_button(
                        p,
                        false,
                        "folder",
                        "Music folder",
                        cx.listener(|this, _, _, cx| this.choose_folder(cx)),
                        "library-folder",
                    )),
            )
    }
}
