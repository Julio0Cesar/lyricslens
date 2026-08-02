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

use gtk::prelude::WidgetExt;
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use tauri::WebviewWindow;

use super::Geometry;

/// Prepara a janela como camada. Tem que ser chamado antes do primeiro `show`.
pub fn init(window: &WebviewWindow, geo: Geometry) -> Result<(), String> {
    if !gtk_layer_shell::is_supported() {
        return Err("o compositor não implementa wlr-layer-shell".into());
    }

    let gtk_window = window
        .gtk_window()
        .map_err(|e| format!("sem acesso à janela GTK: {e}"))?;

    gtk_window.init_layer_shell();
    gtk_window.set_layer(Layer::Overlay);
    // O namespace é como o compositor identifica a camada em regras próprias.
    gtk_window.set_namespace("lyricslens");

    // Ancorar só embaixo mantém a largura definida por nós; ancorar nos dois
    // lados horizontais faria a camada esticar de ponta a ponta da tela.
    gtk_window.set_anchor(Edge::Bottom, true);
    gtk_window.set_layer_shell_margin(Edge::Bottom, geo.margin_bottom);

    // -1 significa "não reserve espaço para mim": sem isso o compositor
    // encolheria as outras janelas para caber o overlay, como faz com uma
    // barra de tarefas.
    gtk_window.set_exclusive_zone(-1);

    // Sem isso a camada roubaria o teclado do que estiver em foco.
    gtk_window.set_keyboard_mode(KeyboardMode::None);

    gtk_window.set_size_request(geo.width, geo.height);
    Ok(())
}

/// Ajusta o que dá para ajustar com a camada já criada.
pub fn update_geometry(window: &WebviewWindow, geo: Geometry) {
    let Ok(gtk_window) = window.gtk_window() else {
        return;
    };
    gtk_window.set_layer_shell_margin(Edge::Bottom, geo.margin_bottom);
    gtk_window.set_size_request(geo.width, geo.height);
}
