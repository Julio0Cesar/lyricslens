//! Um LRCLIB de mentira, para testar o cliente sem rede.
//!
//! O recuo para a busca ampla e a escolha por duração são a lógica mais sutil
//! do provedor — e a que mais decide se a letra certa aparece. Testá-la contra
//! o LRCLIB de verdade seria lento, intermitente, e mudaria de resposta sem
//! aviso quando o catálogo mudasse. Ver #9.
//!
//! Escrito à mão em vez de trazer uma crate de servidor: são quarenta linhas de
//! HTTP/1.1 contra uma árvore de dependências inteira, e o que se precisa aqui
//! é responder um corpo fixo por rota.
//!
//! Cada resposta fecha a conexão (`Connection: close`), o que dispensa
//! implementar keep-alive — o cliente abre outra, e o custo disso num teste
//! local é irrelevante.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// O que o servidor devolve numa rota.
#[derive(Clone)]
pub struct Resposta {
    pub status: u16,
    pub corpo: String,
}

impl Resposta {
    pub fn json(corpo: impl Into<String>) -> Self {
        Self {
            status: 200,
            corpo: corpo.into(),
        }
    }

    pub fn status(status: u16) -> Self {
        Self {
            status,
            corpo: String::new(),
        }
    }
}

pub struct ServidorFalso {
    pub base: String,
    /// Os caminhos pedidos, na ordem — é o que prova que o recuo aconteceu, e
    /// que ele não aconteceu quando não devia.
    pedidos: Arc<Mutex<Vec<String>>>,
}

impl ServidorFalso {
    /// Sobe numa porta livre e responde conforme o mapa `caminho → resposta`.
    ///
    /// O caminho casa por prefixo, então `/api/get` também atende
    /// `/api/get?artist_name=…`. Caminho não declarado devolve 404, que é o que
    /// o LRCLIB faz.
    pub async fn subir(rotas: HashMap<&'static str, Resposta>) -> Self {
        let escuta = TcpListener::bind("127.0.0.1:0").await.expect("porta livre");
        let porta = escuta.local_addr().unwrap().port();
        let pedidos = Arc::new(Mutex::new(Vec::new()));

        let registro = pedidos.clone();
        tokio::spawn(async move {
            loop {
                let Ok((mut conexao, _)) = escuta.accept().await else {
                    return;
                };
                let rotas = rotas.clone();
                let registro = registro.clone();

                tokio::spawn(async move {
                    let mut bruto = vec![0u8; 4096];
                    let Ok(lidos) = conexao.read(&mut bruto).await else {
                        return;
                    };
                    let pedido = String::from_utf8_lossy(&bruto[..lidos]).to_string();

                    // "GET /api/get?x=1 HTTP/1.1"
                    let caminho = pedido
                        .lines()
                        .next()
                        .and_then(|l| l.split_whitespace().nth(1))
                        .unwrap_or("/")
                        .to_string();
                    registro.lock().unwrap().push(caminho.clone());

                    // **O prefixo mais longo ganha.** `/api/get` é prefixo de
                    // `/api/get/496`, e a ordem de um `HashMap` é
                    // indeterminada — sem isto o mesmo teste passa ou falha
                    // conforme o humor do hash, que é a pior espécie de teste.
                    let resposta = rotas
                        .iter()
                        .filter(|(rota, _)| caminho.starts_with(*rota))
                        .max_by_key(|(rota, _)| rota.len())
                        .map(|(_, r)| r.clone())
                        .unwrap_or_else(|| Resposta::status(404));

                    let saida = format!(
                        "HTTP/1.1 {} OK\r\n\
                         Content-Type: application/json\r\n\
                         Content-Length: {}\r\n\
                         Connection: close\r\n\r\n{}",
                        resposta.status,
                        resposta.corpo.len(),
                        resposta.corpo
                    );
                    let _ = conexao.write_all(saida.as_bytes()).await;
                    let _ = conexao.shutdown().await;
                });
            }
        });

        Self {
            base: format!("http://127.0.0.1:{porta}/api"),
            pedidos,
        }
    }

    pub fn pedidos(&self) -> Vec<String> {
        self.pedidos.lock().unwrap().clone()
    }

    /// Alguma requisição bateu num caminho que começa assim?
    pub fn pediu(&self, prefixo: &str) -> bool {
        self.pedidos().iter().any(|p| p.starts_with(prefixo))
    }
}

/// Um item do catálogo, no formato que o LRCLIB devolve.
pub fn faixa_json(id: i64, nome: &str, duracao: f64, sincronizada: bool) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "trackName": nome,
        "artistName": "Radiohead",
        "albumName": "Pablo Honey",
        "duration": duracao,
        "instrumental": false,
        "plainLyrics": "When you were here before",
        "syncedLyrics": if sincronizada { "[00:19.16] When you were here before" } else { "" },
    })
}
