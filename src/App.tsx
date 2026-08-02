import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AnimatePresence, motion } from "motion/react";
import { formatMs, useNowPlaying } from "./media/useNowPlaying";

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

function App() {
  const { track, state, positionMs, confidence } = useNowPlaying();
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

  const progress = track?.durationMs
    ? Math.min(1, Math.max(0, positionMs / track.durationMs))
    : 0;

  return (
    <div className="flex h-full flex-col gap-2 p-2 font-sans">
      <div
        data-tauri-drag-region
        className="flex flex-1 cursor-grab flex-col justify-center gap-2 rounded-2xl border border-white/8 bg-[rgba(10,10,14,0.55)] px-5 py-4 backdrop-blur-lg"
      >
        <AnimatePresence mode="wait">
          {track ? (
            <motion.div
              key={track.title + track.artist}
              initial={{ opacity: 0, y: 6 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -6 }}
              transition={{ duration: 0.25, ease: "easeOut" }}
              className="flex flex-col gap-1"
            >
              <div className="flex items-center gap-2 text-[10px] tracking-[0.1em] text-white/40 uppercase">
                <span className="rounded bg-white/10 px-1.5 py-0.5">
                  {track.source}
                </span>
                <span>{state === "playing" ? "tocando" : state}</span>
                {confidence && (
                  <span
                    title="precisão da última âncora de posição"
                    className={
                      confidence === "edge" ? "text-emerald-400/70" : "text-amber-400/70"
                    }
                  >
                    ● {confidence}
                  </span>
                )}
              </div>

              <div
                data-tauri-drag-region
                className="truncate text-2xl leading-tight font-semibold text-white"
              >
                {track.title}
              </div>
              <div className="truncate text-sm text-white/55">
                {track.artist}
                {track.album && <span className="text-white/30"> · {track.album}</span>}
              </div>
            </motion.div>
          ) : (
            <motion.div
              key="vazio"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              className="text-sm text-white/40"
            >
              nada tocando — abra o Spotify, YouTube ou qualquer player
            </motion.div>
          )}
        </AnimatePresence>

        {track && (
          <div className="flex items-center gap-2">
            <span className="font-mono text-[10px] text-white/45 tabular-nums">
              {formatMs(positionMs)}
            </span>
            <div className="h-[3px] flex-1 overflow-hidden rounded-full bg-white/10">
              <div
                className="h-full rounded-full bg-white/60"
                style={{ width: `${progress * 100}%` }}
              />
            </div>
            <span className="font-mono text-[10px] text-white/45 tabular-nums">
              {track.durationMs ? formatMs(track.durationMs) : "--:--"}
            </span>
          </div>
        )}
      </div>

      <div className="flex flex-wrap items-center gap-1.5 rounded-xl bg-black/70 px-2.5 py-2 text-[11px] text-white/60">
        <Button onClick={toggleClickThrough}>
          click-through: {String(clickThrough)}
        </Button>
        <Button onClick={() => invoke("apply_hyprland_rules")}>
          flutuar + fixar
        </Button>
        <span className="ml-auto font-mono text-[10px] text-white/35">
          fase 1 · detecção
        </span>
      </div>
    </div>
  );
}

export default App;
