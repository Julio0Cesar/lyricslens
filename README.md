<div align="center">

<img src="assets/logo.svg" alt="" width="96">

# LyricsLens

**Synced lyrics over any window.**

LyricsLens reads what your player is playing, finds the lyrics on
[LRCLIB](https://lrclib.net), and draws the line being sung on a layer above
everything else — including a window in fullscreen.

[![CI](https://github.com/Julio0Cesar/lyricslens/actions/workflows/ci.yml/badge.svg)](https://github.com/Julio0Cesar/lyricslens/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Julio0Cesar/lyricslens?label=release)](https://github.com/Julio0Cesar/lyricslens/releases/latest)
[![License](https://img.shields.io/github/license/Julio0Cesar/lyricslens)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-b7410e)](https://www.rust-lang.org)

</div>

<!-- A recording of the overlay following a song goes here. -->

---

## Install

Into `~/.local`, no sudo, nothing outside your home directory:

```bash
curl -fsSL https://raw.githubusercontent.com/Julio0Cesar/lyricslens/main/install.sh | sh
```

To read it first — and you should:

```bash
curl -fsSL https://raw.githubusercontent.com/Julio0Cesar/lyricslens/main/install.sh -o install.sh
less install.sh && sh install.sh
```

To remove it:

```bash
curl -fsSL https://raw.githubusercontent.com/Julio0Cesar/lyricslens/main/install.sh | sh -s -- --remove
```

<details>
<summary>On Arch, from the PKGBUILD</summary>

```bash
curl -fsSLO https://raw.githubusercontent.com/Julio0Cesar/lyricslens/main/packaging/PKGBUILD
makepkg -si
```

</details>

<details>
<summary>From source</summary>

```bash
cargo install --path .
```

</details>

It needs GTK4, libadwaita and gtk4-layer-shell, which most desktops already
have. On Arch: `gtk4 libadwaita gtk4-layer-shell`. On Debian and Ubuntu:
`libgtk-4-1 libadwaita-1-0 libgtk4-layer-shell0`.

## Use

```bash
lyricslens
```

The overlay appears near the bottom of the screen and follows the song. It runs
in the background and gives the terminal straight back. `lls` is the same
program under a shorter name.

Launching it again while a copy is running brings that one to the front instead
of starting another.

It puts an icon in the status bar, where a left click shows and hides it and
the menu holds the rest, quitting included. Desktops without a status bar can
use `lyricslens --quit` or the preferences window.

| Command | What it does |
| --- | --- |
| `lyricslens`, or `lls` | runs the overlay and gives the terminal back |
| `lyricslens --foreground` | keeps the terminal, for watching the log |
| `lyricslens --settings` | opens the preferences window |
| `lyricslens --toggle` | hides the overlay, or brings it back |
| `lyricslens --position` | enters the mode where you drag it somewhere else |
| `lyricslens --upgrade` | installs the newest release over this one |
| `lyricslens --uninstall` | removes it from `~/.local` |
| `lyricslens --paths` | prints where the settings and the cached lyrics live |
| `lyricslens --quit` | closes the overlay that is running |
| `lyricslens --help` | the whole list |

### A key to hide it

Wayland gives an ordinary program no way to claim a key combination, so the
binding belongs to your compositor. The preferences window writes the line for
you — type the combination under *Keys* and copy what it shows. On Hyprland it
goes in `hyprland.conf`:

```conf
bind = SUPER, L, exec, lyricslens --toggle
bind = SUPER SHIFT, L, exec, lyricslens --position
```

### Moving it

`--position` turns on a mode where the overlay takes your clicks instead of
letting them through: drag it where you want, then press the key again. Where
you left it is saved.

A layer surface belongs to one screen and cannot be dragged to another, so the
screen is a choice in the preferences rather than a drag across the edge.

### How it looks

The preferences window holds the font and its size and weight, the text
colour, which edge the words line up with, how wide the overlay gets before a
line wraps, how round the strip's corners are, the shadow that keeps it
readable over a bright window, how dark the strip behind the line is,
the line just sung above and how many are shown coming up underneath, a
karaoke mode that fills the line as the song moves through it, and whether the
overlay clears while the player is paused. Every one of them shows on
the overlay straight away.

The same window has the switch that lets you drag the overlay, the screen it
lives on, and whether it starts with your session.

### When the words are wrong

Open the preferences and look under *Lyrics for this track*. The artist and
title come filled in from what is playing; correct them, search, and pick from
everything the service has under that name. The choice is kept for that
recording, so the song comes back right next time.

## What it needs from a player

LyricsLens follows the position the player reports, and some players report
none. Firefox is the notable case: it answers with zero forever, so there is
nothing to synchronise against, and the overlay says so on screen rather than
guessing. Players that do report it — Spotify, mpv with `mpv-mpris`, Rhythmbox,
Amberol — work.

With more than one player open, pick the one you mean in the preferences, or
for a single run:

```bash
LYRICSLENS_PLAYER=spotify lyricslens
```

## Desktops

The overlay is a `wlr-layer-shell` surface, which is what lets it stay above a
fullscreen window. Where that protocol is missing it falls back to an ordinary
X11 window.

| Desktop | Overlay | Above fullscreen |
| --- | --- | --- |
| Hyprland | tested | yes |
| Sway, KWin, COSMIC, niri, Wayfire | expected to work | yes |
| GNOME | fallback | no |
| X11, any window manager | fallback | no |

Only Hyprland has actually been used. The rest is what the protocol implies,
not a report from use.

## How it works

```
MPRIS (D-Bus) ──▶ track ──▶ LRCLIB ──▶ cache on disk
                              │
                    clock ────┘
                      │
                   overlay
```

Two decisions carry the rest, and both came from measurement:

- **The clock anchors on the edge, not on the reading.** Players report the
  position rounded down to the second, so a reading alone carries up to a
  second of error. The instant the integer flips is the one moment the real
  position is known. Measured: the estimate stays within 8ms of the song.
- **On Wayland the compositor owns the window.** Staying on top, choosing a
  place, claiming a key — a program cannot do any of that by itself. It all
  goes through the compositor.

## Where it keeps things

| Path | What |
| --- | --- |
| `~/.config/lyricslens/settings.toml` | preferences, readable and editable by hand |
| `~/.cache/lyricslens/` | one LRC file per recording, so a song already played works offline |
| `~/.local/state/lyricslens/lyricslens.log` | what happened, for a bug report; rolls over at 1 MiB |

`lyricslens --paths` prints all three.

## Contributing

Bug reports and pull requests are welcome — [CONTRIBUTING.md](CONTRIBUTING.md)
says how to build it and what a good change looks like.

## License

[MIT](LICENSE)
