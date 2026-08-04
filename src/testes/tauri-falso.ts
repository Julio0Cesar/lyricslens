/**
 * Um Tauri de mentira, para os componentes poderem ser testados.
 *
 * Todo componente do app conversa com o backend por `invoke` e `listen`. Sem
 * substituir os dois, montar qualquer componente num teste explode no primeiro
 * `useEffect` — e foi justamente aí que os dois defeitos citados na #9
 * apareceram: o hook depois de uma saída antecipada e a âncora `NaN`.
 *
 * O registro fica aqui, separado do `vitest.setup.ts` que faz o `vi.mock`,
 * porque o teste precisa **controlar** as respostas: `responder()` antes de
 * montar, `emitir()` durante.
 */

type Resposta = unknown | ((args: Record<string, unknown>) => unknown);

const comandos = new Map<string, Resposta>();
const ouvintes = new Map<string, Set<(evento: { payload: unknown }) => void>>();

/** Comandos chamados desde o último `limparTauri()`, na ordem. */
export const chamadas: { comando: string; args: Record<string, unknown> }[] = [];

/** O que `invoke("<comando>")` devolve. Valor ou função dos argumentos. */
export function responder(comando: string, resposta: Resposta): void {
  comandos.set(comando, resposta);
}

/** Faz `invoke("<comando>")` rejeitar, para exercitar o caminho de erro. */
export function recusar(comando: string, erro: unknown): void {
  comandos.set(comando, () => {
    throw erro;
  });
}

/** Dispara um evento do backend para quem estiver escutando. */
export function emitir(evento: string, payload: unknown): void {
  for (const ouvinte of ouvintes.get(evento) ?? []) ouvinte({ payload });
}

export function limparTauri(): void {
  comandos.clear();
  ouvintes.clear();
  chamadas.length = 0;
}

// --- o que o `vitest.setup.ts` liga no lugar da API de verdade ---------------

export async function invoke(comando: string, args?: Record<string, unknown>) {
  chamadas.push({ comando, args: args ?? {} });

  if (!comandos.has(comando)) {
    // Barulhento de propósito: comando não declarado quase sempre é o teste
    // que esqueceu de preparar o cenário, e um `undefined` silencioso vira
    // uma falha muito mais longe daqui.
    throw new Error(
      `o teste não declarou resposta para invoke("${comando}") — use responder("${comando}", …)`,
    );
  }

  const resposta = comandos.get(comando);
  return typeof resposta === "function"
    ? (resposta as (a: Record<string, unknown>) => unknown)(args ?? {})
    : resposta;
}

export async function listen(
  evento: string,
  ouvinte: (evento: { payload: unknown }) => void,
) {
  if (!ouvintes.has(evento)) ouvintes.set(evento, new Set());
  ouvintes.get(evento)!.add(ouvinte);
  return () => ouvintes.get(evento)?.delete(ouvinte);
}

/** A janela: só o arraste é usado, e num teste ele não faz nada. */
export function getCurrentWindow() {
  return {
    startDragging: async () => {},
  };
}

/**
 * O elemento cujo texto completo é exatamente `texto`, e que não tem filho
 * dizendo a mesma coisa.
 *
 * O karaokê quebra a linha em um `<span>` por palavra — inclusive os espaços,
 * para não colapsar o texto. Então `getByText("When you were here before")`
 * não acha nada: não existe nó de texto com a frase inteira. Este casador
 * remonta pelo `textContent` e devolve o elemento mais interno que ainda
 * contém a frase toda, que é a linha.
 */
export function casaLinha(texto: string) {
  return (_conteudo: string, elemento: Element | null): boolean => {
    if (!elemento) return false;
    const proprio = elemento.textContent?.replace(/\s+/g, " ").trim();
    if (proprio !== texto) return false;
    return !Array.from(elemento.children).some(
      (filho) => filho.textContent?.replace(/\s+/g, " ").trim() === texto,
    );
  };
}
