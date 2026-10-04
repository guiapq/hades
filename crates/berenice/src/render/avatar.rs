//! Sistema de Composição de Camadas Visuais e Pontos de Ancoragem do Avatar (SPEC-0020).
//!
//! Gerencia a renderização em camadas ordenadas por profundidade (Z-order)
//! dependente do ângulo de visão da câmera (facing relativo), calculando
//! offsets dinâmicos de pontos de ancoragem (Attach Points) para acoplar
//! cabeça, elmos/chapéus e armas ao corpo em movimento sem alocações dinâmicas.

use hades_ro_prere::act_parser::{ActFrame, AttachPoint};
use hades_ro_prere::spr_parser::{Sprite, SpriteFrame};
use super::software_framebuffer::SoftwareFramebuffer;

/// Camadas visuais canônicas de um personagem em MMORPG 2.5D clássico.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualLayer {
    Shadow,
    Weapon,
    Body,
    Head,
    Headgear,
}

/// Determina a ordem Z estrita de desenho das camadas com base na direção visual.
///
/// Para direções 3, 4, 5 (Noroeste, Norte, Nordeste - personagem de costas para a câmera):
/// A arma é empunhada à frente do corpo do personagem, o que significa que, da
/// perspectiva da câmera, ela fica ATRÁS do corpo:
/// `Shadow -> Weapon -> Body -> Head -> Headgear`
///
/// Para direções 0, 1, 2, 6, 7 (Sul, Sudoeste, Oeste, Leste, Sudeste - frente e laterais):
/// A arma fica À FRENTE de todas as camadas corporais:
/// `Shadow -> Body -> Head -> Headgear -> Weapon`
#[inline]
pub fn get_layer_order(act_dir: usize) -> [VisualLayer; 5] {
    if matches!(act_dir, 3..=5) {
        [
            VisualLayer::Shadow,
            VisualLayer::Weapon,
            VisualLayer::Body,
            VisualLayer::Head,
            VisualLayer::Headgear,
        ]
    } else {
        [
            VisualLayer::Shadow,
            VisualLayer::Body,
            VisualLayer::Head,
            VisualLayer::Headgear,
            VisualLayer::Weapon,
        ]
    }
}

/// Calcula o deslocamento em pixels na tela entre dois pontos de fixação (AttachPoint).
///
/// Fórmula:
/// Δx = ((Parent.x - Child.x) * zoom).round()
/// Δy = ((Parent.y - Child.y) * zoom).round()
#[inline]
pub fn calculate_attach_offset(
    parent_attach: Option<&AttachPoint>,
    child_attach: Option<&AttachPoint>,
    zoom: f32,
) -> (i32, i32) {
    match (parent_attach, child_attach) {
        (Some(p), Some(c)) => {
            let dx = ((p.x - c.x) as f32 * zoom).round() as i32;
            let dy = ((p.y - c.y) as f32 * zoom).round() as i32;
            (dx, dy)
        }
        _ => (0, 0),
    }
}

/// Parâmetros de renderização de um avatar composto multi-camada.
///
/// Projetado para execução com zero alocações na pilha de execução (Potato Budget).
pub struct AvatarRenderParams<'a> {
    pub anchor_x: i32,
    pub anchor_y: i32,
    pub act_dir: usize,
    pub zoom: f32,
    pub shadow_frame: Option<&'a SpriteFrame>,
    pub body: Option<(&'a ActFrame, &'a Sprite)>,
    pub head: Option<(&'a ActFrame, &'a Sprite)>,
    pub headgear: Option<(&'a ActFrame, &'a Sprite)>,
    pub weapon: Option<(&'a ActFrame, &'a Sprite)>,
    /// SPEC-0021: Se verdadeiro, inverte horizontalmente as camadas do golpe (corte transversal Hit 2).
    pub flip_h: bool,
}

impl SoftwareFramebuffer {
    /// Renderiza todas as camadas de um avatar com ordenação Z canônica
    /// e offsets dinâmicos de pontos de fixação.
    pub fn draw_avatar(&mut self, params: &AvatarRenderParams) {
        let flip_h = params.flip_h;

        // 1. Ponto de ancoragem da cabeça relativo ao corpo
        let body_neck = params.body.and_then(|(f, _)| f.attach_points.first());
        let head_neck = params.head.and_then(|(f, _)| f.attach_points.first());
        let (head_dx, head_dy) = calculate_attach_offset(body_neck, head_neck, params.zoom);
        let head_x = if flip_h { params.anchor_x - head_dx } else { params.anchor_x + head_dx };
        let head_y = params.anchor_y + head_dy;

        // 2. Ponto de ancoragem do elmo/chapéu relativo à cabeça
        let hg_neck = params.headgear.and_then(|(f, _)| f.attach_points.first());
        let (hg_dx, hg_dy) = calculate_attach_offset(head_neck, hg_neck, params.zoom);
        let headgear_x = if flip_h { head_x - hg_dx } else { head_x + hg_dx };
        let headgear_y = head_y + hg_dy;

        // 3. Obter ordem das camadas dependente da câmera
        let layers = get_layer_order(params.act_dir);

        for layer in layers {
            match layer {
                VisualLayer::Shadow => {
                    if let Some(shadow) = params.shadow_frame {
                        self.draw_sprite_frame(params.anchor_x, params.anchor_y, shadow, params.zoom);
                    }
                }
                VisualLayer::Weapon => {
                    if let Some((w_frame, w_spr)) = params.weapon {
                        self.draw_act_frame_flipped(params.anchor_x, params.anchor_y, w_frame, w_spr, params.zoom, flip_h);
                    }
                }
                VisualLayer::Body => {
                    if let Some((b_frame, b_spr)) = params.body {
                        self.draw_act_frame_flipped(params.anchor_x, params.anchor_y, b_frame, b_spr, params.zoom, flip_h);
                    }
                }
                VisualLayer::Head => {
                    if let Some((h_frame, h_spr)) = params.head {
                        self.draw_act_frame_flipped(head_x, head_y, h_frame, h_spr, params.zoom, flip_h);
                    }
                }
                VisualLayer::Headgear => {
                    if let Some((hg_frame, hg_spr)) = params.headgear {
                        self.draw_act_frame_flipped(headgear_x, headgear_y, hg_frame, hg_spr, params.zoom, flip_h);
                    }
                }
            }
        }
    }
}
