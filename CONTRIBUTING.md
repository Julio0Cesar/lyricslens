# Contributing

## Building

Arch, and anything close enough:

```
sudo pacman -S gtk4 gtk4-layer-shell libadwaita
cargo build
```

Debian and Ubuntu:

```
sudo apt install libgtk-4-dev libgtk4-layer-shell-dev libadwaita-1-dev
cargo build
```

## Running the checks

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
dbus-run-session -- cargo test
```

The tests publish a player on the session bus, so they need one of their own —
that is what `dbus-run-session` is for. Two more tests reach the network and
are skipped unless asked for:

```
cargo test -- --ignored
```

## Working on the overlay without a music player

```
cargo run --example fake_player -- Radiohead Creep 238
LYRICSLENS_PLAYER=lyricslensfake cargo run
```

## What a good change looks like

- One thing per pull request, with the checks above passing.
- A test for anything with logic in it. The LRC parser, the title cleanup and
  the clock are all pure functions on purpose, so this is usually easy.
- Comments that say why, not what. The code already says what.
- Commit titles in the imperative, lowercase, naming what changed:
  `parse lrc lines with multiple timestamps`.
- Everything in English: code, comments, commits, issues.

## Adding a language

No building needed to try one:

1. Copy `locale/es.toml` to `~/.config/lyricslens/locale/`, renamed to the
   language's code: `fr.toml`, `de.toml`, `it.toml`.
2. Replace the text on the right of each `=` with the translation. Keep the
   English on the left as it is: it is how the program finds the sentence.
3. Restart LyricsLens with `LANG` set to that language, for example
   `LANG=fr_FR.UTF-8 lyricslens --foreground`, and check each screen.

To ship it with the program, put the file in `locale/`, add it to `BUILT_IN`
in `src/i18n.rs`, and run the checks: a test fails if it has fewer sentences
than the others.

## Where things are

```
src/media.rs      what is playing, read over MPRIS
src/lyrics.rs     LRC parsing, title cleanup, the LRCLIB client
src/sync.rs       the clock that says where the song is
src/store.rs      settings and the lyrics cache, on disk
src/ui.rs         the overlay and the preferences window
src/i18n.rs       which language is shown; the words are in locale/
src/app.rs        the worker thread that ties them together
```
