import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
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
 * Em Wayland a janela não sabe onde está, e arrastar não avisa ninguém. Depois
 * de soltar, perguntamos ao compositor e guardamos — senão a posição se perde
 * na próxima vez que o overlay for escondido e mostrado.
 */
function useLembrarPosicao() {
  useEffect(() => {
    let arrastando = false;

    const comecou = (e: MouseEvent) => {
      if (e.button === 0) arrastando = true;
    };
    const soltou = () => {
      if (!arrastando) return;
      arrastando = false;
      // O compositor precisa de um instante para assentar a nova posição.
      window.setTimeout(() => invoke("remember_overlay_position"), 250);
    };

    window.addEventListener("mousedown", comecou);
    window.addEventListener("mouseup", soltou);
    // Durante um arraste interativo o webview costuma perder o mouseup;
    // voltar o foco é o outro sinal confiável de que acabou.
    window.addEventListener("focus", soltou);

    return () => {
      window.removeEventListener("mousedown", comecou);
      window.removeEventListener("mouseup", soltou);
      window.removeEventListener("focus", soltou);
    };
  }, []);
}

function App() {
  const { track, positionMs } = useNowPlaying();
  useLembrarPosicao();
  const identity = track ? `${track.artist}|${track.title}` : null;
  const { status, lyrics } = useLyrics(identity);
  const { settings } = useSettings();

  if (!settings) return null;

  const lines = lyrics?.lines ?? [];
  const idx = lineAt(lines, positionMs);
  const progress = lineProgress(lines, idx, positionMs);
  const temLetraSincronizada = lines.length > 0;

  const buscaDemorada = useAtraso(status === "searching", 700);

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
    <div className="flex h-full flex-col p-2 font-sans">
      <div
        data-tauri-drag-region
        onDoubleClick={() => invoke("open_settings")}
        // O menu do WebKit não tem nada de útil aqui e quebra a ilusão de
        // que isto é só a letra na tela.
        onContextMenu={(e) => e.preventDefault()}
        style={{
          background: rgba(settings.backgroundColor, settings.backgroundOpacity),
          borderRadius: `${settings.cornerRadius}px`,
          fontFamily: settings.fontFamily || undefined,
          backdropFilter: settings.blur ? "blur(14px)" : undefined,
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
