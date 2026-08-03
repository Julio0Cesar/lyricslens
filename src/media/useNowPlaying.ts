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
 * De quanto em quanto tempo a posição extrapolada vira uma renderização nova.
 *
 * Não é a taxa de quadros: é o passo a partir do qual o resultado na tela
 * *muda*. O karaokê acende palavra por palavra e a transição de cor dura 260ms,
 * então 100ms é imperceptível; o relógio do progresso conta segundos, e para
 * ele 100ms já é sete vezes mais fino que o necessário.
 */
export const PASSO_FINO_MS = 100;

/** Sem karaokê nem barra de progresso, só a troca de linha muda a tela. */
export const PASSO_GROSSO_MS = 400;

/**
 * O backend manda âncoras esparsas; aqui a posição é extrapolada localmente.
 *
 * A extrapolação acompanha os quadros, mas **só vira estado do React quando o
 * valor cruza um passo** — antes ela chamava `setPositionMs` a cada quadro,
 * inclusive com a música pausada e com o karaokê desligado. Como o karaokê é
 * discreto (uma palavra está acesa ou não), a imensa maioria desses 60
 * renders/s produzia DOM idêntico e ainda assim custava uma repintura da
 * superfície inteira — 8% a 25% de um núcleo, escalando com a área da janela.
 * Ver #39.
 */
export function useNowPlaying(passoMs: number = PASSO_FINO_MS): NowPlaying {
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
          if (setAnchor(anchor, payload.positionMs)) {
            setConfidence(payload.confidence);
            // Pausado não há laço rodando para perceber a âncora nova.
            if (!playing.current) setPositionMs(payload.positionMs);
          }
          break;

        case "seeked":
          // Buscar com a música pausada é justamente quando o laço está
          // desligado — sem isto a tela ficaria na posição antiga.
          if (setAnchor(anchor, payload.positionMs) && !playing.current) {
            setPositionMs(payload.positionMs);
          }
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

  const tocando = state === "playing";

  useEffect(() => {
    // Parado, a posição não muda. Extrapolar a cada quadro era trabalho puro:
    // com a música pausada o overlay consumia o mesmo que tocando.
    if (!tocando) {
      setPositionMs(estimate(anchor.current, false));
      return;
    }

    let raf = 0;
    let ultimoPasso = -1;
    const frame = () => {
      const agora = estimate(anchor.current, true);
      const passo = Math.floor(agora / passoMs);
      // Um valor que cai no mesmo passo desenharia exatamente a mesma tela.
      if (passo !== ultimoPasso) {
        ultimoPasso = passo;
        setPositionMs(agora);
      }
      raf = requestAnimationFrame(frame);
    };
    raf = requestAnimationFrame(frame);
    return () => cancelAnimationFrame(raf);
  }, [tocando, passoMs]);

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
