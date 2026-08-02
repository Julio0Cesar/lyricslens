import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ColorPicker, Row, Section, Select, Slider, Toggle } from "./controls";
import LyricsPicker from "./LyricsPicker";
import { useSettings } from "./useSettings";

export default function SettingsWindow() {
  const { settings, update } = useSettings();
  const [fontes, setFontes] = useState<string[]>([]);

  useEffect(() => {
    invoke<string[]>("list_fonts").then(setFontes).catch(() => setFontes([]));
  }, []);

  if (!settings) {
    return <div className="p-6 text-sm text-white/40">carregando…</div>;
  }

  return (
    <div className="h-full overflow-y-auto bg-neutral-950 text-white">
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
          <Row label="Opacidade do fundo" hint="0 deixa o fundo invisível">
            <Slider
              value={settings.backgroundOpacity}
              min={0}
              max={1}
              step={0.05}
              onChange={(v) => update({ backgroundOpacity: v })}
            />
          </Row>
          <Row label="Desfoque" hint="borra o conteúdo atrás do painel">
            <Toggle value={settings.blur} onChange={(v) => update({ blur: v })} />
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
          <Row label="" hint="reaplica flutuar, fixar, tamanho e posição">
            <button
              onClick={() => invoke("apply_compositor_rules")}
              className="rounded-md border border-white/15 bg-white/6 px-2 py-1 text-[11px] text-white/70 hover:bg-white/12"
            >
              recolocar agora
            </button>
          </Row>
        </Section>

        <p className="pb-2 text-[11px] leading-relaxed text-white/30">
          As preferências ficam em <code>settings.json</code>, no diretório de dados do app. Em
          Wayland, tamanho e posição são pedidos ao compositor — a janela não decide por conta
          própria.
        </p>
      </div>
    </div>
  );
}
