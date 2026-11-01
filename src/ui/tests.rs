use crate::library::Track;
use crate::playback::RepeatMode;
use crate::ui::*;
use std::path::PathBuf;

#[cfg(test)]
mod tests {
    use super::*;
    fn test_tracks() -> Vec<Track> {
        (0..4)
            .map(|index| Track {
                path: PathBuf::from(format!("track-{index}")),
                ..Track::default()
            })
            .collect()
    }
    #[::core::prelude::v1::test]
    fn shuffled_context_visits_every_other_track_once() {
        let tracks = test_tracks();
        let mut view = AppView {
            shuffle: true,
            playback_order: tracks.clone(),
            ..AppView::default()
        };
        view.reset_continuation(&tracks[2].path);
        let mut actual: Vec<_> = view
            .continuation
            .iter()
            .map(|track| track.path.clone())
            .collect();
        actual.sort();
        assert_eq!(
            actual,
            vec![
                tracks[0].path.clone(),
                tracks[1].path.clone(),
                tracks[3].path.clone()
            ]
        );
        view.shuffle = false;
        view.reset_continuation(&tracks[2].path);
        assert_eq!(view.continuation.len(), 1);
        assert_eq!(view.continuation[0].path, tracks[3].path);
    }
    #[::core::prelude::v1::test]
    fn repeat_one_only_affects_automatic_completion_and_preserves_queue() {
        let tracks = test_tracks();
        let mut view = AppView {
            repeat: RepeatMode::One,
            current_track: Some(tracks[0].clone()),
            queue: vec![tracks[1].clone()].into(),
            continuation: vec![tracks[2].clone()].into(),
            ..AppView::default()
        };
        assert_eq!(view.next_track(true).unwrap().path, tracks[0].path);
        assert_eq!(view.queue.len(), 1);
        assert_eq!(view.next_track(false).unwrap().path, tracks[1].path);
        assert_eq!(view.next_track(false).unwrap().path, tracks[2].path);
        assert!(view.next_track(false).is_none());
    }
    #[::core::prelude::v1::test]
    fn repeat_all_restarts_context_and_shuffle_avoids_boundary_duplicate() {
        let tracks = test_tracks();
        let mut view = AppView {
            playback_order: tracks.clone(),
            current_track: Some(tracks[3].clone()),
            ..AppView::default()
        };
        assert!(view.next_track(true).is_none());
        view.repeat = RepeatMode::All;
        assert_eq!(view.next_track(true).unwrap().path, tracks[0].path);
        assert_eq!(view.continuation.len(), 3);
        view.continuation.clear();
        view.shuffle = true;
        let first = view.next_track(true).unwrap();
        assert_ne!(first.path, tracks[3].path);
        assert_eq!(view.continuation.len(), 3);
    }

    #[::core::prelude::v1::test]
    fn unavailable_library_never_produces_a_cache_replacement() {
        let missing = std::env::temp_dir().join(format!(
            "flacpac-missing-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert!(scan_and_cache(missing).is_err());
    }
    #[::core::prelude::v1::test]
    fn large_catalog_preparation_and_search() {
        for count in [1000, 10_000, 100_000] {
            let tracks = (0..count)
                .map(|index| Track {
                    title: format!("Track {index}"),
                    artist: format!("Artist {}", index % 100),
                    album: format!("Album {}", index / 10),
                    added_unix_ms: index as u128,
                    ..Track::default()
                })
                .collect();
            let start = std::time::Instant::now();
            let snapshot = LibrarySnapshot::new(tracks);
            let preparation = start.elapsed();
            let start = std::time::Instant::now();
            let matches = snapshot
                .search_terms
                .iter()
                .filter(|text| text.contains("track 999"))
                .count();
            let search = start.elapsed();
            assert!(matches > 0);
            assert_eq!(snapshot.date_order[0], count - 1);
            assert_eq!(snapshot.tracks.len(), count);
            eprintln!("{count} tracks: prepare {preparation:?}; search {search:?}");
        }
    }
}

#[cfg(test)]
mod layout_tests {
    use super::{AppView, LibrarySnapshot, Page};
    use crate::{library::Track, search::SearchInput};
    use gpui::{AppContext, TestAppContext, px, size};

    #[gpui::test]
    fn compact_and_wide_layouts_keep_search_and_player_inside_the_window(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| {
            let search = cx.new(SearchInput::new);
            let mut view = AppView {
                search: Some(search),
                root_focus: Some(cx.focus_handle()),
                album_grid: true,
                ..AppView::default()
            };
            view.prepare_tracks(LibrarySnapshot::new(
                (0..100)
                    .map(|index| Track {
                        title: format!("A very long track title {index}"),
                        artist: "Artist".into(),
                        album: format!("Album {}", index / 10),
                        track_number: index % 10 + 1,
                        ..Track::default()
                    })
                    .collect(),
            ));
            view
        });
        for light in [false, true] {
            for (width, height) in [(960., 640.), (1440., 900.)] {
                cx.simulate_resize(size(px(width), px(height)));
                cx.update(|_, cx| {
                    view.update(cx, |view, cx| {
                        view.light = light;
                        view.page = Page::Albums;
                        cx.notify();
                    })
                });
                cx.run_until_parked();
                let player = cx.debug_bounds("player-bar").expect("player rendered");
                let header = cx.debug_bounds("app-header").expect("header rendered");
                let search = cx.debug_bounds("search-field").expect("search rendered");
                let sidebar = cx.debug_bounds("sidebar").expect("sidebar rendered");
                assert!(player.bottom() <= px(height) + px(1.));
                assert!((player.size.height - px(94.)).abs() < px(1.));
                assert!(search.left() >= header.left());
                assert!(search.right() <= header.right());
                assert!(
                    ((search.left() + search.right()) / 2. - (header.left() + header.right()) / 2.)
                        .abs()
                        < px(1.)
                );
                assert!((header.left() - sidebar.right()).abs() < px(1.));
                cx.update(|_, cx| {
                    view.update(cx, |view, cx| {
                        view.artwork_modal = Some(std::path::PathBuf::from("/missing-cover.png"));
                        cx.notify();
                    })
                });
                cx.run_until_parked();
                let art = cx.debug_bounds("expanded-artwork").expect("modal artwork");
                assert!(art.left() >= px(0.) && art.top() >= px(0.));
                assert!(art.right() <= px(width) && art.bottom() <= px(height));
                cx.update(|_, cx| {
                    view.update(cx, |view, cx| {
                        view.artwork_modal = None;
                        cx.notify();
                    })
                });
            }
        }
    }
}

#[cfg(test)]
mod interaction_regressions {
    use super::{AppView, LibrarySnapshot, Page, Palette, duration};
    use crate::components::rail;
    use crate::library::Track;
    use gpui::{
        Context, IntoElement, Modifiers, MouseButton, Render, TestAppContext, Window, div, point,
        prelude::*, px, size,
    };
    use std::path::PathBuf;

    #[derive(Default)]
    struct RailHarness {
        value: f32,
        dragging: std::rc::Rc<std::cell::Cell<bool>>,
    }
    impl Render for RailHarness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let view = cx.weak_entity();
            div().w(px(300.)).h(px(100.)).p_5().child(rail(
                "seek",
                super::SliderValue {
                    fraction: self.value,
                    maximum: 100.,
                    step: 5.,
                },
                true,
                Palette::from_name(crate::settings::PaletteName::Forest, false),
                Some(duration((self.value * 100_000.) as u64)),
                self.dragging.clone(),
                move |value, _, app| {
                    let _ = view.update(app, |this, cx| {
                        this.value = value;
                        cx.notify();
                    });
                },
            ))
        }
    }

    #[gpui::test]
    fn scrubber_updates_before_release_and_captures_moves_outside_the_rail(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(|_, _| RailHarness::default());
        cx.run_until_parked();
        let bounds = cx.debug_bounds("rail-seek").unwrap();
        cx.simulate_mouse_down(bounds.center(), MouseButton::Left, Modifiers::default());
        let target = point(
            bounds.left() + bounds.size.width * 0.8,
            bounds.bottom() + px(20.),
        );
        cx.simulate_mouse_move(target, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
        cx.update(|_, cx| {
            assert!((view.read(cx).value - 0.8).abs() < 0.001);
            assert!(view.read(cx).dragging.get());
        });
        let tip = cx.debug_bounds("seek-time-tooltip").unwrap();
        assert_eq!(tip.size.width, px(54.));
        assert!(tip.size.height < px(30.));
        cx.simulate_mouse_up(target, MouseButton::Left, Modifiers::default());
        cx.update(|_, cx| assert!(!view.read(cx).dragging.get()));
    }

    #[gpui::test]
    fn album_list_and_queue_fill_the_width_after_sidebar_changes(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| {
            let mut view = AppView {
                page: Page::Albums,
                ..AppView::default()
            };
            view.prepare_tracks(LibrarySnapshot::new(
                (0..10)
                    .map(|i| Track {
                        title: format!("Track {i}"),
                        album: format!("Album {i}"),
                        artist: "Artist".into(),
                        path: PathBuf::from(format!("{i}.flac")),
                        ..Track::default()
                    })
                    .collect(),
            ));
            view.continuation = view.tracks.iter().cloned().collect();
            let _ = cx;
            view
        });
        cx.simulate_resize(size(px(1200.), px(800.)));
        for sidebar_width in [184., 64.] {
            for queue_open in [false, true] {
                cx.update(|_, cx| {
                    view.update(cx, |view, cx| {
                        view.sidebar_width = sidebar_width;
                        view.sidebar_collapsed = sidebar_width == 64.;
                        view.queue_sidebar_open = queue_open;
                        view.album_grid = false;
                        cx.notify();
                    })
                });
                cx.run_until_parked();
                let row = cx.debug_bounds("album-row-0").unwrap();
                assert!((row.left() - px(sidebar_width + 24.)).abs() < px(1.));
                assert!(
                    (row.right() - px(1200. - if queue_open { 300. } else { 0. } - 24.)).abs()
                        < px(1.)
                );
                if queue_open {
                    let panel = cx.debug_bounds("queue-sidebar").unwrap();
                    let row = cx.debug_bounds("queue-row-0").unwrap();
                    assert!(
                        (row.left() - panel.left()).abs() <= px(1.),
                        "row {row:?}, panel {panel:?}"
                    );
                    assert!(
                        (row.right() - panel.right()).abs() <= px(1.),
                        "row {row:?}, panel {panel:?}"
                    );
                }
                cx.update(|_, cx| {
                    view.update(cx, |view, cx| {
                        view.album_grid = true;
                        cx.notify();
                    })
                });
                cx.run_until_parked();
                let last_card = [
                    "album-card-0-0",
                    "album-card-0-1",
                    "album-card-0-2",
                    "album-card-0-3",
                    "album-card-0-4",
                    "album-card-0-5",
                ]
                .into_iter()
                .filter_map(|selector| cx.debug_bounds(selector))
                .last()
                .unwrap();
                assert!(
                    (last_card.right() - px(1200. - if queue_open { 300. } else { 0. } - 24.))
                        .abs()
                        < px(1.),
                    "card {last_card:?}, sidebar {sidebar_width}, queue {queue_open}"
                );
            }
        }
    }

    #[gpui::test]
    fn track_menu_uses_pointer_position_and_closes_on_outside_press(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, _| AppView {
            context_track: Some(Track::default()),
            context_menu_position: point(px(200.), px(150.)),
            ..AppView::default()
        });
        cx.run_until_parked();
        let menu = cx.debug_bounds("track-context-menu").unwrap();
        assert_eq!(menu.origin, point(px(200.), px(150.)));
        cx.simulate_mouse_down(
            point(px(20.), px(400.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.update(|_, cx| assert!(view.read(cx).context_track.is_none()));
    }

    #[::core::prelude::v1::test]
    fn artist_page_does_not_use_previously_selected_album() {
        let mut view = AppView::default();
        view.prepare_tracks(LibrarySnapshot::new(vec![Track {
            album: "Album".into(),
            artist: "Artist".into(),
            duration_ms: 120_000,
            year: Some(2020),
            ..Track::default()
        }]));
        view.selected_album = Some(("Album".into(), "Artist".into()));
        view.page = Page::AlbumDetail;
        assert_eq!(view.current_album().unwrap().year, Some(2020));
        view.selected_artist = Some("Artist".into());
        view.page = Page::ArtistDetail;
        assert!(view.current_album().is_none());
        // Back navigation can still restore the album selection.
        view.page = Page::AlbumDetail;
        assert_eq!(view.current_album().unwrap().duration_ms, 120_000);
    }

    #[gpui::test]
    fn unavailable_previous_keeps_current_track_and_history(cx: &mut TestAppContext) {
        let current = Track {
            path: "/current.flac".into(),
            ..Track::default()
        };
        let previous = Track {
            path: "/nonexistent-flacpac-review/previous.flac".into(),
            ..Track::default()
        };
        let view = cx.new(|_| AppView {
            current_track: Some(current.clone()),
            history: vec![previous.clone()],
            ..AppView::default()
        });
        view.update(cx, |view, cx| {
            view.play_previous(cx);
            assert_eq!(view.current_track.as_ref().unwrap().path, current.path);
            assert_eq!(view.history[0].path, previous.path);
            assert!(view.continuation.is_empty());
            assert!(view.status.as_ref().unwrap().contains("Cannot play"));
        });
    }

    #[::core::prelude::v1::test]
    fn editing_context_queue_changes_the_actual_playback_order() {
        let tracks: Vec<_> = (0..5)
            .map(|i| Track {
                path: PathBuf::from(format!("{i}.flac")),
                ..Track::default()
            })
            .collect();
        let mut view = AppView {
            queue: tracks[..2].to_vec().into(),
            continuation: tracks[2..].to_vec().into(),
            ..AppView::default()
        };
        assert!(view.reorder_upcoming(4, 0));
        assert!(view.remove_upcoming(2));
        let actual: Vec<_> = std::iter::from_fn(|| view.next_track(false))
            .map(|track| track.path)
            .collect();
        assert_eq!(actual, [4, 0, 2, 3].map(|i| tracks[i].path.clone()));
    }
}

#[cfg(test)]
mod design_regressions {
    use super::{AppView, Page, Palette, SliderValue, Track};
    use crate::components::rail;
    use gpui::{
        AppContext, Context, FocusHandle, IntoElement, Modifiers, MouseButton, Render,
        TestAppContext, Window, div, point, prelude::*, px,
    };
    use std::sync::Arc;

    #[::core::prelude::v1::test]
    fn empty_source_and_empty_search_results_offer_the_right_action() {
        let mut view = AppView {
            search_query: "missing".into(),
            ..AppView::default()
        };
        assert_eq!(view.empty_content().2, Some("Choose folder"));
        view.scanning = true;
        assert_eq!(view.empty_content().0, "Loading your library");
        assert_eq!(view.empty_content().2, None);
        view.scanning = false;
        view.library_unavailable = true;
        assert_eq!(view.empty_content().0, "Music folder unavailable");
        view.tracks = Arc::new(vec![Track::default()]);
        assert_eq!(view.empty_content().0, "No results");
        assert_eq!(view.empty_content().2, Some("Clear search"));
        view.page = Page::Queue;
        assert_eq!(view.empty_content().0, "Your queue is empty");
    }

    #[gpui::test]
    fn keyboard_opens_settings_traps_focus_and_returns_to_the_opener(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| AppView {
            root_focus: Some(cx.focus_handle()),
            search: Some(cx.new(crate::search::SearchInput::new)),
            ..AppView::default()
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            let focus = view.read(cx).overlay_focus["app-menu"].clone();
            window.focus(&focus, cx);
        });
        cx.run_until_parked();
        cx.simulate_keystrokes("enter");
        cx.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse("enter").unwrap(),
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert!(view.read(cx).overlay_focus["menu-statistics"].is_focused(window))
        });
        cx.simulate_keystrokes("tab");
        cx.run_until_parked();
        cx.simulate_keystrokes("enter");
        cx.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse("enter").unwrap(),
        });
        cx.run_until_parked();
        cx.read_entity(&view, |view, _| assert!(view.settings_open));
        let behind = cx.debug_bounds("search-field").unwrap().center();
        cx.simulate_click(behind, Modifiers::default());
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert!(
                !view
                    .read(cx)
                    .search
                    .as_ref()
                    .unwrap()
                    .read(cx)
                    .focus
                    .is_focused(window)
            )
        });
        for _ in 0..20 {
            cx.simulate_keystrokes("tab");
            cx.run_until_parked();
            cx.update(|window, cx| {
                let state = view.read(cx);
                assert!(
                    state
                        .overlay_stops()
                        .iter()
                        .any(|id| state.overlay_focus[id].is_focused(window))
                );
            });
        }
        cx.simulate_keystrokes("shift-tab");
        cx.update(|window, cx| {
            let focus = view.read(cx).overlay_focus["settings-tab-true"].clone();
            window.focus(&focus, cx);
        });
        cx.run_until_parked();
        cx.simulate_keystrokes("enter");
        cx.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse("enter").unwrap(),
        });
        cx.run_until_parked();
        cx.read_entity(&view, |view, _| assert!(view.settings_about));
        for _ in 0..5 {
            cx.simulate_keystrokes("tab");
            cx.run_until_parked();
        }
        cx.update(|window, cx| {
            let state = view.read(cx);
            assert!(
                state
                    .overlay_stops()
                    .iter()
                    .any(|id| state.overlay_focus[id].is_focused(window))
            );
        });
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert!(!view.read(cx).settings_open);
            assert!(view.read(cx).overlay_focus["app-menu"].is_focused(window));
        });
    }

    #[gpui::test]
    fn menu_outside_click_dismisses_without_reopening(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| AppView {
            root_focus: Some(cx.focus_handle()),
            ..AppView::default()
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            let focus = view.read(cx).overlay_focus["app-menu"].clone();
            window.focus(&focus, cx);
        });
        cx.run_until_parked();
        cx.simulate_keystrokes("enter");
        cx.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse("enter").unwrap(),
        });
        cx.run_until_parked();
        cx.read_entity(&view, |view, _| assert!(view.menu_open));
        cx.simulate_mouse_down(
            point(px(10.), px(300.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.simulate_mouse_up(
            point(px(10.), px(300.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        cx.run_until_parked();
        cx.read_entity(&view, |view, _| assert!(!view.menu_open));
        cx.simulate_keystrokes("enter");
        cx.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse("enter").unwrap(),
        });
        cx.run_until_parked();
        cx.read_entity(&view, |view, _| assert!(view.menu_open));
        let opener = cx.debug_bounds("app-menu").unwrap().center();
        cx.simulate_click(opener, Modifiers::default());
        cx.run_until_parked();
        cx.read_entity(&view, |view, _| assert!(!view.menu_open));
    }

    #[derive(Default)]
    struct SliderHarness {
        value: f32,
        enabled: bool,
        root: Option<FocusHandle>,
        bubbled: usize,
    }
    impl Render for SliderHarness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let target = cx.weak_entity();
            div()
                .tab_group()
                .when_some(self.root.as_ref(), |el, focus| el.track_focus(focus))
                .on_key_down(cx.listener(|this, _, _, _| this.bubbled += 1))
                .w(px(300.))
                .h(px(100.))
                .child(rail(
                    "volume",
                    SliderValue {
                        fraction: self.value,
                        maximum: 100.,
                        step: 5.,
                    },
                    self.enabled,
                    Palette::from_name(Default::default(), false),
                    None,
                    Default::default(),
                    move |value, _, cx| {
                        let _ = target.update(cx, |this, cx| {
                            this.value = value;
                            cx.notify();
                        });
                    },
                ))
        }
    }

    #[gpui::test]
    fn slider_keyboard_steps_clamps_and_stops_arrow_propagation(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| SliderHarness {
            value: 0.5,
            enabled: true,
            root: Some(cx.focus_handle()),
            ..Default::default()
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            let focus = view.read(cx).root.as_ref().unwrap().clone();
            window.focus(&focus, cx);
            window.focus_next(cx);
        });
        cx.simulate_keystrokes("right");
        cx.read_entity(&view, |view, _| {
            assert!((view.value - 0.55).abs() < 0.001);
            assert_eq!(view.bubbled, 0);
        });
        cx.simulate_keystrokes("end right");
        cx.read_entity(&view, |view, _| assert_eq!(view.value, 1.));
        cx.simulate_keystrokes("home left");
        cx.read_entity(&view, |view, _| assert_eq!(view.value, 0.));
        cx.update(|_, cx| {
            view.update(cx, |view, cx| {
                view.enabled = false;
                cx.notify();
            })
        });
        cx.run_until_parked();
        cx.simulate_keystrokes("right");
        cx.read_entity(&view, |view, _| assert_eq!(view.value, 0.));
    }
    #[gpui::test]
    fn compact_window_contains_search_recovery_banner_and_open_queue(cx: &mut TestAppContext) {
        let (view, cx) = cx.add_window_view(|_, cx| AppView {
            search: Some(cx.new(crate::search::SearchInput::new)),
            root_focus: Some(cx.focus_handle()),
            queue_sidebar_open: true,
            sidebar_width: 184.,
            library_unavailable: true,
            library_error: Some("A long technical error kept in details".into()),
            ..Default::default()
        });
        cx.simulate_resize(gpui::size(px(960.), px(640.)));
        for light in [false, true] {
            cx.update(|_, cx| {
                view.update(cx, |view, cx| {
                    view.light = light;
                    cx.notify();
                })
            });
            cx.run_until_parked();
            let header = cx.debug_bounds("app-header").unwrap();
            let search = cx.debug_bounds("search-field").unwrap();
            let banner = cx.debug_bounds("status-banner").unwrap();
            let queue = cx.debug_bounds("queue-sidebar").unwrap();
            assert!(search.left() >= header.left() && search.right() <= header.right());
            assert!(banner.left() >= header.left() && banner.right() <= queue.left());
            assert!(banner.bottom() < cx.debug_bounds("player-bar").unwrap().top());
            assert!(queue.right() <= px(960.));
        }
    }
}
