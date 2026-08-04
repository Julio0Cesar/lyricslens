<div align="center">

<img src="assets/logo.svg" alt="LyricsLens" width="96">

# LyricsLens

**Synced lyrics on top of any application.**

Detects what is playing on your system, fetches the lyrics and shows them in a
floating window — following the song word by word.

**English** | [Português](README.pt-BR.md)

[![CI](https://github.com/Julio0Cesar/lyricslens/actions/workflows/ci.yml/badge.svg)](https://github.com/Julio0Cesar/lyricslens/actions/workflows/ci.yml)
[![Version](https://img.shields.io/github/v/release/Julio0Cesar/lyricslens?label=version)](https://github.com/Julio0Cesar/lyricslens/releases)
[![License](https://img.shields.io/github/license/Julio0Cesar/lyricslens)](LICENSE)

</div>

<!-- #20 — the GIF of the overlay in action goes here, right below the title.
     Record ~5s with music playing, lyrics following word by word, over any
     window. Save it as assets/demo.gif and replace this comment with:

     <div align="center"><img src="assets/demo.gif" alt="The overlay following the lyrics" width="720"></div>

     A second image of the settings window (assets/settings.png) goes in the
     Usage section. -->

---

## Installation

> **Linux · x86_64 · Wayland or X11.** No Windows yet ([#5](https://github.com/Julio0Cesar/lyricslens/issues/5)).

### Debian, Ubuntu, Mint

Download the `.deb` from [Releases](https://github.com/Julio0Cesar/lyricslens/releases/latest), then:

```bash
sudo apt install ./lyricslens-x86_64.deb
```

### Fedora, openSUSE

Download the `.rpm` from [Releases](https://github.com/Julio0Cesar/lyricslens/releases/latest), then:

```bash
sudo dnf install ./lyricslens-x86_64.rpm
```

`apt install ./file.deb` and `dnf install ./file.rpm` resolve dependencies.
`dpkg -i` and `rpm -i` do not — they fail with unmet dependencies and leave the
package half-installed.

### Arch, NixOS, or no sudo

The script installs into `~/.local` and touches nothing outside your home
directory:

```bash
curl -fsSL https://raw.githubusercontent.com/Julio0Cesar/lyricslens/main/install.sh | sh
```

If you would rather read it before running it — and you should:

```bash
curl -fsSL https://raw.githubusercontent.com/Julio0Cesar/lyricslens/main/install.sh -o install.sh
less install.sh && sh install.sh
```

Then press **Super** and search for *LyricsLens*. To remove it:

```bash
curl -fsSL https://raw.githubusercontent.com/Julio0Cesar/lyricslens/main/install.sh | sh -s -- --remove
```

### Verifying the download

Every release publishes a `SHA256SUMS`. `install.sh` checks it for you; to
verify a package you downloaded by hand:

```bash
curl -LO https://github.com/Julio0Cesar/lyricslens/releases/latest/download/SHA256SUMS
sha256sum -c SHA256SUMS --ignore-missing
```

## Requirements

- Linux, x86_64
- `webkit2gtk-4.1`, `gtk3`, `libayatana-appindicator`, `gtk-layer-shell` — the
  `.deb` and `.rpm` pull these in automatically; the AppImage bundles them
- A player that exposes MPRIS: Spotify, Chromium, Firefox, VLC…

### Compatibility by desktop environment

On Wayland the compositor owns the window, so behaviour varies. This table
records what was actually tested:

| Environment | Overlay | Always on top | Automatic positioning | Global hotkey |
|---|---|---|---|---|
| Hyprland (Wayland) | ✅ | ✅ | ✅ | ✅ via `hyprctl` |
| Sway, river, other wlroots | untested ¹ | untested ¹ | ❌ | manual |
| KDE Plasma (Wayland) | untested | untested | ❌ | manual |
| GNOME (Wayland) | untested | ❌ ² | ❌ | manual |
| X11 (any WM) | untested | untested | ❌ | manual |

Only Hyprland has actually been tested. The rest is what the protocol implies,
not a report from use — if you run it on any of them, a comment on
[#24](https://github.com/Julio0Cesar/lyricslens/issues/24) fills in the row.

¹ Should work through `wlr-layer-shell`, which those compositors implement.
² GNOME does not implement `wlr-layer-shell`, so the overlay falls back to an
ordinary window and cannot stay above fullscreen.

*Automatic positioning* and *global hotkey* currently only know Hyprland — in
any other environment, use your own keybinding to call `lyricslens toggle` (see
[Usage](#usage)). Widening that is
[#12](https://github.com/Julio0Cesar/lyricslens/issues/12); a blank row in the
table is a request for help, not an oversight.

## Usage

The app lives in the system tray. Closing the window **hides** the overlay;
quitting is an explicit choice, from the tray menu.

| Action | How |
|---|---|
| Show / hide | Click the tray icon, or the global hotkey |
| Open settings | **Double click** on the overlay |
| Close settings | **Double click** outside the controls |
| Move the overlay | Drag it |
| Fix the wrong lyrics | Settings → *Lyrics for this track* |

### Global hotkey

In **Settings → Behaviour → Global hotkey**, press whatever combination you
want.

Wayland does not let an application register a system-wide hotkey — the
compositor does. On Hyprland the app asks for it through `hyprctl`, pointing
back at its own executable. Nothing is written to your config: the compositor
forgets the hotkey when it restarts and the app reapplies it every time it comes
up.

In other environments, use your system's keybinding to call:

```bash
lyricslens toggle     # show or hide
lyricslens settings   # open settings
lyricslens hide
lyricslens            # show
```

### Reporting a problem

A desktop app launched from the menu has no visible stderr, so failures are
written to a log file instead. These two answer most of what a bug report needs:

```bash
lyricslens --version
lyricslens --paths     # where the log, cache and preferences live
```

The log lives in `~/.local/state/lyricslens/`, rotates at 1 MiB and keeps one
previous file. Failures you can act on — a hotkey the compositor refused, a
lyrics lookup that never reached LRCLIB — also show up in the settings window.

## How it works

```
MPRIS (D-Bus) ──▶ track detection ──▶ lyrics lookup (LRCLIB) ──▶ local cache
                                                                     │
                                                    sync engine ─────┘
                                                          │
                                                   overlay (React)
```

The Rust backend owns the state and the clock; the frontend only draws and
interpolates between ticks.

Two decisions carry the rest of the project, and both came from measurement
rather than guesswork — the reasoning is in
[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md):

- **Playback position is anchored on the *edge*, not on the reading.** Many
  sources report position rounded to the second. Anchoring on the instant the
  value flips brings the error down from ±1000ms to ±71ms.
- **On Wayland the compositor owns the window.** Staying on top, choosing a
  position, registering a hotkey — an application cannot do any of that on its
  own. It all goes through a request to the compositor.

## Status

| Area | State |
|---|---|
| Media detection (MPRIS) | done |
| Synced lyrics (LRCLIB) | done |
| Always-on-top overlay | done, including over fullscreen |
| Tray + global hotkey | done |
| Local cache (SQLite) | done |
| Settings window | done |
| Manual lyrics override | done |
| Offline mode | [#2](https://github.com/Julio0Cesar/lyricslens/issues/2) |
| Album art | [#3](https://github.com/Julio0Cesar/lyricslens/issues/3) |
| English interface | done, follows your `LANG` |
| Lyrics translation | [#1](https://github.com/Julio0Cesar/lyricslens/issues/1) |
| Windows | [#5](https://github.com/Julio0Cesar/lyricslens/issues/5) |

## Development

```bash
pnpm install
pnpm tauri dev
```

```bash
cd src-tauri && cargo test    # 88 tests
pnpm test           # 68 tests
pnpm exec tsc --noEmit
```

### Versioning

Versions come straight out of the commits, following
[Conventional Commits](https://www.conventionalcommits.org/):

| Prefix | Effect on `X.Y.Z` |
|---|---|
| `fix:` | `Z` |
| `feat:` | `Y` |
| `feat!:` or `BREAKING CHANGE:` | `X` |
| `docs:`, `chore:`, `test:` | none |

On landing in `main`, `release-please` opens a release PR with the CHANGELOG and
the new version. When that PR is approved the tag is created, the packages are
built, installed in a clean container as a check, and only then published.

Issues and commit messages are in Portuguese; that is deliberate and does not
affect contributions in English.

## License

[MIT](LICENSE)
