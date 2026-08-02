//! Ícone na bandeja do sistema.
//!
//! É o que faz o app existir em segundo plano: fechar a janela esconde o
//! overlay em vez de encerrar o programa, e a bandeja é como ele volta.
//!
//! No Linux isto passa por `StatusNotifierItem` (libayatana-appindicator).
//! Ambientes sem bandeja — alguns setups de Wayland puro — simplesmente não
//! mostram o ícone; por isso o atalho global existe como caminho paralelo,
//! nunca como conveniência.

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::App;

use crate::overlay;

pub fn setup(app: &App) -> tauri::Result<()> {
    let mostrar = MenuItem::with_id(app, "toggle", "Mostrar / ocultar", true, None::<&str>)?;
    let recolocar = MenuItem::with_id(app, "recolocar", "Recolocar overlay", true, None::<&str>)?;
    let config = MenuItem::with_id(app, "config", "Configurações…", true, None::<&str>)?;
    let separador = PredefinedMenuItem::separator(app)?;
    let sair = MenuItem::with_id(app, "sair", "Sair", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&mostrar, &recolocar, &config, &separador, &sair])?;

    TrayIconBuilder::with_id("principal")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("LyricsLens")
        .menu(&menu)
        // O menu é do botão direito; o esquerdo alterna direto.
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle" => overlay::toggle(app, crate::geometry(app)),
            "recolocar" => overlay::show(app, crate::geometry(app)),
            "config" => overlay::open_settings(app),
            "sair" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                overlay::toggle(app, crate::geometry(app));
            }
        })
        .build(app)?;

    Ok(())
}
