import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNowPlaying } from "../media/useNowPlaying";
import { useLyrics } from "../lyrics/useLyrics";
import { Section } from "./controls";

type Candidate = {
  providerId: string;
  title: string;
  artist: string;
  album: string;
  durationS: number | null;
  hasSynced: boolean;
};

function duracao(s: number | null): string {
  if (s === null) return "--:--";
  const t = Math.round(s);
  return `${Math.floor(t / 60)}:${String(t % 60).padStart(2, "0")}`;
}

/**
 * O resgate para quando a busca automática erra — o caso do título de YouTube
 * bagunçado. A escolha vai para o cache sob a chave da faixa, então da próxima
 * vez que a mesma música tocar ela já vem certa.
 */
export default function LyricsPicker() {
  const { track } = useNowPlaying();
  const identity = track ? `${track.artist}|${track.title}` : null;
  const { status, lyrics } = useLyrics(identity);

  const [artist, setArtist] = useState("");
  const [title, setTitle] = useState("");
  const [candidatos, setCandidatos] = useState<Candidate[] | null>(null);
  const [buscando, setBuscando] = useState(false);
  const [erro, setErro] = useState<string | null>(null);
  const [aplicando, setAplicando] = useState<string | null>(null);

  // Os campos acompanham a faixa até o usuário encostar neles.
  useEffect(() => {
    if (!track) return;
    setArtist(track.artist);
    setTitle(track.title);
    setCandidatos(null);
    setErro(null);
  }, [identity]);

  async function buscar() {
    setBuscando(true);
    setErro(null);
    try {
      setCandidatos(await invoke<Candidate[]>("search_lyrics", { artist, title }));
    } catch (e) {
      setErro(String(e));
      setCandidatos(null);
    } finally {
      setBuscando(false);
    }
  }

  async function usar(c: Candidate) {
    setAplicando(c.providerId);
    setErro(null);
    try {
      await invoke("apply_candidate", { providerId: c.providerId });
      setCandidatos(null);
    } catch (e) {
      setErro(String(e));
    } finally {
      setAplicando(null);
    }
  }

  const resumo = !track
    ? "nada tocando"
    : status === "found" && lyrics?.lines.length
      ? `${lyrics.lines.length} linhas sincronizadas · ${lyrics.source}`
      : status === "found" && lyrics?.instrumental
        ? "instrumental"
        : status === "found"
          ? "letra sem sincronia"
          : status === "notFound"
            ? "nenhuma letra encontrada"
            : status === "searching"
              ? "procurando…"
              : "—";

  return (
    <Section title="Letra desta faixa">
      <div className="flex items-baseline justify-between gap-3">
        <span className="min-w-0 truncate text-sm text-white/85">
          {track ? `${track.artist} — ${track.title}` : "nada tocando"}
        </span>
        <span className="shrink-0 text-[11px] text-white/40">{resumo}</span>
      </div>

      <div className="flex gap-2">
        <input
          value={artist}
          onChange={(e) => setArtist(e.currentTarget.value)}
          placeholder="artista"
          className="min-w-0 flex-1 rounded border border-white/15 bg-white/5 px-2 py-1 text-[12px] text-white/85 placeholder:text-white/25"
        />
        <input
          value={title}
          onChange={(e) => setTitle(e.currentTarget.value)}
          placeholder="música"
          className="min-w-0 flex-1 rounded border border-white/15 bg-white/5 px-2 py-1 text-[12px] text-white/85 placeholder:text-white/25"
        />
        <button
          onClick={buscar}
          disabled={buscando || (!artist && !title)}
          className="shrink-0 rounded border border-white/15 bg-white/8 px-3 py-1 text-[12px] text-white/80 hover:bg-white/14 disabled:opacity-40"
        >
          {buscando ? "…" : "buscar"}
        </button>
      </div>

      {erro && <p className="text-[11px] text-rose-400/80">{erro}</p>}

      {candidatos !== null && (
        <div className="flex max-h-72 flex-col gap-1 overflow-y-auto">
          {candidatos.length === 0 && (
            <p className="text-[12px] text-white/40">nenhum resultado</p>
          )}
          {candidatos.map((c) => (
            <div
              key={c.providerId}
              className="flex items-center gap-3 rounded border border-white/8 bg-white/4 px-2.5 py-1.5"
            >
              <div className="min-w-0 flex-1">
                <div className="truncate text-[12px] text-white/85">{c.title}</div>
                <div className="truncate text-[11px] text-white/40">
                  {c.artist}
                  {c.album && ` · ${c.album}`}
                </div>
              </div>
              <span className="shrink-0 font-mono text-[10px] text-white/35 tabular-nums">
                {duracao(c.durationS)}
              </span>
              <span
                className={`shrink-0 text-[10px] ${
                  c.hasSynced ? "text-emerald-400/70" : "text-amber-400/60"
                }`}
                title={c.hasSynced ? "sincronizada" : "sem sincronia"}
              >
                {c.hasSynced ? "sync" : "texto"}
              </span>
              <button
                onClick={() => usar(c)}
                disabled={aplicando !== null || !track}
                className="shrink-0 rounded border border-white/15 bg-white/8 px-2 py-0.5 text-[11px] text-white/80 hover:bg-white/14 disabled:opacity-40"
              >
                {aplicando === c.providerId ? "…" : "usar"}
              </button>
            </div>
          ))}
        </div>
      )}

      <p className="text-[11px] leading-relaxed text-white/30">
        A escolha fica guardada para esta faixa — da próxima vez que ela tocar, a letra já vem
        certa sem passar por aqui.
      </p>
    </Section>
  );
}
