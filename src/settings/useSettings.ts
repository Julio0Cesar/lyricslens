import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import {
  resolverIdioma,
  textos,
  traduzirErro,
  type Idioma,
  type PreferenciaIdioma,
} from "../i18n";

export type TextAlign = "left" | "center";

export type Settings = {
  fontFamily: string;
  fontSize: number;
  fontWeight: number;
  textColor: string;
  dimColor: string;
  backgroundColor: string;
  backgroundOpacity: number;
  blur: boolean;
  cornerRadius: number;

  showContextLines: boolean;
  showTrackInfo: boolean;
  showProgress: boolean;
  textAlign: TextAlign;
  karaoke: boolean;

  clickThrough: boolean;
  hideWhenPaused: boolean;
  syncOffsetMs: number;
  hotkey: string;
  language: PreferenciaIdioma;

  width: number;
  height: number;
  marginBottom: number;
  positionX: number | null;
  positionY: number | null;
  layerShell: boolean;
};

/**
 * O modo camada é a única preferência que pode não estar valendo: ele depende
 * do ambiente gráfico e é decidido na partida do app. Sem isto o toggle mostra
 * o que o usuário pediu, que pode não ser o que está acontecendo.
 */
export type OverlayStatus = {
  layerShellRequested: boolean;
  layerShellActive: boolean;
  /** Por que os dois diferem. `null` quando não diferem. */
  layerShellFallback: string | null;
};

/**
 * As preferências vivem no backend; toda janela é espectadora do mesmo estado.
 * Quem edita escreve, e o evento devolve o resultado já saneado para todas —
 * inclusive para quem editou, então a UI nunca mostra um valor que o backend
 * recusou.
 */
export function useSettings() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [status, setStatus] = useState<OverlayStatus | null>(null);
  // O idioma do sistema vem do backend: o webview não enxerga o `LANG` da
  // sessão. Não muda em tempo de execução, então basta ler uma vez.
  const [idiomaDoSistema, setIdiomaDoSistema] = useState<Idioma | null>(null);
  // Guardado cru, não traduzido: o idioma pode mudar depois de o erro
  // acontecer, e a frase precisa acompanhar.
  const [erroBruto, setErroBruto] = useState<unknown>(null);
  const pendente = useRef<number | undefined>(undefined);

  useEffect(() => {
    invoke<Settings>("get_settings").then(setSettings);
    // Não muda em tempo de execução — o tipo da superfície é definido antes de
    // a janela existir —, então basta ler uma vez.
    invoke<OverlayStatus>("overlay_status").then(setStatus);
    invoke<Idioma>("system_language").then(setIdiomaDoSistema).catch(() => {});

    const un = listen<Settings>("settings", ({ payload }) => setSettings(payload));
    return () => {
      un.then((f) => f());
      window.clearTimeout(pendente.current);
    };
  }, []);

  /**
   * Aplica na hora e grava depois. Arrastar um controle deslizante dispararia
   * dezenas de escritas em disco; a espera agrupa tudo numa só.
   */
  const update = useCallback((patch: Partial<Settings>) => {
    setSettings((atual) => {
      if (!atual) return atual;
      const proximo = { ...atual, ...patch };

      window.clearTimeout(pendente.current);
      pendente.current = window.setTimeout(() => {
        setErroBruto(null);
        // Uma recusa do backend — combinação que o compositor não aceitou,
        // por exemplo — precisa aparecer, senão a UI mostra um estado que
        // não foi gravado.
        invoke("save_settings", { settings: proximo }).catch((e) => {
          setErroBruto(e);
          invoke<Settings>("get_settings").then(setSettings);
        });
      }, 150);

      return proximo;
    });
  }, []);

  const idioma = resolverIdioma(settings?.language, idiomaDoSistema);
  const t = textos(idioma);

  return {
    settings,
    status,
    /** Já traduzido para o idioma da vez. */
    erro: erroBruto === null ? null : traduzirErro(t, erroBruto),
    update,
    idioma,
    t,
  };
}
