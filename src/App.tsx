import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion } from "motion/react";

// SPIKE — descartável. Serve para avaliar o overlay no Hyprland.

type Environment = {
  session_type: string;
  desktop: string;
  wayland_display: string;
  scale_factor: number;
  outer_size: [number, number];
  is_decorated: boolean;
};

const FAKE_LINE = "we're no strangers to love, you know the rules and so do i";

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
      className="rounded-md border border-white/15 bg-white/6 px-2 py-1 text-white transition-colors hover:bg-white/12"
    >
      {children}
    </button>
  );
}

function App() {
  const [env, setEnv] = useState<Environment | null>(null);
  const [log, setLog] = useState<string[]>([]);
  const [onTop, setOnTop] = useState(true);
  const [clickThrough, setClickThrough] = useState(false);
  const [opacity, setOpacity] = useState(0.45);
  const [progress, setProgress] = useState(0);

  const push = (msg: string) => setLog((l) => [msg, ...l].slice(0, 5));

  useEffect(() => {
    invoke<Environment>("probe_environment").then(setEnv).catch(String);
  }, []);

  // Simula o karaokê: varre a linha em 6s, como faria a engine de sync.
  useEffect(() => {
    let raf = 0;
    const start = performance.now();
    const tick = (now: number) => {
      setProgress(((now - start) / 6000) % 1);
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, []);

  async function toggleClickThrough() {
    const next = !clickThrough;
    await invoke("set_click_through", { enabled: next });
    setClickThrough(next);
    push(next ? "click-through ON (volta em 8s)" : "click-through OFF");

    // Sem isto o spike se tranca: com click-through ativo não dá para clicar
    // no botão que o desliga.
    if (next) {
      setTimeout(async () => {
        await invoke("set_click_through", { enabled: false });
        setClickThrough(false);
        push("click-through OFF (auto)");
      }, 8000);
    }
  }

  async function toggleOnTop() {
    const next = !onTop;
    await invoke("set_always_on_top", { enabled: next });
    setOnTop(next);
    push(`always-on-top -> ${next} (Ok != funcionou)`);
  }

  async function applyHyprRules() {
    try {
      push(await invoke<string>("apply_hyprland_rules"));
    } catch (e) {
      push(String(e));
    }
  }

  const cut = Math.floor(FAKE_LINE.length * progress);

  return (
    <div className="flex h-full flex-col gap-2 p-2 font-sans">
      <motion.div
        data-tauri-drag-region
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.35, ease: "easeOut" }}
        style={{ background: `rgba(10, 10, 14, ${opacity})` }}
        className="flex flex-1 cursor-grab flex-col justify-center gap-1.5 rounded-2xl border border-white/8 px-5 py-3.5 backdrop-blur-lg"
      >
        <div className="text-[11px] tracking-[0.08em] text-white/45 uppercase">
          Linkin Park — Numb
        </div>
        <div data-tauri-drag-region className="text-2xl leading-tight font-semibold">
          <span className="text-white">{FAKE_LINE.slice(0, cut)}</span>
          <span className="text-white/30">{FAKE_LINE.slice(cut)}</span>
        </div>
        <div className="text-[15px] text-white/25">
          a próxima linha entraria aqui, mais apagada
        </div>
      </motion.div>

      <div className="flex flex-col gap-1.5 rounded-xl bg-black/75 px-2.5 py-2 text-[11px] text-white/70">
        <div className="flex flex-wrap items-center gap-1.5">
          <Button onClick={toggleOnTop}>always-on-top: {String(onTop)}</Button>
          <Button onClick={toggleClickThrough}>
            click-through: {String(clickThrough)}
          </Button>
          <Button onClick={applyHyprRules}>aplicar regras hyprland</Button>
          <label className="flex items-center gap-1.5">
            opacidade
            <input
              type="range"
              min={0}
              max={1}
              step={0.05}
              value={opacity}
              onChange={(e) => setOpacity(Number(e.currentTarget.value))}
              className="w-24"
            />
          </label>
        </div>

        {env && (
          <div className="font-mono text-[10px] text-white/45">
            {env.session_type} · {env.desktop} · {env.wayland_display} · scale{" "}
            {env.scale_factor} · {env.outer_size[0]}x{env.outer_size[1]} ·
            decorada: {String(env.is_decorated)}
          </div>
        )}

        <div className="font-mono text-[10px] whitespace-pre-wrap text-white/45">
          {log.map((l, i) => (
            <div key={i}>{l}</div>
          ))}
        </div>
      </div>
    </div>
  );
}

export default App;
