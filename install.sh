#!/bin/sh
# Installs LyricsLens into ~/.local. Nothing outside your home directory is
# touched, and no sudo is asked for.
#
#   curl -fsSL https://raw.githubusercontent.com/Julio0Cesar/lyricslens/main/install.sh | sh
#   ... | sh -s -- --remove
set -eu

REPO="Julio0Cesar/lyricslens"
NAME="lyricslens"
SHORT="lls"
PREFIX="${XDG_DATA_HOME:-$HOME/.local/share}"
BIN="$HOME/.local/bin"
APPS="$PREFIX/applications"
ICONS="$PREFIX/icons/hicolor"
HOME_DIR="$PREFIX/$NAME"

say() { printf '%s\n' "$*"; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

remove() {
    rm -rf "$HOME_DIR"
    rm -f "$BIN/$NAME" "$BIN/$SHORT" "$APPS/$NAME.desktop"
    for size in 32 64 128 256; do
        rm -f "$ICONS/${size}x${size}/apps/$NAME.png"
    done
    command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APPS" 2>/dev/null || true
    say "LyricsLens removed. Your settings in ~/.config/$NAME and cached lyrics"
    say "in ~/.cache/$NAME were left alone; delete them by hand if you want to."
}

if [ "${1:-}" = "--remove" ] || [ "${1:-}" = "-r" ]; then
    remove
    exit 0
fi

command -v curl >/dev/null 2>&1 || die "curl is needed"
command -v tar >/dev/null 2>&1 || die "tar is needed"

case "$(uname -m)" in
    x86_64) ;;
    *) die "only x86_64 is published; build from source with: cargo install --path ." ;;
esac

# The libraries the binary links against are not bundled: they are the ones a
# desktop already has, and bundling GTK would turn 16MB into 80MB.
missing=""
for library in libgtk-4.so.1 libadwaita-1.so.0 libgtk4-layer-shell.so.0; do
    if ! ldconfig -p 2>/dev/null | grep -q "$library"; then
        missing="$missing $library"
    fi
done
if [ -n "$missing" ]; then
    say "These libraries are missing:$missing"
    say "On Arch:   sudo pacman -S gtk4 libadwaita gtk4-layer-shell"
    say "On Debian: sudo apt install libgtk-4-1 libadwaita-1-0 libgtk4-layer-shell0"
    die "install them and run this again"
fi

# The binary is not static: it is built against the libraries of the machine
# that built it, and an older system cannot run it. Better to say so than to
# hand over a download that dies on its first symbol.
check_glibc() {
    needed=$1
    have=$(ldd --version 2>/dev/null | head -n 1 | grep -o '[0-9]\+\.[0-9]\+' | head -n 1)
    [ -n "$have" ] || return 0

    oldest=$(printf '%s\n%s\n' "$needed" "$have" | sort -V | head -n 1)
    [ "$oldest" = "$needed" ] && return 0

    say "This build needs glibc $needed and this system has $have."
    say "Install the .deb or .rpm from the release instead, or build from"
    say "source with: cargo install --git https://github.com/$REPO"
    die "not installed"
}

TAG="${LYRICSLENS_VERSION:-}"
if [ -z "$TAG" ]; then
    TAG=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
        | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n 1)
fi
[ -n "$TAG" ] || die "could not find the latest release"

ARCHIVE="$NAME-$TAG-x86_64-linux.tar.gz"
BASE="https://github.com/$REPO/releases/download/$TAG"

needed=$(curl -fsSL "$BASE/MINIMUM_GLIBC" 2>/dev/null | tr -d '[:space:]')
[ -n "$needed" ] && check_glibc "$needed"

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT INT TERM

say "Downloading $NAME $TAG…"
curl -fsSL "$BASE/$ARCHIVE" -o "$WORK/$ARCHIVE" || die "could not download $ARCHIVE"

# A download nobody checks is a download nobody can trust.
if curl -fsSL "$BASE/SHA256SUMS" -o "$WORK/SHA256SUMS" 2>/dev/null; then
    if command -v sha256sum >/dev/null 2>&1; then
        (cd "$WORK" && sha256sum -c SHA256SUMS --ignore-missing >/dev/null) \
            || die "the checksum does not match; the download was not installed"
        say "Checksum verified."
    fi
else
    say "warning: this release publishes no SHA256SUMS; skipping the check"
fi

# The archive carries a top directory named after the release; the paths below
# are written without it.
tar -xzf "$WORK/$ARCHIVE" -C "$WORK" --strip-components=1

mkdir -p "$HOME_DIR" "$BIN" "$APPS"
install -Dm755 "$WORK/$NAME" "$HOME_DIR/$NAME"

# A launcher rather than a symlink: the .desktop file and the shell both point
# here, and the real binary can move without either noticing.
cat > "$BIN/$NAME" <<LAUNCHER
#!/bin/sh
exec "$HOME_DIR/$NAME" "\$@"
LAUNCHER
chmod 755 "$BIN/$NAME"

# The same launcher under a name worth typing.
ln -sf "$NAME" "$BIN/$SHORT"

for size in 32 64 128 256; do
    if [ -f "$WORK/icons/$size.png" ]; then
        install -Dm644 "$WORK/icons/$size.png" "$ICONS/${size}x${size}/apps/$NAME.png"
    fi
done

cat > "$APPS/$NAME.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=LyricsLens
GenericName=Lyrics overlay
Comment=Synced lyrics on top of any application
Exec=$BIN/$NAME
Icon=$NAME
Terminal=false
Categories=AudioVideo;Audio;Player;
StartupNotify=false
DESKTOP

command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APPS" 2>/dev/null || true

say ""
say "LyricsLens $TAG installed."
case ":$PATH:" in
    *":$BIN:"*) say "Run it with: $NAME" ;;
    *) say "Run it with: $BIN/$NAME"
       say "($BIN is not on your PATH; add it to use the short name.)" ;;
esac
say "Short name: $SHORT"
say "Preferences: $NAME --settings"
