# Arquitetura — LyricsLens

Documento vivo. Registra as decisões e o *porquê* delas, para não redecidir
a mesma coisa duas vezes.

## Princípio central

> O Rust é dono do estado e do relógio. O React só desenha.

Todo overlay de letras que consome muita CPU comete o mesmo erro: um `setInterval`
no frontend chamando o backend a cada 200ms. Aqui o backend emite eventos só quando
algo muda, e manda um tick de sincronização leve 1×/s. O frontend interpola entre os
ticks com `requestAnimationFrame`. Resultado: karaokê a 60fps com CPU perto de zero.

## Decisões tomadas

| # | Decisão | Motivo |
|---|---|---|
| 1 | Linux primeiro, mas com `trait MediaProvider` | Custo quase zero hoje; evita reescrever a detecção inteira se um dia houver Windows |
| 2 | LRCLIB como provider primário | Gratuito, sem auth, LRC sincronizado, boa cobertura. É o que o AlwaysOnLyrics (macOS) usa |
| 3 | Duas janelas separadas (`overlay` e `settings`) | A janela de overlay nunca carrega o CSS/JS de configuração — é o que mantém a RAM baixa |
| 4 | Tradução adiada, campo reservado no schema | Ver issue #1. Evita migração de cache depois |
| 5 | Overlay: janela flutuante + regras do compositor por padrão, `gtk-layer-shell` como modo opcional | Medido no spike: `alwaysOnTop` do Tauri é no-op em Wayland; float+pin resolve o uso normal mas não sobrevive a fullscreen |
| 6 | Relógio ancorado por borda da `Position` | Medido: reduz o erro de ±1000ms para ±71ms sem custo relevante |

## Resultado do spike de overlay (Hyprland v0.56.1)

| Teste | Resultado |
|---|---|
| `alwaysOnTop: true` do Tauri | **No-op.** A janela abriu tiled, ignorando o tamanho pedido |
| `hyprctl setfloating` + `pin` | Funciona; geometria exata aceita |
| `transparent: true` | Funciona — o desktop aparece através da janela |
| `decorations: false` | Funciona |
| Sobreviver a fullscreen | **Não.** Só `gtk-layer-shell` resolve |

Em Wayland o cliente não pode se posicionar nem se colocar acima dos outros: isso é
decisão do compositor, por design do protocolo. Daí a dupla estratégia.

## Camadas

```
src-tauri/src/
├── media/
│   ├── mod.rs          trait MediaProvider  ← a abstração que salva o projeto
│   ├── mpris.rs        impl Linux (zbus / D-Bus)
│   └── smtc.rs         impl Windows (windows-rs)          [futuro]
├── lyrics/
│   ├── mod.rs          trait LyricsProvider
│   ├── lrclib.rs       primário — LRC sincronizado
│   ├── netease.rs      fallback + traduções humanas       [issue #1]
│   ├── lrc.rs          parser/serializer LRC
│   └── normalize.rs    limpeza de títulos ("| HD", "[Official Video]", …)
├── store/
│   ├── cache.rs        SQLite: letras, traduções, capas (modo offline)
│   ├── mappings.rs     título bagunçado → track fixado pelo usuário
│   └── settings.rs
├── sync/engine.rs      relógio + correção de drift
├── overlay/
│   ├── window.rs       transparência, click-through
│   └── hypr.rs         regras do compositor via IPC
├── tray.rs
└── ipc.rs              single-instance + comandos de CLI (atalho global)
```

```
src/  (React)
├── overlay/            janela 1 — só a letra, transparente
├── settings/           janela 2 — config, biblioteca, resolução manual
└── shared/             tema, tokens de estilo, store
```

## Fluxo de dados

```
D-Bus PropertiesChanged ──▶ MediaProvider ──▶ TrackChanged
                                                  │
                                    cache hit? ───┼─── LRCLIB
                                                  ▼
                                            SyncEngine
                                                  │
        emit("lyrics:loaded")  +  emit("sync:tick") 1×/s
                                                  ▼
                                   React interpola em rAF → 60fps
```

## Detecção de mídia (Linux)

MPRIS via D-Bus (`org.mpris.MediaPlayer2.Player`). Vantagem sobre o equivalente do
Windows: emite `PropertiesChanged`, então troca de faixa chega por evento.

Na implementação isso aparece de um jeito discreto: o `zbus` mantém um cache das
propriedades alimentado pelo próprio `PropertiesChanged`. Ler `Metadata` a cada
500ms **não é uma ida ao barramento** — é leitura de cache já atualizado pelo sinal.
Só `Position` faz round-trip de verdade, porque por spec ela não emite sinal.

Metadados úteis: `xesam:title`, `xesam:artist`, `xesam:album`, `xesam:url`,
`mpris:length`, `mpris:artUrl`.

### Medido (Firefox + Spotify Web, Hyprland v0.56.1)

Metadados vêm limpos: `artist`, `title`, `album` e `length` corretos — o suficiente
para a busca exata no LRCLIB acertar de primeira.

`Position` **existe**, mas é **quantizada em 1 segundo** — a leitura crua tem até
1000ms de erro. A solução não é ler o valor, é **detectar a borda**:

> Quando o inteiro vira de 93 para 94, sabemos que a posição real é 94.000s
> *naquele instante de relógio*. Isso ancora o relógio com precisão de polling,
> não de quantização.

Medição real com poll a 100ms, relógio ancorado na primeira borda:

```
intervalo entre bordas: min=0.931s  max=1.036s
erro nas bordas seguintes: +36ms +70ms +1ms +36ms +71ms +2ms
```

Erro máximo de 71ms ao longo de 6s — invisível para karaokê de linha, que troca a
cada 2–5s. **Sincronia é viável nesta fonte.**

`mpris:artUrl` vem **vazio** no Firefox. Capa de álbum precisa de outra origem
(iTunes Search / Deezer / Cover Art Archive) ou simplesmente não aparece.

Nenhum `PropertiesChanged` periódico durante reprodução estável — o barramento fica
silencioso, então escutar sinal não gera ruído.

**Ainda não medido:** app do Spotify nativo, Chromium, e o comportamento do sinal
`Seeked`.

## Engine de sincronização

O problema não é achar a linha, é o **drift**: a posição chega com latência e o
usuário dá seek/pause.

```rust
struct Clock {
    anchor_pos_ms: u64,   // última posição confirmada pela fonte
    anchor_at: Instant,   // quando ela foi confirmada
    playing: bool,
    offset_ms: i64,       // ajuste manual do usuário, por player
}
// now()  = anchor_pos_ms + elapsed  (se playing) + offset_ms
// re-ancora em toda borda; se |real - estimado| > 250ms fora de borda → seek
```

**Ancoragem por borda, não por valor.** Poll a 100ms lendo `Position`; só re-ancora
quando o valor *muda*. Custo: uma chamada D-Bus a cada 100ms, que é barato — mas a
precisão passa de ±1000ms para ±100ms. Ver a seção de medição acima.

O `offset_ms` por player não é firula: cada fonte tem latência diferente, e é o que
salva a experiência quando a posição vem ruim.

Cadência: re-sync a cada ~1s (chamada D-Bus é barata), interpolação no frontend.
Para referência, o AlwaysOnLyrics usa 15Hz local + re-sync a cada 3s, mas sem
detecção de seek — o que deixa até 3s de posição errada depois que o usuário pula.

## Resolução de faixa (a feature mais valiosa)

Títulos de YouTube são caóticos: `Música | HD Ao Vivo`, `[Official Video]`,
`(Lyrics)`, `feat.`, canal como "artista". Pipeline:

1. Normalizar o título (remover ruído conhecido)
2. `GET /api/get` do LRCLIB com assinatura exata (artista + faixa + álbum + duração)
3. Falhou? `GET /api/search` e mostrar candidatos ao usuário
4. Usuário escolhe → salvar mapeamento `hash(título original) → track_id`
5. Da próxima vez, resolve sozinho

O passo 3–5 é o que nenhum concorrente faz bem.

## Parser LRC — requisitos

Além do básico `[mm:ss.xx]texto`:

- múltiplos timestamps na mesma linha (`[00:12][01:30]refrão`) — é LRC válido
- tags de metadado, principalmente `[offset:+500]` — ajuste grátis de sincronia
- **preservar linhas vazias**: intervalo instrumental precisa apagar a tela, não
  manter a última linha congelada

## Referências estudadas

- [xsaardo/alwaysonlyrics](https://github.com/xsaardo/alwaysonlyrics) — Swift/macOS,
  Spotify-only. Confirma LRCLIB e o relógio híbrido (interpolação local + re-sync).
  Não tem cache offline, resolução manual, nem normalização de título.
