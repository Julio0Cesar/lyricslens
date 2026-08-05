// @vitest-environment jsdom
/**
 * A janela de configurações montada de verdade.
 *
 * É o maior componente do app e o único que ainda não tinha teste nenhum. O que
 * se verifica aqui não é a aparência, é a regra que a #18 e a #35 introduziram:
 * um controle só aparece quando o compositor sabe obedecer a ele. Um botão que
 * não faz nada é pior que botão nenhum — manda o usuário atrás de um defeito no
 * lugar errado, que foi exatamente o que a #38 registrou.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import SettingsWindow from "./SettingsWindow";
import type { OverlayStatus, Settings } from "./useSettings";
import { chamadas, limparTauri, responder } from "../testes/tauri-falso";

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

const STATUS: OverlayStatus = {
  layerShellRequested: false,
  layerShellActive: false,
  layerShellFallback: null,
  blurAvailable: true,
};

/** O cenário mínimo para a janela montar sem explodir num `useEffect`. */
function cenario(status: Partial<OverlayStatus> = {}, prefs: Partial<Settings> = {}) {
  responder("get_settings", { ...PREFERENCIAS, ...prefs });
  responder("overlay_status", { ...STATUS, ...status });
  responder("system_language", "pt-BR");
  responder("last_problem", null);
  responder("autostart_enabled", false);
  responder("list_fonts", []);
  responder("cache_stats", { tracks: 0, synced: 0, pinned: 0, misses: 0 });
  responder("check_update", {
    current: "0.10.1",
    latest: null,
    available: false,
    canApply: false,
  });
  responder("save_settings", (args: Record<string, unknown>) => args.settings);
  // O seletor de letra vive dentro desta janela e conversa com o backend por
  // conta própria.
  responder("now_playing", { track: null, positionMs: 0, playing: false });
  responder("current_lyrics", null);
}

async function montar() {
  render(<SettingsWindow />);
  // Sem isto o primeiro `expect` corre antes de as preferências chegarem, e a
  // janela ainda não desenhou controle nenhum.
  await screen.findByText("Posição");
}

beforeEach(() => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
  limparTauri();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("controle de desfoque", () => {
  // O campo `blur` existia nas preferências desde sempre e não fazia nada:
  // resíduo da tentativa que a #18 descreve e abandonou.
  it("aparece quando o compositor sabe ligar e desligar o desfoque", async () => {
    cenario({ blurAvailable: true });
    await montar();
    expect(screen.getByText("Desfoque atrás")).toBeInTheDocument();
  });

  it("some quando o compositor não sabe", async () => {
    cenario({ blurAvailable: false });
    await montar();
    expect(screen.queryByText("Desfoque atrás")).toBeNull();
  });

  it("desligar o desfoque grava a preferência", async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    cenario({ blurAvailable: true }, { blur: true });
    await montar();

    const linha = screen.getByText("Desfoque atrás").closest("div")!;
    await user.click(linha.parentElement!.querySelector("button")!);

    await waitFor(() => {
      const gravadas = chamadas.filter((c) => c.comando === "save_settings");
      const gravou = gravadas[gravadas.length - 1];
      expect((gravou?.args.settings as Settings).blur).toBe(false);
    });
  });
});

describe("modo camada", () => {
  // O texto prometia o que o app não entregava até a #35: dizia que a camada
  // "deixa de ser arrastável". Ela é.
  it("não anuncia mais que a camada não pode ser arrastada", async () => {
    cenario();
    await montar();
    expect(screen.queryByText(/deixa de ser arrastável/)).toBeNull();
    expect(
      screen.getByText("fica acima até de tela cheia · exige reiniciar o app"),
    ).toBeInTheDocument();
  });

  it("ligada mas sem efeito, diz por quê em vez de fingir que está valendo", async () => {
    cenario(
      { layerShellActive: false, layerShellFallback: "a sessão é X11" },
      { layerShell: true },
    );
    await montar();
    expect(
      screen.getByText(/ligada, mas sem efeito nesta sessão — a sessão é X11/),
    ).toBeInTheDocument();
  });
});
