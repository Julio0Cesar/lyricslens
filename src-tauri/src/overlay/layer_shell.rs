//! Overlay como camada do compositor (`wlr-layer-shell`).
//!
//! É o único jeito de o overlay sobreviver a uma janela em tela cheia: uma
//! janela comum, por mais flutuante e fixada que esteja, fica abaixo do
//! fullscreen — o compositor decide assim por design. Uma superfície na camada
//! `Overlay` fica acima de tudo, inclusive de jogos.
//!
//! O preço é que ela **precisa ser inicializada antes de a janela ser
//! realizada**. Depois disso o tipo da superfície já está definido no
//! protocolo e não há como trocar — por isso a opção só vale ao iniciar o app.

use gtk::gdk;
use gtk::gdk::prelude::MonitorExt;
use gtk::glib::prelude::ObjectExt;
use gtk::prelude::WidgetExt;
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use tauri::WebviewWindow;

use super::compositor::Retangulo;
use super::Geometry;

/// O backend que o GTK realmente abriu, pelo tipo concreto do `GdkDisplay`.
///
/// Não dá para deduzir isso das variáveis de ambiente: sob XWayland a sessão é
/// Wayland, `WAYLAND_DISPLAY` está definido, e ainda assim o cliente fala X11.
/// Quem sabe a verdade é o display que o GTK abriu.
fn backend(window: &WebviewWindow) -> Option<String> {
    let gtk_window = window.gtk_window().ok()?;
    Some(gtk_window.display().type_().name().to_string())
}

/// Por que a camada não está disponível.
///
/// `gtk_layer_is_supported()` só sabe dizer "não". Dizer *por quê* importa: a
/// causa mais comum não é o compositor, e culpá-lo manda o usuário atrás da
/// coisa errada — possivelmente abrir issue no projeto do compositor em vez de
/// aqui. Ver #38.
fn motivo_indisponivel(window: &WebviewWindow) -> String {
    match backend(window).as_deref() {
        Some("GdkX11Display") if std::env::var_os("WAYLAND_DISPLAY").is_some() => {
            "o app está rodando sob XWayland, e cliente X11 não fala protocolo Wayland — \
             wlr-layer-shell não existe aí. Não é limitação do seu compositor; \
             se você instalou pelo AppImage, ver a #31"
                .into()
        }
        Some("GdkX11Display") => {
            "a sessão é X11 e wlr-layer-shell é um protocolo Wayland — não existe aí".into()
        }
        Some("GdkWaylandDisplay") => "o compositor não implementa wlr-layer-shell".into(),
        Some(outro) => {
            format!("backend gráfico inesperado ({outro}); wlr-layer-shell indisponível")
        }
        None => "não consegui identificar o backend gráfico; wlr-layer-shell indisponível".into(),
    }
}

/// Prepara a janela como camada. Tem que ser chamado antes do primeiro `show`.
pub fn init(window: &WebviewWindow, geo: Geometry) -> Result<(), String> {
    if !gtk_layer_shell::is_supported() {
        return Err(motivo_indisponivel(window));
    }

    let gtk_window = window
        .gtk_window()
        .map_err(|e| format!("sem acesso à janela GTK: {e}"))?;

    gtk_window.init_layer_shell();
    gtk_window.set_layer(Layer::Overlay);
    // O namespace é como o compositor identifica a camada em regras próprias.
    gtk_window.set_namespace(super::compositor::NAMESPACE);

    // Ancorar só embaixo mantém a largura definida por nós; ancorar nos dois
    // lados horizontais faria a camada esticar de ponta a ponta da tela.
    gtk_window.set_anchor(Edge::Bottom, true);
    posicionar(&gtk_window, geo);

    // -1 significa "não reserve espaço para mim": sem isso o compositor
    // encolheria as outras janelas para caber o overlay, como faz com uma
    // barra de tarefas.
    gtk_window.set_exclusive_zone(-1);

    // Sem isso a camada roubaria o teclado do que estiver em foco.
    gtk_window.set_keyboard_mode(KeyboardMode::None);

    gtk_window.set_size_request(geo.width, geo.height);
    Ok(())
}

/// Põe a camada onde as preferências mandam.
///
/// Numa layer surface o cliente não pede "mover janela" ao compositor — quem
/// decide a posição são as âncoras e as margens da própria superfície. É por
/// isso que dá para arrastar aqui sem protocolo novo: é aritmética local. Ver
/// #35.
///
/// Sem âncora horizontal o compositor centraliza, que é o padrão desejado; a
/// âncora à esquerda só entra quando o usuário arrastou e existe uma posição
/// escolhida para respeitar.
fn posicionar(gtk_window: &gtk::ApplicationWindow, geo: Geometry) {
    gtk_window.set_layer_shell_margin(Edge::Bottom, geo.margin_bottom);

    match geo.layer_margin_left {
        Some(esquerda) => {
            gtk_window.set_anchor(Edge::Left, true);
            gtk_window.set_layer_shell_margin(Edge::Left, esquerda);
        }
        None => {
            gtk_window.set_anchor(Edge::Left, false);
            gtk_window.set_layer_shell_margin(Edge::Left, 0);
        }
    }
}

/// Ajusta o que dá para ajustar com a camada já criada.
pub fn update_geometry(window: &WebviewWindow, geo: Geometry) {
    let Ok(gtk_window) = window.gtk_window() else {
        return;
    };
    posicionar(&gtk_window, geo);
    gtk_window.set_size_request(geo.width, geo.height);
}

/// O monitor do GDK cuja área é `retangulo`.
///
/// O casamento é por geometria, não por nome: o nome que o compositor usa
/// (`HDMI-A-1`) e o que o GDK usa não são garantidamente o mesmo, e a área é o
/// que os dois medem igual.
fn monitor_gdk(window: &gtk::ApplicationWindow, retangulo: Retangulo) -> Option<gdk::Monitor> {
    let display = gtk::prelude::WidgetExt::display(window);
    let (x, y, largura, altura) = retangulo;
    (0..display.n_monitors()).find_map(|i| {
        let m = display.monitor(i)?;
        let g = m.geometry();
        (g.x() == x && g.y() == y && g.width() == largura && g.height() == altura).then_some(m)
    })
}

/// Move a camada durante o arraste, sem tocar no disco.
///
/// Separado do `update_geometry` porque o arraste manda dezenas de posições por
/// segundo e não pode arrastar junto o `set_size_request` — pedir tamanho a
/// cada quadro faz o GTK renegociar a superfície inteira à toa.
///
/// A troca de monitor só é pedida quando muda de verdade. Uma layer surface
/// pertence a um output, e refazer esse vínculo a cada quadro faria o
/// compositor recriar a superfície sessenta vezes por segundo.
pub fn mover_margens(window: &WebviewWindow, monitor: Retangulo, (esquerda, inferior): (i32, i32)) {
    let Ok(gtk_window) = window.gtk_window() else {
        return;
    };

    if let Some(alvo) = monitor_gdk(&gtk_window, monitor) {
        let atual = gtk_window.monitor().map(|m| m.geometry());
        let mudou = atual.is_none_or(|g| {
            (g.x(), g.y(), g.width(), g.height())
                != (
                    alvo.geometry().x(),
                    alvo.geometry().y(),
                    alvo.geometry().width(),
                    alvo.geometry().height(),
                )
        });
        if mudou {
            gtk_window.set_monitor(&alvo);
        }
    }

    gtk_window.set_anchor(Edge::Left, true);
    gtk_window.set_layer_shell_margin(Edge::Left, esquerda);
    gtk_window.set_layer_shell_margin(Edge::Bottom, inferior);
}
