//! Detecção via MPRIS (D-Bus) — Spotify, navegadores, VLC, qualquer player
//! que siga a especificação.
//!
//! O ponto delicado está em [`MprisProvider::poll_position`]: a `Position` de
//! várias fontes é quantizada em 1 segundo, então a leitura crua carrega até
//! 1000ms de erro. O que tem precisão real é o *instante da virada*. Por isso
//! o poll é rápido e só a mudança de valor vira âncora confiável.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;
use zbus::zvariant::{OwnedValue, Value};
use zbus::{fdo, Connection};

use super::{
    AnchorConfidence, MediaError, MediaEvent, MediaProvider, PlaybackState, Track,
};
use crate::sync::Clock;

const MPRIS_PREFIX: &str = "org.mpris.MediaPlayer2.";
/// Agregador que espelha os outros players. Contar com ele duplicaria tudo.
const AGGREGATOR: &str = "org.mpris.MediaPlayer2.playerctld";

/// Intervalo do poll de posição. Define o erro máximo da âncora por borda.
const POLL: Duration = Duration::from_millis(100);
/// A cada quantos ticks reavaliamos quem é o player ativo.
const RESCAN_EVERY: u64 = 20;
/// A cada quantos ticks relemos metadados e estado de reprodução.
///
/// Todo tick, e isso é barato: o `zbus` mantém as propriedades num cache
/// alimentado pelo próprio `PropertiesChanged`, então reler não é ida ao
/// barramento. O ganho é direto na experiência — a troca de faixa passa a ser
/// notada em até 100ms em vez de até 500ms, que era metade do intervalo até a
/// letra nova aparecer.
const METADATA_EVERY: u64 = 1;
/// Intervalo mínimo entre âncoras enviadas, fora saltos.
const ANCHOR_THROTTLE: Duration = Duration::from_millis(500);

#[zbus::proxy(
    interface = "org.mpris.MediaPlayer2.Player",
    default_path = "/org/mpris/MediaPlayer2",
    gen_blocking = false
)]
trait Player {
    /// Micros. Não emite `PropertiesChanged` — por spec, tem que ser lida.
    #[zbus(property(emits_changed_signal = "false"))]
    fn position(&self) -> zbus::Result<i64>;

    #[zbus(property)]
    fn metadata(&self) -> zbus::Result<HashMap<String, OwnedValue>>;

    #[zbus(property)]
    fn playback_status(&self) -> zbus::Result<String>;
}

pub struct MprisProvider;

impl Default for MprisProvider {
    fn default() -> Self {
        Self
    }
}

/// O player escolhido e tudo que já sabemos sobre ele.
struct Active<'a> {
    bus_name: String,
    source: String,
    proxy: PlayerProxy<'a>,
    track: Track,
    state: PlaybackState,
    clock: Clock,
    last_raw_position: Option<i64>,
    last_anchor_sent: Option<Instant>,
}

impl MediaProvider for MprisProvider {
    async fn run(self, tx: mpsc::Sender<MediaEvent>) -> Result<(), MediaError> {
        let conn = Connection::session()
            .await
            .map_err(|e| MediaError::Bus(format!("sem barramento de sessão: {e}")))?;
        let dbus = fdo::DBusProxy::new(&conn)
            .await
            .map_err(|e| MediaError::Bus(e.to_string()))?;

        let mut active: Option<Active> = None;
        let mut ticks: u64 = 0;
        let mut timer = tokio::time::interval(POLL);
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            timer.tick().await;
            if tx.is_closed() {
                return Ok(());
            }
            ticks += 1;

            if ticks % RESCAN_EVERY == 1 {
                match pick_player(&conn, &dbus, active.as_ref().map(|a| a.bus_name.as_str())).await
                {
                    Some((bus_name, proxy)) => {
                        let trocou = active.as_ref().is_none_or(|a| a.bus_name != bus_name);
                        if trocou {
                            let source = source_name(&bus_name);
                            active = Some(Active {
                                bus_name,
                                source,
                                proxy,
                                track: Track::default(),
                                state: PlaybackState::Stopped,
                                clock: Clock::new(),
                                last_raw_position: None,
                                last_anchor_sent: None,
                            });
                        }
                    }
                    None => {
                        if active.take().is_some() && tx.send(MediaEvent::Gone).await.is_err() {
                            return Ok(());
                        }
                        continue;
                    }
                }
            }

            let Some(a) = active.as_mut() else { continue };

            if ticks % METADATA_EVERY == 0 && !refresh_metadata(a, &tx).await {
                // O player sumiu no meio do caminho; o próximo rescan resolve.
                active = None;
                continue;
            }

            if !poll_position(a, &tx).await {
                active = None;
            }
        }
    }
}

/// Relê metadados e estado. Devolve `false` se o player não responde mais.
async fn refresh_metadata(a: &mut Active<'_>, tx: &mpsc::Sender<MediaEvent>) -> bool {
    let Ok(status) = a.proxy.playback_status().await else {
        return false;
    };
    let Ok(metadata) = a.proxy.metadata().await else {
        return false;
    };

    let state = match status.as_str() {
        "Playing" => PlaybackState::Playing,
        "Paused" => PlaybackState::Paused,
        _ => PlaybackState::Stopped,
    };
    if state != a.state {
        a.state = state;
        a.clock.set_playing(state.is_playing());
        if tx.send(MediaEvent::PlaybackChanged { state }).await.is_err() {
            return false;
        }
    }

    let track = parse_track(&metadata, &a.source);
    if !track.is_empty() && track != a.track {
        a.track = track.clone();
        // Faixa nova: a posição antiga não vale mais nada.
        a.clock = Clock::new();
        a.clock.set_playing(a.state.is_playing());
        a.last_raw_position = None;
        a.last_anchor_sent = None;
        if tx.send(MediaEvent::TrackChanged { track }).await.is_err() {
            return false;
        }
    }

    true
}

/// O coração da precisão. Devolve `false` se o player não responde mais.
async fn poll_position(a: &mut Active<'_>, tx: &mpsc::Sender<MediaEvent>) -> bool {
    let Ok(raw) = a.proxy.position().await else {
        return false;
    };
    let observed_at = Instant::now();
    let position_ms = (raw.max(0) / 1_000) as u64;

    let mudou = a.last_raw_position != Some(raw);
    a.last_raw_position = Some(raw);

    // Primeira leitura da faixa: ainda sem borda, mas é melhor mostrar algo
    // impreciso agora do que nada por até um segundo.
    if !a.clock.is_anchored() {
        a.clock.anchor_at(position_ms, observed_at);
        a.last_anchor_sent = Some(observed_at);
        return tx
            .send(MediaEvent::PositionAnchored {
                position_ms,
                confidence: AnchorConfidence::Sample,
            })
            .await
            .is_ok();
    }

    if !mudou {
        // Entre bordas o valor é velho — não diz nada que já não saibamos.
        return true;
    }

    if a.clock.looks_like_seek(position_ms) {
        a.clock.anchor_at(position_ms, observed_at);
        a.last_anchor_sent = Some(observed_at);
        return tx.send(MediaEvent::Seeked { position_ms }).await.is_ok();
    }

    // Borda: a posição real é exatamente esta, neste instante.
    a.clock.anchor_at(position_ms, observed_at);

    let recente = a
        .last_anchor_sent
        .is_some_and(|t| t.elapsed() < ANCHOR_THROTTLE);
    if recente {
        // Fontes de alta resolução geram borda a cada poll. Ancorar sempre é
        // grátis; avisar o resto do app dez vezes por segundo não é.
        return true;
    }

    a.last_anchor_sent = Some(observed_at);
    tx.send(MediaEvent::PositionAnchored {
        position_ms,
        confidence: AnchorConfidence::Edge,
    })
    .await
    .is_ok()
}

/// Escolhe o player que interessa: quem estiver tocando ganha; na dúvida,
/// mantém o atual para não ficar pulando entre fontes.
async fn pick_player<'a>(
    conn: &Connection,
    dbus: &fdo::DBusProxy<'_>,
    current: Option<&str>,
) -> Option<(String, PlayerProxy<'a>)> {
    let names = dbus.list_names().await.ok()?;
    let candidatos: Vec<String> = names
        .into_iter()
        .map(|n| n.to_string())
        .filter(|n| n.starts_with(MPRIS_PREFIX) && n != AGGREGATOR)
        .collect();

    if candidatos.is_empty() {
        return None;
    }

    let mut fallback: Option<(String, PlayerProxy<'a>)> = None;
    let mut atual_vivo: Option<(String, PlayerProxy<'a>)> = None;

    for name in candidatos {
        let Ok(proxy) = PlayerProxy::builder(conn)
            .destination(name.clone())
            .ok()?
            .build()
            .await
        else {
            continue;
        };

        let tocando = proxy
            .playback_status()
            .await
            .map(|s| s == "Playing")
            .unwrap_or(false);

        if tocando {
            return Some((name, proxy));
        }
        if Some(name.as_str()) == current {
            atual_vivo = Some((name, proxy));
        } else if fallback.is_none() {
            fallback = Some((name, proxy));
        }
    }

    atual_vivo.or(fallback)
}

fn source_name(bus_name: &str) -> String {
    bus_name
        .strip_prefix(MPRIS_PREFIX)
        .unwrap_or(bus_name)
        .split(".instance")
        .next()
        .unwrap_or("desconhecido")
        .to_string()
}

fn parse_track(metadata: &HashMap<String, OwnedValue>, source: &str) -> Track {
    Track {
        title: metadata.get("xesam:title").and_then(as_text).unwrap_or_default(),
        artist: metadata
            .get("xesam:artist")
            .and_then(as_text_list)
            .unwrap_or_default(),
        album: metadata.get("xesam:album").and_then(as_text).unwrap_or_default(),
        duration_ms: metadata.get("mpris:length").and_then(as_micros_to_ms),
        art_url: metadata
            .get("mpris:artUrl")
            .and_then(as_text)
            .filter(|s| !s.is_empty()),
        url: metadata
            .get("xesam:url")
            .and_then(as_text)
            .filter(|s| !s.is_empty()),
        source: source.to_string(),
    }
}

fn as_text(v: &OwnedValue) -> Option<String> {
    match &**v {
        Value::Str(s) => Some(s.to_string()),
        Value::ObjectPath(p) => Some(p.to_string()),
        _ => None,
    }
}

/// `xesam:artist` é uma lista por spec, mas há player que manda string solta.
fn as_text_list(v: &OwnedValue) -> Option<String> {
    match &**v {
        Value::Array(arr) => {
            let nomes: Vec<String> = arr
                .iter()
                .filter_map(|item| match item {
                    Value::Str(s) => Some(s.to_string()),
                    _ => None,
                })
                .filter(|s| !s.is_empty())
                .collect();
            (!nomes.is_empty()).then(|| nomes.join(", "))
        }
        Value::Str(s) => Some(s.to_string()),
        _ => None,
    }
}

/// `mpris:length` vem em microssegundos, com o tipo variando por player.
fn as_micros_to_ms(v: &OwnedValue) -> Option<u64> {
    let micros = match &**v {
        Value::I64(n) => *n,
        Value::U64(n) => *n as i64,
        Value::I32(n) => *n as i64,
        Value::U32(n) => *n as i64,
        Value::F64(n) => *n as i64,
        _ => return None,
    };
    (micros > 0).then(|| (micros / 1_000) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extrai_o_nome_da_fonte_do_bus() {
        assert_eq!(source_name("org.mpris.MediaPlayer2.firefox.instance_1_32"), "firefox");
        assert_eq!(source_name("org.mpris.MediaPlayer2.spotify"), "spotify");
        assert_eq!(source_name("org.mpris.MediaPlayer2.vlc.instance7"), "vlc");
    }

    #[test]
    fn chave_da_faixa_ignora_caixa_e_espaco() {
        let a = Track {
            title: " Infinito Particular ".into(),
            artist: "Marisa Monte".into(),
            album: "Infinito Particular".into(),
            ..Default::default()
        };
        let b = Track {
            title: "infinito particular".into(),
            artist: "MARISA MONTE".into(),
            album: "Infinito particular".into(),
            ..Default::default()
        };
        assert_eq!(a.key(), b.key());
    }

    #[test]
    fn lista_de_artistas_vira_texto_unico() {
        let v = OwnedValue::try_from(Value::from(vec!["Jay-Z", "Linkin Park"])).unwrap();
        assert_eq!(as_text_list(&v).as_deref(), Some("Jay-Z, Linkin Park"));
    }

    #[test]
    fn artista_como_string_solta_tambem_funciona() {
        let v = OwnedValue::try_from(Value::from("Marisa Monte")).unwrap();
        assert_eq!(as_text_list(&v).as_deref(), Some("Marisa Monte"));
    }

    #[test]
    fn duracao_converte_de_micros_e_recusa_zero() {
        let v = OwnedValue::try_from(Value::from(251_000_000i64)).unwrap();
        assert_eq!(as_micros_to_ms(&v), Some(251_000));

        let zero = OwnedValue::try_from(Value::from(0i64)).unwrap();
        assert_eq!(as_micros_to_ms(&zero), None);
    }
}
