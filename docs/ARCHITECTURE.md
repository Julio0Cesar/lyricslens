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
| 5 | Overlay: definir por spike, não por aposta | Wayland é a parte de maior risco do projeto |

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
Windows: emite `PropertiesChanged`, então troca de faixa chega por evento — não
precisa polling. Só a posição precisa ser lida periodicamente.

Metadados úteis: `xesam:title`, `xesam:artist`, `xesam:album`, `xesam:url`,
`mpris:length`, `mpris:artUrl`.

**Risco conhecido:** nem todo player implementa `Position` com precisão. Chromium
reporta; Firefox historicamente não. Precisa ser medido na máquina alvo antes de
qualquer decisão de UI — se a fonte não dá posição confiável, aquela fonte cai para
modo de letra estática.

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
// a cada re-sync: se |real - estimado| > 250ms → re-ancora (seek detectado)
```

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
