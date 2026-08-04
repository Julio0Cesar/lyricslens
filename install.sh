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

# A origem é configurável para o teste de fumaça poder apontar o script para os
# artefatos da release que está sendo construída, em vez da última publicada —
# testar contra a release anterior não diria nada sobre esta. Ver #33.
BASE="${LYRICSLENS_BASE:-https://github.com/$REPO/releases/latest/download}"

# --- escolher o formato ------------------------------------------------------
#
# O AppImage traz WebKit e GTK dentro: 82MB, contra 8MB do tarball. Medido no
# pacote da v0.6.0, WebKit e JavaScriptCore sozinhos são 119MB dos 265MB
# extraídos — não há como o AppImage ser pequeno.
#
# Quem já tem essas bibliotecas (a maioria dos desktops) não precisa baixá-las
# de novo. Quem não tem continua com um caminho que funciona sem instalar nada.
# Ver #8.

tem_no_sistema() {
  command -v ldconfig >/dev/null 2>&1 || return 1
  cache="$(ldconfig -p 2>/dev/null)" || return 1
  for lib in libwebkit2gtk-4.1 libgtk-3 libayatana-appindicator3 libgtk-layer-shell; do
    case "$cache" in
      *"$lib"*) ;;
      *) return 1 ;;
    esac
  done
  return 0
}

if [ -n "${LYRICSLENS_FORMATO:-}" ]; then
  FORMATO="$LYRICSLENS_FORMATO"
elif tem_no_sistema; then
  FORMATO=tarball
else
  FORMATO=appimage
fi

# --- baixar o pacote ---------------------------------------------------------

if [ "$FORMATO" = tarball ]; then
  PACOTE="$NOME-x86_64.tar.gz"
  info "baixando a última versão (8MB — as bibliotecas já estão no seu sistema)"
else
  PACOTE="$NOME-x86_64.AppImage"
  info "baixando a última versão (82MB — o pacote traz o WebKit e o GTK dentro,"
  info "porque não encontrei os do sistema)"
fi

if ! curl -fsSL "$BASE/$PACOTE" -o "$TMP/pacote"; then
  # Releases anteriores à v0.7.0 não publicam o tarball. Sem esta queda, quem
  # rodasse o script novo contra uma release velha tomaria 404 e não instalaria
  # nada — e o script é buscado sempre do `main`, então isso aconteceria com
  # todo mundo até a release seguinte sair.
  if [ "$FORMATO" = tarball ]; then
    info "esta release não publica o tarball — usando o AppImage"
    FORMATO=appimage
    PACOTE="$NOME-x86_64.AppImage"
    curl -fsSL "$BASE/$PACOTE" -o "$TMP/pacote" \
      || erro "não consegui baixar o pacote — confira se já existe uma release em https://github.com/$REPO/releases"
  else
    erro "não consegui baixar o pacote — confira se já existe uma release em https://github.com/$REPO/releases"
  fi
fi
curl -fsSL "$BASE/$NOME.png" -o "$TMP/icone.png" || erro "não consegui baixar o ícone"

# --- conferir a integridade -------------------------------------------------
#
# O download é HTTPS direto do GitHub, então o risco é baixo — mas um pacote
# truncado por conexão caída falha de formas confusas lá na frente, e quem lê o
# script antes de rodar espera ver esta conferência.

soma() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | cut -d' ' -f1
  fi
}

if curl -fsSL "$BASE/SHA256SUMS" -o "$TMP/SHA256SUMS" 2>/dev/null; then
  esperado="$(grep " $PACOTE\$" "$TMP/SHA256SUMS" | cut -d' ' -f1)"
  obtido="$(soma "$TMP/pacote")"
  if [ -z "$obtido" ]; then
    info "sem sha256sum nem shasum — pulando a conferência de integridade"
  elif [ -z "$esperado" ]; then
    info "o SHA256SUMS não lista $PACOTE — pulando a conferência"
  elif [ "$esperado" != "$obtido" ]; then
    erro "o pacote baixado não confere com o checksum publicado
  esperado: $esperado
  obtido:   $obtido
Não vou instalar. Tente de novo; se persistir, abra uma issue."
  else
    info "integridade conferida"
  fi
else
  # Releases anteriores à v0.2.2 não publicam SHA256SUMS.
  info "esta release não publica checksums — pulando a conferência"
fi

# --- extrair ----------------------------------------------------------------
#
# O AppImage é executado extraído, e não montado: montar exige libfuse2, que
# várias distribuições já não instalam por padrão. Extrair funciona em todas.

info "extraindo"
if [ "$FORMATO" = tarball ]; then
  mkdir -p "$TMP/conteudo"
  tar -xzf "$TMP/pacote" -C "$TMP/conteudo" --strip-components=1 \
    || erro "não consegui extrair o pacote"
  EXECUTAVEL="bin/$NOME"
  ICONES_NO_PACOTE="share/icons/hicolor"
else
  chmod +x "$TMP/pacote"
  ( cd "$TMP" && ./pacote --appimage-extract >/dev/null 2>&1 ) \
    || erro "não consegui extrair o pacote"
  mv "$TMP/squashfs-root" "$TMP/conteudo"
  EXECUTAVEL="AppRun"
  ICONES_NO_PACOTE="usr/share/icons/hicolor"
fi

# O AppRun põe o `usr/lib/` do pacote na frente do LD_LIBRARY_PATH, então toda
# biblioteca empacotada sequestra a do sistema. Para a maioria isso é o que se
# quer — é o que torna o AppImage autossuficiente. Para estas, não: elas
# precisam casar com o compositor e com o Mesa da máquina, e a versão que veio
# do contêiner de build costuma ser velha demais (falta `wl_proxy_get_queue`,
# por exemplo), o que derruba o EGL e abre o overlay em branco.
#
# A partir da v0.2.2 o workflow de release já não as empacota; esta limpeza
# cobre quem instalar uma release anterior. Ver #30.
rm -f "$TMP"/conteudo/usr/lib/libwayland-*

# A troca é feita de lado e só então movida para o lugar. Apagar o destino
# antes de copiar deixaria a instalação inexistente por alguns segundos — e se
# o app estiver rodando dali, ele perde os próprios arquivos no meio do
# caminho. `mv` dentro do mesmo sistema de arquivos é praticamente instantâneo.
NOVO="$DESTINO.novo"
ANTIGO="$DESTINO.antigo"

rm -rf "$NOVO" "$ANTIGO"
mkdir -p "$NOVO"
cp -a "$TMP/conteudo/." "$NOVO/"

if [ -d "$DESTINO" ]; then
  mv "$DESTINO" "$ANTIGO"
fi
mv "$NOVO" "$DESTINO"
rm -rf "$ANTIGO"

# --- instalar ---------------------------------------------------------------

mkdir -p "$BIN" "$ATALHOS"

# Um symlink não serve: o AppRun se localiza por `dirname "$0"`, e através do
# link isso aponta para a pasta do link, não para a instalação. O invólucro
# chama o caminho real, então o AppRun encontra os próprios arquivos.
cat > "$BIN/$NOME" <<EOF
#!/bin/sh
exec "$DESTINO/$EXECUTAVEL" "\$@"
EOF
chmod +x "$BIN/$NOME"

for tamanho in 32 64 128 256; do
  mkdir -p "$ICONES/${tamanho}x${tamanho}/apps"
done
install -m 644 "$TMP/icone.png" "$ICONES/128x128/apps/$NOME.png"
# O pacote já traz os outros tamanhos; aproveita os que existirem.
for tamanho in 32 64 256; do
  origem="$DESTINO/$ICONES_NO_PACOTE/${tamanho}x${tamanho}/apps/$NOME.png"
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
StartupWMClass=Lyricslens
EOF

command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$ATALHOS" 2>/dev/null || true
command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -f -t "$ICONES" 2>/dev/null || true

# --- conferir as dependências de runtime -------------------------------------
#
# Sem estas bibliotecas o app não abre. Lançado pelo menu, o stderr dele não
# aparece em lugar nenhum: o usuário clica no ícone e simplesmente não acontece
# nada. Vale gastar meia dúzia de linhas para dizer o que falta. Ver #22.

comando_de_instalacao() {
  distro=""
  [ -r /etc/os-release ] && distro="$(. /etc/os-release 2>/dev/null && printf '%s %s' "${ID:-}" "${ID_LIKE:-}")"
  case " $distro " in
    *debian*|*ubuntu*)   printf 'sudo apt install libwebkit2gtk-4.1-0 libgtk-3-0 libayatana-appindicator3-1' ;;
    *fedora*|*rhel*)     printf 'sudo dnf install webkit2gtk4.1 gtk3 libayatana-appindicator-gtk3' ;;
    *suse*)              printf 'sudo zypper install libwebkit2gtk-4_1-0 gtk3 libayatana-appindicator3-1' ;;
    *arch*)              printf 'sudo pacman -S webkit2gtk-4.1 gtk3 libayatana-appindicator' ;;
    *) printf '' ;;
  esac
}

faltando=""
if command -v ldconfig >/dev/null 2>&1; then
  cache="$(ldconfig -p 2>/dev/null || true)"
  for lib in libwebkit2gtk-4.1 libgtk-3 libayatana-appindicator3; do
    case "$cache" in
      *"$lib"*) ;;
      *) faltando="$faltando $lib" ;;
    esac
  done
fi

# --- resultado --------------------------------------------------------------

printf '\n'

if [ -n "$faltando" ]; then
  vermelho "LyricsLens foi copiado, mas ainda não vai abrir."
  printf '\n'
  info "Faltam bibliotecas de sistema:"
  for lib in $faltando; do info "  - $lib"; done
  printf '\n'
  cmd="$(comando_de_instalacao)"
  if [ -n "$cmd" ]; then
    info "Instale com:"
    info "  $cmd"
  else
    info "Instale-as pelo gerenciador de pacotes da sua distribuição e rode o app de novo."
  fi
  printf '\n'
  info "Depois disso, aperte Super e procure por \"LyricsLens\"."
  printf '\n'
  exit 1
fi

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
