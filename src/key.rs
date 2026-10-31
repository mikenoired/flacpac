use crate::AppView;
use crate::ui::Page;
use gpui::{prelude::*, *};

impl AppView {
    pub fn key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;
        if key.key == "escape"
            && (self.artwork_modal.is_some()
                || self.menu_open
                || self.statistics_open
                || self.settings_open
                || self.context_track.is_some())
        {
            self.artwork_modal = None;
            self.menu_open = false;
            self.statistics_open = false;
            self.settings_open = false;
            self.context_track = None;
            cx.notify();
            cx.stop_propagation();
            return;
        }
        if self.overlay().is_some() {
            if key.key == "tab" {
                let stops = self.overlay_stops();
                if !stops.is_empty() {
                    let current = stops
                        .iter()
                        .position(|id| self.overlay_focus[id].is_focused(window));
                    let index = match current {
                        Some(i) if key.modifiers.shift => (i + stops.len() - 1) % stops.len(),
                        Some(i) => (i + 1) % stops.len(),
                        None if key.modifiers.shift => stops.len() - 1,
                        None => 0,
                    };
                    window.focus(&self.overlay_focus[&stops[index]], cx);
                }
                cx.stop_propagation();
            }
            return;
        }
        if self.context_track.take().is_some() {
            cx.notify();
        }
        if self.artwork_modal.is_some() || self.statistics_open || self.settings_open {
            return;
        }
        if (key.modifiers.platform || key.modifiers.control) && key.key == "f" {
            if let Some(search) = &self.search {
                let focus = search.read(cx).focus.clone();
                window.focus(&focus, cx);
            }
            cx.stop_propagation();
            return;
        }
        if key.key == "tab" {
            if key.modifiers.shift {
                window.focus_prev(cx);
            } else {
                window.focus_next(cx);
            }
            cx.stop_propagation();
            return;
        }
        if matches!(key.key.as_str(), "space" | "enter")
            && self
                .root_focus
                .as_ref()
                .is_some_and(|focus| !focus.is_focused(window))
        {
            return;
        }
        if self
            .search
            .as_ref()
            .is_some_and(|search| search.read(cx).focus.is_focused(window))
        {
            return;
        }
        match key.key.as_str() {
            "space" => self.toggle_playback(cx),
            "enter" => {
                if let Some(path) = &self.selected_track
                    && let Some(track) = self
                        .tracks
                        .iter()
                        .find(|track| &track.path == path)
                        .cloned()
                {
                    self.play_track(track, cx);
                }
            }
            "up" | "down" if !matches!(self.page, Page::Albums | Page::Artists | Page::Queue) => {
                let indices = self.detail_indices();
                if indices.is_empty() {
                    return;
                }
                let current = indices.iter().position(|index| {
                    self.selected_track.as_ref() == Some(&self.tracks[*index].path)
                });
                let index = match (current, key.key.as_str()) {
                    (Some(index), "up") => index.saturating_sub(1),
                    (Some(index), _) => (index + 1).min(indices.len() - 1),
                    _ => 0,
                };
                self.selected_track = Some(self.tracks[indices[index]].path.clone());
                if let Some(handle) = self.scrolls.get(&self.page_key()) {
                    handle.scroll_to_item(index, ScrollStrategy::Nearest);
                }
                cx.notify();
            }
            _ => return,
        }
        cx.stop_propagation();
    }
}
