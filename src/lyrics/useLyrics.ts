import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type LyricLine = { atMs: number; text: string };

export type Lyrics = {
  lines: LyricLine[];
  plain: string | null;
  instrumental: boolean;
  source: string;
  providerId: string | null;
  translation: LyricLine[] | null;
};

type LyricsEvent =
  | { status: "searching"; trackKey: string }
  | { status: "found"; trackKey: string; lyrics: Lyrics }
  | { status: "notFound"; trackKey: string };

export type LyricsStatus = "idle" | "searching" | "found" | "notFound";

export function useLyrics(trackIdentity: string | null) {
  const [status, setStatus] = useState<LyricsStatus>("idle");
  const [lyrics, setLyrics] = useState<Lyrics | null>(null);

  useEffect(() => {
    const aplicar = (payload: LyricsEvent) => {
      switch (payload.status) {
        case "searching":
          setStatus("searching");
          setLyrics(null);
          break;
        case "found":
          setStatus("found");
          setLyrics(payload.lyrics);
          break;
        case "notFound":
          setStatus("notFound");
          setLyrics(null);
          break;
      }
    };

    // A busca pode ter terminado antes desta janela existir. Sem recuperar o
    // último resultado, a UI esperaria para sempre por um evento que já passou.
    invoke<LyricsEvent | null>("current_lyrics").then((ev) => {
      if (ev) aplicar(ev);
    });

    const un = listen<LyricsEvent>("lyrics", ({ payload }) => aplicar(payload));
    return () => {
      un.then((f) => f());
    };
  }, []);

  // Trocou de música: o que estava na tela não vale mais, mesmo antes de a
  // busca começar.
  useEffect(() => {
    if (trackIdentity === null) {
      setStatus("idle");
      setLyrics(null);
    }
  }, [trackIdentity]);

  return { status, lyrics };
}

/**
 * Índice da linha que deve estar na tela.
 *
 * `-1` antes da primeira linha — é o silêncio da introdução, em que a tela
 * fica limpa de propósito. Não é erro nem "linha 0".
 */
export function lineAt(lines: LyricLine[], positionMs: number): number {
  if (lines.length === 0 || positionMs < lines[0].atMs) return -1;

  let lo = 0;
  let hi = lines.length - 1;
  while (lo < hi) {
    const mid = Math.ceil((lo + hi) / 2);
    if (lines[mid].atMs <= positionMs) lo = mid;
    else hi = mid - 1;
  }
  return lo;
}

/**
 * Quanto da linha atual já foi cantada, de 0 a 1.
 *
 * É uma aproximação: o LRC marca quando a linha começa, não como ela se
 * distribui no tempo. Varre do início da linha até a próxima, com teto — sem
 * ele, uma linha seguida de trinta segundos de instrumental varreria devagar
 * demais para parecer viva.
 */
const VARREDURA_MAX_MS = 8000;

export function lineProgress(
  lines: LyricLine[],
  index: number,
  positionMs: number,
): number {
  if (index < 0 || index >= lines.length) return 0;

  const inicio = lines[index].atMs;
  const proxima = lines[index + 1]?.atMs ?? inicio + VARREDURA_MAX_MS;
  const duracao = Math.min(proxima - inicio, VARREDURA_MAX_MS);
  if (duracao <= 0) return 1;

  return Math.min(1, Math.max(0, (positionMs - inicio) / duracao));
}
