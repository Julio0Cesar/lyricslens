// @vitest-environment jsdom
/**
 * O overlay montado de verdade.
 *
 * A #9 lista dois defeitos que teriam sido pegos aqui: o hook chamado depois de
 * uma saída antecipada, que derrubou o componente e deixou a janela em branco, e
 * a âncora `NaN` que envenenava o relógio em silêncio. Os dois passaram por
 * `cargo test` e por `tsc` sem uma reclamação — só apareciam com o componente
 * montado.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, render, screen, waitFor } from "@testing-library/react";

import App from "./App";
import type { Settings } from "./settings/useSettings";
import type { Lyrics } from "./lyrics/useLyrics";
import { casaLinha, emitir, limparTauri, responder } from "./testes/tauri-falso";

const PREFERENCIAS: Settings = {
  fontFamily: "",
  fontSize: 26,
  fontWeight: 600,
  textColor: "#ffffff",
  dimColor: "#ffffff4d",
  backgroundColor: "#0a0a0e",
  backgroundOpacity: 0.55,
  blur: true,
  cornerRadius: 16,
  showContextLines: true,
  showTrackInfo: false,
  showProgress: false,
  showCover: false,
  textAlign: "left",
  karaoke: true,
  clickThrough: false,
  hideWhenPaused: false,
  syncOffsetMs: 0,
  hotkey: "",
  language: "pt-BR",
  width: 780,
  height: 160,
  marginBottom: 80,
  positionX: null,
  positionY: null,
  layerMarginLeft: null,
  layerShell: false,
};

const FAIXA = {
  title: "Creep",
  artist: "Radiohead",
  album: "Pablo Honey",
  durationMs: 238_000,
  artUrl: null,
  url: null,
  source: "spotify",
};

function comLetra(linhas: [number, string][]): Lyrics {
  return {
    lines: linhas.map(([atMs, text]) => ({ atMs, text })),
    plain: null,
    instrumental: false,
    source: "lrclib",
    providerId: "496",
    translation: null,
  };
}

/** O cenário base: preferências carregadas, nada tocando, sem letra. */
function cenario(ajustes: Partial<Settings> = {}) {
  responder("get_settings", { ...PREFERENCIAS, ...ajustes });
  responder("overlay_status", {
    layerShellRequested: false,
    layerShellActive: false,
    layerShellFallback: null,
  });
  responder("system_language", "pt-BR");
  responder("last_problem", null);
  responder("now_playing", { track: null, state: "stopped", positionMs: 0 });
  responder("current_lyrics", null);
  responder("current_cover", null);
  responder("open_settings", null);
  responder("remember_overlay_position", null);
}

/** Monta e espera as preferências chegarem — antes disso é tela vazia. */
async function montar() {
  render(<App />);
  await waitFor(() => expect(document.body.textContent).not.toBe(""));
}

beforeEach(() => cenario());
afterEach(() => {
  limparTauri();
  vi.useRealTimers();
});

describe("avisos do overlay", () => {
  it("sem faixa, diz que não há nada tocando", async () => {
    await montar();
    expect(await screen.findByText("nada tocando")).toBeInTheDocument();
  });

  it("faixa sem letra encontrada", async () => {
    await montar();
    act(() => {
      emitir("media", { kind: "trackChanged", track: FAIXA });
      emitir("lyrics", { status: "notFound", trackKey: "radiohead|creep" });
    });
    expect(await screen.findByText("sem letra para esta faixa")).toBeInTheDocument();
  });

  it("faixa instrumental", async () => {
    await montar();
    act(() => {
      emitir("media", { kind: "trackChanged", track: FAIXA });
      emitir("lyrics", {
        status: "found",
        trackKey: "radiohead|creep",
        lyrics: { ...comLetra([]), instrumental: true },
      });
    });
    expect(await screen.findByText("faixa instrumental")).toBeInTheDocument();
  });

  it("letra encontrada mas sem marcação de tempo", async () => {
    await montar();
    act(() => {
      emitir("media", { kind: "trackChanged", track: FAIXA });
      emitir("lyrics", {
        status: "found",
        trackKey: "radiohead|creep",
        lyrics: { ...comLetra([]), plain: "When you were here before" },
      });
    });
    expect(await screen.findByText("letra sem sincronia")).toBeInTheDocument();
  });

  it("com letra sincronizada, mostra a letra e nenhum aviso", async () => {
    await montar();
    act(() => {
      emitir("media", { kind: "trackChanged", track: FAIXA });
      emitir("lyrics", {
        status: "found",
        trackKey: "radiohead|creep",
        lyrics: comLetra([
          [0, "When you were here before"],
          [5000, "Couldn't look you in the eye"],
        ]),
      });
    });

    expect(
      await screen.findByText(casaLinha("When you were here before")),
    ).toBeInTheDocument();
    expect(screen.queryByText("nada tocando")).not.toBeInTheDocument();
    expect(screen.queryByText("sem letra para esta faixa")).not.toBeInTheDocument();
  });
});

/**
 * O aviso de busca é atrasado de propósito: aparecer e sumir numa busca de
 * 300ms é mais perturbador que o intervalo em branco. Sem teste, o atraso é
 * fácil de remover sem ninguém perceber.
 */
describe("aviso de busca demorada", () => {
  it("não pisca numa busca rápida", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    await montar();

    act(() => {
      emitir("media", { kind: "trackChanged", track: FAIXA });
      emitir("lyrics", { status: "searching", trackKey: "radiohead|creep" });
    });
    act(() => void vi.advanceTimersByTime(300));
    expect(screen.queryByText("procurando a letra…")).not.toBeInTheDocument();
  });

  it("aparece quando a busca demora", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    await montar();

    act(() => {
      emitir("media", { kind: "trackChanged", track: FAIXA });
      emitir("lyrics", { status: "searching", trackKey: "radiohead|creep" });
    });
    act(() => void vi.advanceTimersByTime(800));
    expect(screen.getByText("procurando a letra…")).toBeInTheDocument();
  });
});

/**
 * O defeito: um `return` no meio do componente, antes de todos os hooks, fazia
 * o React ver quantidades diferentes de hooks entre a renderização sem
 * preferências e a com — e derrubava o componente inteiro. A janela ficava em
 * branco e nada no console dizia por quê.
 */
describe("chegada das preferências", () => {
  it("não desenha nada antes de elas chegarem", () => {
    const { container } = render(<App />);
    expect(container).toBeEmptyDOMElement();
  });

  it("sobrevive à transição de sem preferências para com preferências", async () => {
    const { container } = render(<App />);
    expect(container).toBeEmptyDOMElement();
    await waitFor(() => expect(container).not.toBeEmptyDOMElement());
    expect(await screen.findByText("nada tocando")).toBeInTheDocument();
  });

  it("reage a preferências trocadas em tempo de execução", async () => {
    await montar();
    act(() => emitir("settings", { ...PREFERENCIAS, showTrackInfo: true }));
    act(() => emitir("media", { kind: "trackChanged", track: FAIXA }));

    // Com a informação de faixa ligada, a fonte e o título aparecem.
    expect(await screen.findByText("spotify")).toBeInTheDocument();
  });
});

/**
 * A capa é enfeite, e enfeite errado é pior que enfeite nenhum: capa de outro
 * disco faz parecer que o app se confundiu de música, não de imagem.
 */
describe("capa do álbum", () => {
  const CAPA = "https://exemplo/capa.jpg";

  async function comCapa(ajustes: Partial<Settings>) {
    responder("get_settings", { ...PREFERENCIAS, ...ajustes });
    await montar();
    act(() => {
      emitir("media", { kind: "trackChanged", track: FAIXA });
      emitir("lyrics", {
        status: "found",
        trackKey: "radiohead|creep",
        lyrics: comLetra([[0, "When you were here before"]]),
      });
    });
  }

  it("desligada, não aparece nem quando existe", async () => {
    await comCapa({ showCover: false });
    act(() => emitir("cover", { trackKey: "radiohead|creep", url: CAPA }));
    expect(document.querySelector("img")).toBeNull();
  });

  it("ligada, aparece quando o backend acha", async () => {
    await comCapa({ showCover: true });
    act(() => emitir("cover", { trackKey: "radiohead|creep", url: CAPA }));
    await waitFor(() => expect(document.querySelector("img")).not.toBeNull());
    expect(document.querySelector("img")).toHaveAttribute("src", CAPA);
  });

  it("ligada mas sem capa encontrada, não deixa buraco na tela", async () => {
    await comCapa({ showCover: true });
    act(() => emitir("cover", { trackKey: "radiohead|creep", url: null }));
    expect(document.querySelector("img")).toBeNull();
  });

  // Uma busca lenta pode responder depois da troca de música. A capa do disco
  // anterior sobre a letra do novo é o defeito que mais parece bug de faixa.
  it("ignora a capa que chega para outra faixa", async () => {
    await comCapa({ showCover: true });
    act(() => emitir("cover", { trackKey: "outra|musica", url: CAPA }));
    expect(document.querySelector("img")).toBeNull();
  });
});

describe("idioma", () => {
  it("com o sistema em inglês, os avisos saem em inglês", async () => {
    responder("get_settings", { ...PREFERENCIAS, language: "auto" });
    responder("system_language", "en");
    await montar();
    expect(await screen.findByText("nothing playing")).toBeInTheDocument();
  });

  it("a escolha do usuário ganha do sistema", async () => {
    responder("get_settings", { ...PREFERENCIAS, language: "pt-BR" });
    responder("system_language", "en");
    await montar();
    expect(await screen.findByText("nada tocando")).toBeInTheDocument();
  });
});
