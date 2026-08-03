import { describe, expect, it } from "vitest";
import { formatMs, PASSO_FINO_MS, PASSO_GROSSO_MS } from "./useNowPlaying";
import { rgba } from "../App";

describe("formatMs", () => {
  it("formata minutos e segundos com dois dígitos", () => {
    expect(formatMs(0)).toBe("0:00");
    expect(formatMs(9_000)).toBe("0:09");
    expect(formatMs(65_000)).toBe("1:05");
    expect(formatMs(600_000)).toBe("10:00");
  });

  it("trunca em vez de arredondar", () => {
    // Arredondar mostraria 0:01 antes de o segundo virar, e a letra ficaria
    // parecendo adiantada em relação ao relógio.
    expect(formatMs(999)).toBe("0:00");
    expect(formatMs(1_999)).toBe("0:01");
  });

  it("não mostra tempo negativo", () => {
    // O deslocamento de sincronia pode levar a posição abaixo de zero.
    expect(formatMs(-5_000)).toBe("0:00");
  });

  it("aguenta faixa mais longa que uma hora", () => {
    expect(formatMs(3_723_000)).toBe("62:03");
  });
});

describe("rgba", () => {
  it("converte hexadecimal de seis dígitos", () => {
    expect(rgba("#0a0a0e", 0.55)).toBe("rgba(10, 10, 14, 0.55)");
    expect(rgba("#ffffff", 1)).toBe("rgba(255, 255, 255, 1)");
    expect(rgba("#000000", 0)).toBe("rgba(0, 0, 0, 0)");
  });

  it("aceita sem o cerquilha", () => {
    expect(rgba("ff8800", 0.5)).toBe("rgba(255, 136, 0, 0.5)");
  });

  it("ignora o canal alfa embutido no hexadecimal", () => {
    // `dimColor` tem oito dígitos por padrão (`#ffffff4d`); o alfa vem do
    // parâmetro, não do texto.
    expect(rgba("#ffffff4d", 0.3)).toBe("rgba(255, 255, 255, 0.3)");
  });

  it("cai para preto quando o texto não é uma cor", () => {
    // Um campo de cor vazio nas configurações não pode pintar a janela de
    // `rgba(NaN, NaN, NaN)`, que o WebKit descarta silenciosamente.
    expect(rgba("", 0.4)).toBe("rgba(0, 0, 0, 0.4)");
    expect(rgba("#abc", 0.4)).toBe("rgba(0, 0, 0, 0.4)");
  });
});

describe("passos de atualização", () => {
  it("o passo grosso é maior que o fino", () => {
    expect(PASSO_GROSSO_MS).toBeGreaterThan(PASSO_FINO_MS);
  });

  it("o passo fino cabe folgado na transição do karaokê", () => {
    // A cor de cada palavra leva 260ms para trocar; um passo maior que isso
    // faria a varredura andar aos saltos visíveis.
    expect(PASSO_FINO_MS).toBeLessThan(260);
  });
});
