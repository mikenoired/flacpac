use super::*;
use crate::library::Track;
use crate::playback::RepeatMode;
use gpui::prelude::*;
use rand::seq::SliceRandom;
use std::time::Duration;

impl AppView {
    pub(crate) fn upcoming_tracks(&self) -> Vec<Track> {
        self.queue
            .iter()
            .chain(self.continuation.iter())
            .cloned()
            .collect()
    }
    pub(crate) fn remove_upcoming(&mut self, index: usize) -> bool {
        if index < self.queue.len() {
            self.queue.remove(index).is_some()
        } else {
            self.continuation.remove(index - self.queue.len()).is_some()
        }
    }
    pub(crate) fn remove_queued_track(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.remove_upcoming(index) {
            self.persist();
            cx.notify();
        }
    }
    pub(crate) fn reorder_upcoming(&mut self, from: usize, to: usize) -> bool {
        let mut upcoming = self.upcoming_tracks();
        if from >= upcoming.len() || to >= upcoming.len() || from == to {
            return false;
        }
        let explicit_count = self.queue.len();
        let track = upcoming.remove(from);
        upcoming.insert(to, track);
        self.continuation = upcoming.split_off(explicit_count).into();
        self.queue = upcoming.into();
        true
    }
    pub(crate) fn reorder_queue(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        if self.reorder_upcoming(from, to) {
            self.persist();
            cx.notify();
        }
    }
    pub(crate) fn move_queued_track(&mut self, index: usize, delta: isize, cx: &mut Context<Self>) {
        if let Some(target) = index.checked_add_signed(delta) {
            self.reorder_queue(index, target, cx);
        }
    }
    pub(crate) fn play_queued_track(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.queue.len() + self.continuation.len() {
            return;
        }
        let track = if index < self.queue.len() {
            self.queue.drain(..=index).next_back()
        } else {
            let context_index = index - self.queue.len();
            self.queue.clear();
            self.continuation.drain(..=context_index).next_back()
        };
        if let Some(track) = track {
            self.start_track(track, cx);
            self.persist();
        }
    }
    pub(crate) fn start_track(&mut self, track: Track, cx: &mut Context<Self>) -> bool {
        let started = match self.playback.play(&track.path) {
            Ok(()) => {
                if let Some(previous) = self.current_track.take()
                    && previous.path != track.path
                {
                    self.history.push(previous);
                }
                self.statistics.start_session(&track);
                self.statistics_tick = Some(std::time::Instant::now());
                self.current_track = Some(track);
                self.paused = false;
                self.position_ms = 0;
                self.playback.set_volume(self.volume);
                self.status = None;
                true
            }
            Err(error) => {
                self.status = Some(format!("Cannot play this file: {error}"));
                false
            }
        };
        cx.notify();
        started
    }
    pub fn play_track(&mut self, track: Track, cx: &mut Context<Self>) {
        let context: Vec<Track> = if self.page == Page::Queue {
            self.queue.iter().cloned().collect()
        } else {
            self.detail_indices()
                .iter()
                .filter_map(|index| self.tracks.get(*index).cloned())
                .collect()
        };
        if self.page == Page::Queue {
            if let Some(index) = context
                .iter()
                .position(|candidate| candidate.path == track.path)
            {
                self.queue.drain(..=index);
            }
            self.playback_order = context;
            self.continuation.clear();
        } else {
            self.playback_order = context;
            self.reset_continuation(&track.path);
        }
        self.start_track(track, cx);
        self.persist();
    }
    pub fn toggle_playback(&mut self, cx: &mut Context<Self>) {
        if let Some(track) = self.current_track.clone() {
            if self.playback.is_finished() {
                self.start_track(track, cx);
            } else {
                self.playback.toggle();
                self.paused = self.playback.is_paused();
            }
        } else if !self.queue.is_empty() {
            self.play_next(cx);
        } else if let Some(track) = self
            .filtered_track_indices
            .first()
            .and_then(|index| self.tracks.get(*index))
            .cloned()
        {
            self.play_track(track, cx);
        }
        cx.notify();
    }
    pub(crate) fn reset_continuation(&mut self, current: &std::path::Path) {
        let index = self
            .playback_order
            .iter()
            .position(|track| track.path == current);
        let mut remaining: Vec<Track> = if self.shuffle {
            self.playback_order
                .iter()
                .filter(|track| track.path != current)
                .cloned()
                .collect()
        } else {
            self.playback_order
                .iter()
                .skip(index.map_or(0, |index| index + 1))
                .cloned()
                .collect()
        };
        if self.shuffle {
            remaining.shuffle(&mut rand::rng());
        }
        self.continuation = remaining.into();
    }
    pub fn toggle_shuffle(&mut self, cx: &mut Context<Self>) {
        self.shuffle = !self.shuffle;
        if let Some(track) = &self.current_track {
            let path = track.path.clone();
            self.reset_continuation(&path);
        }
        self.persist();
        cx.notify();
    }
    pub fn cycle_repeat(&mut self, cx: &mut Context<Self>) {
        self.repeat = self.repeat.next();
        self.persist();
        cx.notify();
    }
    pub(crate) fn play_collection(&mut self, shuffle: bool, cx: &mut Context<Self>) {
        self.shuffle = shuffle;
        let indices = self.detail_indices();
        let index = if shuffle {
            use rand::Rng;
            (!indices.is_empty()).then(|| rand::rng().random_range(0..indices.len()))
        } else {
            (!indices.is_empty()).then_some(0)
        };
        if let Some(track) = index
            .and_then(|index| self.tracks.get(indices[index]))
            .cloned()
        {
            self.play_track(track, cx);
        }
    }
    pub(crate) fn finish_track(&mut self, cx: &mut Context<Self>) {
        if let Some(track) = self.next_track(true) {
            self.start_track(track, cx);
            self.persist();
        } else {
            self.paused = true;
        }
        cx.notify();
    }
    pub(crate) fn next_track(&mut self, automatic: bool) -> Option<Track> {
        if automatic && self.repeat == RepeatMode::One {
            return self.current_track.clone();
        }
        let mut next = self
            .queue
            .pop_front()
            .or_else(|| self.continuation.pop_front());
        if next.is_none() && self.repeat == RepeatMode::All && !self.playback_order.is_empty() {
            let mut cycle = self.playback_order.clone();
            if self.shuffle {
                cycle.shuffle(&mut rand::rng());
                // Avoid playing the boundary track twice when another track is available.
                if cycle.len() > 1
                    && self
                        .current_track
                        .as_ref()
                        .is_some_and(|track| track.path == cycle[0].path)
                {
                    cycle.swap(0, 1);
                }
            }
            self.continuation = cycle.into();
            next = self.continuation.pop_front();
        }
        next
    }
    pub fn play_next(&mut self, cx: &mut Context<Self>) {
        let next = self.next_track(false);
        if let Some(track) = next {
            self.start_track(track, cx);
            self.persist();
        } else {
            self.paused = true;
        }
        cx.notify();
    }
    pub fn play_previous(&mut self, cx: &mut Context<Self>) {
        if self.position_ms > 3000 {
            if self.playback.is_finished() {
                if let Some(track) = self.current_track.clone() {
                    self.start_track(track, cx);
                }
            } else {
                self.playback.seek(Duration::ZERO);
                self.position_ms = 0;
            }
        } else if let Some(track) = self.history.last().cloned() {
            let history_len = self.history.len();
            let current = self.current_track.clone();
            if self.start_track(track, cx) {
                self.history.truncate(history_len - 1);
                if let Some(current) = current {
                    self.continuation.push_front(current);
                }
            }
        }
        cx.notify();
    }
}
