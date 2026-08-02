mod media;
mod overlay;
mod sync;

use std::sync::Mutex;

use media::{MediaEvent, MediaProvider, PlatformProvider, PlaybackState, Track};
use serde::Serialize;
use sync::Clock;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;

/// Tudo que o app sabe sobre o que está tocando agora.
#[derive(Default)]
struct NowPlaying {
    track: Option<Track>,
    state: PlaybackState,
    clock: Clock,
}

#[derive(Default)]
struct AppState {
    now_playing: Mutex<NowPlaying>,
}

/// Fotografia do estado, para a UI se recuperar de um reload sem esperar o
/// próximo evento.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    track: Option<Track>,
    state: PlaybackState,
    position_ms: u64,
}

#[tauri::command]
fn now_playing(state: tauri::State<'_, AppState>) -> Snapshot {
    let np = state.now_playing.lock().unwrap();
    Snapshot {
        track: np.track.clone(),
        state: np.state,
        position_ms: np.clock.position_ms(),
    }
}

/// Ajuste fino da sincronia, em milissegundos.
#[tauri::command]
fn set_sync_offset(state: tauri::State<'_, AppState>, offset_ms: i64) {
    state.now_playing.lock().unwrap().clock.set_offset_ms(offset_ms);
}

/// Consome os eventos do provider, mantém o estado do app e repassa para a UI.
async fn consume(app: AppHandle, mut rx: mpsc::Receiver<MediaEvent>) {
    while let Some(event) = rx.recv().await {
        {
            let state = app.state::<AppState>();
            let mut np = state.now_playing.lock().unwrap();
            match &event {
                MediaEvent::TrackChanged { track } => {
                    let playing = np.state.is_playing();
                    np.track = Some(track.clone());
                    np.clock = Clock::new();
                    np.clock.set_playing(playing);
                }
                MediaEvent::PlaybackChanged { state } => {
                    np.state = *state;
                    np.clock.set_playing(state.is_playing());
                }
                MediaEvent::PositionAnchored { position_ms, .. }
                | MediaEvent::Seeked { position_ms } => {
                    np.clock.anchor(*position_ms);
                }
                MediaEvent::Gone => {
                    *np = NowPlaying::default();
                }
            }
        }

        let _ = app.emit("media", &event);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            now_playing,
            set_sync_offset,
            overlay::probe_environment,
            overlay::set_click_through,
            overlay::set_always_on_top,
            overlay::apply_hyprland_rules,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let (tx, rx) = mpsc::channel(64);

            tauri::async_runtime::spawn(consume(handle, rx));
            tauri::async_runtime::spawn(async move {
                if let Err(e) = PlatformProvider::default().run(tx).await {
                    eprintln!("[media] provider parou: {e}");
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
