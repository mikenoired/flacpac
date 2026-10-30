# Flacpac UI audit

Date: October 27, 2026. Basis: the current code and saved screenshots from the last run. No new app run or performance measurement was performed for this audit.

## Overall assessment

The app has a working foundation: virtualized lists, a persistent SQLite cache, background scanning, artwork, a catalog, and basic playback. Visually and interactively, it is still a prototype. The main reason is the lack of a shared system for components, states, and layout rules. Local button improvements alone will not address this.

## Comparison checklist

| Area | Current state | What to improve or add | Priority |
|---|---|---|---|
| Layout | Sidebar, header, list, bottom player | Reduce header height and empty space; separate utility controls from content; define minimum window width and narrow-window behavior | P0 |
| Navigation | Four section views | Persistent active-section highlight; back navigation from album/artist; navigation history and saved list position | P0 |
| Track list | Virtualization, artwork, metadata, actions | Use one grid for header and rows (rows currently have a gap missing from the header); compact density; track number; current and selected track states | P0 |
| Icons | Symbols ▶, Ⅱ, ⏭, ＋, ⌕ | One vector icon set with consistent size and stroke; hover labels; “Remove” currently overflows a 32 px button | P0 |
| Control states | Hover and partial disabled colors | Consistent normal/hover/pressed/focus/disabled/active states; disabled buttons currently get bright hover styling and retain click handlers | P0 |
| Mini player | Artwork, title, artist, total duration, play/pause, next | Real elapsed time and seek progress; previous; volume; queue controls; center transport geometrically (it currently shifts right) | P0 |
| Playback | File start, manual pause and next from queue | Advance when a track ends; sync UI with actual Sink state; define Play behavior for an empty queue; previous-track history | P0 |
| Search | Filter tracks, albums, artists | Full text field: cursor, selection, paste, Cmd/Ctrl+A, IME; Cmd/Ctrl+F; “No results” message; clear search area | P0 |
| Albums | List with artwork and navigation to tracks | Artwork grid as an optional view; detail page with large cover, artist, year, duration, Play and Add to queue; disc/track ordering | P1 |
| Artists | Split on `;` / `feat.` and list tracks | Artist page with albums and tracks; accurate heading instead of “Album tracks”; navigate from artist names in lists and player | P1 |
| Metadata | Title, artist, album, duration | Read album artist, track/disc number, year, and multi-value tags; current first-artist heuristic can misgroup or split albums | P0 |
| Queue | Add, remove, manual next | Current/next track, clear, reorder, persist; remove a specific occurrence rather than the first matching path when duplicates exist | P1 |
| Artwork | Embedded images and folder files, placeholders | Consistent proportions and loading states; resized image cache; avoid synchronous `is_file` during UI construction; diagnose missing artwork | P1 |
| Screen states | Status line, separate placeholders | Separate empty library, no results, disconnected drive, error, and initial scan states; unobtrusive continuous scan status | P0 |
| Visual system | Dark green palette, repeated spacing | Shared color, typography, radius, and density tokens and Button, IconButton, Input, NavItem, TrackRow components; remove competing grays | P1 |
| Themes and accessibility | Dark theme only; partial search focus | Light theme from `PLAN.md`; keyboard navigation, focus ring, accessible action names, contrast check | P1 |
| Text | English labels and technical errors | One chosen locale; correct singular/plural; short actionable messages; remove duplicate catalog counts at top and bottom | P1 |

P0 means fix before the next visual pass. P1 means the next product-completion stage.

## Technical gaps that affect UX

- The cache is saved in a SQLite transaction, but every launch and watcher event rereads tags for the entire library. Check changes by path, size, and modification time, and reindex only changed files.
- WalkDir errors are ignored. An inaccessible library can look like a successful empty result and replace the cache. Preserve the cache when a drive is unavailable and distinguish no files from access errors.
- Scanning after a folder change runs detached without checking whether its result is still current. An older scan can overwrite the new root's UI data. Add a generation identifier or cancel obsolete work.
- Sorting and catalog construction run during the UI update after background scanning. This may pause large libraries; prepare the snapshot in the background.
- Search lowercases strings again on every keystroke. Precompute search fields and measure on large libraries before making search more complex.
- Only the library path is saved. Sort order, queue, current page, and scroll positions are not saved.
- “Recently changed” sorts by file mtime. This is not the date a track was added to the library, as required by the original plan. Store the first-seen date separately.

## Redesign sequence

1. **Interaction foundations.** Playback states, safe cache updates, full search, correct metadata.
2. **Shared shell.** Sidebar with active item, compact top bar, one table grid, stable mini-player area.
3. **Components.** Vector icons, buttons, input, and consistent interaction states. Use them across all pages.
4. **Catalog screens.** Dedicated album and artist pages with artwork, metadata, and collection actions.
5. **Quality checks.** Exercise the scenarios below with a real library and several window sizes.

## Acceptance criteria for the next version

- [ ] In a narrow window, the title, search, and actions do not overlap; the list remains useful.
- [ ] The active section, selected track, and actually playing track are visible.
- [ ] Header and row columns align at different window widths.
- [ ] All icons come from one set and have hover labels.
- [ ] Disabled buttons do not highlight as available or perform actions.
- [ ] Search supports paste, selection, and editing, including Cyrillic; empty results are explained.
- [ ] The player shows actual time, supports seeking, and advances automatically.
- [ ] An album with featured artists appears as one correct entity; tracks are in album order.
- [ ] Disconnecting an external drive and restarting preserve the visible cached library.
- [ ] Changing folders during a scan does not restore data from the previous library.
- [ ] Background indexing does not reset selection, queue, or scroll position.
- [ ] Empty libraries, long names, missing artwork, corrupt files, and queues with duplicates are checked.
- [ ] Startup from cache, search input, and scrolling are measured with libraries of 1,000 / 10,000 / 100,000 tracks; target limits are set before acceptance.

## Redesign results

The audit above is retained as the baseline. The following items were implemented afterward:

- [x] Compact header, active navigation, navigation history, and separate player bar.
- [x] Shared SVG icons, tooltips, hover/pressed/focus/disabled states, and accessible action names.
- [x] User-provided logo from `assets/flacpac-face.svg`, embedded in the app.
- [x] Shared geometry for table headers and rows, plus playing and selected track states.
- [x] Artwork grid and album list with view switching.
- [x] Album and artist pages with artwork, metadata, collection playback, and queue actions.
- [x] Album artist, disc/track number, year, first-seen date, and album track ordering.
- [x] Artwork, actual playback position, seeking, volume, previous/next track, and automatic continuation.
- [x] Native text input for search: selection, copy/cut/paste, IME, Unicode, cursor movement, clearing, and Cmd/Ctrl+F.
- [x] Queue with removal of a specific occurrence, reordering, clearing, and persistence.
- [x] Light/dark themes and persistence for view, sort order, volume, queue, and list positions.
- [x] Preserve cache when the library is inaccessible; protect the screen from stale scan results.
- [x] Reuse metadata for unchanged files and avoid rewriting SQLite when data has not changed.
- [x] Prepare the catalog, search strings, and sort orders in the background; avoid synchronous artwork file checks during rendering.
- [x] Cache resized artwork up to 384 px; separate empty-catalog, no-results, and inaccessible-folder states.

Checks reported at the time:

- `cargo clippy --all-targets -- -D warnings`.
- `cargo test -- --nocapture`: album grouping and ordering, individual artist credits, Unicode and native search input, SQLite migration and failed-transaction rollback, and inaccessible library root.
- GPUI layout tests at 960×640 and 1440×900 in both themes: search field, header, sidebar, and player remain within the window.
- Native window reviewed with a real library of 1,294 tracks; new logo and artwork grid checked. This does not replace automated checks of every user scenario.
- Synthetic debug-build measurement with 100,000 records: catalog preparation around 1 second in the background and search over prepared strings around 10 ms. These figures exclude audio file reads, UI frames, and disk startup.

Still to verify separately:

- Actual startup time from cache, input latency, and scrolling FPS on large real libraries.
- Playback/seeking of every supported format and behavior with different audio devices.
- Full screen reader pass and visual acceptance of both themes on different displays.
- Native app packaging and macOS Dock icon; currently the development executable is launched and the logo is embedded in the interface.
- Albums without an album artist still need a leading-artist heuristic; missing tags cannot be reliably reconstructed automatically.

## October 27, 2026 — transport, table, and artwork follow-up

Work was split among three subagents (transport/panic, table, statistics) and independently reviewed and integrated by the primary agent.

- Removed synchronous macOS folder picker use inside GPUI's borrowed event listener. The asynchronous sheet now opens without nesting the App borrow. The native picker opened and cancelled successfully; the original panic had no supplied backtrace beyond `async_context`, so its exact trigger remains unproven.
- Shuffle and repeat Off / All / One persist. Automatic completion honors One; manual Next advances. An explicit queue takes priority; shuffled cycles avoid immediate boundary repeats.
- All Tracks: visible ordinal numbering, title/album/added/duration columns, sortable highlighted headers with arrows, draggable boundaries, saved widths and direction. Removed format and per-row play/add controls. Added Play and Shuffle toolbar actions.
- Added date formatting follows the supplied Spotifast commit: relative seconds/minutes/hours/days/weeks before 30 days, then `Jan 5, 2024` style absolute dates. Zero date displays a dash.
- Centered search in the header, application menu with listening statistics and a reserved settings entry. Local statistics persist via a worker and count actual listening time. History starts with this version; earlier listening cannot be reconstructed.
- Mini-player artwork opens a viewport-sized overlay; title opens the album; individual artist credits open artist pages. The album hero cover is enlarged to 168 logical pixels and opens the same overlay. Escape dismisses overlays.
- Independent review moved album/duration sort preparation into the background snapshot. Native dark UI inspected; layout checks cover 960×640 and 1440×900, both themes, centered search, and contained modal artwork.
- Native inspection also found AppleDouble `._*.flac` sidecars counted as unreadable audio; the scanner now excludes them.

Final checks reported: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, all 20 tests, and native build pass. Native asynchronous folder picker opening and explicit Cancel were verified without panic. Exactly 1,294 AppleDouble audio sidecars were found on the current T7 library, explaining the former misleading warning. Dependency `block 0.1.6` still emits Cargo's existing future-compatibility notice.
