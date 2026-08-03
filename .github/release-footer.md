---

### Instalação

**Linux · x86_64 · Wayland ou X11.** Windows ainda não ([#5](https://github.com/Julio0Cesar/lyricslens/issues/5)).

| Sistema | Arquivo | Comando |
|---|---|---|
| Debian, Ubuntu, Mint | `lyricslens-x86_64.deb` | `sudo apt install ./lyricslens-x86_64.deb` |
| Fedora, openSUSE | `lyricslens-x86_64.rpm` | `sudo dnf install ./lyricslens-x86_64.rpm` |
| Arch, NixOS, sem sudo | `lyricslens-x86_64.AppImage` | `curl -fsSL https://raw.githubusercontent.com/Julio0Cesar/lyricslens/main/install.sh \| sh` |

Use `apt install ./arquivo.deb` e `dnf install ./arquivo.rpm`, não `dpkg -i` nem
`rpm -i`: só os primeiros resolvem as dependências.

Para conferir o download antes de instalar:

```bash
curl -LO https://github.com/Julio0Cesar/lyricslens/releases/latest/download/SHA256SUMS
sha256sum -c SHA256SUMS --ignore-missing
```

[Documentação completa](https://github.com/Julio0Cesar/lyricslens#instalação)
