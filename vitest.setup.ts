/**
 * Troca a API do Tauri pela de mentira, para todo teste.
 *
 * O `vi.mock` mora aqui, e não em cada arquivo de teste, por dois motivos: ele
 * é içado para o topo do módulo e referenciar variável de fora do factory é
 * frágil; e nenhum componente do app funciona sem essa troca, então repeti-la
 * em cada arquivo seria cerimônia sem escolha.
 *
 * Os controles ficam no `src/testes/tauri-falso.ts` — é de lá que o teste
 * chama `responder()` e `emitir()`.
 */
import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import React from "react";
import { afterEach, vi } from "vitest";

// A limpeza automática do Testing Library só se registra quando o `afterEach`
// é global, e aqui ele não é. Sem isto cada `render` deixa o anterior no
// `document.body`, e a segunda consulta por um mesmo texto falha com "found
// multiple elements" — falha confusa, porque acusa o teste errado.
afterEach(cleanup);

/**
 * O `motion` vira marcação simples nos testes.
 *
 * O `AnimatePresence` com `mode="wait"` só monta o elemento novo depois de o
 * antigo terminar de sair, e essa saída depende de quadros de animação que não
 * acontecem sob timers falsos — o elemento esperado nunca aparecia. Animação
 * não é o que estes testes verificam; conteúdo é.
 */
vi.mock("motion/react", () => {
  const semAnimacao = (tag: string) =>
    function Simples({
      children,
      initial: _i,
      animate: _a,
      exit: _e,
      transition: _t,
      layout: _l,
      ...resto
    }: Record<string, unknown> & { children?: React.ReactNode }) {
      return React.createElement(tag, resto, children);
    };

  return {
    motion: new Proxy({} as Record<string, unknown>, {
      get: (_alvo, tag: string) => semAnimacao(tag),
    }),
    AnimatePresence: ({ children }: { children?: React.ReactNode }) =>
      React.createElement(React.Fragment, null, children),
  };
});

vi.mock("@tauri-apps/api/core", async () => {
  const falso = await import("./src/testes/tauri-falso");
  return { invoke: falso.invoke };
});

vi.mock("@tauri-apps/api/event", async () => {
  const falso = await import("./src/testes/tauri-falso");
  return { listen: falso.listen };
});

vi.mock("@tauri-apps/api/window", async () => {
  const falso = await import("./src/testes/tauri-falso");
  return { getCurrentWindow: falso.getCurrentWindow };
});
