import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ColorPicker, Row, Section, Select, Slider, Toggle } from "./controls";
import HotkeyCapture from "./HotkeyCapture";
import LyricsPicker from "./LyricsPicker";
import { traduzirErro } from "../i18n";
import { useSettings } from "./useSettings";

type CacheStats = { tracks: number; synced: number; pinned: number; misses: number };
type UpdateInfo = {
  current: string;
  latest: string | null;
  available: boolean;
  canApply: boolean;
};

/** Um duplo clique em cima de um controle está mexendo nele, não fechando. */
function emCimaDeControle(alvo: EventTarget | null): boolean {
  return alvo instanceof Element && alvo.closest("input,button,select,textarea,label") !== null;
}

export default function SettingsWindow() {
  const { settings, status, erro, update, t } = useSettings();
  const [fontes, setFontes] = useState<string[]>([]);
  const [cache, setCache] = useState<CacheStats | null>(null);
  const [versao, setVersao] = useState<UpdateInfo | null>(null);
  const [atualizando, setAtualizando] = useState(false);
  const [erroUpdate, setErroUpdate] = useState<string | null>(null);
  // O estado do autostart é a existência do `.desktop`, não uma preferência
  // gravada: se o usuário apagar o arquivo por fora, o toggle concorda.
  const [autostart, setAutostart] = useState(false);
  const [erroAutostart, setErroAutostart] = useState<string | null>(null);

  useEffect(() => {
    invoke<boolean>("autostart_enabled").then(setAutostart).catch(() => {});
  }, []);

  async function alternarAutostart(valor: boolean) {
    setErroAutostart(null);
    try {
      // O backend devolve o estado real depois de mexer no arquivo, não o
      // pedido — se a escrita falhar pela metade, a UI mostra o que de fato há.
      setAutostart(await invoke<boolean>("set_autostart", { enabled: valor }));
    } catch (e) {
      setErroAutostart(traduzirErro(t, e));
      invoke<boolean>("autostart_enabled").then(setAutostart).catch(() => {});
    }
  }

  useEffect(() => {
    invoke<string[]>("list_fonts").then(setFontes).catch(() => setFontes([]));
    invoke<CacheStats>("cache_stats").then(setCache).catch(() => setCache(null));
    invoke<UpdateInfo>("check_update").then(setVersao).catch(() => setVersao(null));
  }, []);

  if (!settings) {
    return <div className="p-6 text-sm text-white/40">{t["settings.loading"]}</div>;
  }

  return (
    <div
      className="h-full overflow-y-auto bg-neutral-950 text-white"
      onDoubleClick={(e) => {
        if (!emCimaDeControle(e.target)) invoke("close_settings");
      }}
    >
      <div className="mx-auto flex max-w-lg flex-col gap-7 p-6">
        <header className="flex items-baseline justify-between">
          <h1 className="text-lg font-semibold">{t["settings.title"]}</h1>
          <button
            onClick={() => invoke("toggle_overlay")}
            className="rounded-md border border-white/15 bg-white/6 px-2 py-1 text-[11px] text-white/70 hover:bg-white/12"
          >
            {t["settings.toggleOverlay"]}
          </button>
        </header>

        {erro && (
          <p className="rounded border border-rose-500/30 bg-rose-500/10 px-3 py-2 text-[12px] text-rose-300">
            {erro}
          </p>
        )}

        <LyricsPicker t={t} />

        <Section title={t["section.appearance"]}>
          <Row
            label={t["font.label"]}
            hint={fontes.length ? t["font.count"](fontes.length) : t["font.systemHint"]}
          >
            <Select
              value={settings.fontFamily}
              onChange={(v) => update({ fontFamily: v })}
              options={[
                { value: "", label: t["font.systemOption"] },
                ...fontes.map((f) => ({ value: f, label: f })),
              ]}
            />
          </Row>
          <Row label={t["fontSize.label"]}>
            <Slider
              value={settings.fontSize}
              min={10}
              max={96}
              suffix="px"
              onChange={(v) => update({ fontSize: v })}
            />
          </Row>
          <Row label={t["fontWeight.label"]}>
            <Slider
              value={settings.fontWeight}
              min={100}
              max={900}
              step={100}
              onChange={(v) => update({ fontWeight: v })}
            />
          </Row>
          <Row label={t["textColor.label"]}>
            <ColorPicker value={settings.textColor} onChange={(v) => update({ textColor: v })} />
          </Row>
          <Row label={t["dimColor.label"]} hint={t["dimColor.hint"]}>
            <ColorPicker value={settings.dimColor} onChange={(v) => update({ dimColor: v })} />
          </Row>
          <Row label={t["backgroundColor.label"]}>
            <ColorPicker
              value={settings.backgroundColor}
              onChange={(v) => update({ backgroundColor: v })}
            />
          </Row>
          <Row
            label={t["backgroundOpacity.label"]}
            hint={t["backgroundOpacity.hint"]}
          >
            <Slider
              value={settings.backgroundOpacity}
              min={0}
              max={1}
              step={0.05}
              onChange={(v) => update({ backgroundOpacity: v })}
            />
          </Row>
          <Row label={t["cornerRadius.label"]}>
            <Slider
              value={settings.cornerRadius}
              min={0}
              max={64}
              suffix="px"
              onChange={(v) => update({ cornerRadius: v })}
            />
          </Row>
        </Section>

        <Section title={t["section.content"]}>
          <Row label={t["contextLines.label"]} hint={t["contextLines.hint"]}>
            <Toggle
              value={settings.showContextLines}
              onChange={(v) => update({ showContextLines: v })}
            />
          </Row>
          <Row label={t["trackInfo.label"]} hint={t["trackInfo.hint"]}>
            <Toggle value={settings.showTrackInfo} onChange={(v) => update({ showTrackInfo: v })} />
          </Row>
          <Row label={t["progressBar.label"]}>
            <Toggle value={settings.showProgress} onChange={(v) => update({ showProgress: v })} />
          </Row>
          <Row label={t["karaoke.label"]} hint={t["karaoke.hint"]}>
            <Toggle value={settings.karaoke} onChange={(v) => update({ karaoke: v })} />
          </Row>
          <Row label={t["align.label"]}>
            <Select
              value={settings.textAlign}
              onChange={(v) => update({ textAlign: v })}
              options={[
                { value: "left", label: t["align.left"] },
                { value: "center", label: t["align.center"] },
              ]}
            />
          </Row>
        </Section>

        <Section title={t["section.behaviour"]}>
          <Row label={t["language.label"]} hint={t["language.hint"]}>
            <Select
              value={settings.language}
              onChange={(v) => update({ language: v })}
              options={[
                { value: "auto", label: t["language.auto"] },
                // Os nomes dos idiomas ficam em si mesmos, não traduzidos:
                // quem procura o próprio idioma numa interface que não entende
                // precisa reconhecer a palavra, não a tradução dela.
                { value: "pt-BR", label: "Português" },
                { value: "en", label: "English" },
              ]}
            />
          </Row>
          <Row label={t["hotkey.label"]} hint={t["hotkey.hint"]}>
            <HotkeyCapture
              value={settings.hotkey}
              onChange={(v) => update({ hotkey: v })}
              t={t}
            />
          </Row>
          <Row label={t["clickThrough.label"]} hint={t["clickThrough.hint"]}>
            <Toggle value={settings.clickThrough} onChange={(v) => update({ clickThrough: v })} />
          </Row>
          <Row label={t["hideWhenPaused.label"]}>
            <Toggle
              value={settings.hideWhenPaused}
              onChange={(v) => update({ hideWhenPaused: v })}
            />
          </Row>
          <Row
            label={t["autostart.label"]}
            hint={erroAutostart ?? t["autostart.hint"]}
          >
            <Toggle value={autostart} onChange={alternarAutostart} />
          </Row>
          <Row label={t["syncOffset.label"]} hint={t["syncOffset.hint"]}>
            <Slider
              value={settings.syncOffsetMs}
              min={-3000}
              max={3000}
              step={50}
              suffix="ms"
              onChange={(v) => update({ syncOffsetMs: v })}
            />
          </Row>
        </Section>

        <Section title={t["section.window"]}>
          <Row label={t["width.label"]}>
            <Slider
              value={settings.width}
              min={240}
              max={1920}
              step={10}
              suffix="px"
              onChange={(v) => update({ width: v })}
            />
          </Row>
          <Row label={t["height.label"]}>
            <Slider
              value={settings.height}
              min={80}
              max={900}
              step={10}
              suffix="px"
              onChange={(v) => update({ height: v })}
            />
          </Row>
          <Row label={t["marginBottom.label"]}>
            <Slider
              value={settings.marginBottom}
              min={0}
              max={800}
              step={10}
              suffix="px"
              onChange={(v) => update({ marginBottom: v })}
            />
          </Row>
          <Row
            label={t["position.label"]}
            hint={
              settings.positionX !== null
                ? t["position.pinned"](settings.positionX, settings.positionY ?? 0)
                : t["position.auto"]
            }
          >
            <button
              onClick={() => invoke("center_overlay")}
              disabled={settings.positionX === null}
              className="rounded-md border border-white/15 bg-white/6 px-2 py-1 text-[11px] text-white/70 hover:bg-white/12 disabled:opacity-40"
            >
              {t["position.center"]}
            </button>
          </Row>
          <Row
            label={t["layerShell.label"]}
            hint={
              // Quando a camada não sobe, a opção aparecia ligada enquanto na
              // prática não estava em uso, e nada indicava o porquê.
              settings.layerShell && status && !status.layerShellActive
                ? t["layerShell.inactive"](
                    status.layerShellFallback ?? t["layerShell.unavailable"],
                  )
                : t["layerShell.hint"]
            }
          >
            <Toggle value={settings.layerShell} onChange={(v) => update({ layerShell: v })} />
          </Row>
          <Row label="" hint={t["reapply.hint"]}>
            <button
              onClick={() => invoke("apply_compositor_rules")}
              className="rounded-md border border-white/15 bg-white/6 px-2 py-1 text-[11px] text-white/70 hover:bg-white/12"
            >
              {t["reapply.button"]}
            </button>
          </Row>
        </Section>

        {versao && (
          <Section title={t["section.version"]}>
            <div className="flex items-center justify-between gap-4">
              <span className="text-[12px] text-white/55">
                {t["version.installed"]}{" "}
                <strong className="text-white/85">{versao.current}</strong>
                {versao.available && versao.latest && (
                  <>
                    {" · "}
                    <strong className="text-emerald-300">{t["version.available"](versao.latest)}</strong>
                  </>
                )}
              </span>
              {versao.available && versao.canApply && (
                <button
                  onClick={async () => {
                    setAtualizando(true);
                    setErroUpdate(null);
                    try {
                      // Em caso de sucesso o app fecha e reabre sozinho; o
                      // que vem depois desta linha só roda se algo falhar.
                      await invoke("apply_update");
                    } catch (e) {
                      setErroUpdate(traduzirErro(t, e));
                      setAtualizando(false);
                    }
                  }}
                  disabled={atualizando}
                  className="shrink-0 rounded-md border border-emerald-400/40 bg-emerald-400/10 px-3 py-1 text-[12px] text-emerald-200 hover:bg-emerald-400/20 disabled:opacity-50"
                >
                  {atualizando ? t["version.updating"] : t["version.update"]}
                </button>
              )}
            </div>

            {erroUpdate && <p className="text-[11px] text-rose-400/80">{erroUpdate}</p>}

            {versao.available && !versao.canApply && (
              <p className="text-[11px] leading-relaxed text-white/40">
                {t["version.notFromScript"]}
              </p>
            )}
            {!versao.available && (
              <p className="text-[11px] text-white/30">{t["version.upToDate"]}</p>
            )}
          </Section>
        )}

        {cache && (
          <Section title={t["section.cache"]}>
            <p className="text-[12px] leading-relaxed text-white/55">
              <strong className="text-white/85">{cache.tracks}</strong>{" "}
              {t["cache.tracks"](cache.tracks)}
              {cache.synced > 0 && <> · {t["cache.synced"](cache.synced)}</>}
              {cache.pinned > 0 && <> · {t["cache.pinned"](cache.pinned)}</>}
              {cache.misses > 0 && <> · {t["cache.misses"](cache.misses)}</>}
            </p>
            <p className="text-[11px] leading-relaxed text-white/30">
              {t["cache.explanation"]}
            </p>
          </Section>
        )}

        <p className="pb-2 text-[11px] leading-relaxed text-white/30">
          {t["settings.footer"]}
        </p>
      </div>
    </div>
  );
}
