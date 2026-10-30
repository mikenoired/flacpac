# Project context and roadmap

## Current project

Flacpac is a native desktop music player built with Rust, GPUI CE, Rodio, and SQLite. The application currently provides a virtualized track library, background folder scanning, cached audio metadata and artwork, album and artist catalogs, Unicode-aware search, a persistent playback queue, playback controls, listening statistics, and light and dark themes.

The crate entry point is `src/main.rs`. UI and application state live primarily in `src/ui.rs`; library scanning and catalog preparation are under `src/library/`. Playback, settings, cache, search, statistics, and media controls have separate modules. The design and known UX/technical gaps are documented in [DESIGN_AUDIT.md](DESIGN_AUDIT.md).

## Build and quality checks

The crate uses Rust edition 2024 and declares Rust 1.95. macOS is the currently tested platform. Build and check the locked dependency graph with:

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo build --release --locked
```

A local audio decoding test is opt-in and needs a user-provided music directory; it is not suitable for CI. Do not add real audio files to the repository.

## Near-term priorities

1. Close the remaining interaction, accessibility, and layout gaps listed in `DESIGN_AUDIT.md`.
2. Measure launch, search, and scrolling performance on large real-world libraries before setting performance targets.
3. Verify playback across supported formats and audio devices, and exercise the app on additional platforms before claiming support.
4. Establish automated CI checks and a macOS release workflow. Release artifacts should be clearly identified as unsigned unless code signing and notarization are configured.

## Maintenance notes

- Preserve the user's music files and metadata; the app reads them without editing tags.
- Keep unsafe Rust forbidden, as enforced by `Cargo.toml`.
- Prefer background work for library scans and catalog preparation so large collections do not block UI updates.
- The documented performance figures in `DESIGN_AUDIT.md` are synthetic observations, not product guarantees.
