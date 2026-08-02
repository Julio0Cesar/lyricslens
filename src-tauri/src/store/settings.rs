//! Preferências do usuário.
//!
//! Ficam num JSON no diretório de dados do app, e não no SQLite: é um punhado
//! de valores que o usuário pode querer ler, editar na mão ou versionar. Banco
//! aqui só atrapalharia.
//!
//! Campo desconhecido no arquivo é ignorado e campo ausente assume o padrão —
//! assim uma versão nova nunca quebra a configuração de uma versão velha, nem
//! o contrário.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    // ---- aparência ----
    /// Vazio = fonte do sistema.
    pub font_family: String,
    pub font_size: u32,
    pub font_weight: u32,
    pub text_color: String,
    /// Cor das linhas ainda não cantadas e do contexto.
    pub dim_color: String,
    pub background_color: String,
    /// 0 = totalmente transparente.
    pub background_opacity: f32,
    pub blur: bool,
    pub corner_radius: u32,

    // ---- o que aparece ----
    pub show_context_lines: bool,
    pub show_track_info: bool,
    pub show_progress: bool,
    pub text_align: TextAlign,
    pub karaoke: bool,

    // ---- comportamento ----
    pub click_through: bool,
    pub hide_when_paused: bool,
    /// Ajuste fino da sincronia. Negativo adianta a letra.
    pub sync_offset_ms: i64,

    // ---- geometria ----
    pub width: u32,
    pub height: u32,
    pub margin_bottom: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextAlign {
    Left,
    Center,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            font_family: String::new(),
            font_size: 26,
            font_weight: 600,
            text_color: "#ffffff".into(),
            dim_color: "#ffffff4d".into(),
            background_color: "#0a0a0e".into(),
            background_opacity: 0.55,
            blur: true,
            corner_radius: 16,

            show_context_lines: true,
            show_track_info: true,
            show_progress: true,
            text_align: TextAlign::Left,
            karaoke: true,

            click_through: false,
            hide_when_paused: false,
            sync_offset_ms: 0,

            width: 780,
            height: 300,
            margin_bottom: 80,
        }
    }
}

impl Settings {
    /// Impede que um arquivo editado na mão deixe o overlay inutilizável —
    /// invisível, gigante, ou fora da tela.
    pub fn sanitize(&mut self) {
        self.font_size = self.font_size.clamp(10, 96);
        self.font_weight = self.font_weight.clamp(100, 900);
        self.background_opacity = self.background_opacity.clamp(0.0, 1.0);
        self.corner_radius = self.corner_radius.min(64);
        self.width = self.width.clamp(240, 3840);
        self.height = self.height.clamp(80, 2160);
        self.margin_bottom = self.margin_bottom.min(2000);
        self.sync_offset_ms = self.sync_offset_ms.clamp(-10_000, 10_000);
    }
}

pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub fn new(dir: &Path) -> Self {
        Self {
            path: dir.join("settings.json"),
        }
    }

    /// Configuração ilegível não é motivo para o app não abrir: cai no padrão.
    pub fn load(&self) -> Settings {
        let mut s = std::fs::read_to_string(&self.path)
            .ok()
            .and_then(|raw| match serde_json::from_str::<Settings>(&raw) {
                Ok(s) => Some(s),
                Err(e) => {
                    eprintln!("[settings] arquivo ilegível, usando padrões: {e}");
                    None
                }
            })
            .unwrap_or_default();
        s.sanitize();
        s
    }

    /// Escreve em arquivo temporário e renomeia: uma queda no meio da escrita
    /// deixa a configuração anterior intacta, nunca um JSON pela metade.
    pub fn save(&self, settings: &Settings) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(settings)?;
        let temp = self.path.with_extension("json.tmp");
        std::fs::write(&temp, json)?;
        std::fs::rename(&temp, &self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir_temporario(nome: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("lyricslens-teste-{nome}"));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn sem_arquivo_devolve_padrao() {
        let s = SettingsStore::new(&dir_temporario("vazio")).load();
        assert_eq!(s, Settings::default());
    }

    #[test]
    fn grava_e_le_de_volta() {
        let d = dir_temporario("ida-e-volta");
        let store = SettingsStore::new(&d);

        let mut s = Settings::default();
        s.font_size = 40;
        s.background_opacity = 0.2;
        s.text_align = TextAlign::Center;
        store.save(&s).unwrap();

        let lida = store.load();
        assert_eq!(lida.font_size, 40);
        assert_eq!(lida.background_opacity, 0.2);
        assert_eq!(lida.text_align, TextAlign::Center);
    }

    #[test]
    fn campo_ausente_assume_o_padrao() {
        let d = dir_temporario("parcial");
        std::fs::write(d.join("settings.json"), r#"{"fontSize": 33}"#).unwrap();

        let s = SettingsStore::new(&d).load();
        assert_eq!(s.font_size, 33);
        assert_eq!(s.blur, Settings::default().blur, "o resto veio do padrão");
    }

    #[test]
    fn campo_desconhecido_nao_quebra() {
        let d = dir_temporario("futuro");
        std::fs::write(
            d.join("settings.json"),
            r#"{"fontSize": 30, "recursoQueAindaNaoExiste": true}"#,
        )
        .unwrap();

        assert_eq!(SettingsStore::new(&d).load().font_size, 30);
    }

    #[test]
    fn json_corrompido_cai_no_padrao_em_vez_de_explodir() {
        let d = dir_temporario("corrompido");
        std::fs::write(d.join("settings.json"), "{isto não é json").unwrap();

        assert_eq!(SettingsStore::new(&d).load(), Settings::default());
    }

    #[test]
    fn valores_absurdos_sao_contidos() {
        let d = dir_temporario("absurdo");
        std::fs::write(
            d.join("settings.json"),
            r#"{"fontSize": 9000, "width": 1, "backgroundOpacity": 4.2, "syncOffsetMs": -99999}"#,
        )
        .unwrap();

        let s = SettingsStore::new(&d).load();
        assert_eq!(s.font_size, 96);
        assert_eq!(s.width, 240);
        assert_eq!(s.background_opacity, 1.0);
        assert_eq!(s.sync_offset_ms, -10_000);
    }
}
