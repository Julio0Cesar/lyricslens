//! Qual idioma a interface fala.
//!
//! O backend entra nisto por um motivo só: **o frontend não tem como saber o
//! idioma do sistema**. Ele roda num webview, e o `navigator.language` de um
//! WebKit embarcado não acompanha o `LANG` da sessão — reporta o locale do
//! processo, que numa janela do Tauri costuma ser `en-US` mesmo com o sistema
//! inteiro em português. Quem sabe a verdade é quem tem acesso ao ambiente.
//!
//! Traduzir texto continua sendo assunto da UI. Daqui sai só a resposta a
//! "qual idioma", e os erros do backend viajam como código, não como frase
//! pronta — ver `UiError`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Erro destinado à tela.
///
/// Viaja como **código estável mais parâmetros**, nunca como frase pronta: quem
/// sabe o idioma da vez é a UI, e ela pode trocar de idioma sem o backend saber.
/// Frase montada aqui ficaria presa ao idioma de quando o erro aconteceu.
///
/// Só os erros que o usuário lê passam por aqui. O que vai para o `stderr` —
/// diagnóstico de quem está depurando — continua texto solto, e em português
/// mesmo: é do projeto, não do produto.
#[derive(Debug, Clone, Serialize)]
pub struct UiError {
    /// Chave estável, casada no `traduzirErro` do frontend.
    pub code: &'static str,
    pub args: BTreeMap<String, String>,
}

impl UiError {
    pub fn new(code: &'static str) -> Self {
        Self {
            code,
            args: BTreeMap::new(),
        }
    }

    /// Acrescenta um parâmetro que a frase traduzida vai interpolar.
    pub fn arg(mut self, chave: &str, valor: impl std::fmt::Display) -> Self {
        self.args.insert(chave.to_string(), valor.to_string());
        self
    }
}

impl std::fmt::Display for UiError {
    /// Para o `stderr` e para os testes. Não é o que o usuário lê.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code)?;
        if !self.args.is_empty() {
            let pares: Vec<String> = self.args.iter().map(|(k, v)| format!("{k}={v}")).collect();
            write!(f, " ({})", pares.join(", "))?;
        }
        Ok(())
    }
}

/// Idioma da interface.
///
/// `Auto` só existe como *preferência* do usuário. A detecção nunca devolve
/// `Auto`: ela resolve para um idioma concreto ou cai no inglês.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "pt-BR")]
    PtBr,
    #[serde(rename = "en")]
    En,
}

/// Lê o idioma de uma variável no formato POSIX (`pt_BR.UTF-8`, `en_US`, `C`).
///
/// Inglês é a reserva deliberada: quem não tem `LANG` configurado, ou tem `C`,
/// tem mais chance de ler inglês do que português. O contrário deixaria o app
/// em português para o mundo inteiro, que é justamente o que a #11 corrige.
fn da_variavel(valor: &str) -> Language {
    // `pt_BR.UTF-8` e `pt-BR` valem os dois; só o prefixo de idioma importa.
    let idioma = valor
        .split(['.', '@'])
        .next()
        .unwrap_or("")
        .replace('-', "_")
        .to_lowercase();

    match idioma.split('_').next().unwrap_or("") {
        "pt" => Language::PtBr,
        _ => Language::En,
    }
}

/// O idioma do sistema, na ordem de precedência que o POSIX manda.
///
/// `LC_ALL` sobrepõe tudo, `LC_MESSAGES` manda no texto de programa, e `LANG` é
/// o padrão geral. Variável vazia não conta como definida — é assim que se
/// desliga uma delas.
pub fn do_sistema() -> Language {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|chave| {
            std::env::var(chave)
                .ok()
                .filter(|v| !v.trim().is_empty())
                .map(|v| da_variavel(&v))
        })
        .unwrap_or(Language::En)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconhece_portugues_em_qualquer_forma() {
        for valor in ["pt_BR.UTF-8", "pt_BR", "pt", "pt-BR", "PT_br.utf8", "pt_PT"] {
            assert_eq!(da_variavel(valor), Language::PtBr, "não casou: {valor}");
        }
    }

    #[test]
    fn qualquer_outro_idioma_cai_no_ingles() {
        for valor in ["en_US.UTF-8", "es_ES", "fr_FR", "de_DE", "ja_JP.UTF-8"] {
            assert_eq!(da_variavel(valor), Language::En, "não casou: {valor}");
        }
    }

    /// `C` e `POSIX` são "sem localização", não "português".
    #[test]
    fn locale_neutro_cai_no_ingles() {
        for valor in ["C", "POSIX", "C.UTF-8", ""] {
            assert_eq!(da_variavel(valor), Language::En, "não casou: {valor}");
        }
    }

    /// O sufixo de modificador (`@euro`) não pode virar parte do idioma.
    #[test]
    fn descarta_codificacao_e_modificador() {
        assert_eq!(da_variavel("pt_BR.ISO-8859-1@euro"), Language::PtBr);
        assert_eq!(da_variavel("de_DE@euro"), Language::En);
    }

    /// `Auto` é preferência, nunca resultado de detecção.
    #[test]
    fn deteccao_nunca_devolve_auto() {
        for valor in ["pt_BR", "en_US", "C", "xx_YY", ""] {
            assert_ne!(da_variavel(valor), Language::Auto);
        }
    }

    /// O frontend casa pelo `code`; o formato do JSON é contrato entre os dois.
    #[test]
    fn erro_serializa_como_codigo_e_parametros() {
        let erro = UiError::new("hotkey.refused").arg("atalho", "SUPER, L");
        let json = serde_json::to_value(&erro).unwrap();
        assert_eq!(json["code"], "hotkey.refused");
        assert_eq!(json["args"]["atalho"], "SUPER, L");
    }

    #[test]
    fn erro_sem_parametro_leva_args_vazio() {
        let json = serde_json::to_value(UiError::new("lyrics.notFound")).unwrap();
        assert_eq!(json["code"], "lyrics.notFound");
        assert!(json["args"].as_object().unwrap().is_empty());
    }

    /// O `Display` é para o log, e precisa dizer o suficiente para depurar.
    #[test]
    fn display_mostra_codigo_e_parametros() {
        let erro = UiError::new("autostart.write").arg("motivo", "permissão negada");
        assert_eq!(
            erro.to_string(),
            "autostart.write (motivo=permissão negada)"
        );
        assert_eq!(
            UiError::new("lyrics.notFound").to_string(),
            "lyrics.notFound"
        );
    }
}
