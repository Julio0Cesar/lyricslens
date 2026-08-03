//! Aviso de versão nova.
//!
//! O atualizador oficial do Tauri não serve aqui: no Linux ele localiza o
//! aplicativo pela variável `APPIMAGE` para trocar o arquivo `.AppImage`. A
//! instalação do LyricsLens é um AppImage **extraído** — escolha feita porque
//! montá-lo exige `libfuse2`, que várias distribuições já não trazem — e nela
//! só existe `APPDIR`.
//!
//! Então o caminho é o mesmo do instalador: baixar a release e trocar a
//! instalação de lado. Aqui só descobrimos que há versão nova e chamamos o
//! script que já sabe fazer isso.

use serde::{Deserialize, Serialize};

pub const ATUAL: &str = env!("CARGO_PKG_VERSION");

const API_ULTIMA: &str = "https://api.github.com/repos/Julio0Cesar/lyricslens/releases/latest";
const INSTALADOR: &str = "https://raw.githubusercontent.com/Julio0Cesar/lyricslens/main/install.sh";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    pub latest: Option<String>,
    pub available: bool,
    /// Só instalações feitas pelo script sabem se atualizar sozinhas. Em
    /// desenvolvimento, ou num `.deb`, quem atualiza é outro.
    pub can_apply: bool,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

/// Extrai `1.2.3` de qualquer coisa que contenha uma versão — `v1.2.3`,
/// `lyricslens-v1.2.3`, `1.2.3-beta`.
fn versao(texto: &str) -> Option<(u32, u32, u32)> {
    let pedacos: Vec<&str> = texto
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .filter(|p| p.contains('.'))
        .collect();

    for pedaco in pedacos {
        let n: Vec<&str> = pedaco.split('.').collect();
        if n.len() >= 3 {
            if let (Ok(a), Ok(b), Ok(c)) = (
                n[0].parse::<u32>(),
                n[1].parse::<u32>(),
                n[2].parse::<u32>(),
            ) {
                return Some((a, b, c));
            }
        }
    }
    None
}

/// A instalação sabe se trocar? Só quando veio do script, que deixa o app
/// dentro de um AppImage extraído.
pub fn instalado_pelo_script() -> bool {
    std::env::var("APPDIR").is_ok()
}

pub async fn check() -> Result<UpdateInfo, String> {
    let http = reqwest::Client::builder()
        .user_agent(format!("LyricsLens/{ATUAL}"))
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let release: Release = http
        .get(API_ULTIMA)
        .send()
        .await
        .map_err(|e| format!("não consegui consultar as versões: {e}"))?
        .json()
        .await
        .map_err(|e| format!("resposta inesperada do GitHub: {e}"))?;

    let publicada = versao(&release.tag_name);
    let disponivel = match (versao(ATUAL), publicada) {
        (Some(atual), Some(nova)) => nova > atual,
        _ => false,
    };

    Ok(UpdateInfo {
        current: ATUAL.to_string(),
        latest: publicada.map(|(a, b, c)| format!("{a}.{b}.{c}")),
        available: disponivel,
        can_apply: instalado_pelo_script(),
    })
}

/// Roda o instalador, que baixa a release e troca a instalação de lado.
pub fn apply() -> Result<(), String> {
    if !instalado_pelo_script() {
        return Err("esta instalação não foi feita pelo script — atualize pelo mesmo caminho que usou para instalar".into());
    }

    let saida = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("curl -fsSL {INSTALADOR} | sh"))
        .output()
        .map_err(|e| format!("não consegui executar o instalador: {e}"))?;

    if saida.status.success() {
        Ok(())
    } else {
        Err(format!(
            "a instalação falhou: {}",
            String::from_utf8_lossy(&saida.stderr).trim()
        ))
    }
}

/// Sobe a versão nova depois que este processo morrer.
///
/// A ordem importa: se o novo processo subisse antes, a instância única o
/// mandaria repassar os argumentos para o processo velho e ele sairia sem
/// abrir nada. Por isso quem espera é um shell solto, olhando o PID.
pub fn restart_after_exit() -> Result<(), String> {
    let appdir = std::env::var("APPDIR").map_err(|_| "sem APPDIR".to_string())?;
    let pid = std::process::id();

    std::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "while kill -0 {pid} 2>/dev/null; do sleep 0.2; done; exec '{appdir}/AppRun'"
        ))
        .spawn()
        .map_err(|e| format!("não consegui agendar a reabertura: {e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_versao_de_varios_formatos() {
        assert_eq!(versao("0.2.1"), Some((0, 2, 1)));
        assert_eq!(versao("v0.2.1"), Some((0, 2, 1)));
        // Formato exato que o release-please gera para este repositório.
        assert_eq!(versao("lyricslens-v0.2.1"), Some((0, 2, 1)));
        assert_eq!(versao("1.10.3-beta"), Some((1, 10, 3)));
    }

    #[test]
    fn recusa_o_que_nao_e_versao() {
        assert_eq!(versao("sem numero"), None);
        assert_eq!(versao("v1.2"), None, "faltando o patch não serve");
        assert_eq!(versao(""), None);
    }

    #[test]
    fn compara_por_componente_e_nao_por_texto() {
        // Comparado como texto, "0.10.0" viria antes de "0.9.0".
        assert!(versao("0.10.0") > versao("0.9.0"));
        assert!(versao("1.0.0") > versao("0.99.99"));
        assert!(versao("0.2.2") > versao("0.2.1"));
        assert_eq!(versao("0.2.1"), versao("v0.2.1"));
    }

    #[test]
    fn a_versao_compilada_e_legivel() {
        assert!(
            versao(ATUAL).is_some(),
            "a versão do próprio pacote precisa ser reconhecível: {ATUAL}"
        );
    }
}
