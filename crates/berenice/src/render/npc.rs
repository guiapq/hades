//! SPEC-0028 / VALVE.md: Entidade de Guia NPC Interativa (Living World).
//!
//! Implementa NPCs orgânicos do mundo que reagem à proximidade do jogador com
//! giro de olhar/corpo (Facing Proximity Tracking), balões de diálogo não-modais
//! (Non-Modal Organic Dialogue) e emoticons expressivos (Emotions/ACT), sem
//! travar a movimentação ou poluir a tela do jogador.

use hades_core::types::{Direction, Position};
use std::time::{Duration, Instant};

/// Estado e lógica do Guia de Prontera (Guide NPC).
pub struct GuideNpc {
    pub position: Position,
    pub facing: Direction,
    pub default_facing: Direction,
    pub is_talking: bool,
    pub dialogue_page: usize,
    pub last_interaction_time: Instant,
    pub active_emotion: Option<(usize, Instant)>,
}

impl GuideNpc {
    pub fn new(position: Position) -> Self {
        let now = Instant::now() - Duration::from_secs(60);
        Self {
            position,
            facing: Direction::SouthEast,
            default_facing: Direction::SouthEast,
            is_talking: false,
            dialogue_page: 0,
            last_interaction_time: now,
            active_emotion: None,
        }
    }

    /// Atualiza o NPC a cada frame com base na posição do jogador.
    pub fn tick(&mut self, player_pos: Position, now: Instant) {
        let dx = player_pos.x as i32 - self.position.x as i32;
        let dy = player_pos.y as i32 - self.position.y as i32;
        let dist_sq = dx * dx + dy * dy;

        // Se o jogador estiver em um raio de até 5 células, o NPC olha diretamente para ele
        if dist_sq <= 25 {
            if let Some(dir) = Direction::from_delta(dx, dy) {
                self.facing = dir;
            }
        } else {
            // Retorna suavemente para a postura padrão receptiva
            self.facing = self.default_facing;
            // Se o jogador se afastar enquanto falava, fecha o diálogo organicamente sem modal lock
            if self.is_talking && dist_sq > 36 {
                self.is_talking = false;
            }
        }

        // Expira o emoticon após 1.8 segundos
        if let Some((_, start)) = self.active_emotion {
            if now.duration_since(start).as_millis() > 1800 {
                self.active_emotion = None;
            }
        }
    }

    /// Interage com o NPC (via clique do mouse sobre ela ou tecla de ação/Espaço).
    pub fn interact(&mut self, now: Instant) {
        self.last_interaction_time = now;
        if !self.is_talking {
            self.is_talking = true;
            self.dialogue_page = 0;
            // Emite emoticon alegre de saudação (Ação 1 ou 2 do emotion.act)
            self.active_emotion = Some((1, now));
        } else {
            self.dialogue_page = (self.dialogue_page + 1) % 3;
            // A cada avanço de página, emite uma expressão sutil
            let emo_idx = match self.dialogue_page {
                1 => 2, // Piscadela / consentimento
                2 => 0, // Coração / contentamento
                _ => 1, // Música / alegria
            };
            self.active_emotion = Some((emo_idx, now));
        }
    }

    /// Fecha o diálogo manualmente se necessário.
    pub fn close_dialogue(&mut self) {
        self.is_talking = false;
    }

    /// Retorna o conteúdo da página atual de diálogo: `(título, linhas, dica)`.
    pub fn current_dialogue(&self) -> (&'static str, &'static [&'static str], Option<&'static str>) {
        match self.dialogue_page {
            0 => (
                "GUIA DE PRONTERA",
                &[
                    "Bem-vindo ao coracao de Prontera!",
                    "Respire a brisa fresca da praca central.",
                    "Aqui voce esta seguro e em boa companhia.",
                ],
                Some("[ESPACO / CLIQUE PARA OUVIR MAIS]"),
            ),
            1 => (
                "SISTEMA DE COMBATE SUAVE",
                &[
                    "Nosso combate valoriza ritmo e cadencia.",
                    "Encadeie seus golpes leves com fluidez.",
                    "Ao esquivar com L2 ou Espaco, deslize suavemente.",
                ],
                Some("[ESPACO / CLIQUE PARA AVANCAR]"),
            ),
            _ => (
                "O MUNDO VIVO DE HADES",
                &[
                    "Interaja com a agua da fonte para ver as ondas.",
                    "Descanse sentado para acelerar sua recuperacao.",
                    "Explore os arredores com calma e no seu ritmo!",
                ],
                Some("[ESPACO / CLIQUE PARA REINICIAR]"),
            ),
        }
    }
}
