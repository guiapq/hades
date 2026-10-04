//! SPEC-0022: Entidade de Treino Gelatinosa e Feedback Visual de Combate em Tempo Real.
//!
//! Gerencia a entidade alvo gelatinosa saltitante (Bouncy Slime Training Dummy),
//! estados de reação a dano (Hurt), derrota (Die) e renascimento (Respawn),
//! além de números de dano flutuantes (Floating Combat Text) com zero alocações na heap.

use hades_core::types::{Direction, Position};
use std::time::Instant;

/// Estados de animação e combate da entidade alvo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DummyState {
    /// Salto gelatinoso padrão (Ações 00..07 do ACT).
    Idle,
    /// Compressão elástica e reação de impacto ao sofrer golpe (Ações 24..31 do ACT).
    Hurt,
    /// Dissolução / estouro aquoso ao ser derrotado (Ações 32..39 do ACT).
    Die,
}

/// Entidade de treino gelatinosa reativa.
#[derive(Debug, Clone)]
pub struct TrainingDummy {
    pub position: Position,
    pub spawn_pos: Position,
    pub max_hp: i32,
    pub current_hp: i32,
    pub state: DummyState,
    pub state_start_time: Instant,
    pub last_hit_time: Instant,
    pub facing: Direction,
}

impl TrainingDummy {
    /// Cria uma nova entidade alvo de treino na posição especificada.
    pub fn new(position: Position) -> Self {
        let now = Instant::now();
        Self {
            position,
            spawn_pos: position,
            max_hp: 60,
            current_hp: 60,
            state: DummyState::Idle,
            state_start_time: now,
            last_hit_time: now,
            facing: Direction::South,
        }
    }

    /// Retorna se a entidade é elegível para sofrer dano (não está no estado Die).
    #[inline]
    pub fn is_hittable(&self) -> bool {
        self.state != DummyState::Die
    }

    /// Aplica dano à entidade alvo e calcula transições de estado.
    /// Retorna `(foi_letal, dano_efetivo)`.
    pub fn take_hit(&mut self, damage: i32, now: Instant) -> (bool, i32) {
        if !self.is_hittable() {
            return (false, 0);
        }

        self.current_hp = (self.current_hp - damage).max(0);
        self.last_hit_time = now;
        self.state_start_time = now;

        if self.current_hp == 0 {
            self.state = DummyState::Die;
            (true, damage)
        } else {
            self.state = DummyState::Hurt;
            (false, damage)
        }
    }

    /// Atualização de ciclo por quadro a 60 FPS: decaimento de Hurt e Respawn automático.
    pub fn tick(&mut self, now: Instant) {
        let elapsed = now.duration_since(self.state_start_time).as_millis();

        match self.state {
            DummyState::Hurt => {
                // Após 250ms de reação elástica, retorna para Idle saltitante
                if elapsed >= 250 {
                    self.state = DummyState::Idle;
                    self.state_start_time = now;
                }
            }
            DummyState::Die => {
                // Após 2000ms de derrota, renasce com HP completo na posição de spawn
                if elapsed >= 2000 {
                    self.current_hp = self.max_hp;
                    self.position = self.spawn_pos;
                    self.state = DummyState::Idle;
                    self.state_start_time = now;
                }
            }
            DummyState::Idle => {}
        }
    }

    /// Determina o índice de ação ACT da criatura baseado no estado atual e direção visual da câmera.
    #[inline]
    pub fn action_index(&self, act_dir: usize) -> usize {
        match self.state {
            DummyState::Idle => act_dir,           // Grupo 00: Salto Idle (Ações 0..7)
            DummyState::Hurt => 24 + act_dir,      // Grupo 03: Reação de Dano (Ações 24..31)
            DummyState::Die => 32 + act_dir,       // Grupo 04: Derrota / Estouro (Ações 32..39)
        }
    }

    /// Tempo decorrido no estado atual para reprodução dos quadros ACT.
    #[inline]
    pub fn anim_elapsed_ms(&self, now: Instant) -> f32 {
        now.duration_since(self.state_start_time).as_millis() as f32
    }
}

/// Registro estático de número de dano flutuante na tela (Potato Budget).
#[derive(Debug, Clone, Copy)]
pub struct FloatingNumber {
    pub world_x: f32,
    pub world_y: f32,
    pub damage: i32,
    pub color: u32,
    pub start_time: Instant,
    pub is_crit: bool,
}

/// Fila estática de textos de combate flutuantes com zero alocações na heap.
pub struct FloatingNumberPool {
    entries: [Option<FloatingNumber>; 8],
    next_slot: usize,
}

impl Default for FloatingNumberPool {
    fn default() -> Self {
        Self::new()
    }
}

impl FloatingNumberPool {
    pub fn new() -> Self {
        Self {
            entries: [None; 8],
            next_slot: 0,
        }
    }

    /// Registra um novo número de dano no pool estático circular.
    pub fn spawn(
        &mut self,
        world_x: f32,
        world_y: f32,
        damage: i32,
        color: u32,
        is_crit: bool,
        now: Instant,
    ) {
        self.entries[self.next_slot] = Some(FloatingNumber {
            world_x,
            world_y,
            damage,
            color,
            start_time: now,
            is_crit,
        });
        self.next_slot = (self.next_slot + 1) % self.entries.len();
    }

    /// Itera sobre os números de combate ativos, atualiza suas posições verticais e expira os antigos.
    pub fn active_numbers(&mut self, now: Instant) -> impl Iterator<Item = (f32, f32, i32, u32, bool, f32)> + '_ {
        for slot in self.entries.iter_mut() {
            if let Some(entry) = slot {
                if now.duration_since(entry.start_time).as_millis() > 650 {
                    *slot = None;
                }
            }
        }

        self.entries.iter().filter_map(move |slot| {
            slot.map(|entry| {
                let elapsed_ms = now.duration_since(entry.start_time).as_millis() as f32;
                let float_offset_y = elapsed_ms * 0.045; // Sobe suavemente em direção ao topo da tela
                (
                    entry.world_x,
                    entry.world_y,
                    entry.damage,
                    entry.color,
                    entry.is_crit,
                    float_offset_y,
                )
            })
        })
    }
}
