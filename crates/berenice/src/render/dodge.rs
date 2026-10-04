//! SPEC-0023: Rastreador de Manobra Evasiva (L2 / Esquiva).
//!
//! Controla cooldown, tempo de deslocamento rápido e quadros de invulnerabilidade (i-frames).

use std::time::Instant;

/// Gerenciador de estado e temporização da manobra evasiva (Dodge Roll / Dash).
#[derive(Debug, Clone)]
pub struct DodgeTracker {
    pub last_dodge_time: Instant,
    pub is_dodging: bool,
    pub dodge_start_time: Instant,
    pub cooldown_ms: u64,
    pub duration_ms: u64,
    pub iframe_duration_ms: u64,
}

impl Default for DodgeTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl DodgeTracker {
    pub fn new() -> Self {
        let past = Instant::now() - std::time::Duration::from_secs(60);
        Self {
            last_dodge_time: past,
            is_dodging: false,
            dodge_start_time: past,
            cooldown_ms: 650,
            duration_ms: 180,
            iframe_duration_ms: 260,
        }
    }

    /// Verifica se a manobra evasiva está pronta para uso (fora do cooldown).
    #[inline]
    pub fn can_dodge(&self, now: Instant) -> bool {
        now.duration_since(self.last_dodge_time).as_millis() as u64 >= self.cooldown_ms
    }

    /// Dispara a manobra evasiva se o cooldown permitir.
    #[inline]
    pub fn trigger_dodge(&mut self, now: Instant) -> bool {
        if self.can_dodge(now) {
            self.is_dodging = true;
            self.dodge_start_time = now;
            self.last_dodge_time = now;
            true
        } else {
            false
        }
    }

    /// Retorna se o jogador está em período ativo de quadros de invulnerabilidade (i-frames).
    #[inline]
    pub fn has_iframes(&self, now: Instant) -> bool {
        self.is_dodging
            || now.duration_since(self.dodge_start_time).as_millis() as u64 <= self.iframe_duration_ms
    }

    /// Atualiza estado a cada quadro.
    #[inline]
    pub fn tick(&mut self, now: Instant) {
        if self.is_dodging
            && now.duration_since(self.dodge_start_time).as_millis() as u64 >= self.duration_ms
        {
            self.is_dodging = false;
        }
    }
}
