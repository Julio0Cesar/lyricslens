mod lyrics;
mod media;
mod overlay;
mod store;
mod sync;
mod tray;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use lyrics::{lrclib::LrcLib, Candidate, Lyrics, LyricsProvider};
use media::{MediaEvent, MediaProvider, PlatformProvider, PlaybackState, Track};
use overlay::Geometry;
use serde::Serialize;
use store::{Cache, Settings, SettingsStore};
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

struct AppState {
    now_playing: Mutex<NowPlaying>,
    /// Último resultado de busca, para a UI se recuperar de um reload ou de
    /// ter montado depois do evento.
    last_lyrics: Mutex<Option<LyricsEvent>>,
    cache: Cache,
    provider: LrcLib,
    settings: Mutex<Settings>,
    settings_store: SettingsStore,
    /// Sobe a cada troca de faixa. Uma busca que volta atrasada compara a
    /// geração com a atual e se descarta em vez de escrever letra errada.
    generation: AtomicU64,
}

/// Geometria atual do overlay, conforme as preferências.
fn geometry(app: &AppHandle) -> Geometry {
    Geometry::from(&*app.state::<AppState>().settings.lock().unwrap())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    track: Option<Track>,
    state: PlaybackState,
    position_ms: u64,
}

#[derive(Clone, Serialize)]
#[serde(tag = "status", rename_all = "camelCase", rename_all_fields = "camelCase")]
enum LyricsEvent {
    Searching { track_key: String },
    Found { track_key: String, lyrics: Lyrics },
    NotFound { track_key: String },
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

/// A última resolução de letra. Sem isto, uma janela que monta depois do
/// evento fica esperando para sempre por algo que já passou.
#[tauri::command]
fn current_lyrics(state: tauri::State<'_, AppState>) -> Option<LyricsEvent> {
    state.last_lyrics.lock().unwrap().clone()
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

/// Grava as preferências e propaga o efeito de cada uma para onde ela vale:
/// o relógio, a geometria da janela, e as janelas abertas.
#[tauri::command]
fn save_settings(app: AppHandle, mut settings: Settings) -> Result<Settings, String> {
    settings.sanitize();

    let state = app.state::<AppState>();
    let geometria_mudou = {
        let anterior = state.settings.lock().unwrap().clone();
        anterior.width != settings.width
            || anterior.height != settings.height
            || anterior.margin_bottom != settings.margin_bottom
    };

    state
        .settings_store
        .save(&settings)
        .map_err(|e| format!("não consegui gravar as preferências: {e}"))?;

    state
        .now_playing
        .lock()
        .unwrap()
        .clock
        .set_offset_ms(settings.sync_offset_ms);
    *state.settings.lock().unwrap() = settings.clone();

    if let Some(window) = app.get_webview_window(overlay::OVERLAY_LABEL) {
        let _ = window.set_ignore_cursor_events(settings.click_through);
        if geometria_mudou {
            let _ = overlay::apply_rules(&window, Geometry::from(&settings));
        }
    }

    // A janela de configurações e o overlay reagem juntos.
    let _ = app.emit("settings", &settings);
    Ok(settings)
}

#[tauri::command]
fn open_settings(app: AppHandle) {
    overlay::open_settings(&app);
}

#[tauri::command]
fn toggle_overlay(app: AppHandle) {
    let geo = geometry(&app);
    overlay::toggle(&app, geo);
}

#[tauri::command]
fn apply_compositor_rules(app: AppHandle, window: tauri::WebviewWindow) -> Result<String, String> {
    let geo = geometry(&app);
    overlay::apply_rules(&window, geo)
}

/// Alternativas para o usuário escolher quando a busca automática erra.
#[tauri::command]
async fn search_lyrics(
    state: tauri::State<'_, AppState>,
    artist: String,
    title: String,
) -> Result<Vec<Candidate>, String> {
    state
        .provider
        .search(&artist, &title)
        .await
        .map_err(|e| e.to_string())
}

/// Deixa a letra disponível offline.
#[tauri::command]
fn pin_lyrics(state: tauri::State<'_, AppState>, track_key: String, pinned: bool) -> Result<(), String> {
    state
        .cache
        .set_pinned(&track_key, pinned)
        .map_err(|e| e.to_string())
}

/// Adota uma letra escolhida a dedo pelo usuário.
///
/// Grava no cache sob a chave da faixa atual — é isso que faz a escolha valer
/// para as próximas vezes que a mesma música tocar, sem precisar escolher de
/// novo.
#[tauri::command]
async fn apply_candidate(app: AppHandle, provider_id: String) -> Result<(), String> {
    let track = {
        let state = app.state::<AppState>();
        let np = state.now_playing.lock().unwrap();
        np.track.clone().ok_or("nada tocando agora")?
    };

    let lyrics = {
        let state = app.state::<AppState>();
        state
            .provider
            .fetch_by_id(&provider_id)
            .await
            .map_err(|e| e.to_string())?
    };

    let state = app.state::<AppState>();
    state
        .cache
        .put(&track, &lyrics)
        .map_err(|e| format!("não consegui guardar a escolha: {e}"))?;

    let evento = LyricsEvent::Found {
        track_key: track.key(),
        lyrics,
    };
    *state.last_lyrics.lock().unwrap() = Some(evento.clone());
    let _ = app.emit("lyrics", evento);
    Ok(())
}

/// Cache primeiro, rede depois — e nunca duas vezes para a mesma ausência.
async fn resolve_lyrics(app: AppHandle, track: Track, generation: u64) {
    let key = track.key();
    let state = app.state::<AppState>();

    let atual = || state.generation.load(Ordering::SeqCst) == generation;
    let emit = |ev: LyricsEvent| {
        *state.last_lyrics.lock().unwrap() = Some(ev.clone());
        let _ = app.emit("lyrics", ev);
    };

    emit(LyricsEvent::Searching {
        track_key: key.clone(),
    });

    // As operações de SQLite são síncronas, mas custam microssegundos e isto
    // roda na sua própria task — não bloqueia a UI nem o detector.
    match state.cache.get(&key) {
        Ok(Some(lyrics)) => {
            if atual() {
                emit(LyricsEvent::Found {
                    track_key: key,
                    lyrics,
                });
            }
            return;
        }
        Ok(None) => {}
        Err(e) => eprintln!("[lyrics] cache ilegível: {e}"),
    }

    if !state.cache.should_retry(&key).unwrap_or(true) {
        if atual() {
            emit(LyricsEvent::NotFound { track_key: key });
        }
        return;
    }

    let resultado = state.provider.fetch(&track).await;

    // Enquanto a rede respondia, a música pode ter mudado.
    if !atual() {
        return;
    }

    match resultado {
        Ok(lyrics) => {
            if let Err(e) = state.cache.put(&track, &lyrics) {
                eprintln!("[lyrics] falha ao gravar no cache: {e}");
            }
            emit(LyricsEvent::Found {
                track_key: key,
                lyrics,
            });
        }
        Err(e) => {
            // Só um "não existe" merece ser lembrado. Falha de rede é
            // temporária e não pode virar ausência permanente.
            if matches!(e, lyrics::LyricsError::NotFound) {
                eprintln!(
                    "[lyrics] sem letra para {:?} / {:?} (álbum {:?}, {:?}ms)",
                    track.artist, track.title, track.album, track.duration_ms
                );
                let _ = state.cache.mark_miss(&key);
            } else {
                eprintln!("[lyrics] busca falhou para {key:?}: {e}");
            }
            emit(LyricsEvent::NotFound { track_key: key });
        }
    }
}

/// Consome os eventos do provider, mantém o estado do app e repassa para a UI.
async fn consume(app: AppHandle, mut rx: mpsc::Receiver<MediaEvent>) {
    while let Some(event) = rx.recv().await {
        let mut buscar: Option<(Track, u64)> = None;
        let mut visibilidade: Option<bool> = None;

        {
            let state = app.state::<AppState>();
            let mut np = state.now_playing.lock().unwrap();
            match &event {
                MediaEvent::TrackChanged { track } => {
                    let playing = np.state.is_playing();
                    np.track = Some(track.clone());
                    np.clock = Clock::new();
                    np.clock.set_playing(playing);

                    let generation = state.generation.fetch_add(1, Ordering::SeqCst) + 1;
                    buscar = Some((track.clone(), generation));
                }
                MediaEvent::PlaybackChanged { state: playback } => {
                    np.state = *playback;
                    np.clock.set_playing(playback.is_playing());

                    if state.settings.lock().unwrap().hide_when_paused {
                        visibilidade = Some(playback.is_playing());
                    }
                }
                MediaEvent::PositionAnchored { position_ms, .. }
                | MediaEvent::Seeked { position_ms } => {
                    np.clock.anchor(*position_ms);
                }
                MediaEvent::Gone => {
                    *np = NowPlaying::default();
                    state.generation.fetch_add(1, Ordering::SeqCst);
                }
            }
        }

        let _ = app.emit("media", &event);

        match visibilidade {
            Some(true) => overlay::show(&app, geometry(&app)),
            Some(false) => overlay::hide(&app),
            None => {}
        }

        if let Some((track, generation)) = buscar {
            tauri::async_runtime::spawn(resolve_lyrics(app.clone(), track, generation));
        }
    }
}

/// Interpreta o que veio na linha de comando.
///
/// É o caminho do atalho global: o Wayland não deixa um app registrar um
/// atalho de sistema, então quem registra é o compositor, e ele executa
/// `lyricslens toggle`. O plugin de instância única entrega esses argumentos
/// para o processo que já está rodando em vez de abrir um segundo.
fn handle_cli(app: &AppHandle, argv: &[String]) {
    let geo = geometry(app);
    match argv.iter().skip(1).find(|a| !a.starts_with('-')).map(String::as_str) {
        Some("toggle") => overlay::toggle(app, geo),
        Some("hide") => overlay::hide(app),
        Some("settings") => overlay::open_settings(app),
        // Sem comando reconhecido, a intenção de reabrir o app é aparecer.
        _ => overlay::show(app, geo),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Precisa ser o primeiro plugin registrado.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            handle_cli(app, &argv);
        }))
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            now_playing,
            current_lyrics,
            get_settings,
            save_settings,
            open_settings,
            toggle_overlay,
            apply_compositor_rules,
            search_lyrics,
            apply_candidate,
            pin_lyrics,
            overlay::probe_environment,
            overlay::set_click_through,
            overlay::list_fonts,
        ])
        .on_window_event(|window, event| {
            // Fechar esconde. O app vive na bandeja; sair é decisão explícita.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let settings_store = SettingsStore::new(&dir);
            let settings = settings_store.load();

            let mut now_playing = NowPlaying::default();
            now_playing.clock.set_offset_ms(settings.sync_offset_ms);

            app.manage(AppState {
                now_playing: Mutex::new(now_playing),
                last_lyrics: Mutex::new(None),
                cache: Cache::open(&dir.join("lyrics.db"))?,
                provider: LrcLib::new()?,
                settings: Mutex::new(settings.clone()),
                settings_store,
                generation: AtomicU64::new(0),
            });

            tray::setup(app)?;

            // O Wayland ignora tamanho e posição pedidos pela janela; quem
            // decide é o compositor. Ver `overlay::apply_compositor_rules`.
            if let Some(window) = app.get_webview_window(overlay::OVERLAY_LABEL) {
                let _ = window.set_ignore_cursor_events(settings.click_through);

                let iniciar_oculto = std::env::args().any(|a| a == "hide");
                if iniciar_oculto {
                    let _ = window.hide();
                } else {
                    tauri::async_runtime::spawn(overlay::apply_rules_when_mapped(
                        window,
                        Geometry::from(&settings),
                    ));
                }
            }

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
