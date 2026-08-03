import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { AnimatePresence, motion } from "motion/react";
import { formatMs, useNowPlaying } from "./media/useNowPlaying";
import LyricLines from "./lyrics/LyricLines";
import { lineAt, lineProgress, useLyrics } from "./lyrics/useLyrics";
import { useSettings, type Settings } from "./settings/useSettings";

/** `#0a0a0e` + 0.55 → `rgba(10, 10, 14, 0.55)`. */
function rgba(hex: string, alpha: number): string {
  const limpo = hex.replace("#", "").slice(0, 6);
  if (limpo.length !== 6) return `rgba(0, 0, 0, ${alpha})`;
  const n = parseInt(limpo, 16);
  return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${alpha})`;
}

/** Tudo que não é letra: nada tocando, procurando, sem letra, instrumental. */
function Aviso({ texto, settings }: { texto: string; settings: Settings }) {
  return (
    <motion.div
      key={texto}
      initial={{ opacity: 0, y: 10, filter: "blur(6px)" }}
      animate={{ opacity: 1, y: 0, filter: "blur(0px)" }}
      exit={{ opacity: 0, y: -10, filter: "blur(6px)" }}
      transition={{ duration: 0.3, ease: "easeOut" }}
      style={{
        color: settings.dimColor,
        fontSize: Math.round(settings.fontSize * 0.72),
        fontWeight: 500,
      }}
    >
      {texto}
    </motion.div>
  );
}

/**
 * Só vira `true` depois de `ms` com a condição ativa.
 *
 * Serve para não piscar "procurando a letra…" numa busca que dura 300ms — o
 * aviso aparecendo e sumindo é mais perturbador que o intervalo em branco.
 */
function useAtraso(ativo: boolean, ms: number): boolean {
  const [passou, setPassou] = useState(false);

  useEffect(() => {
    if (!ativo) {
      setPassou(false);
      return;
    }
    const t = window.setTimeout(() => setPassou(true), ms);
    return () => window.clearTimeout(t);
  }, [ativo, ms]);

  return passou;
}

/**
 * Arraste manual, em vez de `data-tauri-drag-region`.
 *
 * A região de arraste do Tauri chama o arraste do compositor já no
 * `mousedown` — e a partir daí o ponteiro é do compositor, então o segundo
 * clique de um duplo clique nunca chega ao webview. Aqui o arraste só começa
 * quando o ponteiro **se move**, então um clique parado continua sendo um
 * clique.
 *
 * Em Wayland a janela também não sabe onde está e soltar não avisa ninguém;
 * ao recuperar o foco, perguntamos ao compositor e guardamos.
 */
function useArrastar() {
  useEffect(() => {
    const salvar = () => {
      invoke("remember_overlay_position").catch(() => {});
    };
    window.addEventListener("focus", salvar);
    return () => window.removeEventListener("focus", salvar);
  }, []);

  return (e: React.MouseEvent) => {
    // `detail > 1` é o segundo clique de um duplo: nunca vira arraste.
    if (e.button !== 0 || e.detail > 1) return;

    const origem = { x: e.clientX, y: e.clientY };

    const mover = (m: MouseEvent) => {
      if (Math.hypot(m.clientX - origem.x, m.clientY - origem.y) < 4) return;
      limpar();
      getCurrentWindow().startDragging().catch(() => {});
    };
    const limpar = () => {
      window.removeEventListener("mousemove", mover);
      window.removeEventListener("mouseup", limpar);
    };

    window.addEventListener("mousemove", mover);
    window.addEventListener("mouseup", limpar);
  };
}

function App() {
  // Todos os hooks antes de qualquer saída antecipada: o React exige que a
  // ordem e a quantidade sejam iguais em toda renderização, e um `return`
  // no meio derruba o componente inteiro quando as preferências chegam.
  const { track, positionMs } = useNowPlaying();
  const identity = track ? `${track.artist}|${track.title}` : null;
  const { status, lyrics } = useLyrics(identity);
  const { settings } = useSettings();
  const buscaDemorada = useAtraso(status === "searching", 700);
  const iniciarArraste = useArrastar();

  if (!settings) return null;

  const lines = lyrics?.lines ?? [];
  const idx = lineAt(lines, positionMs);
  const progress = lineProgress(lines, idx, positionMs);
  const temLetraSincronizada = lines.length > 0;

  const aviso = !track
    ? "nada tocando"
    : status === "searching"
      ? buscaDemorada
        ? "procurando a letra…"
        : null
      : status === "notFound"
        ? "sem letra para esta faixa"
        : lyrics?.instrumental
          ? "faixa instrumental"
          : status === "found" && !temLetraSincronizada
            ? "letra sem sincronia"
            : null;

  return (
    <div className="flex h-full flex-col font-sans select-none">
      <div
        onMouseDown={iniciarArraste}
        onDoubleClick={() => invoke("open_settings")}
        // O menu do WebKit não tem nada de útil aqui e quebra a ilusão de
        // que isto é só a letra na tela.
        onContextMenu={(e) => e.preventDefault()}
        style={{
          background: rgba(settings.backgroundColor, settings.backgroundOpacity),
          borderRadius: `${settings.cornerRadius}px`,
          fontFamily: settings.fontFamily || undefined,
          // Nada de `backdrop-filter` aqui. Ele borra o que está atrás do
          // elemento *dentro da página* — e atrás desta página não há página
          // nenhuma, só o desktop, que pertence ao compositor. Numa janela
          // transparente ele não tem o que amostrar e desenha lixo: foi o que
          // embaralhava a letra com a opacidade no mínimo.
          border:
            settings.backgroundOpacity > 0
              ? "1px solid rgba(255,255,255,0.08)"
              : "none",
          justifyContent: "center",
          alignItems: settings.textAlign === "center" ? "center" : "stretch",
        }}
        className="flex flex-1 flex-col overflow-hidden px-6 py-4"
      >
        {settings.showTrackInfo && track && (
          <div
            className="mb-2 flex items-center gap-2 truncate text-[10px] tracking-[0.1em] uppercase"
            style={{ color: settings.dimColor }}
          >
            <span className="rounded bg-white/10 px-1.5 py-0.5">{track.source}</span>
            <span className="truncate">
              {track.artist} — {track.title}
            </span>
          </div>
        )}

        <AnimatePresence mode="wait">
          {aviso ? (
            <Aviso key={aviso} texto={aviso} settings={settings} />
          ) : (
            <motion.div key="letra" className="min-w-0">
              <LyricLines
                lines={lines}
                index={idx}
                progress={progress}
                settings={settings}
              />
            </motion.div>
          )}
        </AnimatePresence>

        {settings.showProgress && track && (
          <div className="mt-3 flex items-center gap-2">
            <span
              className="font-mono text-[10px] tabular-nums"
              style={{ color: settings.dimColor }}
            >
              {formatMs(positionMs)}
            </span>
            <div className="h-[3px] flex-1 overflow-hidden rounded-full bg-white/10">
              <div
                className="h-full rounded-full"
                style={{
                  background: settings.textColor,
                  width: `${
                    track.durationMs
                      ? Math.min(100, Math.max(0, (positionMs / track.durationMs) * 100))
                      : 0
                  }%`,
                }}
              />
            </div>
            <span
              className="font-mono text-[10px] tabular-nums"
              style={{ color: settings.dimColor }}
            >
              {track.durationMs ? formatMs(track.durationMs) : "--:--"}
            </span>
          </div>
        )}
      </div>
    </div>
  );
}

export default App;
