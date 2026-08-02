import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type Track = {
  title: string;
  artist: string;
  album: string;
  durationMs: number | null;
  artUrl: string | null;
  url: string | null;
  source: string;
};

export type PlaybackState = "playing" | "paused" | "stopped";

type MediaEvent =
  | { kind: "trackChanged"; track: Track }
  | { kind: "playbackChanged"; state: PlaybackState }
  | { kind: "positionAnchored"; positionMs: number; confidence: "edge" | "sample" }
  | { kind: "seeked"; positionMs: number }
  | { kind: "gone" };

type Snapshot = {
  track: Track | null;
  state: PlaybackState;
  positionMs: number;
};

/** Âncora: uma posição real e o instante local em que ela valia. */
type Anchor = { positionMs: number; at: number };

export type NowPlaying = {
  track: Track | null;
  state: PlaybackState;
  /** Posição interpolada, atualizada a cada quadro. */
  positionMs: number;
  /** Quão precisa foi a última âncora recebida do backend. */
  confidence: "edge" | "sample" | null;
};

/**
 * O backend manda âncoras esparsas; aqui a posição é extrapolada a 60fps.
 * Nenhum `setInterval` chamando o Rust — é isso que mantém o custo em ~zero.
 */
export function useNowPlaying(): NowPlaying {
  const [track, setTrack] = useState<Track | null>(null);
  const [state, setState] = useState<PlaybackState>("stopped");
  const [confidence, setConfidence] = useState<"edge" | "sample" | null>(null);
  const [positionMs, setPositionMs] = useState(0);

  const anchor = useRef<Anchor>({ positionMs: 0, at: performance.now() });
  const playing = useRef(false);

  useEffect(() => {
    invoke<Snapshot>("now_playing").then((s) => {
      setTrack(s.track);
      setState(s.state);
      playing.current = s.state === "playing";
      anchor.current = { positionMs: s.positionMs, at: performance.now() };
    });

    const un = listen<MediaEvent>("media", ({ payload }) => {
      switch (payload.kind) {
        case "trackChanged":
          setTrack(payload.track);
          setConfidence(null);
          anchor.current = { positionMs: 0, at: performance.now() };
          break;

        case "playbackChanged": {
          // Congela onde está antes de trocar o estado, senão o tempo parado
          // conta como tempo tocado.
          const now = performance.now();
          anchor.current = { positionMs: estimate(anchor.current, playing.current), at: now };
          playing.current = payload.state === "playing";
          setState(payload.state);
          break;
        }

        case "positionAnchored":
          if (setAnchor(anchor, payload.positionMs)) setConfidence(payload.confidence);
          break;

        case "seeked":
          setAnchor(anchor, payload.positionMs);
          break;

        case "gone":
          setTrack(null);
          setState("stopped");
          setConfidence(null);
          playing.current = false;
          anchor.current = { positionMs: 0, at: performance.now() };
          break;
      }
    });

    return () => {
      un.then((f) => f());
    };
  }, []);

  useEffect(() => {
    let raf = 0;
    const frame = () => {
      setPositionMs(estimate(anchor.current, playing.current));
      raf = requestAnimationFrame(frame);
    };
    raf = requestAnimationFrame(frame);
    return () => cancelAnimationFrame(raf);
  }, []);

  return { track, state, positionMs, confidence };
}

function estimate(a: Anchor, playing: boolean): number {
  return playing ? a.positionMs + (performance.now() - a.at) : a.positionMs;
}

/**
 * Uma âncora NaN envenena o relógio em silêncio: tudo que for somado a ela
 * vira NaN e a UI só mostra `NaN:NaN`. Melhor recusar a âncora e continuar
 * extrapolando da última boa.
 */
function setAnchor(ref: { current: Anchor }, positionMs: unknown): boolean {
  if (typeof positionMs !== "number" || !Number.isFinite(positionMs)) {
    console.warn("[media] âncora de posição inválida, ignorada:", positionMs);
    return false;
  }
  ref.current = { positionMs, at: performance.now() };
  return true;
}

export function formatMs(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m}:${String(s).padStart(2, "0")}`;
}
