import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AnimatePresence, motion } from "motion/react";
import { formatMs, useNowPlaying } from "./media/useNowPlaying";
import { lineAt, lineProgress, useLyrics } from "./lyrics/useLyrics";

function Button({
  onClick,
  children,
}: {
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className="rounded-md border border-white/15 bg-white/6 px-2 py-1 text-white/80 transition-colors hover:bg-white/12"
    >
      {children}
    </button>
  );
}

/** A linha em foco, com a varredura de karaokê. */
function CurrentLine({ text, progress }: { text: string; progress: number }) {
  if (!text) {
    // Linha vazia no LRC é o intervalo instrumental: a tela limpa é a resposta
    // certa, não um bug.
    return <div className="h-9" />;
  }

  const cut = Math.floor(text.length * progress);
  return (
    <div className="text-[26px] leading-tight font-semibold">
      <span className="text-white">{text.slice(0, cut)}</span>
      <span className="text-white/30">{text.slice(cut)}</span>
    </div>
  );
}

function App() {
  const { track, state, positionMs, confidence } = useNowPlaying();
  const identity = track ? `${track.artist}|${track.title}` : null;
  const { status, lyrics } = useLyrics(identity);
  const [clickThrough, setClickThrough] = useState(false);

  async function toggleClickThrough() {
    const next = !clickThrough;
    await invoke("set_click_through", { enabled: next });
    setClickThrough(next);
    // Sem isto a janela se tranca: com click-through ativo não dá para clicar
    // no botão que o desliga. Some quando houver atalho global.
    if (next) {
      setTimeout(async () => {
        await invoke("set_click_through", { enabled: false });
        setClickThrough(false);
      }, 8000);
    }
  }

  const lines = lyrics?.lines ?? [];
  const idx = lineAt(lines, positionMs);
  const progress = lineProgress(lines, idx, positionMs);

  return (
    <div className="flex h-full flex-col gap-2 p-2 font-sans">
      <div
        data-tauri-drag-region
        className="flex flex-1 cursor-grab flex-col justify-center gap-1 rounded-2xl border border-white/8 bg-[rgba(10,10,14,0.55)] px-5 py-4 backdrop-blur-lg"
      >
        {track ? (
          <>
            <div className="flex items-center gap-2 text-[10px] tracking-[0.1em] text-white/40 uppercase">
              <span className="rounded bg-white/10 px-1.5 py-0.5">{track.source}</span>
              <span className="truncate">
                {track.artist} — {track.title}
              </span>
              {confidence === "edge" && <span className="text-emerald-400/60">●</span>}
            </div>

            <div className="min-h-[104px] py-1">
              {lines.length > 0 ? (
                <div className="flex flex-col gap-0.5">
                  <div className="truncate text-[15px] text-white/25">
                    {lines[idx - 1]?.text ?? ""}
                  </div>
                  <AnimatePresence mode="popLayout">
                    <motion.div
                      key={idx}
                      initial={{ opacity: 0, y: 4 }}
                      animate={{ opacity: 1, y: 0 }}
                      exit={{ opacity: 0, y: -4 }}
                      transition={{ duration: 0.18, ease: "easeOut" }}
                    >
                      <CurrentLine text={lines[idx]?.text ?? ""} progress={progress} />
                    </motion.div>
                  </AnimatePresence>
                  <div className="truncate text-[15px] text-white/25">
                    {lines[idx + 1]?.text ?? ""}
                  </div>
                </div>
              ) : (
                <div className="flex h-full items-center text-sm text-white/40">
                  {status === "searching" && "procurando a letra…"}
                  {status === "notFound" && "sem letra para esta faixa"}
                  {status === "found" && lyrics?.instrumental && "instrumental"}
                  {status === "found" &&
                    !lyrics?.instrumental &&
                    lyrics?.plain &&
                    "letra encontrada, mas sem sincronia"}
                  {status === "idle" && "…"}
                </div>
              )}
            </div>

            <div className="flex items-center gap-2">
              <span className="font-mono text-[10px] text-white/45 tabular-nums">
                {formatMs(positionMs)}
              </span>
              <div className="h-[3px] flex-1 overflow-hidden rounded-full bg-white/10">
                <div
                  className="h-full rounded-full bg-white/60"
                  style={{
                    width: `${
                      track.durationMs
                        ? Math.min(100, Math.max(0, (positionMs / track.durationMs) * 100))
                        : 0
                    }%`,
                  }}
                />
              </div>
              <span className="font-mono text-[10px] text-white/45 tabular-nums">
                {track.durationMs ? formatMs(track.durationMs) : "--:--"}
              </span>
            </div>
          </>
        ) : (
          <div className="text-sm text-white/40">
            nada tocando — abra o Spotify, YouTube ou qualquer player
          </div>
        )}
      </div>

      <div className="flex flex-wrap items-center gap-1.5 rounded-xl bg-black/70 px-2.5 py-2 text-[11px] text-white/60">
        <Button onClick={toggleClickThrough}>
          click-through: {String(clickThrough)}
        </Button>
        <Button onClick={() => invoke("apply_hyprland_rules")}>flutuar + fixar</Button>
        <span className="ml-auto font-mono text-[10px] text-white/35">
          {state} · {lyrics ? `${lines.length} linhas · ${lyrics.source}` : status}
        </span>
      </div>
    </div>
  );
}

export default App;
