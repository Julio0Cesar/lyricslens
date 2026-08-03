import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ColorPicker, Row, Section, Select, Slider, Toggle } from "./controls";
import HotkeyCapture from "./HotkeyCapture";
import LyricsPicker from "./LyricsPicker";
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
  const { settings, status, erro, update } = useSettings();
  const [fontes, setFontes] = useState<string[]>([]);
  const [cache, setCache] = useState<CacheStats | null>(null);
  const [versao, setVersao] = useState<UpdateInfo | null>(null);
  const [atualizando, setAtualizando] = useState(false);
  const [erroUpdate, setErroUpdate] = useState<string | null>(null);

  useEffect(() => {
    invoke<string[]>("list_fonts").then(setFontes).catch(() => setFontes([]));
    invoke<CacheStats>("cache_stats").then(setCache).catch(() => setCache(null));
    invoke<UpdateInfo>("check_update").then(setVersao).catch(() => setVersao(null));
  }, []);

  if (!settings) {
    return <div className="p-6 text-sm text-white/40">carregando…</div>;
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
          <h1 className="text-lg font-semibold">Configurações</h1>
          <button
            onClick={() => invoke("toggle_overlay")}
            className="rounded-md border border-white/15 bg-white/6 px-2 py-1 text-[11px] text-white/70 hover:bg-white/12"
          >
            mostrar / ocultar overlay
          </button>
        </header>

        {erro && (
          <p className="rounded border border-rose-500/30 bg-rose-500/10 px-3 py-2 text-[12px] text-rose-300">
            {erro}
          </p>
        )}

        <LyricsPicker />

        <Section title="Aparência">
          <Row label="Fonte" hint={fontes.length ? `${fontes.length} instaladas` : "do sistema"}>
            <Select
              value={settings.fontFamily}
              onChange={(v) => update({ fontFamily: v })}
              options={[
                { value: "", label: "Do sistema" },
                ...fontes.map((f) => ({ value: f, label: f })),
              ]}
            />
          </Row>
          <Row label="Tamanho da letra">
            <Slider
              value={settings.fontSize}
              min={10}
              max={96}
              suffix="px"
              onChange={(v) => update({ fontSize: v })}
            />
          </Row>
          <Row label="Peso">
            <Slider
              value={settings.fontWeight}
              min={100}
              max={900}
              step={100}
              onChange={(v) => update({ fontWeight: v })}
            />
          </Row>
          <Row label="Cor do texto">
            <ColorPicker value={settings.textColor} onChange={(v) => update({ textColor: v })} />
          </Row>
          <Row label="Cor do texto apagado" hint="linhas ainda não cantadas">
            <ColorPicker value={settings.dimColor} onChange={(v) => update({ dimColor: v })} />
          </Row>
          <Row label="Cor do fundo">
            <ColorPicker
              value={settings.backgroundColor}
              onChange={(v) => update({ backgroundColor: v })}
            />
          </Row>
          <Row
            label="Opacidade do fundo"
            hint="0 deixa o fundo invisível — o desfoque atrás vem do compositor"
          >
            <Slider
              value={settings.backgroundOpacity}
              min={0}
              max={1}
              step={0.05}
              onChange={(v) => update({ backgroundOpacity: v })}
            />
          </Row>
          <Row label="Arredondamento">
            <Slider
              value={settings.cornerRadius}
              min={0}
              max={64}
              suffix="px"
              onChange={(v) => update({ cornerRadius: v })}
            />
          </Row>
        </Section>

        <Section title="Conteúdo">
          <Row label="Linhas de contexto" hint="a anterior e a próxima">
            <Toggle
              value={settings.showContextLines}
              onChange={(v) => update({ showContextLines: v })}
            />
          </Row>
          <Row label="Informação da faixa" hint="fonte, artista e título">
            <Toggle value={settings.showTrackInfo} onChange={(v) => update({ showTrackInfo: v })} />
          </Row>
          <Row label="Barra de progresso">
            <Toggle value={settings.showProgress} onChange={(v) => update({ showProgress: v })} />
          </Row>
          <Row label="Varredura de karaokê" hint="acende a linha conforme ela é cantada">
            <Toggle value={settings.karaoke} onChange={(v) => update({ karaoke: v })} />
          </Row>
          <Row label="Alinhamento">
            <Select
              value={settings.textAlign}
              onChange={(v) => update({ textAlign: v })}
              options={[
                { value: "left", label: "À esquerda" },
                { value: "center", label: "Centralizado" },
              ]}
            />
          </Row>
        </Section>

        <Section title="Comportamento">
          <Row label="Atalho global" hint="mostra e esconde o overlay">
            <HotkeyCapture
              value={settings.hotkey}
              onChange={(v) => update({ hotkey: v })}
            />
          </Row>
          <Row label="Cliques atravessam" hint="o overlay deixa de responder ao mouse">
            <Toggle value={settings.clickThrough} onChange={(v) => update({ clickThrough: v })} />
          </Row>
          <Row label="Esconder quando pausado">
            <Toggle
              value={settings.hideWhenPaused}
              onChange={(v) => update({ hideWhenPaused: v })}
            />
          </Row>
          <Row label="Ajuste de sincronia" hint="negativo adianta a letra">
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

        <Section title="Janela">
          <Row label="Largura">
            <Slider
              value={settings.width}
              min={240}
              max={1920}
              step={10}
              suffix="px"
              onChange={(v) => update({ width: v })}
            />
          </Row>
          <Row label="Altura">
            <Slider
              value={settings.height}
              min={80}
              max={900}
              step={10}
              suffix="px"
              onChange={(v) => update({ height: v })}
            />
          </Row>
          <Row label="Distância do rodapé">
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
            label="Posição"
            hint={
              settings.positionX !== null
                ? `fixada em ${settings.positionX}, ${settings.positionY}`
                : "rodapé central · arraste o overlay para mudar"
            }
          >
            <button
              onClick={() => invoke("center_overlay")}
              disabled={settings.positionX === null}
              className="rounded-md border border-white/15 bg-white/6 px-2 py-1 text-[11px] text-white/70 hover:bg-white/12 disabled:opacity-40"
            >
              centralizar
            </button>
          </Row>
          <Row
            label="Camada do compositor"
            hint={
              // Quando a camada não sobe, a opção aparecia ligada enquanto na
              // prática não estava em uso, e nada indicava o porquê.
              settings.layerShell && status && !status.layerShellActive
                ? `ligada, mas sem efeito nesta sessão — ${status.layerShellFallback ?? "indisponível"}`
                : "fica acima até de tela cheia, mas deixa de ser arrastável · exige reiniciar o app"
            }
          >
            <Toggle value={settings.layerShell} onChange={(v) => update({ layerShell: v })} />
          </Row>
          <Row label="" hint="reaplica flutuar, fixar, tamanho e posição">
            <button
              onClick={() => invoke("apply_compositor_rules")}
              className="rounded-md border border-white/15 bg-white/6 px-2 py-1 text-[11px] text-white/70 hover:bg-white/12"
            >
              recolocar agora
            </button>
          </Row>
        </Section>

        {versao && (
          <Section title="Versão">
            <div className="flex items-center justify-between gap-4">
              <span className="text-[12px] text-white/55">
                Instalada: <strong className="text-white/85">{versao.current}</strong>
                {versao.available && versao.latest && (
                  <>
                    {" · "}
                    <strong className="text-emerald-300">{versao.latest} disponível</strong>
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
                      setErroUpdate(String(e));
                      setAtualizando(false);
                    }
                  }}
                  disabled={atualizando}
                  className="shrink-0 rounded-md border border-emerald-400/40 bg-emerald-400/10 px-3 py-1 text-[12px] text-emerald-200 hover:bg-emerald-400/20 disabled:opacity-50"
                >
                  {atualizando ? "atualizando…" : "atualizar e reabrir"}
                </button>
              )}
            </div>

            {erroUpdate && <p className="text-[11px] text-rose-400/80">{erroUpdate}</p>}

            {versao.available && !versao.canApply && (
              <p className="text-[11px] leading-relaxed text-white/40">
                Esta instalação não veio do script, então a atualização é pelo mesmo caminho que
                você usou para instalar.
              </p>
            )}
            {!versao.available && (
              <p className="text-[11px] text-white/30">Esta é a versão mais recente.</p>
            )}
          </Section>
        )}

        {cache && (
          <Section title="Cache">
            <p className="text-[12px] leading-relaxed text-white/55">
              <strong className="text-white/85">{cache.tracks}</strong>{" "}
              {cache.tracks === 1 ? "faixa guardada" : "faixas guardadas"}
              {cache.synced > 0 && <> · {cache.synced} com sincronia</>}
              {cache.pinned > 0 && <> · {cache.pinned} fixadas para offline</>}
              {cache.misses > 0 && <> · {cache.misses} sem letra conhecida</>}
            </p>
            <p className="text-[11px] leading-relaxed text-white/30">
              Música que já tocou não vai à rede de novo. Faixa sem letra também é lembrada, para
              a busca não ser refeita a cada vez — esse registro caduca em três dias, porque letra
              nova entra no LRCLIB o tempo todo.
            </p>
          </Section>
        )}

        <p className="pb-2 text-[11px] leading-relaxed text-white/30">
          As preferências ficam em <code>settings.json</code>, no diretório de dados do app. Em
          Wayland, tamanho e posição são pedidos ao compositor — a janela não decide por conta
          própria.
        </p>
      </div>
    </div>
  );
}
