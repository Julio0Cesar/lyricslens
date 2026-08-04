/**
 * Qual idioma a interface fala, e como o erro do backend vira frase.
 *
 * A escolha do idioma não é do frontend: o `navigator.language` de um WebKit
 * embarcado reporta o locale do processo, não o `LANG` da sessão, e mente com
 * frequência. Quem detecta é o Rust (`i18n::do_sistema`); aqui só se combina a
 * preferência do usuário com o que ele respondeu.
 */
import { en, type Dicionario } from "./en";
import { ptBR } from "./pt-BR";

/** Idioma concreto, o que a UI de fato fala. */
export type Idioma = "pt-BR" | "en";

/** O que o usuário escolhe. `auto` segue o sistema. */
export type PreferenciaIdioma = "auto" | Idioma;

const DICIONARIOS: Record<Idioma, Dicionario> = {
  en,
  "pt-BR": ptBR,
};

export function textos(idioma: Idioma): Dicionario {
  return DICIONARIOS[idioma];
}

/**
 * A preferência manda; `auto` devolve o que o sistema disse.
 *
 * Inglês é a reserva quando o backend ainda não respondeu — é o que menos
 * surpreende quem não fala português, e a resposta chega em milissegundos.
 */
export function resolverIdioma(
  preferencia: PreferenciaIdioma | undefined,
  sistema: Idioma | null,
): Idioma {
  if (preferencia && preferencia !== "auto") return preferencia;
  return sistema ?? "en";
}

/**
 * Erro estruturado que o backend manda: código estável mais os parâmetros que
 * a frase precisa. O texto é montado aqui, no idioma da vez — ver `UiError`
 * no Rust.
 */
export type ErroDoBackend = {
  code: string;
  args: Record<string, string>;
};

function estruturado(erro: unknown): ErroDoBackend | null {
  if (typeof erro !== "object" || erro === null) return null;
  const e = erro as Partial<ErroDoBackend>;
  if (typeof e.code !== "string") return null;
  return { code: e.code, args: e.args ?? {} };
}

/**
 * Converte o erro do backend na frase do idioma ativo.
 *
 * O `switch` é explícito de propósito: é ele que faz o TypeScript conferir que
 * cada código recebe os parâmetros que a tradução espera. Um mapa genérico
 * `args → função` aceitaria qualquer coisa e o erro só apareceria na tela.
 *
 * Código desconhecido não some nem vira tela em branco — o app pode ser mais
 * novo que a tradução, ou o contrário.
 */
export function traduzirErro(t: Dicionario, erro: unknown): string {
  const e = estruturado(erro);
  // Comando ainda não convertido, ou falha que não veio do backend: mostra o
  // que der, que é melhor que engolir.
  if (!e) return String(erro);

  switch (e.code) {
    case "hotkey.onlyHyprland":
      return t["error.hotkey.onlyHyprland"];
    case "hotkey.refused":
      return t["error.hotkey.refused"](e.args.atalho ?? "", e.args.motivo ?? "");
    case "hotkey.noExecutable":
      return t["error.hotkey.noExecutable"];
    case "hotkey.compositorFailed":
      return t["error.hotkey.compositorFailed"](e.args.motivo ?? "");
    case "update.notFromScript":
      return t["error.update.notFromScript"];
    case "update.failed":
      return t["error.update.failed"](e.args.motivo ?? "");
    case "autostart.remove":
      return t["error.autostart.remove"](e.args.motivo ?? "");
    case "autostart.write":
      return t["error.autostart.write"](e.args.motivo ?? "");
    case "autostart.noConfigDir":
      return t["error.autostart.noConfigDir"];
    case "autostart.noExecutable":
      return t["error.autostart.noExecutable"];
    case "lyrics.network":
      return t["error.lyrics.network"](e.args.motivo ?? "");
    case "lyrics.notFound":
      return t["error.lyrics.notFound"];
    case "lyrics.decode":
      return t["error.lyrics.decode"](e.args.motivo ?? "");
    case "settings.write":
      return t["error.settings.write"](e.args.motivo ?? "");
    case "cache.write":
      return t["error.cache.write"](e.args.motivo ?? "");
    case "overlay.noWindow":
      return t["error.overlay.noWindow"];
    case "overlay.rules":
      return t["error.overlay.rules"](e.args.motivo ?? "");
    default:
      return t["error.unknown"](e.code);
  }
}

export type { Dicionario };
