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

## Uso

O app vive na bandeja do sistema. Fechar a janela **esconde** o overlay; sair é
decisão explícita, pelo menu da bandeja.

- **clique esquerdo** no ícone — mostra/esconde o overlay
- **clique direito** — menu com recolocar e sair

### Atalho global

Wayland não deixa um aplicativo registrar um atalho de sistema — quem registra é
o compositor. O app aceita comandos na linha de comando e os entrega para a
instância que já está rodando:

```bash
lyricslens toggle   # mostra ou esconde
lyricslens hide
lyricslens          # mostra
```

No Hyprland, adicione em `~/.config/hypr/hyprland/keybinds.conf`:

```conf
bind = Super, L, exec, lyricslens toggle
```

Em desenvolvimento, use o caminho do binário:
`src-tauri/target/debug/lyricslens toggle`.

### Posicionamento

O Wayland também não deixa a janela escolher onde fica nem ficar por cima —
quem decide é o compositor. No Hyprland o app pede isso via IPC ao aparecer, e
o overlay se coloca no rodapé central. Em outros compositores a janela abre
onde o sistema mandar; use "recolocar" depois de mover.

## Status

| Área | Estado |
|---|---|
| Detecção de mídia (MPRIS) | **pronto** |
| Letras sincronizadas (LRCLIB) | **pronto** |
| Overlay always-on-top | **pronto**, inclusive sobre tela cheia |
| Tray + atalho global | **pronto** |
| Cache local (SQLite) | **pronto** |
| Janela de configurações | **pronto** |
| Escolha manual da letra | **pronto** |
| Modo offline | parcial — cache pronto, UI pendente |
| Tradução | adiado — ver issues |

## Licença

A definir.
