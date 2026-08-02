//! Relógio de reprodução.
//!
//! A fonte de mídia diz onde a música está só de vez em quando, e com precisão
//! limitada. Este relógio guarda a última âncora confiável e extrapola dali,
//! para que a letra possa ser desenhada a 60fps sem consultar nada.

use std::time::Instant;

/// Acima disto, a diferença entre o observado e o estimado não é imprecisão:
/// o usuário pulou na faixa.
pub const SEEK_THRESHOLD_MS: i64 = 500;

#[derive(Debug, Clone)]
pub struct Clock {
    anchor_ms: u64,
    anchor_at: Instant,
    playing: bool,
    /// Ajuste manual do usuário. Cada fonte tem latência própria.
    offset_ms: i64,
    anchored: bool,
}

impl Default for Clock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock {
    pub fn new() -> Self {
        Self {
            anchor_ms: 0,
            anchor_at: Instant::now(),
            playing: false,
            offset_ms: 0,
            anchored: false,
        }
    }

    /// Fixa uma posição conhecida no instante atual.
    pub fn anchor(&mut self, position_ms: u64) {
        self.anchor_ms = position_ms;
        self.anchor_at = Instant::now();
        self.anchored = true;
    }

    /// Como [`Clock::anchor`], mas para um instante que já passou — o caso de
    /// uma leitura que levou tempo para voltar do barramento.
    pub fn anchor_at(&mut self, position_ms: u64, observed_at: Instant) {
        self.anchor_ms = position_ms;
        self.anchor_at = observed_at;
        self.anchored = true;
    }

    pub fn set_playing(&mut self, playing: bool) {
        if playing == self.playing {
            return;
        }
        // Congela onde está antes de trocar de estado, senão o tempo parado
        // conta como tempo tocado.
        self.anchor_ms = self.position_ms();
        self.anchor_at = Instant::now();
        self.playing = playing;
    }

    pub fn set_offset_ms(&mut self, offset_ms: i64) {
        self.offset_ms = offset_ms;
    }

    pub fn is_anchored(&self) -> bool {
        self.anchored
    }

    /// Posição estimada agora, sem o ajuste do usuário.
    pub fn raw_position_ms(&self) -> u64 {
        if !self.playing {
            return self.anchor_ms;
        }
        self.anchor_ms + self.anchor_at.elapsed().as_millis() as u64
    }

    /// Posição estimada agora, com o ajuste do usuário aplicado.
    pub fn position_ms(&self) -> u64 {
        let raw = self.raw_position_ms() as i64 + self.offset_ms;
        raw.max(0) as u64
    }

    /// Quanto a estimativa está adiantada em relação ao que a fonte reporta.
    /// Positivo = o relógio está à frente da música.
    pub fn drift_ms(&self, observed_ms: u64) -> i64 {
        self.raw_position_ms() as i64 - observed_ms as i64
    }

    /// Só faz sentido chamar depois de já haver uma âncora: antes disso
    /// qualquer diferença é ruído de partida, não um salto.
    pub fn looks_like_seek(&self, observed_ms: u64) -> bool {
        self.anchored && self.drift_ms(observed_ms).abs() > SEEK_THRESHOLD_MS
    }

    #[cfg(test)]
    fn rewind_anchor(&mut self, by: std::time::Duration) {
        self.anchor_at -= by;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn parado_nao_avanca() {
        let mut c = Clock::new();
        c.anchor(10_000);
        c.rewind_anchor(Duration::from_secs(5));
        assert_eq!(c.position_ms(), 10_000);
    }

    #[test]
    fn tocando_avanca_com_o_tempo() {
        let mut c = Clock::new();
        c.set_playing(true);
        c.anchor(10_000);
        c.rewind_anchor(Duration::from_secs(3));
        let p = c.position_ms();
        assert!((12_900..=13_100).contains(&p), "posição inesperada: {p}");
    }

    #[test]
    fn pausar_congela_a_posicao_corrida() {
        let mut c = Clock::new();
        c.set_playing(true);
        c.anchor(10_000);
        c.rewind_anchor(Duration::from_secs(2));

        c.set_playing(false);
        let ao_pausar = c.position_ms();
        c.rewind_anchor(Duration::from_secs(30));

        assert_eq!(c.position_ms(), ao_pausar, "avançou enquanto pausado");
        assert!((11_900..=12_100).contains(&ao_pausar));
    }

    #[test]
    fn offset_desloca_a_posicao() {
        let mut c = Clock::new();
        c.anchor(10_000);
        c.set_offset_ms(-250);
        assert_eq!(c.position_ms(), 9_750);

        // Offset negativo maior que a posição não pode virar tempo negativo.
        c.anchor(100);
        c.set_offset_ms(-5_000);
        assert_eq!(c.position_ms(), 0);
    }

    #[test]
    fn salto_so_conta_depois_da_primeira_ancora() {
        let mut c = Clock::new();
        assert!(!c.looks_like_seek(90_000), "sem âncora não existe salto");

        c.set_playing(true);
        c.anchor(10_000);
        assert!(!c.looks_like_seek(10_100), "100ms é imprecisão, não salto");
        assert!(c.looks_like_seek(90_000), "80s de diferença é salto");
    }

    #[test]
    fn drift_tem_sinal_util() {
        let mut c = Clock::new();
        c.set_playing(true);
        c.anchor(10_000);
        // Fonte reporta menos do que estimamos: o relógio está adiantado.
        assert!(c.drift_ms(9_000) > 0);
        assert!(c.drift_ms(11_000) < 0);
    }
}
