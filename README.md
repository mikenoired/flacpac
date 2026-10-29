<p align="center">
  <img src="assets/flacpac-face.svg" alt="Flacpac logo" width="88" height="88">
</p>

<h1 align="center">Flacpac</h1>

<p align="center"><strong>Your music. Your files.</strong><br>A native desktop music player built with Rust and GPUI.</p>

<p align="center">
  <a href="#build-and-run"><strong>Build & run</strong></a> ·
  <a href="#start-listening">Getting started</a> ·
  <a href="#controls">Controls</a> ·
  <a href="https://github.com/mikenoired/flacpac/issues">Report a bug</a>
</p>

<p align="center">
  <a href="Cargo.toml"><img src="https://img.shields.io/badge/Built_with-Rust-b2d77c" alt="Built with Rust"></a>
  <a href="https://github.com/gpui-ce/gpui-ce"><img src="https://img.shields.io/badge/UI-GPUI_CE-b2d77c" alt="GPUI CE"></a>
  <img src="https://img.shields.io/badge/Status-Early_development-24292f" alt="Early development">
</p>

Flacpac plays music from folders on your computer or an external drive. Browse
tracks, albums, and artists, build a queue, and keep your listening history
locally. No account or streaming subscription is required.

The interface uses virtualized lists, background indexing, and a SQLite metadata
cache. Audio playback is handled by Rodio; tags and artwork are read with Lofty.

## Your collection, on your desktop

| Feature | What you can do |
|---|---|
| **Library and search** | Choose a music folder, scan its subfolders, and search track titles, artists, and albums. |
| **Albums and artists** | Browse dedicated pages, album grids or lists, disc and track ordering, and individual artist credits. |
| **Audio formats** | Play FLAC, MP3, WAV, Ogg Vorbis, AIFF, and AAC or ALAC in M4A/MP4 containers. |
| **Queue and playback** | Add tracks or collections, reorder upcoming tracks, shuffle, repeat one or all, and seek within a track. |
| **Artwork** | Display embedded covers or common folder artwork; open a larger view. |
| **Appearance** | Choose dark or light mode and one of eight color palettes. |
| **Listening statistics** | See your top tracks, artists, and albums, with listening time and play counts stored locally. |
| **Library updates** | Watch the selected folder and refresh metadata in the background when files change. |
| **Desktop controls** | Use system media controls where supported and reveal a track in your file manager. |

## Build and run

Flacpac is in early development. Build from source; packaged downloads are not
provided yet. **macOS is the currently tested platform.** Linux and Windows builds
have not been verified. Windows system media controls are not connected yet.

Install [Rust](https://www.rust-lang.org/tools/install) **1.95 or newer**, Git, and
your platform's native compiler toolchain. On macOS, install the Xcode Command
Line Tools with `xcode-select --install`.

```sh
git clone https://github.com/mikenoired/flacpac.git
cd flacpac
cargo run --release --locked
```

The executable is named `flacpac-rusty` and is built in `target/release/`.

## Start listening

1. Launch Flacpac and choose **Music folder** in the sidebar or the folder button
   in the header.
2. Select the folder containing your music. Subfolders are included. External
   drives work too; keep the drive connected while listening.
3. Double-click a track, or choose **Play** on the library, album, or artist page.
4. Open the queue from the bottom player bar. Use a track's context menu to add
   it to the queue, browse its album or artist, or reveal its file.

On later launches, cached tracks appear while the library refreshes in the
background. The selected folder, explicit queue, volume, and appearance settings
are remembered. Playback position is not restored.

## Controls

| Control | Action |
|---|---|
| **Space** | Play or pause when the main view has focus. |
| **Enter** | Play the selected track when the main view has focus. |
| **Up / Down** | Select a track in a track list. |
| **Cmd+F / Ctrl+F** | Focus library search. |
| **Escape** | Close an open menu or dialog; in search, clear the query and leave the field. |
| **Double-click a track** | Start playback. |
| **Right-click a track** | Open track actions. |

Open the application menu in the header for **Settings** and **Listening
statistics**. Use the seek and volume sliders in the bottom player bar to adjust
playback.

## Local data

Preferences, the SQLite library cache, and listening history are stored in your
platform's local application data directory under `flacpac-rusty`. Extracted
artwork is stored in the platform cache directory under `flacpac-rusty/artwork`.
Music files are read without modifying their tags or contents.

## Development

`rust-toolchain.toml` pins the compiler, Clippy, and rustfmt used locally and in
CI. Update the pinned version in a separate change and run the checks below.
Pull requests and pushes to `main` run checks on Apple Silicon and Intel macOS.

To release, update `package.version` in `Cargo.toml` and refresh `Cargo.lock`,
then push a matching `v<version>` tag. Publication requires both architectures
to pass checks and produce validated app bundles. Build artifacts are retained
for seven days; published release assets remain available on GitHub Releases.

```sh
cargo fmt --check
cargo test --locked
```

An optional test opens local audio files and reads their first decoded sample
without using an audio output device or changing the library:

```sh
FLACPAC_TEST_LIBRARY=/path/to/music cargo test --locked local_library_decodes -- --ignored --nocapture
```

This checks decoder initialization, not complete playback of every file. The
player has been checked against a local collection of 1,294 FLAC, MP3, and M4A
files, plus generated AIFF and ALAC samples.

Bug reports should include your operating system, audio format, reproduction
steps, and any error shown in the app. Do not attach music you do not have
permission to share.

## Acknowledgements

Built with [GPUI CE](https://github.com/gpui-ce/gpui-ce),
[Rodio](https://github.com/RustAudio/rodio),
[Lofty](https://github.com/Serial-ATA/lofty-rs),
[rusqlite](https://github.com/rusqlite/rusqlite), and
[Souvlaki](https://github.com/Sinono3/souvlaki).

README layout inspired by [Spotifast](https://github.com/crmne/spotifast).
