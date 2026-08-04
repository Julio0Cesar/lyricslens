/**
 * Português do Brasil.
 *
 * O `: Dicionario` não é decoração — é ele que faz o TypeScript reprovar
 * chave faltando, chave a mais e interpolação com assinatura diferente do
 * inglês. Traduzir pela metade não compila.
 */
import type { Dicionario } from "./en";

export const ptBR: Dicionario = {
  // ---- overlay ----
  "overlay.nothingPlaying": "nada tocando",
  "overlay.searching": "procurando a letra…",
  "overlay.notFound": "sem letra para esta faixa",
  "overlay.instrumental": "faixa instrumental",
  "overlay.unsynced": "letra sem sincronia",

  // ---- janela de configurações ----
  "settings.title": "Configurações",
  "settings.toggleOverlay": "mostrar / ocultar overlay",
  "settings.loading": "carregando…",
  "settings.footer":
    "As preferências ficam em settings.json, no diretório de dados do app. Em Wayland, tamanho e posição são pedidos ao compositor — a janela não decide por conta própria.",

  "section.appearance": "Aparência",
  "section.content": "Conteúdo",
  "section.behaviour": "Comportamento",
  "section.window": "Janela",
  "section.version": "Versão",
  "section.cache": "Cache",

  // ---- aparência ----
  "font.label": "Fonte",
  "font.count": (n) => `${n} instaladas`,
  "font.systemHint": "do sistema",
  "font.systemOption": "Do sistema",
  "fontSize.label": "Tamanho da letra",
  "fontWeight.label": "Peso",
  "textColor.label": "Cor do texto",
  "dimColor.label": "Cor do texto apagado",
  "dimColor.hint": "linhas ainda não cantadas",
  "backgroundColor.label": "Cor do fundo",
  "backgroundOpacity.label": "Opacidade do fundo",
  "backgroundOpacity.hint": "0 deixa o fundo invisível — o desfoque atrás vem do compositor",
  "cornerRadius.label": "Arredondamento",

  // ---- conteúdo ----
  "contextLines.label": "Linhas de contexto",
  "contextLines.hint": "a anterior e a próxima",
  "trackInfo.label": "Informação da faixa",
  "trackInfo.hint": "fonte, artista e título",
  "progressBar.label": "Barra de progresso",
  "cover.label": "Capa do álbum",
  "cover.hint": "ao lado da letra, quando o álbum for conhecido",
  "karaoke.label": "Varredura de karaokê",
  "karaoke.hint": "acende a linha conforme ela é cantada",
  "align.label": "Alinhamento",
  "align.left": "À esquerda",
  "align.center": "Centralizado",

  // ---- comportamento ----
  "hotkey.label": "Atalho global",
  "hotkey.hint": "mostra e esconde o overlay",
  "hotkey.capturing": "pressione…",
  "hotkey.none": "nenhum",
  "clickThrough.label": "Cliques atravessam",
  "clickThrough.hint": "o overlay deixa de responder ao mouse",
  "hideWhenPaused.label": "Esconder quando pausado",
  "autostart.label": "Iniciar com o sistema",
  "autostart.hint": "sobe direto para a bandeja, sem abrir o overlay",
  "syncOffset.label": "Ajuste de sincronia",
  "syncOffset.hint": "negativo adianta a letra",
  "language.label": "Idioma",
  "language.hint": "a interface segue o sistema, a menos que você escolha aqui",
  "language.auto": "Automático",

  // ---- janela ----
  "width.label": "Largura",
  "height.label": "Altura",
  "marginBottom.label": "Distância do rodapé",
  "position.label": "Posição",
  "position.pinned": (x, y) => `fixada em ${x}, ${y}`,
  "position.auto": "rodapé central · arraste o overlay para mudar",
  "position.center": "centralizar",
  "layerShell.label": "Camada do compositor",
  "layerShell.hint":
    "fica acima até de tela cheia, mas deixa de ser arrastável · exige reiniciar o app",
  "layerShell.inactive": (motivo) => `ligada, mas sem efeito nesta sessão — ${motivo}`,
  "layerShell.unavailable": "indisponível",
  "reapply.hint": "reaplica flutuar, fixar, tamanho e posição",
  "reapply.button": "recolocar agora",

  // ---- versão ----
  "version.installed": "Instalada:",
  "version.available": (versao) => `${versao} disponível`,
  "version.update": "atualizar e reabrir",
  "version.updating": "atualizando…",
  "version.notFromScript":
    "Esta instalação não veio do script, então a atualização é pelo mesmo caminho que você usou para instalar.",
  "version.upToDate": "Esta é a versão mais recente.",

  // ---- cache ----
  "cache.tracks": (n) => (n === 1 ? "faixa guardada" : "faixas guardadas"),
  "cache.synced": (n) => `${n} com sincronia`,
  "cache.pinned": (n) => `${n} fixadas para offline`,
  "cache.misses": (n) => `${n} sem letra conhecida`,
  "cache.explanation":
    "Música que já tocou não vai à rede de novo. Faixa sem letra também é lembrada, para a busca não ser refeita a cada vez — esse registro caduca em três dias, porque letra nova entra no LRCLIB o tempo todo.",

  // ---- letra desta faixa ----
  "picker.title": "Letra desta faixa",
  "picker.synced": (n, fonte) => `${n} linhas sincronizadas · ${fonte}`,
  "picker.instrumental": "instrumental",
  "picker.unsynced": "letra sem sincronia",
  "picker.notFound": "nenhuma letra encontrada",
  "picker.searching": "procurando…",
  "picker.offlineBadge": "● offline",
  "picker.offlineTitle": "guardada para uso offline",
  "picker.keepExplanation":
    "O cache já evita ir à rede de novo. Manter offline é o passo a mais: esta nunca é descartada.",
  "picker.keep": "manter offline",
  "picker.kept": "manter offline ✓",
  "picker.artist": "artista",
  "picker.song": "música",
  "picker.search": "buscar",
  "picker.noResults": "nenhum resultado",
  "picker.hasSync": "sync",
  "picker.noSync": "texto",
  "picker.hasSyncTitle": "sincronizada",
  "picker.noSyncTitle": "sem sincronia",
  "picker.use": "usar",
  "picker.choiceExplanation":
    "A escolha fica guardada para esta faixa — da próxima vez que ela tocar, a letra já vem certa sem passar por aqui.",
  "picker.pinnedTitle": "Guardadas offline",
  "picker.release": "soltar",

  // ---- erros vindos do backend ----
  "error.hotkey.onlyHyprland":
    "atalho automático só em Hyprland — use o keybind do seu compositor",
  "error.hotkey.refused": (atalho, motivo) => `o compositor recusou "${atalho}": ${motivo}`,
  "error.hotkey.semCompositor":
    "o atalho automático precisa de um compositor que o app conheça (Hyprland ou Sway) — use o keybind do seu sistema chamando `lyricslens toggle`",
  "error.hotkey.noExecutable": "não descobri o caminho do executável",
  "error.hotkey.compositorFailed": (motivo) => `o compositor não respondeu: ${motivo}`,
  "error.update.notFromScript":
    "esta instalação não foi feita pelo script — atualize pelo mesmo caminho que usou para instalar",
  "error.update.failed": (motivo) => `a atualização falhou: ${motivo}`,
  "error.autostart.remove": (motivo) => `não consegui remover o arquivo: ${motivo}`,
  "error.autostart.write": (motivo) => `não consegui gravar o arquivo: ${motivo}`,
  "error.autostart.noConfigDir": "não descobri onde fica a sua pasta de configuração",
  "error.autostart.noExecutable": "não descobri o caminho do executável",
  "error.lyrics.network": (motivo) => `não consegui falar com o LRCLIB: ${motivo}`,
  "error.lyrics.notFound": "nenhuma letra encontrada para esta faixa",
  "error.lyrics.decode": (motivo) => `o LRCLIB respondeu algo inesperado: ${motivo}`,
  "error.settings.write": (motivo) => `não consegui salvar as preferências: ${motivo}`,
  "error.cache.write": (motivo) => `não consegui gravar no cache: ${motivo}`,
  "error.overlay.noWindow": "a janela do overlay não existe",
  "error.overlay.rules": (motivo) => `o compositor não aplicou as regras: ${motivo}`,
  "error.unknown": (codigo) => `erro inesperado (${codigo})`,
};
