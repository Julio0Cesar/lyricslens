# LyricsLens

Synced lyrics overlay for Linux. Reads the playing track over MPRIS, looks the
lyrics up on LRCLIB and draws the current line on a Wayland layer-shell surface.

Early work in progress.

## Build

```
cargo build
```

## Run

```
cargo run
```

`LYRICSLENS_PLAYER` picks a player by any part of its bus name, for when more
than one is running:

```
LYRICSLENS_PLAYER=spotify cargo run
```

## What it needs from a player

The overlay follows the player's `Position`, which some players do not report.
Firefox is one: it answers with zero forever. There is nothing to synchronise
against in that case, and the overlay says so on screen instead of guessing.

## Desktops

On any compositor implementing `wlr-layer-shell` — Hyprland, Sway, KWin,
COSMIC, niri, Wayfire — the overlay stays visible over a fullscreen window.

Elsewhere, including GNOME and plain X11 sessions, it falls back to an X11
window that asks to be kept above the others. That fallback does not survive a
window in true fullscreen, and where it ends up on screen is the window
manager's decision.

## License

MIT
