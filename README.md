<div align="center">

<img src="assets/logo.svg" alt="LyricsLens" width="96">

# LyricsLens

**Letras sincronizadas sobre qualquer aplicativo.**

Detecta o que está tocando no sistema, busca a letra e a mostra numa janela
flutuante — acompanhando a música palavra por palavra.

[![CI](https://github.com/Julio0Cesar/lyricslens/actions/workflows/ci.yml/badge.svg)](https://github.com/Julio0Cesar/lyricslens/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Julio0Cesar/lyricslens?label=vers%C3%A3o)](https://github.com/Julio0Cesar/lyricslens/releases)
[![Licença](https://img.shields.io/github/license/Julio0Cesar/lyricslens)](LICENSE)

</div>

---

## Instalação

```bash
curl -fsSL https://raw.githubusercontent.com/Julio0Cesar/lyricslens/main/install.sh | sh
```

Instala em `~/.local`, sem sudo. Depois é só apertar **Super** e procurar por
*LyricsLens*.

Para remover:

```bash
curl -fsSL https://raw.githubusercontent.com/Julio0Cesar/lyricslens/main/install.sh | sh -s -- --remove
```

<details>
<summary>Pacotes da distribuição</summary>

Cada release traz `.deb`, `.rpm` e `.AppImage` em
[Releases](https://github.com/Julio0Cesar/lyricslens/releases/latest).

```bash
# Debian, Ubuntu e derivados
sudo dpkg -i lyricslens-x86_64.deb

# Fedora, openSUSE e derivados
sudo rpm -i lyricslens-x86_64.rpm
```

O AppImage funciona direto, mas exige `libfuse2` para ser montado — o script de
instalação evita isso extraindo o conteúdo.

</details>

## Uso

O app vive na bandeja do sistema. Fechar a janela **esconde** o overlay; sair é
decisão explícita, pelo menu da bandeja.

| Ação | Como |
|---|---|
| Mostrar / esconder | Clique no ícone da bandeja, ou o atalho global |
| Abrir configurações | **Duplo clique** no overlay |
| Fechar configurações | **Duplo clique** fora dos controles |
| Mover o overlay | Arraste |
| Corrigir a letra errada | Configurações → *Letra desta faixa* |

### Atalho global

Em **Configurações → Comportamento → Atalho global**, pressione a combinação que
quiser.

Wayland não deixa um aplicativo registrar um atalho de sistema — quem registra é
o compositor. No Hyprland o app pede isso via `hyprctl`, apontando de volta para
o próprio executável. Nada é escrito na sua configuração: o compositor esquece o
atalho ao reiniciar e o app o reaplica toda vez que sobe.

Em outros ambientes, use o keybind do seu sistema chamando:

```bash
lyricslens toggle     # mostra ou esconde
lyricslens settings   # abre as configurações
lyricslens hide
lyricslens            # mostra
```

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

Duas decisões carregam o resto do projeto, e as duas vieram de medição, não de
palpite — o raciocínio está em [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md):

- **A posição da música é ancorada na *borda*, não na leitura.** Várias fontes
  reportam a posição arredondada para o segundo. Ancorar no instante em que o
  valor vira derruba o erro de ±1000ms para ±71ms.
- **Em Wayland, quem manda na janela é o compositor.** Ficar por cima, escolher
  posição, registrar atalho — nada disso o aplicativo pode fazer sozinho. Tudo
  passa por pedido ao compositor.

## Requisitos

- Linux com Wayland ou X11 — o posicionamento automático hoje conhece o Hyprland
- `webkit2gtk-4.1`, `gtk3`, `libayatana-appindicator`
- Um player que exponha MPRIS: Spotify, Chromium, Firefox, VLC…

## Status

| Área | Estado |
|---|---|
| Detecção de mídia (MPRIS) | pronto |
| Letras sincronizadas (LRCLIB) | pronto |
| Overlay always-on-top | pronto, inclusive sobre tela cheia |
| Bandeja + atalho global | pronto |
| Cache local (SQLite) | pronto |
| Janela de configurações | pronto |
| Escolha manual da letra | pronto |
| Modo offline | [#2](https://github.com/Julio0Cesar/lyricslens/issues/2) |
| Capa do álbum | [#3](https://github.com/Julio0Cesar/lyricslens/issues/3) |
| Tradução | [#1](https://github.com/Julio0Cesar/lyricslens/issues/1) |
| Windows | [#5](https://github.com/Julio0Cesar/lyricslens/issues/5) |

## Desenvolvimento

```bash
pnpm install
pnpm tauri dev
```

```bash
cd src-tauri && cargo test    # 48 testes
pnpm exec tsc --noEmit
```

### Versionamento

As versões saem sozinhas dos commits, seguindo
[Conventional Commits](https://www.conventionalcommits.org/pt-br/):

| Prefixo | Efeito em `X.Y.Z` |
|---|---|
| `fix:` | `Z` |
| `feat:` | `Y` |
| `feat!:` ou `BREAKING CHANGE:` | `X` |
| `docs:`, `chore:`, `test:` | nenhum |

Ao entrar na `main`, o `release-please` abre um PR de release com o CHANGELOG e
a versão nova. Quando esse PR é aprovado, a tag é criada e os pacotes são
construídos e publicados automaticamente.

## Licença

[MIT](LICENSE)
