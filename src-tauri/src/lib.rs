mod lyrics;
mod media;
mod overlay;
mod store;
mod sync;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use lyrics::{lrclib::LrcLib, Candidate, Lyrics, LyricsProvider};
use media::{MediaEvent, MediaProvider, PlatformProvider, PlaybackState, Track};
use serde::Serialize;
use store::Cache;
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
    /// Sobe a cada troca de faixa. Uma busca que volta atrasada compara a
    /// geração com a atual e se descarta em vez de escrever letra errada.
    generation: AtomicU64,
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

/// Ajuste fino da sincronia, em milissegundos.
#[tauri::command]
fn set_sync_offset(state: tauri::State<'_, AppState>, offset_ms: i64) {
    state.now_playing.lock().unwrap().clock.set_offset_ms(offset_ms);
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
                    state.generation.fetch_add(1, Ordering::SeqCst);
                }
            }
        }

        let _ = app.emit("media", &event);

        if let Some((track, generation)) = buscar {
            tauri::async_runtime::spawn(resolve_lyrics(app.clone(), track, generation));
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            now_playing,
            current_lyrics,
            set_sync_offset,
            search_lyrics,
            pin_lyrics,
            overlay::probe_environment,
            overlay::set_click_through,
            overlay::set_always_on_top,
            overlay::apply_hyprland_rules,
        ])
        .setup(|app| {
            let db = app.path().app_data_dir()?.join("lyrics.db");
            app.manage(AppState {
                now_playing: Mutex::new(NowPlaying::default()),
                last_lyrics: Mutex::new(None),
                cache: Cache::open(&db)?,
                provider: LrcLib::new()?,
                generation: AtomicU64::new(0),
            });

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
