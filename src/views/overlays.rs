use crate::components::icon_button;
use crate::library::{Track, album_artist};
use crate::ui::{
    AppView, Palette, TrackMenuAction, artwork, icon, label, queue_sidebar_row, reveal_track,
};
use gpui::{prelude::*, *};
use std::path::PathBuf;

impl AppView {
    pub(crate) fn application_menu(&self, p: Palette, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("application-menu")
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                this.menu_open = false;
                cx.notify();
                cx.stop_propagation();
            }))
            .absolute()
            .top(px(58.))
            .right(px(24.))
            .w(px(230.))
            .p_2()
            .bg(rgb(p.raised))
            .border_1()
            .border_color(rgb(p.line))
            .rounded_lg()
            .shadow_lg()
            .child(
                div()
                    .px_3()
                    .py_2()
                    .text_size(px(12.))
                    .text_color(rgb(p.muted))
                    .child("Application"),
            )
            .child(
                div()
                    .id("menu-statistics")
                    .track_focus(&self.overlay_focus["menu-statistics"])
                    .tab_index(0)
                    .role(Role::Button)
                    .focus_visible(move |s| s.bg(rgb(p.accent_bg)))
                    .px_3()
                    .py_3()
                    .rounded_md()
                    .cursor_pointer()
                    .hover(move |s| s.bg(rgb(p.accent_bg)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.menu_open = false;
                        this.statistics_open = true;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(icon("statistics", 16., p.text))
                            .child("Listening statistics"),
                    ),
            )
            .child(
                div()
                    .id("menu-settings")
                    .track_focus(&self.overlay_focus["menu-settings"])
                    .tab_index(0)
                    .role(Role::Button)
                    .focus_visible(move |s| s.bg(rgb(p.accent_bg)))
                    .px_3()
                    .py_3()
                    .rounded_md()
                    .cursor_pointer()
                    .hover(move |s| s.bg(rgb(p.accent_bg)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.menu_open = false;
                        this.settings_open = true;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(icon("settings", 16., p.text))
                            .child("Settings"),
                    ),
            )
            .into_any_element()
    }
    pub(crate) fn settings_modal(&self, p: Palette, cx: &mut Context<Self>) -> AnyElement {
        let about = self.settings_about;
        div()
            .id("settings-modal")
            .absolute()
            .inset_0()
            .occlude()
            .bg(rgba(0x000000bb))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(700.))
                    .h(px(440.))
                    .max_w(relative(0.92))
                    .flex()
                    .rounded_lg()
                    .overflow_hidden()
                    .bg(rgb(p.panel))
                    .border_1()
                    .border_color(rgb(p.line))
                    .child(
                        div()
                            .w(px(180.))
                            .flex_shrink_0()
                            .p_4()
                            .border_r_1()
                            .border_color(rgb(p.line))
                            .child(
                                div()
                                    .mb_6()
                                    .text_size(px(20.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Settings"),
                            )
                            .children([(false, "Appearance"), (true, "About")].into_iter().map(
                                |(tab, title)| {
                                    div()
                                        .id(format!("settings-tab-{tab}"))
                                        .aria_selected(about == tab)
                                        .track_focus(
                                            &self.overlay_focus[&format!("settings-tab-{tab}")],
                                        )
                                        .tab_index(0)
                                        .role(Role::Button)
                                        .focus_visible(move |s| s.bg(rgb(p.raised)))
                                        .p_3()
                                        .mb_1()
                                        .rounded_md()
                                        .bg(rgb(if about == tab { p.raised } else { p.panel }))
                                        .cursor_pointer()
                                        .hover(move |s| s.bg(rgb(p.raised)))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.settings_about = tab;
                                            cx.notify();
                                        }))
                                        .child(title)
                                },
                            )),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .p_6()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .mb_5()
                                    .child(
                                        div()
                                            .text_size(px(22.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(if about { "About" } else { "Appearance" }),
                                    )
                                    .child(
                                        icon_button(
                                            "close-settings",
                                            "close",
                                            "Close settings",
                                            p,
                                            true,
                                            false,
                                            cx.listener(|this, _, _, cx| {
                                                this.settings_open = false;
                                                cx.notify();
                                            }),
                                        )
                                        .track_focus(&self.overlay_focus["close-settings"]),
                                    ),
                            )
                            .when(about, |el| {
                                el.child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_3()
                                        .child(
                                            div()
                                                .text_size(px(20.))
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .child("Flacpac"),
                                        )
                                        .child(label(
                                            concat!("Version ", env!("CARGO_PKG_VERSION")),
                                            p,
                                        ))
                                        .child(label("Local music player", p))
                                        .child(label(
                                            format!(
                                                "Platform: {} · {}",
                                                std::env::consts::OS,
                                                std::env::consts::ARCH
                                            ),
                                            p,
                                        ))
                                        .child(label(
                                            format!("Library: {} tracks", self.tracks.len()),
                                            p,
                                        )),
                                )
                            })
                            .when(!about, |el| {
                                el.child(label("Appearance mode", p))
                                    .child(div().flex().gap_2().mt_2().mb_5().children(
                                        [(false, "Dark"), (true, "Light")].into_iter().map(
                                            |(light, title)| {
                                                div()
                                                    .id(format!("settings-mode-{light}"))
                                                    .aria_toggled(if self.light == light {
                                                        Toggled::True
                                                    } else {
                                                        Toggled::False
                                                    })
                                                    .track_focus(
                                                        &self.overlay_focus
                                                            [&format!("settings-mode-{light}")],
                                                    )
                                                    .tab_index(0)
                                                    .role(Role::Button)
                                                    .focus_visible(move |s| s.bg(rgb(p.raised)))
                                                    .flex_1()
                                                    .p_2()
                                                    .rounded_md()
                                                    .border_1()
                                                    .border_color(rgb(if self.light == light {
                                                        p.accent
                                                    } else {
                                                        p.line
                                                    }))
                                                    .cursor_pointer()
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        this.light = light;
                                                        if let Some(search) = &this.search {
                                                            search.update(cx, |search, cx| {
                                                                search.light = light;
                                                                cx.notify();
                                                            });
                                                        }
                                                        this.persist();
                                                        cx.notify();
                                                    }))
                                                    .child(title)
                                            },
                                        ),
                                    ))
                                    .child(label("Color palette", p))
                                    .child(div().mt_3().grid().grid_cols(2).gap_2().children(
                                        crate::settings::PALETTES.into_iter().map(|name| {
                                            let dark = name.definition(false);
                                            let light = name.definition(true);
                                            let current = self.palette == name;
                                            div()
                                                .id(format!("palette-{}", name.as_str()))
                                                .aria_toggled(if current {
                                                    Toggled::True
                                                } else {
                                                    Toggled::False
                                                })
                                                .track_focus(
                                                    &self.overlay_focus
                                                        [&format!("palette-{}", name.as_str())],
                                                )
                                                .tab_index(0)
                                                .role(Role::Button)
                                                .focus_visible(move |s| s.bg(rgb(p.raised)))
                                                .flex()
                                                .items_center()
                                                .gap_3()
                                                .p_2()
                                                .rounded_md()
                                                .border_1()
                                                .border_color(rgb(if current {
                                                    p.accent
                                                } else {
                                                    p.line
                                                }))
                                                .cursor_pointer()
                                                .hover(move |s| s.bg(rgb(p.raised)))
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.palette = name;
                                                    if let Some(search) = &this.search {
                                                        search.update(cx, |search, cx| {
                                                            search.palette = name;
                                                            cx.notify();
                                                        });
                                                    }
                                                    this.persist();
                                                    cx.notify();
                                                }))
                                                .child(
                                                    div()
                                                        .size(px(28.))
                                                        .flex_shrink_0()
                                                        .rounded_full()
                                                        .bg(linear_gradient(
                                                            45.,
                                                            linear_color_stop(
                                                                rgb(dark.accent),
                                                                0.5,
                                                            ),
                                                            linear_color_stop(
                                                                rgb(light.accent),
                                                                0.5,
                                                            ),
                                                        )),
                                                )
                                                .child(div().flex_1().child(dark.name))
                                                .when(current, |el| {
                                                    el.child(icon("check", 16., p.accent))
                                                })
                                        }),
                                    ))
                            }),
                    ),
            )
            .into_any_element()
    }
    pub(crate) fn track_context_menu(
        &self,
        track: Track,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let artist = crate::library::split_artist_credits(&track.artist)
            .into_iter()
            .next()
            .unwrap_or_default();
        let album = track.album.clone();
        let album_artist = album_artist(&track);
        let path = track.path.clone();
        let menu_item = |id: &'static str,
                         icon_name: &'static str,
                         title: &'static str,
                         action: TrackMenuAction| {
            let item_track = track.clone();
            div()
                .id(id)
                .flex()
                .items_center()
                .gap_3()
                .px_3()
                .py_2()
                .rounded_md()
                .cursor_pointer()
                .hover(move |s| s.bg(rgb(p.accent_bg)))
                .on_click(cx.listener(move |this, _, _, cx| {
                    action(this, &item_track, cx);
                    this.context_track = None;
                    cx.notify();
                }))
                .child(icon(icon_name, 16., p.text))
                .child(title)
        };
        div()
            .debug_selector(|| "track-context-menu".into())
            .absolute()
            .top(self.context_menu_position.y)
            .left(self.context_menu_position.x)
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                this.context_track = None;
                cx.notify();
            }))
            .w(px(220.))
            .p_2()
            .bg(rgb(p.raised))
            .border_1()
            .border_color(rgb(p.line))
            .rounded_lg()
            .shadow_lg()
            .child(menu_item(
                "track-open-artist",
                "artist",
                "Open artist",
                Box::new(move |this, _, cx| this.open_artist(artist.clone(), cx)),
            ))
            .child(menu_item(
                "track-open-album",
                "album",
                "Open album",
                Box::new(move |this, _, cx| {
                    this.open_album(album.clone(), album_artist.clone(), cx)
                }),
            ))
            .child(menu_item(
                "track-show-file",
                "folder",
                if cfg!(target_os = "macos") {
                    "Show in Finder"
                } else if cfg!(target_os = "windows") {
                    "Show in Explorer"
                } else {
                    "Show in file manager"
                },
                Box::new(move |_, _, _| {
                    reveal_track(&path);
                }),
            ))
            .child(menu_item(
                "track-add-queue",
                "plus",
                "Add to queue",
                Box::new(|this, track, cx| {
                    this.queue.push_back(track.clone());
                    this.persist();
                    cx.notify();
                }),
            ))
            .into_any_element()
    }
    pub(crate) fn queue_sidebar(&self, p: Palette, cx: &mut Context<Self>) -> AnyElement {
        let queue = self.upcoming_tracks();
        let count = queue.len();
        let view = cx.weak_entity();
        let list = uniform_list(
            "queue-sidebar-list",
            count,
            cx.processor(move |_, range: std::ops::Range<usize>, _, _| {
                range
                    .filter_map(|index| {
                        queue
                            .get(index)
                            .cloned()
                            .map(|track| queue_sidebar_row(track, index, p, view.clone()))
                    })
                    .collect()
            }),
        )
        .size_full();
        div()
            .debug_selector(|| "queue-sidebar".into())
            .w(px(300.))
            .flex_shrink_0()
            .h_full()
            .flex()
            .flex_col()
            .bg(rgb(p.panel))
            .border_l_1()
            .border_color(rgb(p.line))
            .child(
                div()
                    .h(px(54.))
                    .px_4()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().font_weight(FontWeight::SEMIBOLD).child("Play queue"))
                    .child(icon_button(
                        "close-queue-sidebar",
                        "close",
                        "Close queue",
                        p,
                        true,
                        false,
                        cx.listener(|this, _, _, cx| {
                            this.queue_sidebar_open = false;
                            cx.notify();
                        }),
                    )),
            )
            .when_some(self.current_track.as_ref(), |el, track| {
                el.child(
                    div()
                        .w_full()
                        .px_3()
                        .py_3()
                        .border_b_1()
                        .border_color(rgb(p.line))
                        .child(label("Now playing", p))
                        .child(
                            div()
                                .mt_2()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(artwork(
                                    track.cover_path.clone(),
                                    track.title.clone(),
                                    40.,
                                    p,
                                ))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            div()
                                                .truncate()
                                                .text_color(rgb(p.accent))
                                                .child(track.title.clone()),
                                        )
                                        .child(
                                            div()
                                                .truncate()
                                                .text_color(rgb(p.muted))
                                                .text_size(px(12.))
                                                .child(track.artist.clone()),
                                        ),
                                ),
                        ),
                )
            })
            .child(div().w_full().px_3().py_2().child(label("Up next", p)))
            .child(
                div()
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .when(count > 0, |el| el.child(list))
                    .when(count == 0, |el| {
                        el.child(
                            div().px_4().py_4().child("Nothing queued").child(
                                div()
                                    .mt_2()
                                    .text_color(rgb(p.muted))
                                    .child("Use a track’s menu to add it to the queue."),
                            ),
                        )
                    }),
            )
            .into_any_element()
    }

    pub(crate) fn cover_modal(
        &self,
        path: PathBuf,
        window: &Window,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let size = window.viewport_size();
        let side = (f32::from(size.width) - 96.)
            .min(f32::from(size.height) - 128.)
            .clamp(120., 900.);
        div()
            .absolute()
            .inset_0()
            .occlude()
            .bg(rgba(0x000000dd))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .id("artwork-modal")
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.artwork_modal = None;
                    cx.notify();
                }),
            )
            .child(
                div().w(px(side)).flex().justify_end().child(
                    icon_button(
                        "close-artwork",
                        "close",
                        "Close artwork",
                        p,
                        true,
                        false,
                        cx.listener(|this, _, _, cx| {
                            this.artwork_modal = None;
                            cx.notify();
                        }),
                    )
                    .track_focus(&self.overlay_focus["close-artwork"]),
                ),
            )
            .child(
                div()
                    .debug_selector(|| "expanded-artwork".into())
                    .size(px(side))
                    .child(
                        img(path)
                            .size_full()
                            .object_fit(ObjectFit::Contain)
                            .rounded_lg(),
                    ),
            )
            .into_any_element()
    }
    pub(crate) fn statistics_panel(&self, p: Palette, cx: &mut Context<Self>) -> AnyElement {
        div()
            .absolute()
            .inset_0()
            .occlude()
            .bg(rgba(0x000000bb))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(680.))
                    .max_h(relative(0.85))
                    .p_6()
                    .rounded_lg()
                    .bg(rgb(p.panel))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(22.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Listening statistics"),
                            )
                            .child(
                                icon_button(
                                    "close-statistics",
                                    "close",
                                    "Close statistics",
                                    p,
                                    true,
                                    false,
                                    cx.listener(|this, _, _, cx| {
                                        this.statistics_open = false;
                                        cx.notify();
                                    }),
                                )
                                .track_focus(&self.overlay_focus["close-statistics"]),
                            ),
                    )
                    .child(self.statistics_content(p)),
            )
            .into_any_element()
    }
}
