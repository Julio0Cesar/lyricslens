# Changelog

## [0.10.1](https://github.com/Julio0Cesar/lyricslens/compare/v0.10.0...v0.10.1) (2026-08-04)


### Correções

* **rede:** não entrar em pânico quando o sistema não tem certificados ([#79](https://github.com/Julio0Cesar/lyricslens/issues/79)) ([a0c3446](https://github.com/Julio0Cesar/lyricslens/commit/a0c3446c56c96c80af85b8dbb9b2449ece2edefc))

## [0.10.0](https://github.com/Julio0Cesar/lyricslens/compare/v0.9.0...v0.10.0) (2026-08-04)


### Funcionalidades

* **aur:** PKGBUILD apontando para o tarball da release ([#76](https://github.com/Julio0Cesar/lyricslens/issues/76)) ([715fa05](https://github.com/Julio0Cesar/lyricslens/commit/715fa059ee79b0293d41be878e1b5583af1a7e28))

## [0.9.0](https://github.com/Julio0Cesar/lyricslens/compare/v0.8.0...v0.9.0) (2026-08-04)


### Funcionalidades

* **overlay:** trait Compositor, com Hyprland extraído e Sway novo ([#74](https://github.com/Julio0Cesar/lyricslens/issues/74)) ([82b2fa2](https://github.com/Julio0Cesar/lyricslens/commit/82b2fa2ae00fcb4a50420ecce8157c7b9fc2096d))

## [0.8.0](https://github.com/Julio0Cesar/lyricslens/compare/v0.7.0...v0.8.0) (2026-08-04)


### Funcionalidades

* **install:** tarball de 8MB para quem já tem WebKit e GTK ([#72](https://github.com/Julio0Cesar/lyricslens/issues/72)) ([3a60958](https://github.com/Julio0Cesar/lyricslens/commit/3a60958086a5be03cdbc61941bf1488544338a27))

## [0.7.0](https://github.com/Julio0Cesar/lyricslens/compare/v0.6.0...v0.7.0) (2026-08-04)


### Funcionalidades

* **capa:** capa do álbum, do player ou do Deezer ([#70](https://github.com/Julio0Cesar/lyricslens/issues/70)) ([df3817e](https://github.com/Julio0Cesar/lyricslens/commit/df3817e933a03572e5037e7e07a2b8d700ae097e))

## [0.6.0](https://github.com/Julio0Cesar/lyricslens/compare/v0.5.0...v0.6.0) (2026-08-04)


### Funcionalidades

* **log:** arquivo de log e erros que o usuário pode resolver na tela ([#65](https://github.com/Julio0Cesar/lyricslens/issues/65)) ([1f82f1f](https://github.com/Julio0Cesar/lyricslens/commit/1f82f1f227acbaddc3f131504554880843769aba))

## [0.5.0](https://github.com/Julio0Cesar/lyricslens/compare/v0.4.0...v0.5.0) (2026-08-04)


### Funcionalidades

* **i18n:** interface em inglês, seguindo o idioma do sistema ([#62](https://github.com/Julio0Cesar/lyricslens/issues/62)) ([420f5ce](https://github.com/Julio0Cesar/lyricslens/commit/420f5ce597d8ee29b7a5ad3e48ce8dbb4ed95664))


### Correções

* **overlay:** altura padrão dimensionada para o que o overlay mostra ([#47](https://github.com/Julio0Cesar/lyricslens/issues/47)) ([5d70f72](https://github.com/Julio0Cesar/lyricslens/commit/5d70f72b274fd96412059a4e5efd91972b7dab8b))
* **pacotes:** declarar gtk-layer-shell e só publicar release com pacotes ([#46](https://github.com/Julio0Cesar/lyricslens/issues/46)) ([34e89bd](https://github.com/Julio0Cesar/lyricslens/commit/34e89bd0743a32c46f22b385669f06f13fef5ddc))
* **release:** parar o loop de releases e pôr o portão antes da publicação ([#59](https://github.com/Julio0Cesar/lyricslens/issues/59)) ([0d20517](https://github.com/Julio0Cesar/lyricslens/commit/0d20517c84113c671fe355a12fa1cdd9c0aa3b7f))


### Desempenho

* **ci:** enxugar e instrumentar o teste de fumaça do .deb ([#60](https://github.com/Julio0Cesar/lyricslens/issues/60)) ([1b945dd](https://github.com/Julio0Cesar/lyricslens/commit/1b945dde12bb86f36a540dd3677dae4c383b9e77))
* **lrclib:** reaproveitar a conexão entre faixas em vez de pré-aquecer ([#61](https://github.com/Julio0Cesar/lyricslens/issues/61)) ([997d7a1](https://github.com/Julio0Cesar/lyricslens/commit/997d7a1494bc7b331fe9cc854ff610b3aeabf307))

## [0.4.0](https://github.com/Julio0Cesar/lyricslens/compare/v0.3.0...v0.4.0) (2026-08-03)


### Funcionalidades

* **comportamento:** opção de iniciar com a sessão ([e642c01](https://github.com/Julio0Cesar/lyricslens/commit/e642c012027dbe4822ebd1015f9946340fa8d2d7))
* **offline:** botão para manter a letra offline e lista das guardadas ([c74857a](https://github.com/Julio0Cesar/lyricslens/commit/c74857a1ae953536cbf59af2be673d04f4cc7974))

## [0.3.0](https://github.com/Julio0Cesar/lyricslens/compare/v0.2.1...v0.3.0) (2026-08-03)


### Funcionalidades

* atalho configurável na UI, estatísticas do cache e duplo clique para fechar ([10542f4](https://github.com/Julio0Cesar/lyricslens/commit/10542f4cc7d354979729af4dbaa0d05aed186451))
* aviso de versão nova e atualização pelo app ([97e8d31](https://github.com/Julio0Cesar/lyricslens/commit/97e8d318649ce9ea26ad22d5bdb9e6bc36ef851d))
* bandeja, instância única e atalho global ([1e2a9b8](https://github.com/Julio0Cesar/lyricslens/commit/1e2a9b8f933d916b780a05cdeba624cc5c454c6a))
* busca, cache e renderização de letras sincronizadas ([e23f53b](https://github.com/Julio0Cesar/lyricslens/commit/e23f53b7aa3d82b88e610cbdc98ce284d07a3336))
* detecção de mídia via MPRIS com relógio ancorado por borda ([9602d45](https://github.com/Julio0Cesar/lyricslens/commit/9602d45b817135b994318be81e4bd171665fac3d))
* empacotamento, instalador e esteira de release ([03db9c7](https://github.com/Julio0Cesar/lyricslens/commit/03db9c7f1e8d33bbed28df6378e48cad66c93ad4))
* escolha manual da letra quando a busca automática erra ([a85e5fb](https://github.com/Julio0Cesar/lyricslens/commit/a85e5fba5e3a546fbb52a749fa22ef4d61443089))
* janela de configurações e preferências persistentes ([4f3ef6e](https://github.com/Julio0Cesar/lyricslens/commit/4f3ef6e662b128026d2f6a26424926ad51ff0bfc))
* overlay como camada do compositor, acima até de tela cheia ([a2610a9](https://github.com/Julio0Cesar/lyricslens/commit/a2610a9a991c7ec7e5f049a4f12a93462a3941c4))
* overlay focado só na letra, com transição animada ([7cbd044](https://github.com/Julio0Cesar/lyricslens/commit/7cbd044b6d6a4c06c060589e3305575ce4116cda))


### Correções

* atalho global apontava para o binário interno do AppImage ([d0ddd4a](https://github.com/Julio0Cesar/lyricslens/commit/d0ddd4ab1f65f5232e7312789342f37beec1b205))
* centralizar o overlay perguntando a geometria ao compositor ([695ac02](https://github.com/Julio0Cesar/lyricslens/commit/695ac020e89ef86358a1a83db128bb89a3f71506))
* fundo do overlay com opacidade e cor corretas ([d5114f6](https://github.com/Julio0Cesar/lyricslens/commit/d5114f6fb1310997a437db9c9dc7f08016cd9d79))
* impedir seleção do texto no overlay ([6655b26](https://github.com/Julio0Cesar/lyricslens/commit/6655b263d8b85b67fc8a50811ce8e80b659dbc81))
* **install:** conferir integridade e dependências, e descartar libwayland ([63bb38d](https://github.com/Julio0Cesar/lyricslens/commit/63bb38d85682c4133ab2ca69a3509d437e1d256c))
* invólucro em vez de symlink para o AppRun no instalador ([dcae2c9](https://github.com/Julio0Cesar/lyricslens/commit/dcae2c959598310dbbd2a159fd2679447185e1eb))
* overlay arrastável com posição lembrada, e sem menu de contexto ([3e41ac1](https://github.com/Julio0Cesar/lyricslens/commit/3e41ac151bb70ecd88e91a036d699fec6c413561))
* overlay em branco e duplo clique que não abria as configurações ([b4896c5](https://github.com/Julio0Cesar/lyricslens/commit/b4896c5d2d767c1e42758ad357f2d85124047907))
* **overlay:** separar preferência de estado efetivo e corrigir a geometria da janela ([d3f99ca](https://github.com/Julio0Cesar/lyricslens/commit/d3f99ca456ec32265d3dbe52531ef297f5ca1d68))
* **release:** não empacotar a pilha gráfica no AppImage e testar o pacote antes de publicar ([a161d4c](https://github.com/Julio0Cesar/lyricslens/commit/a161d4c570d15f935179595e075918a286d6cf8a))
* tempo limite de busca e classe da janela no atalho do menu ([495efb1](https://github.com/Julio0Cesar/lyricslens/commit/495efb1a44f322d7367058c9b0c0dc480138ee7a))


### Desempenho

* **overlay:** renderizar só quando a tela muda, e testar as funções puras ([1881987](https://github.com/Julio0Cesar/lyricslens/commit/188198723f3cadccbd0268511def5f6d76932662))


### Documentação

* README em inglês, instalação por distro e compatibilidade explícita ([e65faff](https://github.com/Julio0Cesar/lyricslens/commit/e65faff307628104607ab01397f3cceb6b248f99))
* registrar medições da fase 0 (MPRIS, sync, overlay) ([d92efd5](https://github.com/Julio0Cesar/lyricslens/commit/d92efd5a92e629f38233b6d5ad6d4dc9b6e33259))

## [0.2.1](https://github.com/Julio0Cesar/lyricslens/compare/lyricslens-v0.2.0...lyricslens-v0.2.1) (2026-08-03)


### Correções

* atalho global apontava para o binário interno do AppImage ([d0ddd4a](https://github.com/Julio0Cesar/lyricslens/commit/d0ddd4ab1f65f5232e7312789342f37beec1b205))

## [0.2.0](https://github.com/Julio0Cesar/lyricslens/compare/lyricslens-v0.1.0...lyricslens-v0.2.0) (2026-08-03)


### Funcionalidades

* atalho configurável na UI, estatísticas do cache e duplo clique para fechar ([10542f4](https://github.com/Julio0Cesar/lyricslens/commit/10542f4cc7d354979729af4dbaa0d05aed186451))
* bandeja, instância única e atalho global ([1e2a9b8](https://github.com/Julio0Cesar/lyricslens/commit/1e2a9b8f933d916b780a05cdeba624cc5c454c6a))
* busca, cache e renderização de letras sincronizadas ([e23f53b](https://github.com/Julio0Cesar/lyricslens/commit/e23f53b7aa3d82b88e610cbdc98ce284d07a3336))
* detecção de mídia via MPRIS com relógio ancorado por borda ([9602d45](https://github.com/Julio0Cesar/lyricslens/commit/9602d45b817135b994318be81e4bd171665fac3d))
* empacotamento, instalador e esteira de release ([03db9c7](https://github.com/Julio0Cesar/lyricslens/commit/03db9c7f1e8d33bbed28df6378e48cad66c93ad4))
* escolha manual da letra quando a busca automática erra ([a85e5fb](https://github.com/Julio0Cesar/lyricslens/commit/a85e5fba5e3a546fbb52a749fa22ef4d61443089))
* janela de configurações e preferências persistentes ([4f3ef6e](https://github.com/Julio0Cesar/lyricslens/commit/4f3ef6e662b128026d2f6a26424926ad51ff0bfc))
* overlay como camada do compositor, acima até de tela cheia ([a2610a9](https://github.com/Julio0Cesar/lyricslens/commit/a2610a9a991c7ec7e5f049a4f12a93462a3941c4))
* overlay focado só na letra, com transição animada ([7cbd044](https://github.com/Julio0Cesar/lyricslens/commit/7cbd044b6d6a4c06c060589e3305575ce4116cda))


### Correções

* centralizar o overlay perguntando a geometria ao compositor ([695ac02](https://github.com/Julio0Cesar/lyricslens/commit/695ac020e89ef86358a1a83db128bb89a3f71506))
* invólucro em vez de symlink para o AppRun no instalador ([dcae2c9](https://github.com/Julio0Cesar/lyricslens/commit/dcae2c959598310dbbd2a159fd2679447185e1eb))
* overlay arrastável com posição lembrada, e sem menu de contexto ([3e41ac1](https://github.com/Julio0Cesar/lyricslens/commit/3e41ac151bb70ecd88e91a036d699fec6c413561))
* overlay em branco e duplo clique que não abria as configurações ([b4896c5](https://github.com/Julio0Cesar/lyricslens/commit/b4896c5d2d767c1e42758ad357f2d85124047907))
* tempo limite de busca e classe da janela no atalho do menu ([495efb1](https://github.com/Julio0Cesar/lyricslens/commit/495efb1a44f322d7367058c9b0c0dc480138ee7a))


### Documentação

* registrar medições da fase 0 (MPRIS, sync, overlay) ([d92efd5](https://github.com/Julio0Cesar/lyricslens/commit/d92efd5a92e629f38233b6d5ad6d4dc9b6e33259))
