import { describe, expect, it } from "vitest";

import { en } from "./en";
import { ptBR } from "./pt-BR";
import { resolverIdioma, textos, traduzirErro } from "./index";

describe("resolverIdioma", () => {
  it("a escolha do usuário ganha do sistema", () => {
    expect(resolverIdioma("en", "pt-BR")).toBe("en");
    expect(resolverIdioma("pt-BR", "en")).toBe("pt-BR");
  });

  it("no automático, segue o sistema", () => {
    expect(resolverIdioma("auto", "pt-BR")).toBe("pt-BR");
    expect(resolverIdioma("auto", "en")).toBe("en");
  });

  // O backend responde em milissegundos, mas a primeira renderização acontece
  // antes. Inglês é o que menos surpreende quem não fala português.
  it("cai no inglês enquanto o backend não respondeu", () => {
    expect(resolverIdioma("auto", null)).toBe("en");
    expect(resolverIdioma(undefined, null)).toBe("en");
  });
});

describe("dicionários", () => {
  // O `: Dicionario` do pt-BR já cobre isto em tempo de compilação. Este teste
  // pega o caso em que alguém silencia o compilador com um `as`.
  it("têm exatamente as mesmas chaves", () => {
    expect(Object.keys(ptBR).sort()).toEqual(Object.keys(en).sort());
  });

  it("concordam sobre o que é função e o que é texto", () => {
    for (const chave of Object.keys(en) as (keyof typeof en)[]) {
      expect(typeof ptBR[chave], `divergiu em ${chave}`).toBe(typeof en[chave]);
    }
  });

  it("não deixam tradução vazia", () => {
    for (const [chave, valor] of Object.entries(ptBR)) {
      if (typeof valor === "string") {
        expect(valor.trim(), `vazia em ${chave}`).not.toBe("");
      }
    }
  });
});

describe("traduzirErro", () => {
  const pt = textos("pt-BR");
  const ingles = textos("en");

  it("monta a frase com os parâmetros do backend", () => {
    const erro = {
      code: "hotkey.refused",
      args: { atalho: "SUPER, L", motivo: "unknown key" },
    };
    expect(traduzirErro(pt, erro)).toBe('o compositor recusou "SUPER, L": unknown key');
    expect(traduzirErro(ingles, erro)).toBe('the compositor refused "SUPER, L": unknown key');
  });

  it("traduz erro sem parâmetro", () => {
    const erro = { code: "lyrics.notFound", args: {} };
    expect(traduzirErro(pt, erro)).toBe("nenhuma letra encontrada para esta faixa");
  });

  // O app pode ser mais novo que a tradução, ou o contrário. Em nenhum dos dois
  // casos o usuário pode ficar sem informação nenhuma.
  it("código desconhecido mostra o código em vez de sumir", () => {
    const traduzido = traduzirErro(pt, { code: "coisa.nova", args: {} });
    expect(traduzido).toContain("coisa.nova");
  });

  it("parâmetro faltando não vira 'undefined' na tela", () => {
    const traduzido = traduzirErro(pt, { code: "hotkey.refused", args: {} });
    expect(traduzido).not.toContain("undefined");
  });

  // Nem todo comando foi convertido para UiError, e falha de rede do próprio
  // invoke chega como string solta.
  it("deixa passar o que não é erro estruturado", () => {
    expect(traduzirErro(pt, "deu ruim")).toBe("deu ruim");
    expect(traduzirErro(pt, new Error("caiu"))).toContain("caiu");
    expect(traduzirErro(pt, null)).toBe("null");
  });
});
