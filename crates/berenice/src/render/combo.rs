//! Sistema de Cadência Fixa, Encadeamento de Combos Rítmicos, Input Buffer e Feedback Visual (SPEC-0021).
//!
//! Controla os estágios de combo (Hit 1 -> Hit 2 -> Hit 3 Finisher) com janelas
//! de tempo rítmicas inspiradas em Action-RPGs, input buffering para perdoar
//! desvios de timing, suporte a segurar contínuo (hold-to-repeat) e jab fail-safe
//! (zero alocações na heap).

use std::time::Instant;

/// TTL máximo de retenção no buffer de entrada (350ms perdoa inputs antecipados com conforto).
pub const INPUT_BUFFER_TTL_MS: u64 = 350;

/// Estágios do combo encadeado de combate corpo-a-corpo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComboStage {
    /// Golpe 1: Corte descendente inicial / Jab seguro.
    Hit1,
    /// Golpe 2: Corte transversal de retorno (contra-golpe rítmico invertido).
    Hit2,
    /// Golpe 3: Finalizador pesado (impacto concentrado com follow-through prolongado).
    Hit3,
}

impl ComboStage {
    /// Duração total da execução da animação em milissegundos.
    #[inline]
    pub fn duration_ms(self) -> u64 {
        match self {
            ComboStage::Hit1 => 540,
            ComboStage::Hit2 => 480,
            ComboStage::Hit3 => 720,
        }
    }

    /// Janela de tempo de input para encadeamento rítmico: `(min_ms, max_ms)`.
    /// Pressionar antes de `min_ms` é absorvido pelo Input Buffer sem descarte.
    /// Pressionar entre `min_ms` e `max_ms` avança com sucesso para o próximo estágio.
    #[inline]
    pub fn combo_window_ms(self) -> (u64, u64) {
        match self {
            ComboStage::Hit1 => (340, 650),
            ComboStage::Hit2 => (300, 600),
            ComboStage::Hit3 => (0, 0), // O finalizador conclui o ciclo
        }
    }

    /// Multiplicador de velocidade de reprodução para sincronizar os 9 quadros da ação ACT.
    #[inline]
    pub fn anim_speed_mult(self) -> f32 {
        match self {
            ComboStage::Hit1 => 1.50, // 900ms / 600ms: claro, nítido e com peso
            ComboStage::Hit2 => 1.65, // 900ms / 545ms: contra-golpe fluido
            ComboStage::Hit3 => 1.25, // 900ms / 720ms: impacto pesado e clímax
        }
    }

    /// Multiplicador de dano aplicado no golpe do combo.
    #[inline]
    pub fn damage_multiplier(self) -> f32 {
        match self {
            ComboStage::Hit1 => 1.00,
            ComboStage::Hit2 => 1.15,
            ComboStage::Hit3 => 1.50,
        }
    }

    /// Se o estágio utiliza espelhamento horizontal para criar o golpe invertido de retorno.
    #[inline]
    pub fn is_inverted(self) -> bool {
        matches!(self, ComboStage::Hit2)
    }

    /// Rótulo para exibição no painel HUD de combate.
    #[inline]
    pub fn hud_label(self) -> &'static str {
        match self {
            ComboStage::Hit1 => "COMBO [1/3: CORTE DESCENDENTE (1.0x)]",
            ComboStage::Hit2 => "COMBO [2/3: CORTE RETORNO (1.15x)]",
            ComboStage::Hit3 => "COMBO [3/3: FINALIZADOR PESADO (1.50x)]",
        }
    }
}

/// Tipo de intenção de ataque retida no buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferedAction {
    Combo,
    Jab,
}

/// Rastreador de estado, input buffer e temporização da cadeia de combos do personagem.
#[derive(Debug, Clone)]
pub struct ComboTracker {
    pub current_stage: Option<ComboStage>,
    pub stage_start_time: Instant,
    pub last_attack_time: Instant,
    pub input_buffer: Option<(Instant, BufferedAction)>,
    pub is_jab_mode: bool,
}

impl Default for ComboTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ComboTracker {
    pub fn new() -> Self {
        let past = Instant::now() - std::time::Duration::from_secs(60);
        Self {
            current_stage: None,
            stage_start_time: past,
            last_attack_time: past,
            input_buffer: None,
            is_jab_mode: false,
        }
    }

    #[inline]
    fn execute_stage(&mut self, stage: ComboStage, now: Instant, is_jab: bool) {
        self.current_stage = Some(stage);
        self.stage_start_time = now;
        self.last_attack_time = now;
        self.is_jab_mode = is_jab;
        self.input_buffer = None;
    }

    /// Registra uma intenção de ataque do combo principal (Gatilhos R2/L2 ou Espaço/Z).
    /// Se a janela ainda não abriu, o comando é guardado no input buffer (perdoa erros).
    pub fn trigger_attack(&mut self, now: Instant) -> bool {
        let elapsed = now.duration_since(self.stage_start_time).as_millis() as u64;

        if let Some(stage) = self.current_stage {
            let (min_w, max_w) = stage.combo_window_ms();
            if min_w > 0 && elapsed >= min_w && elapsed <= max_w {
                // Dentro da janela: avança diretamente!
                let next = match stage {
                    ComboStage::Hit1 => ComboStage::Hit2,
                    ComboStage::Hit2 => ComboStage::Hit3,
                    ComboStage::Hit3 => ComboStage::Hit1,
                };
                self.execute_stage(next, now, false);
                true
            } else if elapsed >= stage.duration_ms() {
                // Golpe anterior concluiu: inicia novo combo
                self.execute_stage(ComboStage::Hit1, now, false);
                true
            } else {
                // Pressionado cedo demais: armazena no buffer de entrada!
                self.input_buffer = Some((now, BufferedAction::Combo));
                false
            }
        } else {
            // Golpe inicial do combo
            self.execute_stage(ComboStage::Hit1, now, false);
            true
        }
    }

    /// Registra intenção de Jab Fail-Safe (Botão Quadrado / West).
    /// Sempre aciona ou reseta para o Hit 1 seguro com recuperação rápida.
    pub fn trigger_jab(&mut self, now: Instant) -> bool {
        let elapsed = now.duration_since(self.stage_start_time).as_millis() as u64;

        if let Some(stage) = self.current_stage {
            if elapsed >= 240 || elapsed >= stage.duration_ms() {
                // Cancelamento rápido para o Jab seguro
                self.execute_stage(ComboStage::Hit1, now, true);
                true
            } else {
                // Antecipado: guarda no buffer como Jab
                self.input_buffer = Some((now, BufferedAction::Jab));
                false
            }
        } else {
            self.execute_stage(ComboStage::Hit1, now, true);
            true
        }
    }

    /// Tick por frame (60 FPS): processa segurar contínuo (hold-to-repeat) e drena input buffer.
    pub fn tick(&mut self, now: Instant, is_holding_combo: bool, is_holding_jab: bool) {
        // 1. Limpa buffer expirado (> 350ms)
        if let Some((buf_time, _)) = self.input_buffer {
            if now.duration_since(buf_time).as_millis() as u64 > INPUT_BUFFER_TTL_MS {
                self.input_buffer = None;
            }
        }

        let elapsed = now.duration_since(self.stage_start_time).as_millis() as u64;

        // 2. Se o jogador está SEGURANDO o gatilho de combo normal:
        if is_holding_combo {
            match self.current_stage {
                None => {
                    self.execute_stage(ComboStage::Hit1, now, false);
                }
                Some(ComboStage::Hit1) if elapsed >= 380 => {
                    self.execute_stage(ComboStage::Hit2, now, false);
                }
                Some(ComboStage::Hit2) if elapsed >= 340 => {
                    self.execute_stage(ComboStage::Hit3, now, false);
                }
                Some(ComboStage::Hit3) if elapsed >= 720 => {
                    self.execute_stage(ComboStage::Hit1, now, false);
                }
                _ => {}
            }
            return;
        }

        // 3. Se o jogador está SEGURANDO o botão de jab (Quadrado):
        if is_holding_jab {
            match self.current_stage {
                None => {
                    self.execute_stage(ComboStage::Hit1, now, true);
                }
                Some(_) if elapsed >= 450 => {
                    self.execute_stage(ComboStage::Hit1, now, true);
                }
                _ => {}
            }
            return;
        }

        // 4. Se houver comando no buffer de entrada (perdoando erros menores de timing):
        if let Some((_buf_time, action)) = self.input_buffer {
            match action {
                BufferedAction::Jab => {
                    if !self.is_attacking(now) || elapsed >= 240 {
                        self.execute_stage(ComboStage::Hit1, now, true);
                    }
                }
                BufferedAction::Combo => {
                    if let Some(stage) = self.current_stage {
                        let (min_w, max_w) = stage.combo_window_ms();
                        if min_w > 0 && elapsed >= min_w && elapsed <= max_w {
                            let next = match stage {
                                ComboStage::Hit1 => ComboStage::Hit2,
                                ComboStage::Hit2 => ComboStage::Hit3,
                                ComboStage::Hit3 => ComboStage::Hit1,
                            };
                            self.execute_stage(next, now, false);
                        } else if elapsed >= stage.duration_ms() {
                            self.execute_stage(ComboStage::Hit1, now, false);
                        }
                    } else {
                        self.execute_stage(ComboStage::Hit1, now, false);
                    }
                }
            }
        }
    }

    /// Retorna se o personagem está ativamente desferindo um golpe.
    #[inline]
    pub fn is_attacking(&self, now: Instant) -> bool {
        if let Some(stage) = self.current_stage {
            (now.duration_since(self.stage_start_time).as_millis() as u64) < stage.duration_ms()
        } else {
            false
        }
    }

    /// Retorna se o personagem permanece em postura de combate (Battle Ready Stance)
    /// por até 2,5 segundos após o último ataque.
    #[inline]
    pub fn in_combat_stance(&self, now: Instant) -> bool {
        now.duration_since(self.last_attack_time).as_millis() < 2500
    }

    /// Calcula o tempo decorrido ponderado para sincronização dos quadros ACT.
    #[inline]
    pub fn anim_elapsed_ms(&self, now: Instant) -> f32 {
        if let Some(stage) = self.current_stage {
            let raw_elapsed = now.duration_since(self.stage_start_time).as_millis() as f32;
            raw_elapsed * stage.anim_speed_mult()
        } else {
            0.0
        }
    }

    /// Indica se o golpe atual utiliza corte invertido (Hit 2).
    #[inline]
    pub fn is_inverted(&self, now: Instant) -> bool {
        if self.is_attacking(now) && !self.is_jab_mode {
            self.current_stage.map(|s| s.is_inverted()).unwrap_or(false)
        } else {
            false
        }
    }

    /// Rótulo para o painel HUD de combate.
    #[inline]
    pub fn current_hud_label(&self) -> &'static str {
        if self.is_jab_mode {
            "SWORDIE [JAB: CORTE RÁPIDO (FAIL-SAFE 1.0x)]"
        } else if let Some(stage) = self.current_stage {
            stage.hud_label()
        } else {
            "SWORDIE [ATTACK SWING]"
        }
    }
}
