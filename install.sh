#!/usr/bin/env sh
#
# Instalador do LyricsLens.
#
#   curl -fsSL https://raw.githubusercontent.com/Julio0Cesar/lyricslens/main/install.sh | sh
#
# Instala no diretório do usuário — nada de sudo, nada fora de ~/.local.
# Para desinstalar:
#
#   curl -fsSL https://raw.githubusercontent.com/Julio0Cesar/lyricslens/main/install.sh | sh -s -- --remove

set -eu

REPO="Julio0Cesar/lyricslens"
NOME="lyricslens"

PREFIXO="${XDG_DATA_HOME:-$HOME/.local/share}"
DESTINO="$PREFIXO/$NOME"
BIN="$HOME/.local/bin"
ATALHOS="$PREFIXO/applications"
ICONES="$PREFIXO/icons/hicolor"

vermelho() { printf '\033[31m%s\033[0m\n' "$1" >&2; }
verde()    { printf '\033[32m%s\033[0m\n' "$1"; }
info()     { printf '  %s\n' "$1"; }

erro() {
  vermelho "erro: $1"
  exit 1
}

precisa() {
  command -v "$1" >/dev/null 2>&1 || erro "preciso do comando '$1' e ele não está instalado"
}

desinstalar() {
  rm -rf "$DESTINO"
  rm -f "$BIN/$NOME" "$ATALHOS/$NOME.desktop"
  find "$ICONES" -name "$NOME.png" -delete 2>/dev/null || true
  command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$ATALHOS" 2>/dev/null || true
  verde "LyricsLens removido."
  info "As suas preferências e o cache de letras continuam em:"
  info "  ${XDG_DATA_HOME:-$HOME/.local/share}/com.kintiz.lyricslens"
  exit 0
}

[ "${1:-}" = "--remove" ] && desinstalar
[ "${1:-}" = "--uninstall" ] && desinstalar

precisa curl
precisa install

case "$(uname -m)" in
  x86_64|amd64) ;;
  *) erro "por enquanto só há pacote para x86_64 (esta máquina é $(uname -m))" ;;
esac

printf 'Instalando o LyricsLens…\n\n'

# --- baixar -----------------------------------------------------------------

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

BASE="https://github.com/$REPO/releases/latest/download"

info "baixando a última versão"
curl -fsSL "$BASE/$NOME-x86_64.AppImage" -o "$TMP/app.AppImage" \
  || erro "não consegui baixar o pacote — confira se já existe uma release em https://github.com/$REPO/releases"
curl -fsSL "$BASE/$NOME.png" -o "$TMP/icone.png" || erro "não consegui baixar o ícone"

chmod +x "$TMP/app.AppImage"

# --- extrair ----------------------------------------------------------------
#
# O AppImage é executado extraído, e não montado: montar exige libfuse2, que
# várias distribuições já não instalam por padrão. Extrair funciona em todas.

info "extraindo"
( cd "$TMP" && ./app.AppImage --appimage-extract >/dev/null 2>&1 ) \
  || erro "não consegui extrair o pacote"

rm -rf "$DESTINO"
mkdir -p "$DESTINO"
cp -a "$TMP/squashfs-root/." "$DESTINO/"

# --- instalar ---------------------------------------------------------------

mkdir -p "$BIN" "$ATALHOS"
ln -sf "$DESTINO/AppRun" "$BIN/$NOME"

for tamanho in 32 64 128 256; do
  mkdir -p "$ICONES/${tamanho}x${tamanho}/apps"
done
install -m 644 "$TMP/icone.png" "$ICONES/128x128/apps/$NOME.png"
# O AppImage já traz os outros tamanhos; aproveita os que existirem.
for tamanho in 32 64 256; do
  origem="$DESTINO/usr/share/icons/hicolor/${tamanho}x${tamanho}/apps/$NOME.png"
  [ -f "$origem" ] && install -m 644 "$origem" "$ICONES/${tamanho}x${tamanho}/apps/$NOME.png"
done

cat > "$ATALHOS/$NOME.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=LyricsLens
GenericName=Overlay de letras
Comment=Letras sincronizadas sobre qualquer aplicativo
Exec=$BIN/$NOME
Icon=$NOME
Terminal=false
Categories=AudioVideo;Audio;Music;
Keywords=letra;lyrics;musica;music;karaoke;overlay;
StartupWMClass=lyricslens
EOF

command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$ATALHOS" 2>/dev/null || true
command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -f -t "$ICONES" 2>/dev/null || true

# --- resultado --------------------------------------------------------------

printf '\n'
verde "LyricsLens instalado."
printf '\n'
info "Aperte Super e procure por \"LyricsLens\"."
info "Ou rode: $NOME"
printf '\n'

case ":$PATH:" in
  *":$BIN:"*) ;;
  *)
    vermelho "atenção: $BIN não está no seu PATH."
    info "O app aparece no menu normalmente, mas para chamar pelo terminal, adicione:"
    info "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    ;;
esac
