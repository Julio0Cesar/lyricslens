import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { PASSO_GROSSO_MS, useNowPlaying } from "../media/useNowPlaying";
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
type Pinned = {
  trackKey: string;
  artist: string;
  title: string;
  synced: boolean;
};

export default function LyricsPicker() {
  // A janela de configurações não desenha letra, então não precisa da posição
  // fina — só de saber qual faixa está tocando.
  const { track } = useNowPlaying(PASSO_GROSSO_MS);
  const identity = track ? `${track.artist}|${track.title}` : null;
  const { status, lyrics, trackKey } = useLyrics(identity);

  const [artist, setArtist] = useState("");
  const [title, setTitle] = useState("");
  const [candidatos, setCandidatos] = useState<Candidate[] | null>(null);
  const [buscando, setBuscando] = useState(false);
  const [erro, setErro] = useState<string | null>(null);
  const [aplicando, setAplicando] = useState<string | null>(null);
  const [fixada, setFixada] = useState(false);
  const [fixadas, setFixadas] = useState<Pinned[]>([]);

  const recarregarFixadas = () =>
    invoke<Pinned[]>("pinned_lyrics").then(setFixadas).catch(() => {});

  useEffect(() => {
    recarregarFixadas();
  }, []);

  // Só faz sentido perguntar depois de a letra ter sido resolvida: antes disso
  // não há nada no cache para estar fixado.
  useEffect(() => {
    if (!trackKey || status !== "found") {
      setFixada(false);
      return;
    }
    invoke<boolean>("is_pinned", { trackKey }).then(setFixada).catch(() => setFixada(false));
  }, [trackKey, status]);

  async function alternarFixada(chave: string, valor: boolean) {
    setErro(null);
    try {
      const aplicado = await invoke<boolean>("pin_lyrics", {
        trackKey: chave,
        pinned: valor,
      });
      if (chave === trackKey) setFixada(aplicado);
      await recarregarFixadas();
    } catch (e) {
      setErro(String(e));
    }
  }

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
          {fixada && (
            <span
              className="ml-1.5 text-[11px] text-emerald-400/80"
              title="guardada para uso offline"
            >
              ● offline
            </span>
          )}
        </span>
        <span className="shrink-0 text-[11px] text-white/40">{resumo}</span>
      </div>

      <div className="flex items-center justify-between gap-3">
        <span className="text-[11px] leading-relaxed text-white/35">
          O cache já evita ir à rede de novo. Manter offline é o passo a mais:
          esta nunca é descartada.
        </span>
        <button
          onClick={() => trackKey && alternarFixada(trackKey, !fixada)}
          disabled={!trackKey || status !== "found"}
          className={`shrink-0 rounded border px-2.5 py-1 text-[11px] disabled:opacity-40 ${
            fixada
              ? "border-emerald-400/30 bg-emerald-400/10 text-emerald-300/90 hover:bg-emerald-400/16"
              : "border-white/15 bg-white/6 text-white/70 hover:bg-white/12"
          }`}
        >
          {fixada ? "manter offline ✓" : "manter offline"}
        </button>
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

      {fixadas.length > 0 && (
        <div className="flex flex-col gap-1 border-t border-white/8 pt-3">
          <div className="flex items-baseline justify-between">
            <span className="text-[12px] text-white/70">Guardadas offline</span>
            <span className="text-[11px] text-white/35">{fixadas.length}</span>
          </div>
          <div className="flex max-h-40 flex-col gap-1 overflow-y-auto">
            {fixadas.map((p) => (
              <div
                key={p.trackKey}
                className="flex items-center gap-2 rounded border border-white/8 bg-white/4 px-2.5 py-1.5"
              >
                <div className="min-w-0 flex-1">
                  <div className="truncate text-[12px] text-white/85">{p.title}</div>
                  <div className="truncate text-[11px] text-white/40">{p.artist}</div>
                </div>
                <span
                  className={`shrink-0 text-[10px] ${
                    p.synced ? "text-emerald-400/60" : "text-amber-400/50"
                  }`}
                  title={p.synced ? "sincronizada" : "sem sincronia"}
                >
                  {p.synced ? "sync" : "texto"}
                </span>
                <button
                  onClick={() => alternarFixada(p.trackKey, false)}
                  className="shrink-0 rounded border border-white/15 bg-white/8 px-2 py-0.5 text-[11px] text-white/70 hover:bg-white/14"
                >
                  soltar
                </button>
              </div>
            ))}
          </div>
        </div>
      )}
    </Section>
  );
}
