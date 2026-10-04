//! SPEC-0009: Buffer de renderização em memória (Software Framebuffer) para visualização isométrica e playtesting.

use super::isometric::IsometricProjection;
use hades_core::collision::CollisionGrid;
use hades_core::types::{Direction, Position};
use hades_ro_prere::act_parser::{ActClip, ActFrame};
use hades_ro_prere::bmp_parser::BmpImage;
use hades_ro_prere::gnd_parser::GndMesh;
use hades_ro_prere::rsm_parser::RsmModel;
use hades_ro_prere::spr_parser::{Sprite, SpriteFrame};

/// Cor RGBA empacotada em 32 bits (0xAARRGGBB).
pub const COLOR_BG: u32 = 0xFF14141E; // Fundo escuro azul-noite
pub const COLOR_WALKABLE: u32 = 0xFF2A2A3C; // Losango andável
pub const COLOR_WALL: u32 = 0xFF6E2828; // Bloco sólido de colisão
pub const COLOR_LOCAL_PLAYER: u32 = 0xFF50FA7B; // Verde vibrante
pub const COLOR_REMOTE_ENTITY: u32 = 0xFF8BE9FD; // Ciano vibrante
pub const COLOR_GRID_LINE: u32 = 0xFF3C3C50; // Linhas da grade

pub struct SoftwareFramebuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
    pub projection: IsometricProjection,
}

impl SoftwareFramebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![COLOR_BG; width * height],
            projection: IsometricProjection::default(),
        }
    }

    /// Limpa o framebuffer com a cor de fundo padrão.
    pub fn clear(&mut self, color: u32) {
        self.pixels.fill(color);
    }

    /// Redimensiona o framebuffer para acomodar janelas dinâmicas (ex: tiling window managers como Hyprland/Omarchy).
    pub fn resize(&mut self, new_width: usize, new_height: usize) {
        if self.width != new_width || self.height != new_height {
            self.width = new_width;
            self.height = new_height;
            self.pixels = vec![COLOR_BG; new_width * new_height];
        }
    }

    /// Define um pixel seguro com verificação de limites.
    #[inline]
    pub fn set_pixel(&mut self, x: i32, y: i32, color: u32) {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            self.pixels[y as usize * self.width + x as usize] = color;
        }
    }

    /// Desenha uma linha reta usando o algoritmo de Bresenham.
    pub fn draw_line(&mut self, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: u32) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            self.set_pixel(x0, y0, color);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    /// Desenha um retângulo preenchido com borda opcional.
    pub fn draw_rect(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        fill_color: u32,
        border_color: Option<u32>,
    ) {
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w).min(self.width as i32);
        let y1 = (y + h).min(self.height as i32);

        for py in y0..y1 {
            for px in x0..x1 {
                self.pixels[py as usize * self.width + px as usize] = fill_color;
            }
        }

        if let Some(border) = border_color {
            self.draw_line(x, y, x + w - 1, y, border);
            self.draw_line(x + w - 1, y, x + w - 1, y + h - 1, border);
            self.draw_line(x + w - 1, y + h - 1, x, y + h - 1, border);
            self.draw_line(x, y + h - 1, x, y, border);
        }
    }

    /// Desenha uma barra de vida (HP bar) estilizada de MMORPG sobre entidades.
    pub fn draw_hp_bar(&mut self, center_x: i32, top_y: i32, current_hp: i32, max_hp: i32) {
        if max_hp <= 0 {
            return;
        }
        let bar_w = 40;
        let bar_h = 5;
        let x = center_x - bar_w / 2;
        let y = top_y;

        // Fundo escuro com borda metálica
        self.draw_rect(x - 1, y - 1, bar_w + 2, bar_h + 2, 0xFF14141E, Some(0xFF44475A));

        // Preenchimento com transição de cor (Verde -> Laranja -> Vermelho)
        let pct = (current_hp as f32 / max_hp as f32).clamp(0.0, 1.0);
        let fill_w = (bar_w as f32 * pct).round() as i32;
        let fill_color = if pct > 0.5 {
            0xFF50FA7B // Verde vibrante
        } else if pct > 0.25 {
            0xFFFFB86C // Laranja alerta
        } else {
            0xFFFF5555 // Vermelho crítico
        };

        if fill_w > 0 {
            self.draw_rect(x, y, fill_w, bar_h, fill_color, None);
        }
    }

    /// SPEC-0023: Desenha o HUD do Jogador com barras de HP e SP proporcionais.
    pub fn draw_player_hud(&mut self, x: i32, y: i32, stats: &crate::state::PlayerStats) {
        let frame_w = 210;
        let frame_h = 58;
        // Painel de fundo translúcido estilizado
        self.draw_rect(x, y, frame_w, frame_h, 0xEE14141E, Some(0xFF6272A4));

        // Cabeçalho: Nome e Níveis
        let header = format!("{} | NVL {}/{}", stats.name, stats.base_level, stats.job_level);
        self.draw_text(x + 8, y + 6, &header, 0xFFF1FA8C, 1);

        // Barra de HP
        let bar_x = x + 28;
        let bar_w = 172;
        let bar_h = 10;
        let hp_y = y + 22;
        self.draw_text(x + 8, hp_y + 1, "HP", 0xFF50FA7B, 1);
        self.draw_rect(bar_x, hp_y, bar_w, bar_h, 0xFF282A36, Some(0xFF44475A));
        let hp_pct = (stats.current_hp as f32 / stats.derived.max_hp.max(1) as f32).clamp(0.0, 1.0);
        let hp_fill = (bar_w as f32 * hp_pct).round() as i32;
        let hp_color = if hp_pct > 0.5 {
            0xFF50FA7B
        } else if hp_pct > 0.25 {
            0xFFFFB86C
        } else {
            0xFFFF5555
        };
        if hp_fill > 0 {
            self.draw_rect(bar_x, hp_y, hp_fill, bar_h, hp_color, None);
        }
        let hp_txt = format!("{}/{}", stats.current_hp, stats.derived.max_hp);
        self.draw_text(bar_x + 36, hp_y + 1, &hp_txt, 0xFFFFFFFF, 1);

        // Barra de SP
        let sp_y = y + 38;
        self.draw_text(x + 8, sp_y + 1, "SP", 0xFF8BE9FD, 1);
        self.draw_rect(bar_x, sp_y, bar_w, bar_h, 0xFF282A36, Some(0xFF44475A));
        let sp_pct = (stats.current_sp as f32 / stats.derived.max_sp.max(1) as f32).clamp(0.0, 1.0);
        let sp_fill = (bar_w as f32 * sp_pct).round() as i32;
        if sp_fill > 0 {
            self.draw_rect(bar_x, sp_y, sp_fill, bar_h, 0xFF8BE9FD, None);
        }
        let sp_txt = format!("{}/{}", stats.current_sp, stats.derived.max_sp);
        self.draw_text(bar_x + 36, sp_y + 1, &sp_txt, 0xFFFFFFFF, 1);
    }

    /// SPEC-0023: Janela Flutuante de Atributos do Personagem (Status Window).
    pub fn draw_status_window(&mut self, x: i32, y: i32, stats: &crate::state::PlayerStats) {
        let win_w = 240;
        let win_h = 240;
        self.draw_rect(x, y, win_w, win_h, 0xF0181824, Some(0xFFBD93F9));
        // Barra de Título
        self.draw_rect(x, y, win_w, 20, 0xFF282A36, Some(0xFFBD93F9));
        self.draw_text(x + 8, y + 6, "STATUS / ATRIBUTOS [C]", 0xFFBD93F9, 1);

        // Informações Principais
        let row1 = format!(
            "CLASSE: {:?}  NVL: {}/{}",
            stats.job, stats.base_level, stats.job_level
        );
        self.draw_text(x + 10, y + 28, &row1, 0xFFF1FA8C, 1);

        self.draw_line(x + 8, y + 42, x + win_w - 8, y + 42, 0xFF44475A);

        // Coluna 1: Atributos Base
        let attrs = &stats.base_attributes;
        self.draw_text(x + 10, y + 50, &format!("STR : {:02}", attrs.str), 0xFFFF79C6, 1);
        self.draw_text(x + 10, y + 68, &format!("AGI : {:02}", attrs.agi), 0xFFFF79C6, 1);
        self.draw_text(x + 10, y + 86, &format!("VIT : {:02}", attrs.vit), 0xFFFF79C6, 1);
        self.draw_text(x + 10, y + 104, &format!("INT : {:02}", attrs.int), 0xFFFF79C6, 1);
        self.draw_text(x + 10, y + 122, &format!("DEX : {:02}", attrs.dex), 0xFFFF79C6, 1);
        self.draw_text(x + 10, y + 140, &format!("LUK : {:02}", attrs.luk), 0xFFFF79C6, 1);

        // Divisor vertical
        self.draw_line(x + 85, y + 46, x + 85, y + 160, 0xFF44475A);

        // Coluna 2: Atributos Derivados de Combate
        let d = &stats.derived;
        self.draw_text(x + 95, y + 50, &format!("ATK  : {}", d.status_atk), 0xFF50FA7B, 1);
        self.draw_text(x + 95, y + 68, &format!("DEF  : {}", d.soft_def), 0xFF50FA7B, 1);
        self.draw_text(
            x + 95,
            y + 86,
            &format!("MATK : {}~{}", d.matk_min, d.matk_max),
            0xFF8BE9FD,
            1,
        );
        self.draw_text(x + 95, y + 104, &format!("MDEF : {}", d.soft_mdef), 0xFF8BE9FD, 1);
        self.draw_text(x + 95, y + 122, &format!("HIT  : {}", d.hit), 0xFFFFB86C, 1);
        self.draw_text(x + 95, y + 140, &format!("FLEE : {}", d.flee), 0xFFFFB86C, 1);

        self.draw_line(x + 8, y + 162, x + win_w - 8, y + 162, 0xFF44475A);

        self.draw_text(x + 10, y + 172, &format!("CRITICO : {}%", d.crit), 0xFFFF5555, 1);
        self.draw_text(x + 120, y + 172, &format!("ASPD    : {}", d.aspd), 0xFFFFB86C, 1);
        let hp_sp_line = format!("HP MAX: {}  SP MAX: {}", d.max_hp, d.max_sp);
        self.draw_text(x + 10, y + 192, &hp_sp_line, 0xFFF8F8F2, 1);

        self.draw_text(x + 10, y + 218, "FECHAR: TECLA C OU SELECT", 0xFF6272A4, 1);
    }

    /// SPEC-0023: Janela Flutuante de Inventário e Carga de Peso (Inventory Window).
    pub fn draw_inventory_window(&mut self, x: i32, y: i32, stats: &crate::state::PlayerStats) {
        let win_w = 260;
        let win_h = 240;
        self.draw_rect(x, y, win_w, win_h, 0xF0181824, Some(0xFF50FA7B));
        // Barra de Título
        self.draw_rect(x, y, win_w, 20, 0xFF282A36, Some(0xFF50FA7B));
        self.draw_text(x + 8, y + 6, "INVENTARIO DE ITENS [I]", 0xFF50FA7B, 1);

        // Lista de itens
        let mut item_y = y + 28;
        for (i, item) in stats.inventory.iter().enumerate() {
            let num = i + 1;
            let line = format!("[{}] {:<16} x{:02}", num, item.name, item.amount);
            let color = if item.is_usable { 0xFFF1FA8C } else { 0xFF6272A4 };
            self.draw_text(x + 10, item_y, &line, color, 1);
            let wt = format!("{}wt", item.weight * item.amount);
            self.draw_text(x + win_w - 48, item_y, &wt, 0xFF8BE9FD, 1);
            item_y += 18;
        }

        // Rodapé de Peso e Ajuda
        let footer_y = y + win_h - 40;
        self.draw_line(x + 8, footer_y - 4, x + win_w - 8, footer_y - 4, 0xFF44475A);

        let cur_w = stats.current_weight();
        let max_w = stats.max_weight();
        let pct = (cur_w as f32 / max_w.max(1) as f32 * 100.0) as u32;
        let wt_text = format!("PESO TOTAL: {} / {} ({}%)", cur_w, max_w, pct);
        let wt_color = if pct > 70 { 0xFFFF5555 } else { 0xFF50FA7B };
        self.draw_text(x + 10, footer_y + 2, &wt_text, wt_color, 1);

        self.draw_text(
            x + 10,
            footer_y + 18,
            "USAR: TECLAS 1..4 | FECHAR: I / START",
            0xFF6272A4,
            1,
        );
    }

    /// Desenha texto na tela usando a fonte bitmap 5x7 integrada.
    pub fn draw_text(&mut self, mut x: i32, mut y: i32, text: &str, color: u32, scale: usize) {
        let start_x = x;
        let scale = scale.max(1);
        for c in text.chars() {
            if c == '\n' {
                x = start_x;
                y += (8 * scale) as i32;
                continue;
            }
            let glyph = super::font::get_glyph(c);
            for (col, &col_byte) in glyph.iter().enumerate() {
                for row in 0..7 {
                    if (col_byte & (1 << row)) != 0 {
                        let px = x + (col * scale) as i32;
                        let py = y + (row * scale) as i32;
                        if scale == 1 {
                            self.set_pixel(px, py, color);
                        } else {
                            for dy in 0..scale as i32 {
                                for dx in 0..scale as i32 {
                                    self.set_pixel(px + dx, py + dy, color);
                                }
                            }
                        }
                    }
                }
            }
            x += (6 * scale) as i32;
        }
    }

    /// Rasteriza um triângulo preenchido usando scanline determinística sem alocações.
    pub fn draw_triangle(
        &mut self,
        mut p0: (i32, i32),
        mut p1: (i32, i32),
        mut p2: (i32, i32),
        color: u32,
    ) {
        if p0.1 > p1.1 {
            std::mem::swap(&mut p0, &mut p1);
        }
        if p1.1 > p2.1 {
            std::mem::swap(&mut p1, &mut p2);
        }
        if p0.1 > p1.1 {
            std::mem::swap(&mut p0, &mut p1);
        }

        let (x0, y0) = p0;
        let (x1, y1) = p1;
        let (x2, y2) = p2;

        if y0 == y2 {
            let min_x = x0.min(x1).min(x2).max(0);
            let max_x = x0.max(x1).max(x2).min(self.width as i32 - 1);
            if y0 >= 0 && y0 < self.height as i32 {
                for px in min_x..=max_x {
                    self.pixels[y0 as usize * self.width + px as usize] = color;
                }
            }
            return;
        }

        let total_h = y2 - y0;
        for y in y0..=y2 {
            if y < 0 || y >= self.height as i32 {
                continue;
            }
            let second_half = y > y1 || y1 == y0;
            let segment_h = if second_half { y2 - y1 } else { y1 - y0 };
            if segment_h == 0 {
                continue;
            }

            let alpha = (y - y0) as f32 / total_h as f32;
            let beta = if second_half {
                (y - y1) as f32 / segment_h as f32
            } else {
                (y - y0) as f32 / segment_h as f32
            };

            let mut ax = x0 + ((x2 - x0) as f32 * alpha) as i32;
            let mut bx = if second_half {
                x1 + ((x2 - x1) as f32 * beta) as i32
            } else {
                x0 + ((x1 - x0) as f32 * beta) as i32
            };

            if ax > bx {
                std::mem::swap(&mut ax, &mut bx);
            }

            let start_x = ax.max(0);
            let end_x = bx.min(self.width as i32 - 1);
            for px in start_x..=end_x {
                self.pixels[y as usize * self.width + px as usize] = color;
            }
        }
    }

    /// Desenha um quadrilátero convexo preenchido com borda opcional.
    pub fn draw_convex_quad(
        &mut self,
        p0: (i32, i32),
        p1: (i32, i32),
        p2: (i32, i32),
        p3: (i32, i32),
        fill_color: u32,
        border_color: Option<u32>,
    ) {
        self.draw_triangle(p0, p1, p2, fill_color);
        self.draw_triangle(p0, p2, p3, fill_color);

        if let Some(border) = border_color {
            self.draw_line(p0.0, p0.1, p1.0, p1.1, border);
            self.draw_line(p1.0, p1.1, p2.0, p2.1, border);
            self.draw_line(p2.0, p2.1, p3.0, p3.1, border);
            self.draw_line(p3.0, p3.1, p0.0, p0.1, border);
        }
    }

    /// Rasteriza um triângulo com textura afim, interpolação UV suave e modulação de cor/tint opcional.
    pub fn draw_textured_triangle(
        &mut self,
        mut p0: (f32, f32, f32, f32),
        mut p1: (f32, f32, f32, f32),
        mut p2: (f32, f32, f32, f32),
        texture: &BmpImage,
        tint: Option<u32>,
    ) {
        if p0.1 > p1.1 { std::mem::swap(&mut p0, &mut p1); }
        if p1.1 > p2.1 { std::mem::swap(&mut p1, &mut p2); }
        if p0.1 > p1.1 { std::mem::swap(&mut p0, &mut p1); }

        let (x0, y0, u0, v0) = p0;
        let (x1, y1, u1, v1) = p1;
        let (x2, y2, u2, v2) = p2;

        let y_start = (y0.floor() as i32).max(0);
        let y_end = (y2.ceil() as i32).min(self.height as i32 - 1);
        if y_start > y_end {
            return;
        }

        let total_h = y2 - y0;
        if total_h <= 0.001 {
            return;
        }

        let tw = texture.width as i32;
        let th = texture.height as i32;
        if tw <= 0 || th <= 0 {
            return;
        }

        for y in y_start..=y_end {
            let y_f = y as f32 + 0.5;
            let second_half = y_f > y1 || y1 == y0;
            let segment_h = if second_half { y2 - y1 } else { y1 - y0 };
            if segment_h <= 0.001 {
                continue;
            }

            let alpha = ((y_f - y0) / total_h).clamp(0.0, 1.0);
            let beta = if second_half {
                ((y_f - y1) / segment_h).clamp(0.0, 1.0)
            } else {
                ((y_f - y0) / segment_h).clamp(0.0, 1.0)
            };

            let mut ax = x0 + (x2 - x0) * alpha;
            let mut au = u0 + (u2 - u0) * alpha;
            let mut av = v0 + (v2 - v0) * alpha;

            let mut bx = if second_half {
                x1 + (x2 - x1) * beta
            } else {
                x0 + (x1 - x0) * beta
            };
            let mut bu = if second_half {
                u1 + (u2 - u1) * beta
            } else {
                u0 + (u1 - u0) * beta
            };
            let mut bv = if second_half {
                v1 + (v2 - v1) * beta
            } else {
                v0 + (v1 - v0) * beta
            };

            if ax > bx {
                std::mem::swap(&mut ax, &mut bx);
                std::mem::swap(&mut au, &mut bu);
                std::mem::swap(&mut av, &mut bv);
            }

            let x_span = bx - ax;
            if x_span <= 0.001 {
                continue;
            }

            let x_start = (ax.floor() as i32).max(0);
            let x_end = (bx.ceil() as i32).min(self.width as i32 - 1);
            if x_start > x_end {
                continue;
            }

            let du = (bu - au) / x_span;
            let dv = (bv - av) / x_span;
            let row_offset = y as usize * self.width;

            for x in x_start..=x_end {
                let x_f = x as f32 + 0.5;
                let t = x_f - ax;
                let u = au + du * t;
                let v = av + dv * t;

                let u_wrap = u.rem_euclid(1.0);
                let v_wrap = v.rem_euclid(1.0);
                let tx = ((u_wrap * tw as f32) as i32).clamp(0, tw - 1) as usize;
                let ty = ((v_wrap * th as f32) as i32).clamp(0, th - 1) as usize;

                let tex_color = texture.pixels[ty * tw as usize + tx];
                let final_color = if let Some(t_color) = tint {
                    let tr = (t_color >> 16) & 0xFF;
                    let tg = (t_color >> 8) & 0xFF;
                    let tb = t_color & 0xFF;

                    let pr = (((tex_color >> 16) & 0xFF) * tr) / 255;
                    let pg = (((tex_color >> 8) & 0xFF) * tg) / 255;
                    let pb = ((tex_color & 0xFF) * tb) / 255;
                    0xFF00_0000 | (pr << 16) | (pg << 8) | pb
                } else {
                    tex_color
                };

                self.pixels[row_offset + x as usize] = final_color;
            }
        }
    }

    /// Desenha um quadrilátero texturizado com suporte a coordenadas UV por vértice.
    pub fn draw_textured_quad(
        &mut self,
        p0: (f32, f32, f32, f32),
        p1: (f32, f32, f32, f32),
        p2: (f32, f32, f32, f32),
        p3: (f32, f32, f32, f32),
        texture: &BmpImage,
        tint: Option<u32>,
    ) {
        self.draw_textured_triangle(p0, p1, p2, texture, tint);
        self.draw_textured_triangle(p0, p2, p3, texture, tint);
    }

    /// Desenha o contorno de uma célula da grade (wireframe).
    pub fn draw_isometric_tile(
        &mut self,
        grid_x: u16,
        grid_y: u16,
        is_walkable: bool,
        camera_x: f32,
        camera_y: f32,
    ) {
        let (cx, cy) =
            self.projection
                .world_to_screen(grid_x as f32, grid_y as f32, camera_x, camera_y);
        let tw = (self.projection.tile_width * self.projection.zoom) as i32;
        let th = (self.projection.tile_height * self.projection.zoom) as i32;
        let half_w = tw / 2;
        let half_h = th / 2;

        let center_x = cx as i32;
        let center_y = cy as i32;

        let color = if is_walkable {
            COLOR_GRID_LINE
        } else {
            COLOR_WALL
        };

        if self.projection.yaw.abs() < 0.0001 {
            let x = center_x - half_w;
            let y = center_y - half_h;
            self.draw_line(x, y, x + tw - 1, y, color);
            self.draw_line(x + tw - 1, y, x + tw - 1, y + th - 1, color);
            self.draw_line(x + tw - 1, y + th - 1, x, y + th - 1, color);
            self.draw_line(x, y + th - 1, x, y, color);
        } else {
            let gx = grid_x as f32;
            let gy = grid_y as f32;
            let p0 = self
                .projection
                .world_to_screen(gx - 0.5, gy - 0.5, camera_x, camera_y);
            let p1 = self
                .projection
                .world_to_screen(gx + 0.5, gy - 0.5, camera_x, camera_y);
            let p2 = self
                .projection
                .world_to_screen(gx + 0.5, gy + 0.5, camera_x, camera_y);
            let p3 = self
                .projection
                .world_to_screen(gx - 0.5, gy + 0.5, camera_x, camera_y);

            self.draw_line(p0.0 as i32, p0.1 as i32, p1.0 as i32, p1.1 as i32, color);
            self.draw_line(p1.0 as i32, p1.1 as i32, p2.0 as i32, p2.1 as i32, color);
            self.draw_line(p2.0 as i32, p2.1 as i32, p3.0 as i32, p3.1 as i32, color);
            self.draw_line(p3.0 as i32, p3.1 as i32, p0.0 as i32, p0.1 as i32, color);
        }
    }

    /// Preenche a célula 2.5D com suporte total a rotação (yaw) e zoom regulável.
    pub fn fill_isometric_tile(
        &mut self,
        grid_x: u16,
        grid_y: u16,
        is_walkable: bool,
        camera_x: f32,
        camera_y: f32,
    ) {
        let fill_color = if is_walkable {
            0xFF232536 // Piso andável
        } else {
            0xFF4A1E28 // Parede/obstáculo não andável
        };

        let border_color = if is_walkable {
            0xFF383A4C // Linha de grade sutil
        } else {
            0xFF7E2A3A // Borda de colisão sólida
        };

        if self.projection.yaw.abs() < 0.0001 {
            let (cx, cy) =
                self.projection
                    .world_to_screen(grid_x as f32, grid_y as f32, camera_x, camera_y);
            let tw = (self.projection.tile_width * self.projection.zoom) as i32;
            let th = (self.projection.tile_height * self.projection.zoom) as i32;
            let half_w = tw / 2;
            let half_h = th / 2;

            let center_x = cx as i32;
            let center_y = cy as i32;

            // Culling fora da tela
            if center_x + half_w < 0
                || center_x - half_w >= self.width as i32
                || center_y + half_h < 0
                || center_y - half_h >= self.height as i32
            {
                return;
            }

            self.draw_rect(
                center_x - half_w,
                center_y - half_h,
                tw,
                th,
                fill_color,
                Some(border_color),
            );
        } else {
            let gx = grid_x as f32;
            let gy = grid_y as f32;
            let p0 = self
                .projection
                .world_to_screen(gx - 0.5, gy - 0.5, camera_x, camera_y);
            let p1 = self
                .projection
                .world_to_screen(gx + 0.5, gy - 0.5, camera_x, camera_y);
            let p2 = self
                .projection
                .world_to_screen(gx + 0.5, gy + 0.5, camera_x, camera_y);
            let p3 = self
                .projection
                .world_to_screen(gx - 0.5, gy + 0.5, camera_x, camera_y);

            let pt0 = (p0.0 as i32, p0.1 as i32);
            let pt1 = (p1.0 as i32, p1.1 as i32);
            let pt2 = (p2.0 as i32, p2.1 as i32);
            let pt3 = (p3.0 as i32, p3.1 as i32);

            let min_x = pt0.0.min(pt1.0).min(pt2.0).min(pt3.0);
            let max_x = pt0.0.max(pt1.0).max(pt2.0).max(pt3.0);
            let min_y = pt0.1.min(pt1.1).min(pt2.1).min(pt3.1);
            let max_y = pt0.1.max(pt1.1).max(pt2.1).max(pt3.1);

            if max_x < 0 || min_x >= self.width as i32 || max_y < 0 || min_y >= self.height as i32 {
                return;
            }

            self.draw_convex_quad(pt0, pt1, pt2, pt3, fill_color, Some(border_color));
        }
    }

    /// Desenha o avatar de um personagem (círculo com indicador de direção rotacionado).
    pub fn draw_entity_token(
        &mut self,
        pos: Position,
        facing: Direction,
        is_local_player: bool,
        camera_x: f32,
        camera_y: f32,
    ) {
        let (cx, cy) =
            self.projection
                .world_to_screen(pos.x as f32, pos.y as f32, camera_x, camera_y);
        let cx = cx as i32;
        let cy = cy as i32;

        let color = if is_local_player {
            COLOR_LOCAL_PLAYER
        } else {
            COLOR_REMOTE_ENTITY
        };

        // Desenha corpo (quadrado 7x7 em volta do centro)
        for dy in -3..=3 {
            for dx in -3..=3 {
                self.set_pixel(cx + dx, cy + dy, color);
            }
        }

        // Vetor de direção da face rotacionado conforme a câmera
        let base_angle = match facing {
            Direction::East => 0.0,
            Direction::NorthEast => std::f32::consts::PI * 0.25,
            Direction::North => std::f32::consts::PI * 0.5,
            Direction::NorthWest => std::f32::consts::PI * 0.75,
            Direction::West => std::f32::consts::PI,
            Direction::SouthWest => -std::f32::consts::PI * 0.75,
            Direction::South => -std::f32::consts::PI * 0.5,
            Direction::SouthEast => -std::f32::consts::PI * 0.25,
        };

        let screen_angle = base_angle - self.projection.yaw;
        let dx = (screen_angle.cos() * 8.0) as i32;
        let dy = (-screen_angle.sin() * 8.0) as i32;

        self.draw_line(cx, cy, cx + dx, cy + dy, 0xFFFFFFFF);
    }

    /// Desenha o marcador visual de clique de destino no chão isométrico (círculo / elipse verde pulsante).
    pub fn draw_ground_target_marker(
        &mut self,
        tile: Position,
        timer_norm: f32,
        cam_offset_x: f32,
        cam_offset_y: f32,
    ) {
        let (cx, cy) = self.projection.world_to_screen(
            tile.x as f32 + 0.5,
            tile.y as f32 + 0.5,
            cam_offset_x,
            cam_offset_y,
        );
        let cx = cx as i32;
        let cy = cy as i32;

        let radius = ((1.0 - timer_norm) * 14.0 * self.projection.zoom).max(4.0) as i32;
        let color = 0xFF50FA7B; // Verde neon / target marker vibrante

        let rx = radius;
        let ry = (radius as f32 * self.projection.pitch_deg().to_radians().sin() * 0.5).max(2.0) as i32;

        for a in 0..16 {
            let angle1 = (a as f32) * std::f32::consts::TAU / 16.0;
            let angle2 = ((a + 1) as f32) * std::f32::consts::TAU / 16.0;
            let x1 = cx + (angle1.cos() * rx as f32) as i32;
            let y1 = cy + (angle1.sin() * ry as f32) as i32;
            let x2 = cx + (angle2.cos() * rx as f32) as i32;
            let y2 = cy + (angle2.sin() * ry as f32) as i32;
            self.draw_line(x1, y1, x2, y2, color);
        }

        // Ponto central de fixação
        self.set_pixel(cx, cy, 0xFFFFFFFF);
        self.set_pixel(cx + 1, cy, 0xFFFFFFFF);
        self.set_pixel(cx - 1, cy, 0xFFFFFFFF);
        self.set_pixel(cx, cy + 1, 0xFFFFFFFF);
        self.set_pixel(cx, cy - 1, 0xFFFFFFFF);
    }


    /// Desenha um quadro de sprite 2D com transparência (color key / alpha > 0).
    /// `center_x` e `bottom_y` definem o ponto de ancoragem no chão (parte inferior central).
    pub fn draw_sprite_frame(
        &mut self,
        center_x: i32,
        bottom_y: i32,
        frame: &SpriteFrame,
        scale: f32,
    ) {
        let sw = frame.width as i32;
        let sh = frame.height as i32;
        let draw_w = (sw as f32 * scale).round() as i32;
        let draw_h = (sh as f32 * scale).round() as i32;

        if draw_w <= 0 || draw_h <= 0 {
            return;
        }

        let start_x = center_x - draw_w / 2;
        let start_y = bottom_y - draw_h;

        // Culling fora da tela
        if start_x >= self.width as i32
            || start_x + draw_w <= 0
            || start_y >= self.height as i32
            || start_y + draw_h <= 0
        {
            return;
        }

        if (scale - 1.0).abs() < 0.01 {
            for sy in 0..sh {
                let py = start_y + sy;
                if py < 0 || py >= self.height as i32 {
                    continue;
                }
                let row_offset = (py as usize) * self.width;
                let frame_row = (sy as usize) * (sw as usize);

                for sx in 0..sw {
                    let px = start_x + sx;
                    if px < 0 || px >= self.width as i32 {
                        continue;
                    }
                    let pixel = frame.pixels[frame_row + (sx as usize)];
                    let alpha = (pixel >> 24) & 0xFF;
                    if alpha > 0 {
                        if alpha == 255 {
                            self.pixels[row_offset + (px as usize)] = pixel;
                        } else {
                            let bg = self.pixels[row_offset + (px as usize)];
                            let inv_a = 255 - alpha;
                            let r = (((pixel >> 16) & 0xFF) * alpha + ((bg >> 16) & 0xFF) * inv_a)
                                / 255;
                            let g =
                                (((pixel >> 8) & 0xFF) * alpha + ((bg >> 8) & 0xFF) * inv_a) / 255;
                            let b = ((pixel & 0xFF) * alpha + (bg & 0xFF) * inv_a) / 255;
                            self.pixels[row_offset + (px as usize)] =
                                0xFF000000 | (r << 16) | (g << 8) | b;
                        }
                    }
                }
            }
        } else {
            for dy in 0..draw_h {
                let py = start_y + dy;
                if py < 0 || py >= self.height as i32 {
                    continue;
                }
                let sy = ((dy as f32 / scale) as usize).min(frame.height as usize - 1);
                let row_offset = (py as usize) * self.width;
                let frame_row = sy * (sw as usize);

                for dx in 0..draw_w {
                    let px = start_x + dx;
                    if px < 0 || px >= self.width as i32 {
                        continue;
                    }
                    let sx = ((dx as f32 / scale) as usize).min(frame.width as usize - 1);
                    let pixel = frame.pixels[frame_row + sx];
                    let alpha = (pixel >> 24) & 0xFF;
                    if alpha > 0 {
                        if alpha == 255 {
                            self.pixels[row_offset + (px as usize)] = pixel;
                        } else {
                            let bg = self.pixels[row_offset + (px as usize)];
                            let inv_a = 255 - alpha;
                            let r = (((pixel >> 16) & 0xFF) * alpha + ((bg >> 16) & 0xFF) * inv_a)
                                / 255;
                            let g =
                                (((pixel >> 8) & 0xFF) * alpha + ((bg >> 8) & 0xFF) * inv_a) / 255;
                            let b = ((pixel & 0xFF) * alpha + (bg & 0xFF) * inv_a) / 255;
                            self.pixels[row_offset + (px as usize)] =
                                0xFF000000 | (r << 16) | (g << 8) | b;
                        }
                    }
                }
            }
        }
    }

    /// SPEC-0019: Desenha um quadro composto de animação .act sobre o framebuffer.
    pub fn draw_act_frame(
        &mut self,
        center_x: i32,
        center_y: i32,
        act_frame: &ActFrame,
        sprite: &Sprite,
        global_scale: f32,
    ) {
        self.draw_act_frame_flipped(center_x, center_y, act_frame, sprite, global_scale, false);
    }

    /// SPEC-0021: Desenha um quadro composto com suporte a inversão horizontal (golpe de retorno de combo).
    pub fn draw_act_frame_flipped(
        &mut self,
        center_x: i32,
        center_y: i32,
        act_frame: &ActFrame,
        sprite: &Sprite,
        global_scale: f32,
        flip_h: bool,
    ) {
        for clip in &act_frame.clips {
            if clip.spr_index < 0 {
                continue;
            }
            let spr_idx = clip.spr_index as usize;
            if let Some(frame) = sprite.frames.get(spr_idx) {
                if flip_h {
                    let mut flipped = clip.clone();
                    flipped.offset_x = -flipped.offset_x;
                    flipped.mirror = !flipped.mirror;
                    self.draw_act_clip(center_x, center_y, &flipped, frame, global_scale);
                } else {
                    self.draw_act_clip(center_x, center_y, clip, frame, global_scale);
                }
            }
        }
    }

    /// SPEC-0019: Desenha um recorte individual (layer) com suporte a espelhamento horizontal (mirror) e escala.
    pub fn draw_act_clip(
        &mut self,
        center_x: i32,
        center_y: i32,
        clip: &ActClip,
        frame: &SpriteFrame,
        global_scale: f32,
    ) {
        let sw = frame.width as i32;
        let sh = frame.height as i32;
        let total_scale_x = global_scale * clip.scale_x;
        let total_scale_y = global_scale * clip.scale_y;
        let draw_w = (sw as f32 * total_scale_x).round() as i32;
        let draw_h = (sh as f32 * total_scale_y).round() as i32;

        if draw_w <= 0 || draw_h <= 0 {
            return;
        }

        let clip_cx = center_x + (clip.offset_x as f32 * global_scale).round() as i32;
        let clip_cy = center_y + (clip.offset_y as f32 * global_scale).round() as i32;

        let start_x = clip_cx - draw_w / 2;
        let start_y = clip_cy - draw_h / 2;

        // Culling fora da tela
        if start_x >= self.width as i32
            || start_x + draw_w <= 0
            || start_y >= self.height as i32
            || start_y + draw_h <= 0
        {
            return;
        }

        let mirror = clip.mirror;

        for dy in 0..draw_h {
            let py = start_y + dy;
            if py < 0 || py >= self.height as i32 {
                continue;
            }
            let sy = (dy * sh) / draw_h;
            let frame_row = (sy as usize) * (sw as usize);
            let row_offset = (py as usize) * self.width;

            for dx in 0..draw_w {
                let px = start_x + dx;
                if px < 0 || px >= self.width as i32 {
                    continue;
                }
                let sx = if mirror {
                    (sw - 1) - (dx * sw) / draw_w
                } else {
                    (dx * sw) / draw_w
                };

                let pixel = frame.pixels[frame_row + (sx as usize)];
                let alpha = (pixel >> 24) & 0xFF;
                if alpha == 0 {
                    continue;
                }
                if alpha == 255 {
                    self.pixels[row_offset + (px as usize)] = pixel;
                } else {
                    let bg = self.pixels[row_offset + (px as usize)];
                    let inv_a = 255 - alpha;
                    let r = (((pixel >> 16) & 0xFF) * alpha + ((bg >> 16) & 0xFF) * inv_a) / 255;
                    let g = (((pixel >> 8) & 0xFF) * alpha + ((bg >> 8) & 0xFF) * inv_a) / 255;
                    let b = ((pixel & 0xFF) * alpha + (bg & 0xFF) * inv_a) / 255;
                    self.pixels[row_offset + (px as usize)] = 0xFF000000 | (r << 16) | (g << 8) | b;
                }
            }
        }
    }

    /// Renderiza um trecho visível do CollisionGrid centrado na câmera.
    pub fn render_grid_view(
        &mut self,
        grid: &CollisionGrid,
        center_pos: Position,
        view_radius: u16,
        camera_x: f32,
        camera_y: f32,
    ) {
        let min_x = center_pos.x.saturating_sub(view_radius);
        let max_x = (center_pos.x + view_radius).min(grid.width);
        let min_y = center_pos.y.saturating_sub(view_radius);
        let max_y = (center_pos.y + view_radius).min(grid.height);

        for y in min_y..max_y {
            for x in min_x..max_x {
                let pos = Position::new_unchecked(x, y);
                let walkable = grid.is_walkable(pos);
                self.fill_isometric_tile(x, y, walkable, camera_x, camera_y);
            }
        }
    }

    /// Desenha uma célula isométrica com textura afim, UVs mapeadas e iluminação/tint.
    pub fn draw_textured_isometric_tile(
        &mut self,
        grid_x: u16,
        grid_y: u16,
        is_walkable: bool,
        camera_x: f32,
        camera_y: f32,
        texture: &BmpImage,
        uv_coords: Option<([f32; 4], [f32; 4])>,
    ) {
        let gx = grid_x as f32;
        let gy = grid_y as f32;

        let p0 = self.projection.world_to_screen(gx - 0.5, gy - 0.5, camera_x, camera_y);
        let p1 = self.projection.world_to_screen(gx + 0.5, gy - 0.5, camera_x, camera_y);
        let p2 = self.projection.world_to_screen(gx + 0.5, gy + 0.5, camera_x, camera_y);
        let p3 = self.projection.world_to_screen(gx - 0.5, gy + 0.5, camera_x, camera_y);

        let min_sx = p0.0.min(p1.0).min(p2.0).min(p3.0);
        let max_sx = p0.0.max(p1.0).max(p2.0).max(p3.0);
        let min_sy = p0.1.min(p1.1).min(p2.1).min(p3.1);
        let max_sy = p0.1.max(p1.1).max(p2.1).max(p3.1);

        if max_sx < 0.0 || min_sx >= self.width as f32 || max_sy < 0.0 || min_sy >= self.height as f32 {
            return;
        }

        let tint = if is_walkable {
            None
        } else {
            Some(0xFF7A4545) // Tom avermelhado sutil para indicar parede/obstáculo de colisão
        };

        let (u, v) = uv_coords.unwrap_or(([0.0, 1.0, 1.0, 0.0], [0.0, 0.0, 1.0, 1.0]));

        self.draw_textured_quad(
            (p0.0, p0.1, u[0], v[0]),
            (p1.0, p1.1, u[1], v[1]),
            (p2.0, p2.1, u[2], v[2]),
            (p3.0, p3.1, u[3], v[3]),
            texture,
            tint,
        );


    }

    /// Renderiza a grade de terreno texturizada com suporte a GND e cache de texturas.
    pub fn render_textured_grid_view(
        &mut self,
        grid: &CollisionGrid,
        center_pos: Position,
        view_radius: u16,
        camera_x: f32,
        camera_y: f32,
        gnd: Option<&GndMesh>,
        textures: &std::collections::HashMap<String, BmpImage>,
        fallback_tex: &BmpImage,
    ) {
        let min_x = center_pos.x.saturating_sub(view_radius);
        let max_x = (center_pos.x + view_radius).min(grid.width);
        let min_y = center_pos.y.saturating_sub(view_radius);
        let max_y = (center_pos.y + view_radius).min(grid.height);

        for y in min_y..max_y {
            for x in min_x..max_x {
                let pos = Position::new_unchecked(x, y);
                let walkable = grid.is_walkable(pos);

                let gnd_x = (x / 2) as u32;
                let gnd_y = (y / 2) as u32;
                let sub_x = (x % 2) as f32;
                let sub_y = (y % 2) as f32;

                let (tex, uvs) = if let Some(gnd_mesh) = gnd {
                    if let Some(tile) = gnd_mesh.tile_for_cell(gnd_x, gnd_y) {
                        let tex_path = gnd_mesh.textures.get(tile.texture_index as usize);
                        let texture = tex_path.and_then(|p| textures.get(p)).unwrap_or(fallback_tex);

                        // GND tile corners:
                        // 0: TL, 1: TR, 2: BL, 3: BR
                        let u_min = tile.u[0];
                        let u_max = tile.u[1];
                        let v_min = tile.v[0];
                        let v_max = tile.v[2];

                        let u_start = u_min + (u_max - u_min) * (sub_x * 0.5);
                        let u_end = u_start + (u_max - u_min) * 0.5;
                        let v_start = v_min + (v_max - v_min) * (sub_y * 0.5);
                        let v_end = v_start + (v_max - v_min) * 0.5;

                        // p0=TL, p1=TR, p2=BR, p3=BL
                        let uvs = ([u_start, u_end, u_end, u_start], [v_start, v_start, v_end, v_end]);
                        (texture, Some(uvs))
                    } else {
                        (fallback_tex, None)
                    }
                } else {
                    (fallback_tex, None)
                };

                self.draw_textured_isometric_tile(x, y, walkable, camera_x, camera_y, tex, uvs);
            }
        }
    }

    /// SPEC-0028: Minimapa Dinâmico com Bússola e Radar de Entidades no Canto Superior Direito.
    pub fn draw_minimap(
        &mut self,
        x: i32,
        y: i32,
        size: i32,
        grid: &CollisionGrid,
        player_pos: Position,
        entities: &[(Position, u32)],
    ) {
        self.draw_rect(x - 2, y - 2, size + 4, size + 4, 0xEE14141E, Some(0xFF6272A4));
        self.draw_rect(x, y, size, 16, 0xFF282A36, Some(0xFF44475A));
        self.draw_text(x + 6, y + 4, "MINIMAP [M]", 0xFFF1FA8C, 1);

        let map_w = size;
        let map_h = size - 16;
        let map_y = y + 16;

        let radius = 24i32;
        let step_x = map_w as f32 / (radius * 2) as f32;
        let step_y = map_h as f32 / (radius * 2) as f32;

        let px = player_pos.x as i32;
        let py = player_pos.y as i32;

        for dy in -radius..=radius {
            let gy = py + dy;
            if gy < 0 || gy >= grid.height as i32 {
                continue;
            }
            let sy = map_y + ((dy + radius) as f32 * step_y) as i32;

            for dx in -radius..=radius {
                let gx = px + dx;
                if gx < 0 || gx >= grid.width as i32 {
                    continue;
                }
                let sx = x + ((dx + radius) as f32 * step_x) as i32;

                let is_walk = grid.is_walkable(Position::new_unchecked(gx as u16, gy as u16));
                let color = if is_walk { 0xFF2E3440 } else { 0xFF5E2B38 };

                let w_pix = step_x.ceil() as i32;
                let h_pix = step_y.ceil() as i32;
                self.draw_rect(sx, sy, w_pix, h_pix, color, None);
            }
        }

        for &(e_pos, e_color) in entities {
            let edx = e_pos.x as i32 - px;
            let edy = e_pos.y as i32 - py;
            if edx.abs() <= radius && edy.abs() <= radius {
                let ex = x + ((edx + radius) as f32 * step_x) as i32;
                let ey = map_y + ((edy + radius) as f32 * step_y) as i32;
                self.draw_rect(ex - 1, ey - 1, 3, 3, e_color, Some(0xFFFFFFFF));
            }
        }

        let cx = x + (map_w / 2);
        let cy = map_y + (map_h / 2);
        self.draw_rect(cx - 2, cy - 2, 5, 5, 0xFF50FA7B, Some(0xFFF8F8F2));

        let coord_text = format!("X:{} Y:{}", px, py);
        self.draw_rect(x + 4, y + size - 14, 80, 12, 0xCC14141E, None);
        self.draw_text(x + 6, y + size - 12, &coord_text, 0xFF8BE9FD, 1);
    }

    /// SPEC-0028: Barra de Ações Rápida (Hotbar / Action Bar) no Rodapé da Tela.
    pub fn draw_action_bar(
        &mut self,
        screen_w: i32,
        screen_h: i32,
        red_pots: u32,
        blue_pots: u32,
        combo_label: &str,
    ) {
        let bar_w = 460;
        let bar_h = 44;
        let x = (screen_w - bar_w) / 2;
        let y = screen_h - bar_h - 8;

        self.draw_rect(x, y, bar_w, bar_h, 0xEE14141E, Some(0xFF44475A));

        let slot_w = 48;
        let slot_h = 36;
        let slot_y = y + 4;
        let slots = [
            ("1", "HP POT", format!("x{:02}", red_pots)),
            ("2", "SP POT", format!("x{:02}", blue_pots)),
            ("L-CLK", "JAB", "1.0x".to_string()),
            ("R-CLK", "COMBO", "3-HIT".to_string()),
            ("L2/SPC", "ROLL", "DODGE".to_string()),
            ("C/I", "MENU", "WIN".to_string()),
        ];
        let colors = [0xFFFF5555, 0xFF8BE9FD, 0xFF50FA7B, 0xFFFFB86C, 0xFFBD93F9, 0xFFF1FA8C];

        for (i, &(key, title, ref sub)) in slots.iter().enumerate() {
            let color = colors[i];
            let sx = x + 8 + (i as i32 * 54);
            self.draw_rect(sx, slot_y, slot_w, slot_h, 0xFF282A36, Some(color));

            self.draw_text(sx + 3, slot_y + 3, key, 0xFFF1FA8C, 1);
            self.draw_text(sx + 3, slot_y + 14, title, color, 1);
            self.draw_text(sx + 3, slot_y + 25, sub, 0xFFF8F8F2, 1);
        }

        let label_x = x + 8 + (6 * 54) + 6;
        self.draw_rect(label_x, slot_y, 116, slot_h, 0xFF1E1E2E, Some(0xFF6272A4));
        self.draw_text(label_x + 4, slot_y + 6, "STANCE / RITMO:", 0xFF6272A4, 1);
        let short_label = if combo_label.len() > 18 { &combo_label[..18] } else { combo_label };
        self.draw_text(label_x + 4, slot_y + 18, short_label, 0xFF50FA7B, 1);
    }

    /// SPEC-0028: Barra de Alvo Selecionado / Hover no Topo Central da Tela.
    pub fn draw_target_info_bar(
        &mut self,
        screen_w: i32,
        name: &str,
        cur_hp: i32,
        max_hp: i32,
        dist: f32,
    ) {
        let bar_w = 260;
        let bar_h = 32;
        let x = (screen_w - bar_w) / 2;
        let y = 10;

        self.draw_rect(x, y, bar_w, bar_h, 0xEE14141E, Some(0xFFFF5555));
        self.draw_text(x + 10, y + 4, name, 0xFFF8F8F2, 1);

        let dist_text = format!("{:.1}m", dist);
        self.draw_text(x + bar_w - 40, y + 4, &dist_text, 0xFF8BE9FD, 1);

        let hp_pct = (cur_hp as f32 / max_hp.max(1) as f32).clamp(0.0, 1.0);
        let inner_w = (bar_w - 20) as f32;
        let fill_w = (inner_w * hp_pct).round() as i32;

        self.draw_rect(x + 10, y + 16, bar_w - 20, 10, 0xFF282A36, None);
        self.draw_rect(x + 10, y + 16, fill_w, 10, 0xFFFF5555, None);

        let hp_text = format!("{}/{}", cur_hp, max_hp);
        self.draw_text(x + (bar_w / 2) - 20, y + 17, &hp_text, 0xFFFFFFFF, 1);
    }

    /// Renderiza um modelo 3D (.rsm) posicionado no mundo em coordenadas de célula com iluminação e rotação.
    pub fn draw_rsm_model(
        &mut self,
        model: &RsmModel,
        world_cell_x: f32,
        world_cell_y: f32,
        cam_offset_x: f32,
        cam_offset_y: f32,
        unit_scale: f32,
    ) {
        // Direção de iluminação solar clássica normalizada: sol vindo de cima/oeste
        let light_dir = (0.577f32, 0.577f32, 0.577f32);
        let cos_yaw = self.projection.yaw.cos();
        let sin_yaw = self.projection.yaw.sin();
        let zoom = self.projection.zoom;
        let tw = self.projection.tile_width * zoom;
        let th = self.projection.tile_height * zoom;
        let vert_scale = self.projection.tile_width * zoom * self.projection.pitch.cos();

        for node in &model.nodes {
            // ── Mapeamento de eixos RSM → espaço 2.5D (equivalente ao flip=[1,-1,1] do RoBrowser) ──
            // No RSM: X=lateral, Y cresce para BAIXO (invertido), Z=profundidade.
            // Negamos Y e ancoramos pela base (y_max = ponto mais "para baixo" no RSM = base real do modelo).
            let y_max = node.vertices.iter().map(|v| v.1).fold(f32::NEG_INFINITY, f32::max);

            for face in &node.faces {
                let idx0 = face.vertices[0] as usize;
                let idx1 = face.vertices[1] as usize;
                let idx2 = face.vertices[2] as usize;

                if idx0 >= node.vertices.len() || idx1 >= node.vertices.len() || idx2 >= node.vertices.len() {
                    continue;
                }

                let v0 = node.vertices[idx0];
                let v1 = node.vertices[idx1];
                let v2 = node.vertices[idx2];

                // v.0 = X lateral, -v.1 + y_max = altura a partir do chão, v.2 = profundidade do mapa
                let px0 = world_cell_x + v0.0 * unit_scale;
                let py0 = world_cell_y + v0.2 * unit_scale;
                let pz0 = (-v0.1 + y_max) * unit_scale;

                let px1 = world_cell_x + v1.0 * unit_scale;
                let py1 = world_cell_y + v1.2 * unit_scale;
                let pz1 = (-v1.1 + y_max) * unit_scale;

                let px2 = world_cell_x + v2.0 * unit_scale;
                let py2 = world_cell_y + v2.2 * unit_scale;
                let pz2 = (-v2.1 + y_max) * unit_scale;


                // Projeção isométrica com rotação da câmera (yaw) e inclinação (pitch)
                let rx0 = px0 * cos_yaw - py0 * sin_yaw;
                let ry0 = px0 * sin_yaw + py0 * cos_yaw;
                let sx0 = (rx0 * tw + cam_offset_x) as i32;
                let sy0 = (-ry0 * th - pz0 * vert_scale + cam_offset_y) as i32;

                let rx1 = px1 * cos_yaw - py1 * sin_yaw;
                let ry1 = px1 * sin_yaw + py1 * cos_yaw;
                let sx1 = (rx1 * tw + cam_offset_x) as i32;
                let sy1 = (-ry1 * th - pz1 * vert_scale + cam_offset_y) as i32;

                let rx2 = px2 * cos_yaw - py2 * sin_yaw;
                let ry2 = px2 * sin_yaw + py2 * cos_yaw;
                let sx2 = (rx2 * tw + cam_offset_x) as i32;
                let sy2 = (-ry2 * th - pz2 * vert_scale + cam_offset_y) as i32;

                // Backface culling em 2D na tela
                let cross = (sx1 - sx0) * (sy2 - sy0) - (sy1 - sy0) * (sx2 - sx0);
                if cross <= 0 {
                    continue;
                }

                // Cálculo do vetor normal 3D para sombreamento (flat shading)
                let ax = v1.0 - v0.0;
                let ay = v1.1 - v0.1;
                let az = v1.2 - v0.2;
                let bx = v2.0 - v0.0;
                let by = v2.1 - v0.1;
                let bz = v2.2 - v0.2;

                let nx = ay * bz - az * by;
                let ny = az * bx - ax * bz;
                let nz = ax * by - ay * bx;
                let len = (nx * nx + ny * ny + nz * nz).sqrt();

                let intensity = if len > 0.001 {
                    let dot = (nx * light_dir.0 + ny * light_dir.1 + nz * light_dir.2) / len;
                    (0.40 + 0.60 * dot.abs()).clamp(0.25, 1.0)
                } else {
                    0.65
                };

                // Cor base de pedra/alvenaria de Prontera (R: 210, G: 195, B: 180)
                let r = ((210.0 * intensity) as u32).min(255);
                let g = ((195.0 * intensity) as u32).min(255);
                let b = ((180.0 * intensity) as u32).min(255);
                let color = 0xFF000000 | (r << 16) | (g << 8) | b;

                // Rasterização do triângulo preenchido
                self.draw_triangle((sx0, sy0), (sx1, sy1), (sx2, sy2), color);
            }
        }
    }

    /// Desenha um retículo isométrico projetado no plano do solo (Z = 0) ao redor de uma entidade.
    pub fn draw_ground_reticle(
        &mut self,
        world_x: f32,
        world_y: f32,
        cam_offset_x: f32,
        cam_offset_y: f32,
        color: u32,
        radius_cells: f32,
    ) {
        let (center_x, center_y) = self.projection.world_to_screen(world_x, world_y, cam_offset_x, cam_offset_y);
        let rx = radius_cells * (self.projection.tile_width as f32 * 0.5) * self.projection.zoom;
        let ry = radius_cells * (self.projection.tile_height as f32 * 0.5) * self.projection.zoom;

        const STEPS: usize = 20;
        let mut prev_x = center_x + rx;
        let mut prev_y = center_y;

        for i in 1..=STEPS {
            let angle = (i as f32 / STEPS as f32) * std::f32::consts::TAU;
            let px = center_x + angle.cos() * rx;
            let py = center_y + angle.sin() * ry;
            self.draw_line(
                prev_x.round() as i32,
                prev_y.round() as i32,
                px.round() as i32,
                py.round() as i32,
                color,
            );
            prev_x = px;
            prev_y = py;
        }

        // Quatro marcas nos eixos ortogonais para aspecto de mira limpa
        let tick = 4.0;
        self.draw_line((center_x - rx - tick) as i32, center_y as i32, (center_x - rx) as i32, center_y as i32, color);
        self.draw_line((center_x + rx) as i32, center_y as i32, (center_x + rx + tick) as i32, center_y as i32, color);
        self.draw_line(center_x as i32, (center_y - ry - tick) as i32, center_x as i32, (center_y - ry) as i32, color);
        self.draw_line(center_x as i32, (center_y + ry) as i32, center_x as i32, (center_y + ry + tick) as i32, color);
    }

    /// SPEC-0028 / VALVE.md: Desenha um balão de fala orgânico não-modal sobre uma entidade/NPC.
    pub fn draw_speech_bubble(
        &mut self,
        anchor_x: i32,
        anchor_y: i32,
        speaker_name: &str,
        lines: &[&str],
        hint: Option<&str>,
    ) {
        if lines.is_empty() {
            return;
        }

        let max_text_len = lines.iter().map(|l| l.len()).max().unwrap_or(0).max(speaker_name.len());
        let bubble_w = (max_text_len as i32 * 6 + 28).clamp(160, (self.width as i32 - 20).max(160));
        let content_h = (lines.len() as i32) * 12 + 18 + if hint.is_some() { 14 } else { 0 };
        let bubble_h = content_h + 10;

        let bx = (anchor_x - bubble_w / 2).clamp(10, self.width as i32 - bubble_w - 10);
        let by = (anchor_y - bubble_h - 14).max(10);

        // Sombra suave do balão
        self.draw_rect(bx + 2, by + 2, bubble_w, bubble_h, 0xAA0A0A10, None);

        // Fundo do balão com borda ciano suave
        self.draw_rect(bx, by, bubble_w, bubble_h, 0xF2181824, Some(0xFF8BE9FD));

        // Rabicho indicativo para o NPC/Entidade
        let tail_x = anchor_x.clamp(bx + 12, bx + bubble_w - 12);
        let tail_top_y = by + bubble_h;
        let tail_tip_y = (anchor_y - 2).min(tail_top_y + 8);
        self.draw_line(tail_x - 4, tail_top_y, tail_x, tail_tip_y, 0xFF8BE9FD);
        self.draw_line(tail_x + 4, tail_top_y, tail_x, tail_tip_y, 0xFF8BE9FD);
        self.draw_line(tail_x - 3, tail_top_y, tail_x + 3, tail_top_y, 0xF2181824);

        // Cabeçalho do Orador
        self.draw_text(bx + 10, by + 6, speaker_name, 0xFFF1FA8C, 1);
        self.draw_line(bx + 8, by + 16, bx + bubble_w - 8, by + 16, 0xFF44475A);

        // Linhas de Diálogo
        let mut cur_y = by + 20;
        for line in lines {
            self.draw_text(bx + 10, cur_y, line, 0xFFF8F8F2, 1);
            cur_y += 12;
        }

        // Dica de Interação Discreta
        if let Some(h) = hint {
            self.draw_text(bx + 10, cur_y + 2, h, 0xFF6272A4, 1);
        }
    }

    /// Exporta o framebuffer atual no formato PPM (Portable Pixmap) para inspeção e testes visuais.
    pub fn to_ppm(&self) -> Vec<u8> {
        let mut out = format!("P6\n{} {}\n255\n", self.width, self.height).into_bytes();
        out.reserve(self.width * self.height * 3);

        for &pixel in &self.pixels {
            let r = ((pixel >> 16) & 0xFF) as u8;
            let g = ((pixel >> 8) & 0xFF) as u8;
            let b = (pixel & 0xFF) as u8;
            out.push(r);
            out.push(g);
            out.push(b);
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_software_framebuffer_render_and_ppm_export() {
        let mut fb = SoftwareFramebuffer::new(100, 100);
        fb.clear(COLOR_BG);

        let mut grid = CollisionGrid::new(20, 20, true);
        grid.set_walkable(Position::new_unchecked(5, 5), true);
        grid.set_walkable(Position::new_unchecked(5, 6), false);

        fb.render_grid_view(&grid, Position::new_unchecked(5, 5), 3, 50.0, 50.0);
        fb.draw_entity_token(
            Position::new_unchecked(5, 5),
            Direction::East,
            true,
            50.0,
            50.0,
        );
        fb.draw_text(10, 10, "HADES", 0xFFFFFFFF, 1);
        fb.draw_rect(2, 2, 20, 20, 0xFF00FF00, Some(0xFFFF0000));

        let ppm = fb.to_ppm();
        assert!(ppm.starts_with(b"P6\n100 100\n255\n"));
        assert_eq!(ppm.len(), b"P6\n100 100\n255\n".len() + 100 * 100 * 3);
    }

    #[test]
    fn test_software_framebuffer_resize() {
        let mut fb = SoftwareFramebuffer::new(200, 150);
        assert_eq!(fb.width, 200);
        assert_eq!(fb.height, 150);
        assert_eq!(fb.pixels.len(), 200 * 150);

        // Redimensiona para formato vertical (ex: Omarchy split 480x860)
        fb.resize(480, 860);
        assert_eq!(fb.width, 480);
        assert_eq!(fb.height, 860);
        assert_eq!(fb.pixels.len(), 480 * 860);
        assert_eq!(fb.pixels[0], COLOR_BG);
    }
}
