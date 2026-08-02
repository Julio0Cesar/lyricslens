import { invoke } from "@tauri-apps/api/core";
import { AnimatePresence, motion } from "motion/react";
import { formatMs, useNowPlaying } from "./media/useNowPlaying";
import { lineAt, lineProgress, useLyrics } from "./lyrics/useLyrics";
import { useSettings, type Settings } from "./settings/useSettings";

/** `#0a0a0e` + 0.55 → `rgba(10, 10, 14, 0.55)`. */
function rgba(hex: string, alpha: number): string {
  const limpo = hex.replace("#", "").slice(0, 6);
  if (limpo.length !== 6) return `rgba(0, 0, 0, ${alpha})`;
  const n = parseInt(limpo, 16);
  return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${alpha})`;
}

/** A linha em foco, com a varredura de karaokê. */
function CurrentLine({
  text,
  progress,
  settings,
}: {
  text: string;
  progress: number;
  settings: Settings;
}) {
  const estilo = {
    fontSize: `${settings.fontSize}px`,
    fontWeight: settings.fontWeight,
  };

  if (!text) {
    // Linha vazia no LRC é o intervalo instrumental: a tela limpa é a resposta
    // certa, não um bug.
    return <div style={{ height: `${settings.fontSize * 1.3}px` }} />;
  }

  if (!settings.karaoke) {
    return (
      <div style={{ ...estilo, color: settings.textColor }} className="leading-tight">
        {text}
      </div>
    );
  }

  const cut = Math.floor(text.length * progress);
  return (
    <div style={estilo} className="leading-tight">
      <span style={{ color: settings.textColor }}>{text.slice(0, cut)}</span>
      <span style={{ color: settings.dimColor }}>{text.slice(cut)}</span>
    </div>
  );
}

function App() {
  const { track, state, positionMs } = useNowPlaying();
  const identity = track ? `${track.artist}|${track.title}` : null;
  const { status, lyrics } = useLyrics(identity);
  const { settings } = useSettings();

  if (!settings) return null;

  const lines = lyrics?.lines ?? [];
  const idx = lineAt(lines, positionMs);
  const progress = lineProgress(lines, idx, positionMs);

  const painel = {
    background: rgba(settings.backgroundColor, settings.backgroundOpacity),
    borderRadius: `${settings.cornerRadius}px`,
    fontFamily: settings.fontFamily || undefined,
    backdropFilter: settings.blur ? "blur(14px)" : undefined,
    textAlign: settings.textAlign,
  } as const;

  const contexto = {
    color: settings.dimColor,
    fontSize: `${Math.round(settings.fontSize * 0.58)}px`,
  };

  return (
    <div className="flex h-full flex-col p-2 font-sans">
      <div
        data-tauri-drag-region
        style={painel}
        className="flex flex-1 cursor-grab flex-col justify-center gap-1 border border-white/8 px-5 py-4"
      >
        {track ? (
          <>
            {settings.showTrackInfo && (
              <div
                className="flex items-center gap-2 truncate text-[10px] tracking-[0.1em] uppercase"
                style={{ color: settings.dimColor, justifyContent: settings.textAlign === "center" ? "center" : undefined }}
              >
                <span className="rounded bg-white/10 px-1.5 py-0.5">{track.source}</span>
                <span className="truncate">
                  {track.artist} — {track.title}
                </span>
              </div>
            )}

            <div className="flex flex-col justify-center gap-0.5 py-1">
              {lines.length > 0 ? (
                <>
                  {settings.showContextLines && (
                    <div className="truncate" style={contexto}>
                      {lines[idx - 1]?.text ?? ""}
                    </div>
                  )}
                  <AnimatePresence mode="popLayout">
                    <motion.div
                      key={idx}
                      initial={{ opacity: 0, y: 4 }}
                      animate={{ opacity: 1, y: 0 }}
                      exit={{ opacity: 0, y: -4 }}
                      transition={{ duration: 0.18, ease: "easeOut" }}
                    >
                      <CurrentLine
                        text={lines[idx]?.text ?? ""}
                        progress={progress}
                        settings={settings}
                      />
                    </motion.div>
                  </AnimatePresence>
                  {settings.showContextLines && (
                    <div className="truncate" style={contexto}>
                      {lines[idx + 1]?.text ?? ""}
                    </div>
                  )}
                </>
              ) : (
                <div
                  className="flex items-center gap-3 text-sm"
                  style={{ color: settings.dimColor }}
                >
                  <span>
                    {status === "searching" && "procurando a letra…"}
                    {status === "notFound" && "sem letra para esta faixa"}
                    {status === "found" && lyrics?.instrumental && "instrumental"}
                    {status === "found" &&
                      !lyrics?.instrumental &&
                      lyrics?.plain &&
                      "letra encontrada, mas sem sincronia"}
                    {status === "idle" && "…"}
                  </span>
                  {(status === "notFound" || status === "found") && (
                    <button
                      onClick={() => invoke("open_settings")}
                      className="rounded border border-white/15 px-2 py-0.5 text-[11px] hover:bg-white/10"
                    >
                      escolher letra
                    </button>
                  )}
                </div>
              )}
            </div>

            {settings.showProgress && (
              <div className="flex items-center gap-2">
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
          </>
        ) : (
          <div className="text-sm" style={{ color: settings.dimColor }}>
            {state === "stopped"
              ? "nada tocando — abra o Spotify, YouTube ou qualquer player"
              : "…"}
          </div>
        )}
      </div>
    </div>
  );
}

export default App;
