import { describe, expect, it } from "vitest";
import { lineAt, lineProgress, type LyricLine } from "./useLyrics";

/** Uma letra curta e regular, uma linha a cada dez segundos. */
const LINHAS: LyricLine[] = [
  { atMs: 10_000, text: "primeira" },
  { atMs: 20_000, text: "segunda" },
  { atMs: 30_000, text: "terceira" },
];

describe("lineAt", () => {
  it("devolve -1 antes da primeira linha", () => {
    // O silêncio da introdução: a tela fica limpa de propósito, e isso é
    // diferente de "linha 0".
    expect(lineAt(LINHAS, 0)).toBe(-1);
    expect(lineAt(LINHAS, 9_999)).toBe(-1);
  });

  it("acende a linha no instante exato em que ela começa", () => {
    expect(lineAt(LINHAS, 10_000)).toBe(0);
    expect(lineAt(LINHAS, 20_000)).toBe(1);
  });

  it("mantém a linha até a próxima começar", () => {
    expect(lineAt(LINHAS, 19_999)).toBe(0);
    expect(lineAt(LINHAS, 29_999)).toBe(1);
  });

  it("fica na última linha para sempre depois dela", () => {
    expect(lineAt(LINHAS, 30_000)).toBe(2);
    expect(lineAt(LINHAS, 999_999)).toBe(2);
  });

  it("não estoura com letra vazia", () => {
    expect(lineAt([], 5_000)).toBe(-1);
  });

  it("aguenta uma letra de uma linha só", () => {
    const uma = [{ atMs: 1_000, text: "única" }];
    expect(lineAt(uma, 0)).toBe(-1);
    expect(lineAt(uma, 1_000)).toBe(0);
    expect(lineAt(uma, 500_000)).toBe(0);
  });

  it("acha a linha certa numa letra longa", () => {
    // A busca é binária; uma letra grande é onde um off-by-one apareceria.
    const longa = Array.from({ length: 200 }, (_, i) => ({
      atMs: i * 1_000,
      text: `linha ${i}`,
    }));
    expect(lineAt(longa, 0)).toBe(0);
    expect(lineAt(longa, 137_500)).toBe(137);
    expect(lineAt(longa, 199_000)).toBe(199);
  });
});

describe("lineProgress", () => {
  it("é 0 no começo da linha e ~1 no fim", () => {
    expect(lineProgress(LINHAS, 0, 10_000)).toBe(0);
    expect(lineProgress(LINHAS, 0, 19_999)).toBeCloseTo(1, 2);
  });

  it("varre sobre o teto, não sobre a distância até a próxima linha", () => {
    // As linhas aqui estão a 10s uma da outra, mas a varredura tem teto de 8s.
    // Aos 15s — 5s dentro da linha — já são 5/8, não 5/10. É intencional: com
    // o teto a linha termina de acender antes do intervalo, em vez de rastejar.
    expect(lineProgress(LINHAS, 0, 15_000)).toBeCloseTo(0.625, 5);
    expect(lineProgress(LINHAS, 0, 14_000)).toBeCloseTo(0.5, 5);
  });

  it("fica preso entre 0 e 1 fora do intervalo da linha", () => {
    expect(lineProgress(LINHAS, 1, 5_000)).toBe(0);
    expect(lineProgress(LINHAS, 0, 999_999)).toBe(1);
  });

  it("devolve 0 para índice fora da letra", () => {
    // É o estado da introdução, quando `lineAt` devolveu -1.
    expect(lineProgress(LINHAS, -1, 5_000)).toBe(0);
    expect(lineProgress(LINHAS, 99, 5_000)).toBe(0);
    expect(lineProgress([], 0, 5_000)).toBe(0);
  });

  it("limita a varredura numa linha seguida de instrumental longo", () => {
    // Sem teto, uma linha antes de trinta segundos de instrumental varreria
    // devagar demais para parecer viva.
    const comIntervalo: LyricLine[] = [
      { atMs: 0, text: "antes do instrumental" },
      { atMs: 30_000, text: "depois" },
    ];
    expect(lineProgress(comIntervalo, 0, 8_000)).toBe(1);
    expect(lineProgress(comIntervalo, 0, 4_000)).toBeCloseTo(0.5, 5);
  });

  it("trata duas linhas no mesmo instante como já cantadas", () => {
    // Acontece em LRC malformado; dividir por zero devolveria NaN e a linha
    // nunca acenderia.
    const simultaneas: LyricLine[] = [
      { atMs: 5_000, text: "a" },
      { atMs: 5_000, text: "b" },
    ];
    expect(lineProgress(simultaneas, 0, 5_000)).toBe(1);
  });
});
