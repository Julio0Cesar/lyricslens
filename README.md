# LyricsLens

Synced lyrics over whatever is on your screen.

LyricsLens reads what your music player is playing, finds the lyrics on
[LRCLIB](https://lrclib.net), and draws the line being sung on a layer above
every other window — including one in fullscreen.

Linux only. Rust, GTK4 and `wlr-layer-shell`, no browser engine and no
`node_modules`.

## Install

On Arch, from the `packaging/PKGBUILD` in this repository:

```
makepkg -si
```

From source anywhere else:

```
cargo install --path .
```

It needs GTK4, gtk4-layer-shell and libadwaita. On Debian and Ubuntu those are
`libgtk-4-dev`, `libgtk4-layer-shell-dev` and `libadwaita-1-dev`.

## Use

```
lyricslens
```

The overlay appears near the bottom of the screen and follows the song.

| Command | What it does |
| --- | --- |
| `lyricslens` | runs the overlay |
| `lyricslens --settings` | opens the preferences window |
| `lyricslens --toggle` | hides the overlay, or brings it back |
| `lyricslens --position` | enters the mode where you drag it somewhere else |

### A key to hide it

Wayland gives an ordinary program no way to claim a key combination, so the
binding belongs to your compositor. On Hyprland, in `hyprland.conf`:

```
bind = SUPER, L, exec, lyricslens --toggle
bind = SUPER SHIFT, L, exec, lyricslens --position
```

### Moving it

`--position` turns on a mode where the overlay takes your clicks instead of
letting them through: drag it where you want, then press the key again. Where
you left it is saved.

## What it needs from a player

LyricsLens follows the player's reported position, and some players do not
report one. Firefox is the notable case: it answers with zero forever, so there
is nothing to synchronise against, and the overlay says so on screen rather
than guessing. Players that do report it — Spotify, mpv with `mpv-mpris`,
Rhythmbox, Amberol — work.

With more than one player open, pick the one you mean in the preferences, or
for a single run:

```
LYRICSLENS_PLAYER=spotify lyricslens
```

## Desktops

On any compositor that implements `wlr-layer-shell` — Hyprland, Sway, KWin,
COSMIC, niri, Wayfire — the overlay stays visible over a fullscreen window.

Elsewhere, including GNOME and plain X11 sessions, it falls back to an X11
window that asks to be kept above the others. That fallback does not survive a
window in true fullscreen, and where it lands on screen is the window manager's
decision.

## Where it keeps things

Settings are TOML at `~/.config/lyricslens/settings.toml`, readable and
editable by hand. Fetched lyrics are cached under `~/.cache/lyricslens/`, one
LRC file per recording, so a song you have already played works with the
network off.

## Contributing

[CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT.
