# LyricsLens

Overlay de letras sincronizadas para o desktop. Detecta o que está tocando no
sistema, busca a letra e mostra numa janela flutuante estilo legenda — sobre
qualquer aplicativo.

> Em desenvolvimento. Alvo da v1: Linux (Wayland/Hyprland).

## Como funciona

```
MPRIS (D-Bus) ──▶ detecção da faixa ──▶ busca da letra (LRCLIB) ──▶ cache local
                                                                       │
                                                    engine de sync ────┘
                                                           │
                                                    overlay (React)
```

O backend em Rust é dono do estado e do relógio; o frontend apenas desenha e
interpola entre os ticks. Isso mantém o consumo próximo de zero.

## Stack

- **Tauri 2** — shell nativo, tray, janelas
- **Rust** — detecção de mídia, providers de letra, cache, sincronização
- **React 19 + TypeScript + Vite** — overlay e configurações

## Desenvolvimento

```bash
pnpm install
pnpm tauri dev
```

### Requisitos (Linux)

- `webkit2gtk-4.1`
- `libayatana-appindicator` (ícone na bandeja)
- Um player que exponha MPRIS (Spotify, Chromium, VLC, …)

## Status

| Área | Estado |
|---|---|
| Detecção de mídia (MPRIS) | **pronto** |
| Letras sincronizadas (LRCLIB) | **pronto** |
| Overlay always-on-top | em investigação |
| Tray + atalho global | planejado |
| Cache local (SQLite) | **pronto** |
| Modo offline | parcial — cache pronto, UI pendente |
| Tradução | adiado — ver issues |

## Licença

A definir.
