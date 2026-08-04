import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type CoverEvent = {
  trackKey: string;
  /** `null` é "procurei e não achei" — diferente de ainda não ter chegado. */
  url: string | null;
};

/**
 * A capa do álbum que está tocando.
 *
 * O evento carrega a chave da faixa e o hook só aceita a que combina com a
 * atual. Sem isso, a capa de uma busca lenta chegaria depois da troca de música
 * e ficaria a capa errada na tela — que é pior que capa nenhuma, porque parece
 * que o app se confundiu de faixa.
 */
export function useCover(trackKey: string | null): string | null {
  const [capas, setCapas] = useState<Record<string, string | null>>({});

  useEffect(() => {
    invoke<CoverEvent | null>("current_cover")
      .then((ev) => {
        if (ev) setCapas((c) => ({ ...c, [ev.trackKey]: ev.url }));
      })
      .catch(() => {});

    const un = listen<CoverEvent>("cover", ({ payload }) =>
      setCapas((c) => ({ ...c, [payload.trackKey]: payload.url })),
    );
    return () => {
      un.then((f) => f());
    };
  }, []);

  return trackKey ? (capas[trackKey] ?? null) : null;
}
