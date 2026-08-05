// @vitest-environment jsdom
/**
 * A janela de linhas e a varredura de karaokê.
 *
 * As duas são o produto: é o que fica na tela o tempo todo. E as duas são
 * inteiramente visuais — nenhum teste de unidade em Rust as alcança.
 */
import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";

import LyricLines from "./LyricLines";
import type { Settings } from "../settings/useSettings";
import { casaLinha } from "../testes/tauri-falso";

const PREFERENCIAS: Settings = {
  fontFamily: "",
  fontSize: 26,
  fontWeight: 600,
  textColor: "#ffffff",
  dimColor: "#808080",
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

const LINHAS = [
  { atMs: 0, text: "primeira" },
  { atMs: 1000, text: "segunda" },
  { atMs: 2000, text: "terceira" },
  { atMs: 3000, text: "quarta" },
];

function desenhar(
  index: number,
  progress = 0,
  ajustes: Partial<Settings> = {},
) {
  return render(
    <LyricLines
      lines={LINHAS}
      index={index}
      progress={progress}
      settings={{ ...PREFERENCIAS, ...ajustes }}
    />,
  );
}

describe("janela de linhas", () => {
  it("mostra a anterior, a atual e a próxima", () => {
    desenhar(1);
    for (const texto of ["primeira", "segunda", "terceira"]) {
      expect(screen.getByText(casaLinha(texto))).toBeInTheDocument();
    }
    expect(screen.queryByText(casaLinha("quarta"))).not.toBeInTheDocument();
  });

  it("sem linhas de contexto, mostra só a atual", () => {
    desenhar(1, 0, { showContextLines: false });
    expect(screen.getByText(casaLinha("segunda"))).toBeInTheDocument();
    expect(screen.queryByText(casaLinha("primeira"))).not.toBeInTheDocument();
    expect(screen.queryByText(casaLinha("terceira"))).not.toBeInTheDocument();
  });

  /// Antes da primeira linha o índice é -1: não há "atual", só o que vem.
  it("antes da primeira linha, mostra só o que vem a seguir", () => {
    desenhar(-1);
    expect(screen.getByText(casaLinha("primeira"))).toBeInTheDocument();
    expect(screen.queryByText(casaLinha("segunda"))).not.toBeInTheDocument();
  });

  it("na última linha, não inventa uma próxima", () => {
    desenhar(3);
    expect(screen.getByText(casaLinha("quarta"))).toBeInTheDocument();
    expect(screen.getByText(casaLinha("terceira"))).toBeInTheDocument();
  });

  it("não estoura com letra vazia", () => {
    const { container } = render(
      <LyricLines lines={[]} index={-1} progress={0} settings={PREFERENCIAS} />,
    );
    expect(container).not.toBeEmptyDOMElement();
  });
});

/**
 * A varredura acende palavra por palavra conforme a linha é cantada. O sinal
 * observável é a cor: palavra já cantada usa `textColor`, palavra por cantar
 * usa `dimColor`. As duas cores do teste são bem distintas de propósito.
 */
describe("varredura de karaokê", () => {
  const palavras = (texto: string) =>
    Array.from(
      screen.getByText(casaLinha(texto)).querySelectorAll("span"),
    ).filter((s) => s.textContent?.trim());

  it("no começo da linha, nenhuma palavra está acesa", () => {
    render(
      <LyricLines
        lines={[{ atMs: 0, text: "uma duas tres" }]}
        index={0}
        progress={0}
        settings={PREFERENCIAS}
      />,
    );
    const acesas = palavras("uma duas tres").filter(
      (s) => s.style.color === "rgb(255, 255, 255)",
    );
    expect(acesas).toHaveLength(0);
  });

  it("no fim da linha, todas estão acesas", () => {
    render(
      <LyricLines
        lines={[{ atMs: 0, text: "uma duas tres" }]}
        index={0}
        progress={1}
        settings={PREFERENCIAS}
      />,
    );
    const apagadas = palavras("uma duas tres").filter(
      (s) => s.style.color === "rgb(128, 128, 128)",
    );
    expect(apagadas).toHaveLength(0);
  });

  it("no meio, acende só o começo da linha", () => {
    render(
      <LyricLines
        lines={[{ atMs: 0, text: "uma duas tres" }]}
        index={0}
        progress={0.5}
        settings={PREFERENCIAS}
      />,
    );
    const todas = palavras("uma duas tres");
    expect(todas[0].style.color).toBe("rgb(255, 255, 255)");
    expect(todas[todas.length - 1].style.color).toBe("rgb(128, 128, 128)");
  });

  it("desligada, a linha inteira sai numa cor só", () => {
    render(
      <LyricLines
        lines={[{ atMs: 0, text: "uma duas tres" }]}
        index={0}
        progress={0.5}
        settings={{ ...PREFERENCIAS, karaoke: false }}
      />,
    );
    // Desligado, o componente devolve **um** `<span>` com a linha inteira, e
    // não um por palavra — então o elemento casado já é o span. Que a divisão
    // desapareça é parte do contrato: é ela que custa repintura (#39).
    const linha = screen.getByText(casaLinha("uma duas tres"));
    expect(linha.tagName).toBe("SPAN");
    expect(linha.querySelectorAll("span")).toHaveLength(0);
    expect(linha.style.color).toBe("rgb(255, 255, 255)");
  });

  /// Espaço colapsado junta palavras — o texto na tela deixa de ser a letra.
  it("preserva os espaços entre as palavras", () => {
    render(
      <LyricLines
        lines={[{ atMs: 0, text: "uma duas tres" }]}
        index={0}
        progress={0.5}
        settings={PREFERENCIAS}
      />,
    );
    expect(screen.getByText(casaLinha("uma duas tres"))).toBeInTheDocument();
  });
});
