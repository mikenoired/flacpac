use super::*;
use gpui::{prelude::*, *};
use std::sync::Arc;

impl AppView {
    pub fn prepare_tracks(&mut self, snapshot: LibrarySnapshot) {
        self.tracks = snapshot.tracks;
        self.albums = snapshot.albums;
        self.artists = snapshot.artists;
        self.search_terms = snapshot.search_terms;
        self.date_order = snapshot.date_order;
        self.title_order = snapshot.title_order;
        self.artist_order = snapshot.artist_order;
        self.album_order = snapshot.album_order;
        self.duration_order = snapshot.duration_order;
        if !self.pending_queue.is_empty() {
            self.queue = self
                .pending_queue
                .iter()
                .filter_map(|path| {
                    self.tracks
                        .iter()
                        .find(|track| &track.path == path)
                        .cloned()
                })
                .collect();
            self.pending_queue.clear();
        }
        self.rebuild_order();
    }
    pub fn set_sort(&mut self, sort: SortMode, cx: &mut Context<Self>) {
        self.sort_reversed = if self.sort == sort {
            !self.sort_reversed
        } else {
            false
        };
        self.sort = sort;
        self.rebuild_order();
        self.persist();
        cx.notify();
    }

    pub fn rebuild_order(&mut self) {
        self.sorted_indices = match self.sort {
            SortMode::Date => self.date_order.clone(),
            SortMode::Title => self.title_order.clone(),
            SortMode::Artist => self.artist_order.clone(),
            SortMode::Album => self.album_order.clone(),
            SortMode::Duration => self.duration_order.clone(),
        };
        if self.sort_reversed {
            self.sorted_indices = Arc::new(self.sorted_indices.iter().rev().copied().collect());
        }
        self.rebuild_search();
    }
    pub(crate) fn set_search_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.context_track = None;
        self.search_query = query;
        self.pending_offsets.clear();
        for handle in self.scrolls.values() {
            handle.scroll_to_item_strict(0, ScrollStrategy::Top);
        }
        self.rebuild_search();
        cx.notify();
    }

    pub(crate) fn rebuild_search(&mut self) {
        let query = self.search_query.trim().to_lowercase();
        self.filtered_track_indices = if query.is_empty() {
            self.sorted_indices.clone()
        } else {
            Arc::new(
                self.sorted_indices
                    .iter()
                    .copied()
                    .filter(|index| {
                        query.is_empty()
                            || self
                                .search_terms
                                .get(*index)
                                .is_some_and(|text| text.contains(&query))
                    })
                    .collect(),
            )
        };
        self.filtered_album_indices = Arc::new(
            self.albums
                .iter()
                .enumerate()
                .filter(|(_, album)| {
                    query.is_empty()
                        || album.title.to_lowercase().contains(&query)
                        || album.artist.to_lowercase().contains(&query)
                })
                .map(|(index, _)| index)
                .collect(),
        );
        self.filtered_artist_indices = Arc::new(
            self.artists
                .iter()
                .enumerate()
                .filter(|(_, artist)| {
                    query.is_empty() || artist.name.to_lowercase().contains(&query)
                })
                .map(|(index, _)| index)
                .collect(),
        );
    }

    pub(crate) fn remember_page(&mut self) {
        self.context_track = None;
        self.navigation.push(NavigationEntry {
            page: self.page,
            album: self.selected_album.clone(),
            artist: self.selected_artist.clone(),
        });
    }
    pub fn set_page(&mut self, page: Page, cx: &mut Context<Self>) {
        if self.page != page {
            self.remember_page();
            self.page = page;
            self.persist();
        }
        cx.notify();
    }
    pub(crate) fn back(&mut self, cx: &mut Context<Self>) {
        if let Some(NavigationEntry {
            page,
            album,
            artist,
        }) = self.navigation.pop()
        {
            self.page = page;
            self.selected_album = album;
            self.selected_artist = artist;
            self.persist();
            cx.notify();
        }
    }
    pub fn open_album(&mut self, title: String, artist: String, cx: &mut Context<Self>) {
        self.remember_page();
        self.selected_album = Some((title, artist));
        self.page = Page::AlbumDetail;
        self.persist();
        cx.notify();
    }
    pub fn open_artist(&mut self, artist: String, cx: &mut Context<Self>) {
        self.remember_page();
        self.selected_artist = Some(artist);
        self.page = Page::ArtistDetail;
        self.persist();
        cx.notify();
    }
    pub fn detail_indices(&self) -> Arc<Vec<usize>> {
        let indices = match self.page {
            Page::AlbumDetail => self
                .selected_album
                .as_ref()
                .and_then(|(title, artist)| {
                    self.albums
                        .iter()
                        .find(|a| &a.title == title && &a.artist == artist)
                })
                .map(|a| a.track_indices.clone())
                .unwrap_or_default(),
            Page::ArtistDetail => self
                .selected_artist
                .as_ref()
                .and_then(|name| self.artists.iter().find(|a| &a.name == name))
                .map(|a| a.track_indices.clone())
                .unwrap_or_default(),
            _ => self.filtered_track_indices.clone(),
        };
        let query = self.search_query.trim().to_lowercase();
        if matches!(self.page, Page::AlbumDetail | Page::ArtistDetail) && !query.is_empty() {
            Arc::new(
                indices
                    .iter()
                    .copied()
                    .filter(|index| {
                        self.search_terms
                            .get(*index)
                            .is_some_and(|text| text.contains(&query))
                    })
                    .collect(),
            )
        } else {
            indices
        }
    }
}
