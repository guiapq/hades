//! SPEC-0025 & SPEC-0026: Gritos de Batalha Nórdicos e Descanso Automático (Auto-Idle Sit).
//!
//! Exibe brados temáticos inspirados na mitologia e geografia nórdica/escandinava
//! com proc rate equilibrado (não repetitivo), emoticons de descanso ao sentar
//! após 10s de inatividade e falas rápidas de prontidão ao levantar.
//! Orçamento estrito: zero alocações na heap.

use super::software_framebuffer::SoftwareFramebuffer;
use std::time::Instant;

const JAB_SHOUTS: [&str; 5] = [
    "SKAL!",
    "BY TYR!",
    "HRAST!",
    "FROST BITE!",
    "SWIFT RUNE!",
];

const HIT1_SHOUTS: [&str; 4] = [
    "WRATH OF THE FJORD!",
    "FOR MIDGARD!",
    "VALKYRIE'S STRIKE!",
    "RUNE CLEAVE!",
];

const HIT2_SHOUTS: [&str; 4] = [
    "TEMPEST OF THOR!",
    "EINHERJAR'S FURY!",
    "WOLF OF FENRIR!",
    "NORTHERN GALE!",
];

const HIT3_SHOUTS: [&str; 4] = [
    "FOR VALHALLA!",
    "MJOLNIR'S THUNDERCLAP!",
    "RAGNAROK'S WRATH!",
    "ODIN'S JUDGMENT!",
];

const DODGE_SHOUTS: [&str; 4] = [
    "LIKE THE NORTH WIND!",
    "MIST OF NIFLHEIM!",
    "SHADOW OF YGGDRASIL!",
    "TOO SLOW, MORTAL!",
];

const IDLE_EMOTICONS: [&str; 5] = [
    "( _ _ )zzZ",
    "( ´-ω-) ☕",
    "Camping in Midgard...",
    "Taking a breather...",
    "[Resting: +HP/+SP]",
];

const WAKEUP_DIALOGUES: [&str; 5] = [
    "Back to battle!",
    "Midgard calls!",
    "Let's move!",
    "Rest is over!",
    "Up and ready!",
];

/// Registro do grito ou fala de combate ativo no momento.
#[derive(Debug, Clone, Copy)]
pub struct BattleShout {
    pub text: &'static str,
    pub color: u32,
    pub start_time: Instant,
    pub duration_ms: f32,
    pub scale: usize,
}

/// Gerenciador de gritos nórdicos, falas e emoticons sem alocações dinâmicas.
pub struct BattleShoutTracker {
    pub current_shout: Option<BattleShout>,
    rng_state: u64,
}

impl Default for BattleShoutTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl BattleShoutTracker {
    pub fn new() -> Self {
        Self {
            current_shout: None,
            rng_state: 0xA5A5_F00D_1234_5678,
        }
    }

    #[inline]
    fn next_rand_f32(&mut self) -> f32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        let bits = (self.rng_state >> 40) as u32;
        (bits as f32) / 16777216.0
    }

    #[inline]
    fn next_rand_idx(&mut self, len: usize) -> usize {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        (self.rng_state as usize) % len
    }

    /// Dispara grito para golpe rápido de Jab com 30% de chance de proc.
    pub fn trigger_jab(&mut self, now: Instant) {
        if self.next_rand_f32() <= 0.30 {
            self.trigger_jab_guaranteed(now);
        }
    }

    /// Força disparo garantido de brado de Jab (útil para testes determinísticos).
    pub fn trigger_jab_guaranteed(&mut self, now: Instant) {
        let idx = self.next_rand_idx(JAB_SHOUTS.len());
        self.current_shout = Some(BattleShout {
            text: JAB_SHOUTS[idx],
            color: 0xFF8BE9FD, // Ciano
            start_time: now,
            duration_ms: 450.0,
            scale: 1,
        });
    }

    /// Dispara grito para Hit 1 do combo com 40% de chance de proc.
    pub fn trigger_hit1(&mut self, now: Instant) {
        if self.next_rand_f32() <= 0.40 {
            self.trigger_hit1_guaranteed(now);
        }
    }

    pub fn trigger_hit1_guaranteed(&mut self, now: Instant) {
        let idx = self.next_rand_idx(HIT1_SHOUTS.len());
        self.current_shout = Some(BattleShout {
            text: HIT1_SHOUTS[idx],
            color: 0xFFF1FA8C, // Amarelo
            start_time: now,
            duration_ms: 550.0,
            scale: 1,
        });
    }

    /// Dispara grito para Hit 2 do combo com 50% de chance de proc.
    pub fn trigger_hit2(&mut self, now: Instant) {
        if self.next_rand_f32() <= 0.50 {
            self.trigger_hit2_guaranteed(now);
        }
    }

    pub fn trigger_hit2_guaranteed(&mut self, now: Instant) {
        let idx = self.next_rand_idx(HIT2_SHOUTS.len());
        self.current_shout = Some(BattleShout {
            text: HIT2_SHOUTS[idx],
            color: 0xFFFFB86C, // Laranja
            start_time: now,
            duration_ms: 600.0,
            scale: 1,
        });
    }

    /// Dispara grito de Finalizador Crítico / Hit 3 (100% de proc garantido).
    pub fn trigger_hit3(&mut self, now: Instant) {
        let idx = self.next_rand_idx(HIT3_SHOUTS.len());
        self.current_shout = Some(BattleShout {
            text: HIT3_SHOUTS[idx],
            color: 0xFFFF5555, // Vermelho/Dourado em fonte 2x
            start_time: now,
            duration_ms: 950.0,
            scale: 2,
        });
    }

    /// Dispara grito para manobra evasiva (L2) com 35% de chance de proc.
    pub fn trigger_dodge(&mut self, now: Instant) {
        if self.next_rand_f32() <= 0.35 {
            self.trigger_dodge_guaranteed(now);
        }
    }

    pub fn trigger_dodge_guaranteed(&mut self, now: Instant) {
        let idx = self.next_rand_idx(DODGE_SHOUTS.len());
        self.current_shout = Some(BattleShout {
            text: DODGE_SHOUTS[idx],
            color: 0xFFBD93F9, // Púrpura
            start_time: now,
            duration_ms: 500.0,
            scale: 1,
        });
    }

    /// Emite um emoticon relaxado durante o descanso sentado (Auto-Idle).
    pub fn trigger_idle_emoticon(&mut self, now: Instant) {
        let idx = self.next_rand_idx(IDLE_EMOTICONS.len());
        self.current_shout = Some(BattleShout {
            text: IDLE_EMOTICONS[idx],
            color: 0xFF50FA7B, // Verde relaxado pastel
            start_time: now,
            duration_ms: 1800.0,
            scale: 1,
        });
    }

    /// Emite diálogo rápido ao se levantar do descanso (Wake-up).
    pub fn trigger_wakeup(&mut self, now: Instant) {
        let idx = self.next_rand_idx(WAKEUP_DIALOGUES.len());
        self.current_shout = Some(BattleShout {
            text: WAKEUP_DIALOGUES[idx],
            color: 0xFFF8F8F2, // Branco brilhante
            start_time: now,
            duration_ms: 900.0,
            scale: 1,
        });
    }

    /// Atualização de ciclo por quadro.
    pub fn tick(&mut self, now: Instant) {
        if let Some(shout) = self.current_shout {
            let elapsed = now.duration_since(shout.start_time).as_millis() as f32;
            if elapsed >= shout.duration_ms {
                self.current_shout = None;
            }
        }
    }

    /// Renderiza o balão / banner sobre a cabeça do jogador.
    pub fn render(
        &self,
        fb: &mut SoftwareFramebuffer,
        anchor_x: i32,
        anchor_y: i32,
        now: Instant,
    ) {
        if let Some(shout) = self.current_shout {
            let elapsed = now.duration_since(shout.start_time).as_millis() as f32;
            if elapsed >= shout.duration_ms {
                return;
            }

            // Flutuação ascendente suave
            let progress = elapsed / shout.duration_ms;
            let float_y = (progress * 18.0) as i32;

            let char_w = if shout.scale == 2 { 16 } else { 8 };
            let total_w = (shout.text.len() as i32) * char_w;
            let px = anchor_x - (total_w / 2);
            let py = anchor_y - 65 - float_y;

            let pad_x = 6;
            let pad_y = 4;
            let box_h = if shout.scale == 2 { 22 } else { 14 };

            fb.draw_rect(
                px - pad_x,
                py - pad_y,
                total_w + pad_x * 2,
                box_h + pad_y * 2,
                0xDD14141E,
                Some(shout.color),
            );

            fb.draw_text(px + 1, py + 1, shout.text, 0xFF000000, shout.scale);
            fb.draw_text(px, py, shout.text, shout.color, shout.scale);
        }
    }
}

const RELAXED_EMOTION_ACTIONS: [usize; 5] = [
    5,  // zzZ (Sleep)
    6,  // ♪ (Music / Whistling)
    3,  // ... (Daydreaming / Quiet)
    19, // ☕ (Tea / Coffee)
    2,  // ♥ (Content / Peace)
];

/// SPEC-0026: Rastreador de inatividade para descanso automático (Auto-Idle Sit).
#[derive(Debug, Clone, Copy)]
pub struct AutoIdleTracker {
    pub last_input_time: Instant,
    pub is_sitting: bool,
    pub last_emoticon_time: Instant,
    pub next_interval_ms: u64,
    pub current_emotion_action: Option<usize>,
    pub emotion_start_time: Instant,
    pub idle_threshold_secs: u64,
    rng_state: u64,
}

impl Default for AutoIdleTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl AutoIdleTracker {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            last_input_time: now,
            is_sitting: false,
            last_emoticon_time: now,
            next_interval_ms: 7000,
            current_emotion_action: None,
            emotion_start_time: now,
            idle_threshold_secs: 10,
            rng_state: 0xC001_CAFE_BABE_FACE,
        }
    }

    #[inline]
    fn next_rand_f32(&mut self) -> f32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        let bits = (self.rng_state >> 40) as u32;
        (bits as f32) / 16777216.0
    }

    /// Registra input ativo do jogador. Retorna `true` se o personagem acabou de acordar.
    pub fn register_input(&mut self, now: Instant) -> bool {
        self.last_input_time = now;
        if self.is_sitting {
            self.is_sitting = false;
            self.current_emotion_action = None;
            let r = self.next_rand_f32();
            self.next_interval_ms = 6000 + (r * 8000.0) as u64;
            true
        } else {
            false
        }
    }

    /// Verifica e atualiza o estado de descanso e temporizador de emoticons.
    /// Retorna `Some(action_idx)` quando um novo emoticon animado do GRF é disparado.
    pub fn tick(&mut self, now: Instant) -> Option<usize> {
        // Encerra a animação do emoticon após 1500ms
        if self.current_emotion_action.is_some() {
            if now.duration_since(self.emotion_start_time).as_millis() >= 1500 {
                self.current_emotion_action = None;
            }
        }

        let idle_secs = now.duration_since(self.last_input_time).as_secs();
        if idle_secs >= self.idle_threshold_secs {
            if !self.is_sitting {
                self.is_sitting = true;
                self.last_emoticon_time = now;
                let r = self.next_rand_f32();
                // Primeiro emoticon após sentar: 6.0s a 14.0s de espera inicial relaxada
                self.next_interval_ms = 6000 + (r * 8000.0) as u64;
            }

            if now.duration_since(self.last_emoticon_time).as_millis() >= self.next_interval_ms as u128 {
                self.last_emoticon_time = now;
                let rand_val = self.next_rand_f32();
                // Intervalos seguintes bem espaçados, suaves e irregulares: entre 12.0s e 28.0s
                self.next_interval_ms = 12000 + (rand_val * 16000.0) as u64;

                let idx = ((rand_val * 100.0) as usize) % RELAXED_EMOTION_ACTIONS.len();
                let action = RELAXED_EMOTION_ACTIONS[idx];
                self.current_emotion_action = Some(action);
                self.emotion_start_time = now;
                return Some(action);
            }
        }
        None
    }

    /// Retorna `(action_idx, elapsed_ms)` se houver animação de emoticon ativa no momento.
    pub fn active_emotion(&self, now: Instant) -> Option<(usize, f32)> {
        let action = self.current_emotion_action?;
        let elapsed = now.duration_since(self.emotion_start_time).as_millis() as f32;
        if elapsed < 1500.0 {
            Some((action, elapsed))
        } else {
            None
        }
    }
}
