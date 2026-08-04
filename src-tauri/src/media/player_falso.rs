//! Um player MPRIS de mentira, no barramento de sessão.
//!
//! O laço de detecção — quem é o player ativo, quando a faixa mudou, quando
//! houve salto — nunca foi exercitado por teste: ele só existe conversando com
//! o D-Bus. É a camada onde o relógio ancorado por borda vive, e onde a âncora
//! `NaN` nasceu. Ver #9.
//!
//! ## Por que os testes pulam sozinhos
//!
//! O provedor escolhe **um** player entre os do barramento, preferindo o que
//! está tocando. Se houver um Spotify ou Firefox de verdade na sessão, ele pode
//! ganhar do falso — e o teste falharia por um motivo que não tem nada a ver
//! com o código.
//!
//! Então o teste confere que está sozinho no barramento e, se não estiver,
//! **pula com instrução** em vez de reprovar. Um teste que falha conforme o que
//! o desenvolvedor tem aberto é pior que teste nenhum: ensina a ignorar a
//! suíte. No CI, o `dbus-run-session` garante o barramento limpo.

use std::collections::HashMap;

use zbus::zvariant::{OwnedValue, Value};
use zbus::{connection, Connection};

/// O estado que o player de mentira publica.
#[derive(Clone)]
pub struct Estado {
    pub titulo: String,
    pub artista: String,
    pub album: String,
    pub duracao_us: i64,
    pub posicao_us: i64,
    pub status: String,
}

impl Default for Estado {
    fn default() -> Self {
        Self {
            titulo: "Creep".into(),
            artista: "Radiohead".into(),
            album: "Pablo Honey".into(),
            duracao_us: 238_000_000,
            posicao_us: 0,
            status: "Playing".into(),
        }
    }
}

pub struct PlayerFalso {
    estado: std::sync::Arc<std::sync::Mutex<Estado>>,
}

#[zbus::interface(name = "org.mpris.MediaPlayer2.Player")]
impl PlayerFalso {
    #[zbus(property)]
    async fn metadata(&self) -> HashMap<String, OwnedValue> {
        let e = self.estado.lock().unwrap().clone();
        let mut m = HashMap::new();
        m.insert(
            "xesam:title".to_string(),
            Value::from(e.titulo).try_into().unwrap(),
        );
        // Artista é lista por spec — e foi assim que o parser aprendeu a
        // aceitar as duas formas.
        m.insert(
            "xesam:artist".to_string(),
            Value::from(vec![e.artista]).try_into().unwrap(),
        );
        m.insert(
            "xesam:album".to_string(),
            Value::from(e.album).try_into().unwrap(),
        );
        m.insert(
            "mpris:length".to_string(),
            Value::from(e.duracao_us).try_into().unwrap(),
        );
        m
    }

    #[zbus(property(emits_changed_signal = "false"))]
    async fn position(&self) -> i64 {
        self.estado.lock().unwrap().posicao_us
    }

    #[zbus(property)]
    async fn playback_status(&self) -> String {
        self.estado.lock().unwrap().status.clone()
    }
}

/// Um player publicado no barramento, com controle do que ele reporta.
pub struct PlayerNoBarramento {
    _conexao: Connection,
    estado: std::sync::Arc<std::sync::Mutex<Estado>>,
}

impl PlayerNoBarramento {
    /// Publica sob `org.mpris.MediaPlayer2.<sufixo>`.
    pub async fn publicar(sufixo: &str, inicial: Estado) -> zbus::Result<Self> {
        let estado = std::sync::Arc::new(std::sync::Mutex::new(inicial));
        let conexao = connection::Builder::session()?
            .name(format!("org.mpris.MediaPlayer2.{sufixo}"))?
            .serve_at(
                "/org/mpris/MediaPlayer2",
                PlayerFalso {
                    estado: estado.clone(),
                },
            )?
            .build()
            .await?;

        Ok(Self {
            _conexao: conexao,
            estado,
        })
    }

    /// Muda o que o player reporta **e avisa o barramento**.
    ///
    /// Avisar não é detalhe do teste: propriedade D-Bus é cacheada no cliente
    /// e só é reconsultada quando chega `PropertiesChanged`. Sem o sinal, o
    /// provedor continuaria vendo a faixa antiga para sempre — que foi
    /// exatamente o que os dois primeiros testes acusaram. Player real emite;
    /// o falso precisa emitir também, senão testa uma ficção.
    pub async fn mexer(&self, f: impl FnOnce(&mut Estado)) -> zbus::Result<()> {
        f(&mut self.estado.lock().unwrap());

        let refe = self
            ._conexao
            .object_server()
            .interface::<_, PlayerFalso>("/org/mpris/MediaPlayer2")
            .await?;
        let iface = refe.get().await;
        iface.metadata_changed(refe.signal_emitter()).await?;
        iface.playback_status_changed(refe.signal_emitter()).await?;
        Ok(())
    }
}

/// Há outro player MPRIS no barramento além dos nossos?
///
/// É o que decide entre rodar o teste e pular pedindo `dbus-run-session`.
pub async fn barramento_ocupado(nosso_prefixo: &str) -> Option<Vec<String>> {
    let conn = Connection::session().await.ok()?;
    let dbus = zbus::fdo::DBusProxy::new(&conn).await.ok()?;
    let nomes = dbus.list_names().await.ok()?;

    let alheios: Vec<String> = nomes
        .into_iter()
        .map(|n| n.to_string())
        .filter(|n| n.starts_with("org.mpris.MediaPlayer2."))
        .filter(|n| !n.contains(nosso_prefixo))
        .collect();

    (!alheios.is_empty()).then_some(alheios)
}
