use crate::components::{icon_button, text_button};
use crate::library::{Album, Track};
use crate::ui::{AppView, Page, Palette, artist_cover, artwork, empty_state, label};
use gpui::{prelude::*, *};

impl AppView {
    pub(crate) fn current_album(&self) -> Option<&Album> {
        if self.page != Page::AlbumDetail {
            return None;
        }
        self.selected_album.as_ref().and_then(|(title, artist)| {
            self.albums
                .iter()
                .find(|album| &album.title == title && &album.artist == artist)
        })
    }
    pub(crate) fn title(&self) -> String {
        match self.page {
            Page::Tracks => "All tracks".into(),
            Page::Albums => "Albums".into(),
            Page::Artists => "Artists".into(),
            Page::Queue => "Play queue".into(),
            Page::AlbumDetail => self
                .selected_album
                .as_ref()
                .map(|a| a.0.clone())
                .unwrap_or_default(),
            Page::ArtistDetail => self.selected_artist.clone().unwrap_or_default(),
        }
    }
    pub fn page_key(&self) -> String {
        match self.page {
            Page::Tracks => "tracks".into(),
            Page::Queue => "queue".into(),
            Page::Artists => "artists".into(),
            Page::Albums => format!("albums-{}", self.album_grid),
            Page::AlbumDetail => format!("album-{:?}", self.selected_album),
            Page::ArtistDetail => format!("artist-{:?}", self.selected_artist),
        }
    }
    pub fn overlay(&self) -> Option<&'static str> {
        if self.artwork_modal.is_some() {
            Some("artwork")
        } else if self.settings_open {
            Some("settings")
        } else if self.statistics_open {
            Some("statistics")
        } else if self.menu_open {
            Some("menu")
        } else {
            None
        }
    }

    pub fn overlay_stops(&self) -> Vec<String> {
        match self.overlay() {
            Some("menu") => vec!["menu-statistics".into(), "menu-settings".into()],
            Some("settings") => {
                let mut stops = vec![
                    "settings-tab-false".into(),
                    "settings-tab-true".into(),
                    "close-settings".into(),
                ];
                if !self.settings_about {
                    stops.extend(["settings-mode-false".into(), "settings-mode-true".into()]);
                    stops.extend(
                        crate::settings::PALETTES
                            .iter()
                            .map(|p| format!("palette-{}", p.as_str())),
                    );
                }
                stops
            }
            Some("statistics") => vec!["close-statistics".into()],
            Some("artwork") => vec!["close-artwork".into()],
            _ => vec![],
        }
    }

    pub(crate) fn empty_content(&self) -> (&'static str, &'static str, Option<&'static str>) {
        if self.page == Page::Queue {
            return (
                "Your queue is empty",
                "Use a track’s menu to add it to the queue.",
                None,
            );
        }
        if self.tracks.is_empty() {
            if self.scanning {
                return (
                    "Loading your library",
                    "Indexing continues in the background.",
                    None,
                );
            }
            if self.library_unavailable {
                return (
                    "Music folder unavailable",
                    "Reconnect the drive or choose another music folder.",
                    Some("Choose folder"),
                );
            }
            if self.root.is_none() {
                return (
                    "Your library starts here",
                    "Choose a music folder to start listening.",
                    Some("Choose folder"),
                );
            }
        }
        if !self.search_query.trim().is_empty() {
            return (
                "No results",
                "Try another title, artist or album.",
                Some("Clear search"),
            );
        }
        (
            "No music found",
            "Choose a folder containing supported audio files.",
            Some("Choose folder"),
        )
    }

    pub(crate) fn status_banner(&self, p: Palette, cx: &mut Context<Self>) -> Div {
        div()
            .debug_selector(|| "status-banner".into())
            .px_6()
            .py_2()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(div().flex_1().min_w_0().text_size(px(12.)).child(
                        if self.library_unavailable {
                            if self.tracks.is_empty() {
                                "Music folder unavailable.".into()
                            } else {
                                "Music folder unavailable. Saved library retained.".into()
                            }
                        } else {
                            self.status.clone().unwrap_or_default()
                        },
                    ))
                    .when(self.library_unavailable, |el| {
                        el.child(text_button(
                            "recover-folder",
                            "folder",
                            "Choose folder",
                            p,
                            false,
                            cx.listener(|this, _, _, cx| this.choose_folder(cx)),
                        ))
                    })
                    .when(
                        self.library_unavailable && self.library_error.is_some(),
                        |el| {
                            el.child(text_button(
                                "status-details",
                                "menu",
                                if self.status_details {
                                    "Hide details"
                                } else {
                                    "Details"
                                },
                                p,
                                false,
                                cx.listener(|this, _, _, cx| {
                                    this.status_details = !this.status_details;
                                    cx.notify();
                                }),
                            ))
                        },
                    ),
            )
            .when(self.library_unavailable && self.status_details, |el| {
                el.child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(p.muted))
                        .child(self.library_error.clone().unwrap_or_default()),
                )
            })
            .when(self.library_unavailable && self.status.is_some(), |el| {
                el.child(
                    div()
                        .text_size(px(12.))
                        .child(self.status.clone().unwrap_or_default()),
                )
            })
    }
    pub(crate) fn hero(&self, p: Palette, cx: &mut Context<Self>) -> AnyElement {
        let indices = self.detail_indices();
        let first = indices.first().and_then(|index| self.tracks.get(*index));
        let selected_artist = (self.page == Page::ArtistDetail)
            .then_some(self.selected_artist.as_ref())
            .flatten()
            .and_then(|name| self.artists.iter().find(|artist| &artist.name == name));
        let total = self
            .current_album()
            .map(|album| album.duration_ms)
            .or_else(|| selected_artist.map(|artist| artist.duration_ms))
            .unwrap_or_default();
        let genre = first.and_then(|track| track.genre.clone());
        let title = self.title();
        let artist = self.current_album().map(|album| album.artist.clone());
        let artwork_path = first.and_then(|track| track.cover_path.clone());
        let year = self.current_album().and_then(|album| album.year);
        let play = indices
            .first()
            .and_then(|index| self.tracks.get(*index))
            .cloned();
        let mut hero = div()
            .flex()
            .items_center()
            .gap_5()
            .py_4()
            .flex_shrink_0()
            .child(
                div()
                    .id("hero-artwork")
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.artwork_modal = artwork_path.clone();
                        cx.notify();
                    }))
                    .child(if self.page == Page::AlbumDetail {
                        artwork(
                            first.and_then(|t| t.cover_path.clone()),
                            title.clone(),
                            200.,
                            p,
                        )
                    } else {
                        artist_cover(
                            first.and_then(|t| t.cover_path.clone()),
                            title.clone(),
                            200.,
                            p,
                        )
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(label(
                        if self.page == Page::AlbumDetail {
                            "ALBUM"
                        } else {
                            "ARTIST"
                        },
                        p,
                    ))
                    .child(
                        div()
                            .text_size(px(26.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .truncate()
                            .child(title),
                    )
                    .when_some(artist, |el, credit| {
                        el.child(div().flex().items_center().gap_2().children(
                            credit.split(" · ").enumerate().map(|(index, name)| {
                                let name = name.to_owned();
                                let target = name.clone();
                                div()
                                    .id(format!("album-artist-{index}"))
                                    .cursor_pointer()
                                    .text_size(px(13.))
                                    .text_color(rgb(p.accent))
                                    .hover(move |style| style.text_color(rgb(p.text)))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.open_artist(target.clone(), cx)
                                    }))
                                    .child(name)
                            }),
                        ))
                    })
                    .child(label(
                        format!(
                            "{}{} tracks · {} min{}",
                            year.map_or(String::new(), |year| format!("{year} · ")),
                            indices.len(),
                            total / 60000,
                            genre.map_or(String::new(), |genre| format!(" · {genre}"))
                        ),
                        p,
                    ))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(icon_button(
                                "play-collection",
                                "play",
                                "Play",
                                p,
                                true,
                                false,
                                cx.listener(move |this, _, _, cx| {
                                    if let Some(track) = play.clone() {
                                        this.play_track(track, cx);
                                    }
                                }),
                            ))
                            .child(text_button(
                                "queue-collection",
                                "plus",
                                "Add to queue",
                                p,
                                false,
                                cx.listener(move |this, _, _, cx| {
                                    let queued: Vec<Track> = this
                                        .detail_indices()
                                        .iter()
                                        .map(|index| this.tracks[*index].clone())
                                        .collect();
                                    this.queue.extend(queued);
                                    this.persist();
                                    cx.notify();
                                }),
                            )),
                    ),
            );
        if self.page == Page::ArtistDetail {
            let albums: Vec<Album> = selected_artist
                .map(|artist| {
                    artist
                        .album_indices
                        .iter()
                        .take(4)
                        .filter_map(|index| self.albums.get(*index))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            hero = hero.child(
                div()
                    .w(px(240.))
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(label("ALBUMS", p))
                    .children(albums.into_iter().map(|album| {
                        let title = album.title.clone();
                        let artist = album.artist.clone();
                        div()
                            .id(format!("artist-album-{}", album.title))
                            .flex()
                            .items_center()
                            .gap_2()
                            .cursor_pointer()
                            .rounded_md()
                            .hover(move |s| s.bg(rgb(p.raised)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.open_album(title.clone(), artist.clone(), cx)
                            }))
                            .child(artwork(album.cover_path, album.title.clone(), 28., p))
                            .child(div().text_size(px(12.)).truncate().child(album.title))
                    })),
            );
        }
        hero.into_any_element()
    }
}

impl AppView {
    pub(crate) fn header(&self, p: Palette, cx: &mut Context<Self>) -> Div {
        div()
            .debug_selector(|| "app-header".into())
            .h(px(66.))
            .flex_shrink_0()
            .px_6()
            .border_b_1()
            .border_color(rgb(p.line))
            .flex()
            .items_center()
            .gap_3()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap_2()
                    .when(!self.navigation.is_empty(), |el| {
                        el.child(icon_button(
                            "back",
                            "back",
                            "Go back",
                            p,
                            true,
                            false,
                            cx.listener(|this, _, _, cx| this.back(cx)),
                        ))
                    })
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_size(px(19.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(self.title()),
                    ),
            )
            .when_some(self.search.clone(), |el, search| el.child(search))
            .child(
                div().flex_1().flex().justify_end().gap_2().child(
                    icon_button(
                        "app-menu",
                        "menu",
                        "Application menu",
                        p,
                        true,
                        self.menu_open,
                        cx.listener(|this, _, _, cx| {
                            this.menu_open = !this.menu_open;
                            cx.notify();
                        }),
                    )
                    .track_focus(&self.overlay_focus["app-menu"]),
                ),
            )
    }
    pub(crate) fn toolbar(&self, count: usize, p: Palette, cx: &mut Context<Self>) -> Div {
        let description = match self.page {
            Page::Tracks => self
                .root
                .as_ref()
                .and_then(|path| path.file_name())
                .map(|name| format!("{} · {count} tracks", name.to_string_lossy()))
                .unwrap_or("Your local music collection".into()),
            Page::Albums => format!("{count} albums"),
            Page::Artists => format!("{count} artists"),
            _ => format!("{count} tracks up next"),
        };
        div()
            .h(px(56.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .child(label(description, p))
            .when(self.page == Page::Tracks, |el| {
                el.child(
                    div()
                        .flex()
                        .gap_2()
                        .child(text_button(
                            "play-library",
                            "play",
                            "Play",
                            p,
                            true,
                            cx.listener(|this, _, _, cx| this.play_collection(false, cx)),
                        ))
                        .child(text_button(
                            "shuffle-library",
                            "shuffle",
                            "Shuffle",
                            p,
                            false,
                            cx.listener(|this, _, _, cx| this.play_collection(true, cx)),
                        )),
                )
            })
            .when(self.page == Page::Albums, |el| {
                el.child(icon_button(
                    "album-layout",
                    if self.album_grid { "list" } else { "grid" },
                    "Switch album layout",
                    p,
                    true,
                    false,
                    cx.listener(|this, _, _, cx| {
                        this.album_grid = !this.album_grid;
                        this.persist();
                        cx.notify();
                    }),
                ))
            })
            .when(self.page == Page::Queue && !self.queue.is_empty(), |el| {
                el.child(text_button(
                    "clear-queue",
                    "close",
                    "Clear queue",
                    p,
                    false,
                    cx.listener(|this, _, _, cx| {
                        this.queue.clear();
                        this.pending_queue.clear();
                        this.persist();
                        cx.notify();
                    }),
                ))
            })
    }
    pub(crate) fn library_content(
        &self,
        rows: impl IntoElement,
        count: usize,
        p: Palette,
        cx: &mut Context<Self>,
    ) -> Div {
        let detail = matches!(self.page, Page::AlbumDetail | Page::ArtistDetail);
        let (empty_title, empty_description, action) = self.empty_content();
        div()
            .flex_1()
            .min_h_0()
            .px_6()
            .flex()
            .flex_col()
            .when(detail, |el| el.child(self.hero(p, cx)))
            .when(!detail, |el| el.child(self.toolbar(count, p, cx)))
            .when(
                !matches!(self.page, Page::Albums | Page::Artists) && count > 0,
                |el| el.child(self.table_header(p, cx)),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .when(count > 0, |el| el.child(rows))
                    .when(count == 0, |el| {
                        el.child(
                            div()
                                .size_full()
                                .flex()
                                .flex_col()
                                .items_center()
                                .justify_center()
                                .gap_3()
                                .child(empty_state(empty_title, empty_description, p).size_auto())
                                .when_some(action, |el, action| {
                                    el.child(text_button(
                                        "empty-action",
                                        if action == "Clear search" {
                                            "close"
                                        } else {
                                            "folder"
                                        },
                                        action,
                                        p,
                                        true,
                                        cx.listener(move |this, _, _, cx| {
                                            if action == "Clear search" {
                                                if let Some(search) = &this.search {
                                                    search
                                                        .update(cx, |search, cx| search.clear(cx));
                                                }
                                            } else {
                                                this.choose_folder(cx);
                                            }
                                        }),
                                    ))
                                }),
                        )
                    }),
            )
    }
}
