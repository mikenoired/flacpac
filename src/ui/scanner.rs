use super::*;
use gpui::prelude::*;
use notify::{RecursiveMode, Watcher};
use std::{path::PathBuf, sync::mpsc, time::Duration};

impl AppView {
    pub fn choose_folder(&mut self, cx: &mut Context<Self>) {
        if self.folder_dialog_open {
            return;
        }
        self.folder_dialog_open = true;
        // Never run NSOpenPanel's synchronous nested event loop while GPUI's App
        // RefCell is borrowed by this listener. The async panel yields first.
        cx.spawn(async move |view, cx| {
            let selected = rfd::AsyncFileDialog::new()
                .set_title("Choose music folder")
                .pick_folder()
                .await;
            let _ = view.update(cx, |this, cx| {
                this.folder_dialog_open = false;
                let Some(selected) = selected else { return };
                let path = selected.path().to_path_buf();
                if this.root.as_ref() != Some(&path) {
                    this.prepare_tracks(LibrarySnapshot::new(Vec::new()));
                    this.queue.clear();
                    this.pending_queue.clear();
                }
                this.root = Some(path.clone());
                this.persist();
                this.start_scan(path, cx);
            });
        })
        .detach();
    }

    pub(crate) fn start_scan(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.generation += 1;
        let generation = self.generation;
        self.library_unavailable = false;
        self.library_error = None;
        self.status_details = false;
        self.status = Some("Loading saved library…".into());
        self.scanning = true;
        cx.notify();

        self.watch_library(path.clone(), cx);

        cx.spawn(async move |view, cx| {
            let cache_root = path.clone();
            let cached_task = cx
                .background_executor()
                .spawn(async move { crate::cache::load(&cache_root).map(LibrarySnapshot::new) });
            if let Ok(cached_tracks) = cached_task.await
                && !cached_tracks.tracks.is_empty()
            {
                let _ = view.update(cx, |view, cx| {
                    if view.generation != generation {
                        return;
                    }
                    view.prepare_tracks(cached_tracks);
                    view.status = Some("Refreshing library in background…".into());
                    cx.notify();
                });
            }

            let scan_root = path.clone();
            let task = cx
                .background_executor()
                .spawn(async move { scan_and_cache(scan_root) });
            let result = task.await;
            let _ = view.update(cx, |view, cx| {
                if view.generation != generation {
                    return;
                }
                view.scanning = false;
                match result {
                    Ok((library, cache_result)) => {
                        view.library_unavailable = false;
                        view.library_error = None;
                        let skipped=library.skipped_files;
                        view.prepare_tracks(library);
                        view.status = cache_result.err().map(|error| {
                            format!("Library loaded; cache could not be saved: {error}")
                        }).or_else(||(skipped>0).then(||format!("{skipped} files could not be read. Check their format or file contents.")));
                    }
                    Err(error) => {
                        view.library_unavailable = true;
                        view.library_error = Some(error.to_string());
                        view.status = None;
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn watch_library(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.reindex_task.take();
        self.watcher.take();
        let (sender, receiver) = mpsc::channel();
        let mut watcher =
            match notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                if event.is_ok() {
                    let _ = sender.send(());
                }
            }) {
                Ok(watcher) => watcher,
                Err(error) => {
                    self.status = Some(format!("Could not watch library folder: {error}"));
                    return;
                }
            };
        if let Err(error) = watcher.watch(&path, RecursiveMode::Recursive) {
            self.status = Some(format!("Could not watch library folder: {error}"));
            return;
        }
        self.watcher = Some(watcher);

        let generation = self.generation;
        self.reindex_task = Some(cx.spawn(async move |view, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(750))
                    .await;
                match view.update(cx, |view, _| view.scanning) {
                    Ok(true) => continue,
                    Err(_) => break,
                    _ => {}
                }
                if receiver.try_recv().is_err() {
                    continue;
                }
                while receiver.try_recv().is_ok() {}

                let _ = view.update(cx, |view, cx| {
                    view.scanning = true;
                    cx.notify();
                });
                let scan_path = path.clone();
                let task = cx
                    .background_executor()
                    .spawn(async move { scan_and_cache(scan_path) });
                let result = task.await;
                if view
                    .update(cx, |view, cx| {
                        if view.generation != generation {
                            return;
                        }
                        view.scanning = false;
                        match result {
                            Ok((library, cache_result)) => {
                                view.library_unavailable = false;
                                view.library_error = None;
                                let skipped=library.skipped_files;
                                view.prepare_tracks(library);
                                view.status = cache_result.err().map(|error| {
                                    format!("Library refreshed; cache could not be saved: {error}")
                                }).or_else(||(skipped>0).then(||format!("{skipped} files could not be read. Check their format or file contents.")));
                            }
                            Err(error) => {
                                view.library_unavailable = true;
                                view.library_error = Some(error.to_string());
                                view.status = None;
                            }
                        }
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        }));
    }
}
