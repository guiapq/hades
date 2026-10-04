//! SPEC-0024: Sistema de Partículas em Tempo Real e Impactos de Combate.
//!
//! Simulação física 2.5D de partículas com zero alocações na heap (Potato Budget).
//! Suporta faíscas de lâmina, descargas elétricas, explosão de acerto crítico,
//! estouro aquoso de gelatina (Slime Burst) e rastros de poeira de esquiva.

use super::software_framebuffer::SoftwareFramebuffer;

/// Tipos de partículas especializadas para feedback de combate e movimentação.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleKind {
    /// Faíscas direcionais de corte de espada (Hit 1 e 2).
    Spark,
    /// Microdescarga elétrica ciano/azul para ataques rápidos (Jab).
    ElectricMicroSpark,
    /// Explosão de impacto pesado e cruz luminosa (Hit 3 / Crítico).
    CriticalBurst,
    /// Gotas aquosas de gelatina rosa/magenta emitidas na derrota do monstro.
    SlimeBurst,
    /// Rastro de poeira / arrebentação emitido no rolamento evasivo (L2).
    DodgeDust,
    /// Gotas e borrifos de água da fonte central (Living World).
    FountainSpray,
    /// Ondas de água e respingo ao interagir com água (Feedback Contínuo).
    WaterSplash,
    /// Partículas sutis de ar / poeira de luz flutuando no ambiente.
    AmbientMote,
}

/// Registro individual de partícula com física 3D em espaço de mundo.
#[derive(Debug, Clone, Copy)]
pub struct Particle {
    pub world_x: f32,
    pub world_y: f32,
    pub world_z: f32,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
    pub drag: f32,
    pub gravity: f32,
    pub color: u32,
    pub size: u8,
    pub age_ms: f32,
    pub max_age_ms: f32,
    pub kind: ParticleKind,
    pub additive: bool,
    pub active: bool,
}

impl Default for Particle {
    fn default() -> Self {
        Self {
            world_x: 0.0,
            world_y: 0.0,
            world_z: 0.0,
            vx: 0.0,
            vy: 0.0,
            vz: 0.0,
            drag: 0.92,
            gravity: 9.8,
            color: 0xFFFFFFFF,
            size: 2,
            age_ms: 0.0,
            max_age_ms: 300.0,
            kind: ParticleKind::Spark,
            additive: true,
            active: false,
        }
    }
}

/// Pool estático de 512 partículas com atualização O(N) e zero alocações.
pub struct ParticleSystem {
    particles: [Particle; 512],
    next_slot: usize,
    rng_state: u64,
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl ParticleSystem {
    /// Cria o sistema com 512 partículas inativas pré-alocadas no stack.
    pub fn new() -> Self {
        Self {
            particles: [Particle::default(); 512],
            next_slot: 0,
            rng_state: 0x8543_1248_9A7B_CDEF,
        }
    }

    /// Gerador rápido de números pseudo-aleatórios sem alocações (xorshift64).
    #[inline]
    fn next_rand(&mut self) -> f32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        let bits = (self.rng_state >> 40) as u32;
        (bits as f32) / (16777216.0) // [0.0, 1.0)
    }

    /// Retorna o número de partículas atualmente vivas.
    pub fn active_count(&self) -> usize {
        self.particles.iter().filter(|p| p.active).count()
    }

    /// Limpa todas as partículas ativas.
    pub fn clear(&mut self) {
        for p in self.particles.iter_mut() {
            p.active = false;
        }
    }

    /// Aloca uma partícula do buffer circular estático.
    #[inline]
    fn alloc_particle(&mut self) -> &mut Particle {
        let slot = self.next_slot;
        self.next_slot = (self.next_slot + 1) % self.particles.len();
        &mut self.particles[slot]
    }

    /// Emite faíscas brilhantes de corte direcional de espada (Hit 1 e 2).
    pub fn spawn_slash_sparks(&mut self, x: f32, y: f32, z: f32, base_angle_rad: f32, count: usize) {
        for _ in 0..count {
            let r1 = self.next_rand();
            let r2 = self.next_rand();
            let r3 = self.next_rand();

            let angle = base_angle_rad + (r1 - 0.5) * 1.2;
            let speed = 3.5 + r2 * 5.0;

            let p = self.alloc_particle();
            p.world_x = x;
            p.world_y = y;
            p.world_z = z;
            p.vx = angle.cos() * speed;
            p.vy = angle.sin() * speed;
            p.vz = 1.5 + r3 * 3.5;
            p.drag = 0.88;
            p.gravity = 14.0;
            p.color = if r1 > 0.4 { 0xFFFFFF77 } else { 0xFFFFB86C };
            p.size = if r2 > 0.6 { 2 } else { 1 };
            p.age_ms = 0.0;
            p.max_age_ms = 220.0 + r1 * 120.0;
            p.kind = ParticleKind::Spark;
            p.additive = true;
            p.active = true;
        }
    }

    /// Emite microdescargas elétricas ciano para golpes de jab.
    pub fn spawn_jab_sparks(&mut self, x: f32, y: f32, z: f32, count: usize) {
        for _ in 0..count {
            let r1 = self.next_rand();
            let r2 = self.next_rand();
            let angle = r1 * std::f32::consts::TAU;
            let speed = 4.0 + r2 * 4.0;

            let p = self.alloc_particle();
            p.world_x = x;
            p.world_y = y;
            p.world_z = z;
            p.vx = angle.cos() * speed;
            p.vy = angle.sin() * speed;
            p.vz = (r2 - 0.5) * 4.0;
            p.drag = 0.82;
            p.gravity = 4.0;
            p.color = 0xFF8BE9FD; // Ciano elétrico
            p.size = 1;
            p.age_ms = 0.0;
            p.max_age_ms = 140.0 + r1 * 80.0;
            p.kind = ParticleKind::ElectricMicroSpark;
            p.additive = true;
            p.active = true;
        }
    }

    /// Emite uma explosão radial de acerto crítico / finalizador pesado (Hit 3).
    pub fn spawn_critical_burst(&mut self, x: f32, y: f32, z: f32) {
        for i in 0..28 {
            let r1 = self.next_rand();
            let r2 = self.next_rand();
            let angle = (i as f32 / 28.0) * std::f32::consts::TAU + (r1 - 0.5) * 0.2;
            let speed = 5.0 + r2 * 7.0;

            let p = self.alloc_particle();
            p.world_x = x;
            p.world_y = y;
            p.world_z = z;
            p.vx = angle.cos() * speed;
            p.vy = angle.sin() * speed;
            p.vz = 2.0 + r1 * 5.0;
            p.drag = 0.86;
            p.gravity = 16.0;
            p.color = if i % 2 == 0 { 0xFFFF5555 } else { 0xFFF1FA8C };
            p.size = if i % 4 == 0 { 3 } else { 2 };
            p.age_ms = 0.0;
            p.max_age_ms = 320.0 + r1 * 150.0;
            p.kind = ParticleKind::CriticalBurst;
            p.additive = true;
            p.active = true;
        }
    }

    /// Emite uma explosão aquosa e gelatinosa de derrota (Slime Burst).
    pub fn spawn_slime_death_burst(&mut self, x: f32, y: f32, z: f32) {
        for _ in 0..36 {
            let r1 = self.next_rand();
            let r2 = self.next_rand();
            let r3 = self.next_rand();
            let angle = r1 * std::f32::consts::TAU;
            let speed = 2.5 + r2 * 6.0;

            let p = self.alloc_particle();
            p.world_x = x;
            p.world_y = y;
            p.world_z = z;
            p.vx = angle.cos() * speed;
            p.vy = angle.sin() * speed;
            p.vz = 4.0 + r3 * 6.5; // Salto alto em arco parabólico
            p.drag = 0.93;
            p.gravity = 18.0;
            // Cores gelatinosas rosa translúcido / magenta / gota aquosa
            p.color = match (r1 * 3.0) as u32 {
                0 => 0xFFFF79C6, // Rosa característico
                1 => 0xFFFF92DF, // Rosa claro
                _ => 0xFFFFFFFF, // Ponto de brilho de água
            };
            p.size = if r2 > 0.5 { 3 } else { 2 };
            p.age_ms = 0.0;
            p.max_age_ms = 450.0 + r2 * 250.0;
            p.kind = ParticleKind::SlimeBurst;
            p.additive = false;
            p.active = true;
        }
    }

    /// Emite baforadas de poeira e vento ao rolar ou esquivar (L2).
    pub fn spawn_dodge_dust(&mut self, x: f32, y: f32, z: f32) {
        for _ in 0..10 {
            let r1 = self.next_rand();
            let r2 = self.next_rand();
            let angle = r1 * std::f32::consts::TAU;
            let speed = 1.0 + r2 * 2.5;

            let p = self.alloc_particle();
            p.world_x = x + (r1 - 0.5) * 0.4;
            p.world_y = y + (r2 - 0.5) * 0.4;
            p.world_z = z;
            p.vx = angle.cos() * speed;
            p.vy = angle.sin() * speed;
            p.vz = 0.5 + r1 * 1.0;
            p.drag = 0.85;
            p.gravity = 3.0;
            p.color = 0xAA6272A4; // Cinza-azul suave de poeira
            p.size = 2;
            p.age_ms = 0.0;
            p.max_age_ms = 280.0 + r1 * 120.0;
            p.kind = ParticleKind::DodgeDust;
            p.additive = false;
            p.active = true;
        }
    }

    /// Dispara pequenas partículas sutis de poeira nos passos ao caminhar.
    pub fn spawn_footstep_dust(&mut self, x: f32, y: f32) {
        for _ in 0..3 {
            let r1 = self.next_rand();
            let r2 = self.next_rand();
            let angle = r1 * std::f32::consts::TAU;
            let speed = 0.4 + r2 * 0.8;

            let p = self.alloc_particle();
            p.world_x = x + (r1 - 0.5) * 0.2;
            p.world_y = y + (r2 - 0.5) * 0.2;
            p.world_z = 0.05;
            p.vx = angle.cos() * speed;
            p.vy = angle.sin() * speed;
            p.vz = 0.2 + r1 * 0.4;
            p.drag = 0.80;
            p.gravity = 1.5;
            p.color = 0x66A0A0A0; // Poeira translúcida suave
            p.size = 1;
            p.age_ms = 0.0;
            p.max_age_ms = 180.0 + r1 * 80.0;
            p.kind = ParticleKind::DodgeDust;
            p.additive = false;
            p.active = true;
        }
    }

    /// Emite gotas de água e borrifos suaves da fonte central (Living World).
    pub fn spawn_fountain_spray(&mut self, x: f32, y: f32, z: f32, count: usize) {
        for _ in 0..count {
            let r1 = self.next_rand();
            let r2 = self.next_rand();
            let r3 = self.next_rand();
            let angle = r1 * std::f32::consts::TAU;
            let speed = 0.5 + r2 * 1.5;

            let p = self.alloc_particle();
            p.world_x = x + (r1 - 0.5) * 0.35;
            p.world_y = y + (r2 - 0.5) * 0.35;
            p.world_z = z;
            p.vx = angle.cos() * speed;
            p.vy = angle.sin() * speed;
            p.vz = 1.6 + r3 * 2.4; // Arco ascendente e queda graciosa
            p.drag = 0.94;
            p.gravity = 7.5;
            // Cores suaves de água cristalina
            p.color = match (r1 * 3.0) as u32 {
                0 => 0xCC8BE9FD, // Ciano translúcido
                1 => 0xBBBD93F9, // Reflexo púrpura suave
                _ => 0xEEF8F8F2, // Gotícula iluminada
            };
            p.size = if r2 > 0.75 { 2 } else { 1 };
            p.age_ms = 0.0;
            p.max_age_ms = 480.0 + r2 * 220.0;
            p.kind = ParticleKind::FountainSpray;
            p.additive = true;
            p.active = true;
        }
    }

    /// Emite respingos concentrados ao clicar ou interagir com a água (feedback contínuo).
    pub fn spawn_water_splash(&mut self, x: f32, y: f32, z: f32) {
        for i in 0..12 {
            let r1 = self.next_rand();
            let r2 = self.next_rand();
            let angle = (i as f32 / 12.0) * std::f32::consts::TAU + (r1 - 0.5) * 0.3;
            let speed = 1.0 + r2 * 2.0;

            let p = self.alloc_particle();
            p.world_x = x;
            p.world_y = y;
            p.world_z = z;
            p.vx = angle.cos() * speed;
            p.vy = angle.sin() * speed;
            p.vz = 1.4 + r1 * 2.2;
            p.drag = 0.88;
            p.gravity = 11.0;
            p.color = if i % 2 == 0 { 0xEE8BE9FD } else { 0xFFFFFFFF };
            p.size = 2;
            p.age_ms = 0.0;
            p.max_age_ms = 300.0 + r2 * 120.0;
            p.kind = ParticleKind::WaterSplash;
            p.additive = true;
            p.active = true;
        }
    }

    /// Emite partículas sutis de luz ambiente (motes) flutuando no ar.
    pub fn spawn_ambient_mote(&mut self, x: f32, y: f32, z: f32) {
        let r1 = self.next_rand();
        let r2 = self.next_rand();
        let r3 = self.next_rand();
        let angle = r1 * std::f32::consts::TAU;
        let speed = 0.15 + r2 * 0.3;

        let p = self.alloc_particle();
        p.world_x = x + (r1 - 0.5) * 6.0;
        p.world_y = y + (r2 - 0.5) * 6.0;
        p.world_z = z + r3 * 1.5;
        p.vx = angle.cos() * speed;
        p.vy = angle.sin() * speed;
        p.vz = 0.05 + (r3 - 0.5) * 0.1;
        p.drag = 0.98;
        p.gravity = -0.05; // Leve flutuabilidade
        p.color = 0x88F1FA8C; // Dourado pálido e suave
        p.size = 1;
        p.age_ms = 0.0;
        p.max_age_ms = 900.0 + r2 * 600.0;
        p.kind = ParticleKind::AmbientMote;
        p.additive = true;
        p.active = true;
    }

    /// Atualização de simulação física a cada tick/quadro (dt em segundos).
    pub fn update(&mut self, dt_secs: f32) {
        let dt_ms = dt_secs * 1000.0;

        for p in self.particles.iter_mut() {
            if !p.active {
                continue;
            }

            p.age_ms += dt_ms;
            if p.age_ms >= p.max_age_ms {
                p.active = false;
                continue;
            }

            // Integração de movimento Euler com arraste e gravidade
            p.world_x += p.vx * dt_secs;
            p.world_y += p.vy * dt_secs;
            p.world_z += p.vz * dt_secs;

            // Arraste aerodinâmico
            p.vx *= p.drag;
            p.vy *= p.drag;

            // Gravidade para baixo no eixo Z
            p.vz -= p.gravity * dt_secs;

            // Colisão com o solo (Z = 0)
            if p.world_z < 0.0 {
                p.world_z = 0.0;
                match p.kind {
                    ParticleKind::AmbientMote => {
                        p.vz = 0.1;
                    }
                    ParticleKind::FountainSpray | ParticleKind::WaterSplash => {
                        p.vz = -p.vz * 0.2;
                        p.vx *= 0.5;
                        p.vy *= 0.5;
                    }
                    _ => {
                        p.vz = -p.vz * 0.35; // Quique sutil
                        p.vx *= 0.6;
                        p.vy *= 0.6;
                    }
                }
            }
        }
    }

    /// Renderiza todas as partículas ativas no framebuffer usando projeção isométrica e blending.
    pub fn render(
        &self,
        fb: &mut SoftwareFramebuffer,
        cam_offset_x: f32,
        cam_offset_y: f32,
    ) {
        let tile_h = fb.projection.tile_height;
        let zoom = fb.projection.zoom;

        for p in self.particles.iter() {
            if !p.active {
                continue;
            }

            // Projeta coordenadas do mundo (X, Y) para tela
            let (screen_x, screen_y) = fb.projection.world_to_screen(
                p.world_x,
                p.world_y,
                cam_offset_x,
                cam_offset_y,
            );

            // Elevação Z da partícula (altura em células convertida para pixels em tela)
            let px = screen_x.round() as i32;
            let py = (screen_y - p.world_z * tile_h * zoom).round() as i32;

            // Decaimento de intensidade com base na idade
            let progress = (p.age_ms / p.max_age_ms).clamp(0.0, 1.0);
            let fade = 1.0 - progress;

            let (cr, cg, cb) = (
                ((p.color >> 16) & 0xFF) as f32,
                ((p.color >> 8) & 0xFF) as f32,
                (p.color & 0xFF) as f32,
            );

            let r = (cr * fade) as u32;
            let g = (cg * fade) as u32;
            let b = (cb * fade) as u32;

            if r == 0 && g == 0 && b == 0 {
                continue;
            }

            let s = p.size as i32;
            let half = s / 2;

            for dy in -half..=half {
                for dx in -half..=half {
                    let target_x = px + dx;
                    let target_y = py + dy;

                    if target_x < 0
                        || target_x >= fb.width as i32
                        || target_y < 0
                        || target_y >= fb.height as i32
                    {
                        continue;
                    }

                    let idx = (target_y as usize) * fb.width + (target_x as usize);
                    let bg = fb.pixels[idx];

                    let bg_r = (bg >> 16) & 0xFF;
                    let bg_g = (bg >> 8) & 0xFF;
                    let bg_b = bg & 0xFF;

                    let final_color = if p.additive {
                        // Saturação Aditiva (Sparks / Crits)
                        let out_r = (bg_r + r).min(255);
                        let out_g = (bg_g + g).min(255);
                        let out_b = (bg_b + b).min(255);
                        0xFF00_0000 | (out_r << 16) | (out_g << 8) | out_b
                    } else {
                        // Alpha Blending (Slime / Dust)
                        let alpha = (fade * 220.0) as u32;
                        let inv_a = 255 - alpha;
                        let out_r = (r * alpha + bg_r * inv_a) / 255;
                        let out_g = (g * alpha + bg_g * inv_a) / 255;
                        let out_b = (b * alpha + bg_b * inv_a) / 255;
                        0xFF00_0000 | (out_r << 16) | (out_g << 8) | out_b
                    };

                    fb.pixels[idx] = final_color;
                }
            }
        }
    }
}
