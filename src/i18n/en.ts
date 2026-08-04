/**
 * Inglês — e a fonte dos tipos.
 *
 * Cada idioma novo é um arquivo que declara `: Dicionario`, e o TypeScript
 * cobra chave faltando, chave sobrando e assinatura errada de interpolação.
 * Não há como esquecer de traduzir alguma coisa e descobrir só na tela.
 *
 * Interpolação é função, não `{placeholder}` em string: o compilador confere
 * que quem chama passa os argumentos certos, e a ordem das palavras fica livre
 * para cada idioma — o que uma string com marcador não permite.
 */
export const en = {
  // ---- overlay ----
  // As três mensagens mais vistas do app inteiro.
  "overlay.nothingPlaying": "nothing playing",
  "overlay.searching": "looking for the lyrics…",
  "overlay.notFound": "no lyrics for this track",
  "overlay.instrumental": "instrumental track",
  "overlay.unsynced": "lyrics without timing",

  // ---- janela de configurações ----
  "settings.title": "Settings",
  "settings.toggleOverlay": "show / hide overlay",
  "settings.loading": "loading…",
  "settings.footer":
    "Preferences live in settings.json, in the app data directory. On Wayland, size and position are requested from the compositor — the window does not decide on its own.",

  "section.appearance": "Appearance",
  "section.content": "Content",
  "section.behaviour": "Behaviour",
  "section.window": "Window",
  "section.version": "Version",
  "section.cache": "Cache",

  // ---- aparência ----
  "font.label": "Font",
  "font.count": (n: number) => `${n} installed`,
  "font.systemHint": "from the system",
  "font.systemOption": "System default",
  "fontSize.label": "Text size",
  "fontWeight.label": "Weight",
  "textColor.label": "Text colour",
  "dimColor.label": "Dimmed text colour",
  "dimColor.hint": "lines not sung yet",
  "backgroundColor.label": "Background colour",
  "backgroundOpacity.label": "Background opacity",
  "backgroundOpacity.hint":
    "0 makes the background invisible — the blur behind it comes from the compositor",
  "cornerRadius.label": "Corner radius",

  // ---- conteúdo ----
  "contextLines.label": "Context lines",
  "contextLines.hint": "the previous and the next one",
  "trackInfo.label": "Track info",
  "trackInfo.hint": "source, artist and title",
  "progressBar.label": "Progress bar",
  "cover.label": "Album art",
  "cover.hint": "next to the lyrics, when the album is known",
  "karaoke.label": "Karaoke sweep",
  "karaoke.hint": "lights each line up as it is sung",
  "align.label": "Alignment",
  "align.left": "Left",
  "align.center": "Centred",

  // ---- comportamento ----
  "hotkey.label": "Global hotkey",
  "hotkey.hint": "shows and hides the overlay",
  "hotkey.capturing": "press it…",
  "hotkey.none": "none",
  "clickThrough.label": "Click-through",
  "clickThrough.hint": "the overlay stops responding to the mouse",
  "hideWhenPaused.label": "Hide when paused",
  "autostart.label": "Start with the system",
  "autostart.hint": "goes straight to the tray, without opening the overlay",
  "syncOffset.label": "Sync adjustment",
  "syncOffset.hint": "negative brings the lyrics forward",
  "language.label": "Language",
  "language.hint": "the interface follows the system unless you choose here",
  "language.auto": "Automatic",

  // ---- janela ----
  "width.label": "Width",
  "height.label": "Height",
  "marginBottom.label": "Distance from the bottom",
  "position.label": "Position",
  "position.pinned": (x: number, y: number) => `pinned at ${x}, ${y}`,
  "position.auto": "bottom centre · drag the overlay to change it",
  "position.center": "centre it",
  "layerShell.label": "Compositor layer",
  "layerShell.hint":
    "stays above even fullscreen, but stops being draggable · needs an app restart",
  "layerShell.inactive": (motivo: string) => `on, but with no effect this session — ${motivo}`,
  "layerShell.unavailable": "unavailable",
  "reapply.hint": "reapplies floating, pinning, size and position",
  "reapply.button": "place it again",

  // ---- versão ----
  "version.installed": "Installed:",
  "version.available": (versao: string) => `${versao} available`,
  "version.update": "update and reopen",
  "version.updating": "updating…",
  "version.notFromScript":
    "This install did not come from the script, so updating goes through the same path you used to install.",
  "version.upToDate": "This is the latest version.",

  // ---- cache ----
  // O retorno é anotado porque o ternário estreitaria o tipo para os dois
  // literais em inglês, e aí nenhuma tradução casaria com o contrato.
  "cache.tracks": (n: number): string => (n === 1 ? "cached track" : "cached tracks"),
  "cache.synced": (n: number) => `${n} with timing`,
  "cache.pinned": (n: number) => `${n} kept offline`,
  "cache.misses": (n: number) => `${n} with no known lyrics`,
  "cache.explanation":
    "A song that already played does not go to the network again. A track with no lyrics is remembered too, so the search is not repeated every time — that record expires in three days, because new lyrics land on LRCLIB all the time.",

  // ---- letra desta faixa ----
  "picker.title": "Lyrics for this track",
  "picker.synced": (n: number, fonte: string) => `${n} timed lines · ${fonte}`,
  "picker.instrumental": "instrumental",
  "picker.unsynced": "lyrics without timing",
  "picker.notFound": "no lyrics found",
  "picker.searching": "searching…",
  "picker.offlineBadge": "● offline",
  "picker.offlineTitle": "kept for offline use",
  "picker.keepExplanation":
    "The cache already avoids going to the network again. Keeping it offline is the extra step: this one is never discarded.",
  "picker.keep": "keep offline",
  "picker.kept": "keep offline ✓",
  "picker.artist": "artist",
  "picker.song": "song",
  "picker.search": "search",
  "picker.noResults": "no results",
  "picker.hasSync": "sync",
  "picker.noSync": "text",
  "picker.hasSyncTitle": "timed",
  "picker.noSyncTitle": "no timing",
  "picker.use": "use",
  "picker.choiceExplanation":
    "The choice is kept for this track — next time it plays, the right lyrics come straight through without going through here.",
  "picker.pinnedTitle": "Kept offline",
  "picker.release": "release",

  // ---- erros vindos do backend ----
  // O Rust manda código e parâmetros; a frase é montada aqui. Ver `UiError`.
  "error.hotkey.onlyHyprland":
    "automatic hotkey only on Hyprland — use your compositor's own keybinding",
  "error.hotkey.refused": (atalho: string, motivo: string) =>
    `the compositor refused "${atalho}": ${motivo}`,
  "error.hotkey.semCompositor":
    "automatic hotkey needs a compositor this app knows (Hyprland or Sway) — use your own keybinding calling `lyricslens toggle`",
  "error.hotkey.noExecutable": "could not work out the path to the executable",
  "error.hotkey.compositorFailed": (motivo: string) => `the compositor did not answer: ${motivo}`,
  "error.update.notFromScript":
    "this install did not come from the script — update through the same path you used to install",
  "error.update.failed": (motivo: string) => `the update failed: ${motivo}`,
  "error.autostart.remove": (motivo: string) => `could not remove the file: ${motivo}`,
  "error.autostart.write": (motivo: string) => `could not write the file: ${motivo}`,
  "error.autostart.noConfigDir": "could not work out where your config folder is",
  "error.autostart.noExecutable": "could not work out the path to the executable",
  "error.lyrics.network": (motivo: string) => `could not reach LRCLIB: ${motivo}`,
  "error.lyrics.notFound": "no lyrics found for this track",
  "error.lyrics.decode": (motivo: string) => `LRCLIB answered something unexpected: ${motivo}`,
  "error.settings.write": (motivo: string) => `could not save the preferences: ${motivo}`,
  "error.cache.write": (motivo: string) => `could not write to the cache: ${motivo}`,
  "error.overlay.noWindow": "the overlay window does not exist",
  "error.overlay.rules": (motivo: string) => `the compositor did not apply the rules: ${motivo}`,
  /** Código que este idioma ainda não conhece: mostra o código, não some. */
  "error.unknown": (codigo: string) => `unexpected error (${codigo})`,
};

/**
 * O contrato que todo idioma cumpre.
 *
 * Sem `as const` de propósito: aqui interessa o *formato* (string ou função com
 * tal assinatura), não o texto literal do inglês.
 */
export type Dicionario = typeof en;

/** Toda chave que a UI pode pedir. */
export type Chave = keyof Dicionario;
