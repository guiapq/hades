//! Teste de integração do sistema de animações ACT e ciclos de movimento (SPEC-0019).

use berenice::render::{IsometricProjection, SoftwareFramebuffer};
use hades_core::types::Direction;
use hades_ro_prere::act_parser::{ActAction, ActClip, ActFrame};
use hades_ro_prere::spr_parser::{Sprite, SpriteFrame};

#[test]
fn test_act_direction_mapping_and_cycle_indices() {
    // Valida que as 8 direções mapeiam 1:1 para a convenção canônica do ACT
    assert_eq!(Direction::South.to_act_dir(), 0);
    assert_eq!(Direction::SouthWest.to_act_dir(), 1);
    assert_eq!(Direction::West.to_act_dir(), 2);
    assert_eq!(Direction::NorthWest.to_act_dir(), 3);
    assert_eq!(Direction::North.to_act_dir(), 4);
    assert_eq!(Direction::NorthEast.to_act_dir(), 5);
    assert_eq!(Direction::East.to_act_dir(), 6);
    assert_eq!(Direction::SouthEast.to_act_dir(), 7);

    // Valida offsets de ação (Idle: 0..8, Walk: 8..16, Sit: 16..24, Ready: 32..40, Attack: 80..88)
    for dir in [
        Direction::South,
        Direction::SouthWest,
        Direction::West,
        Direction::NorthWest,
        Direction::North,
        Direction::NorthEast,
        Direction::East,
        Direction::SouthEast,
    ] {
        let act_dir = dir.to_act_dir();
        let idle_idx = act_dir;
        let walk_idx = 8 + act_dir;
        let sit_idx = 16 + act_dir;
        let ready_idx = 32 + act_dir;
        let attack_idx = 80 + act_dir;

        assert!(idle_idx < 8);
        assert!((8..16).contains(&walk_idx));
        assert!((16..24).contains(&sit_idx));
        assert!((32..40).contains(&ready_idx));
        assert!((80..88).contains(&attack_idx));
    }
}

#[test]
fn test_act_frame_timing_and_advance() {
    let action = ActAction {
        frames: vec![
            ActFrame::default(),
            ActFrame::default(),
            ActFrame::default(),
            ActFrame::default(),
        ],
        delay_ms: 100.0,
    };

    // 0 ms -> frame 0
    let (_, f0) = action.frame_by_time(0.0).unwrap();
    assert_eq!(f0, 0);

    // 50 ms -> frame 0
    let (_, f0_mid) = action.frame_by_time(50.0).unwrap();
    assert_eq!(f0_mid, 0);

    // 100 ms -> frame 1
    let (_, f1) = action.frame_by_time(100.0).unwrap();
    assert_eq!(f1, 1);

    // 250 ms -> frame 2
    let (_, f2) = action.frame_by_time(250.0).unwrap();
    assert_eq!(f2, 2);

    // 399 ms -> frame 3
    let (_, f3) = action.frame_by_time(399.0).unwrap();
    assert_eq!(f3, 3);

    // 400 ms -> loop de volta para frame 0
    let (_, f_loop) = action.frame_by_time(400.0).unwrap();
    assert_eq!(f_loop, 0);
}

#[test]
fn test_act_frame_composite_rendering_on_framebuffer() {
    let mut fb = SoftwareFramebuffer::new(100, 100);

    // Sprite sintético 4x4 com pixel vermelho no canto superior esquerdo (0,0)
    let mut pixels = vec![0u32; 16];
    pixels[0] = 0xFFFF0000; // Vermelho opaco em (0, 0)
    pixels[3] = 0xFF00FF00; // Verde opaco em (3, 0)

    let sprite = Sprite {
        version: 0x0201,
        frames: vec![SpriteFrame {
            width: 4,
            height: 4,
            pixels,
        }],
    };

    // Frame de ACT que posiciona o clip no centro (0, 0) com mirror=false
    let mut act_frame_normal = ActFrame::default();
    act_frame_normal.clips.push(ActClip {
        offset_x: 0,
        offset_y: 0,
        spr_index: 0,
        mirror: false,
        ..Default::default()
    });

    fb.draw_act_frame(50, 50, &act_frame_normal, &sprite, 1.0);

    // No centro (50, 50) com tamanho 4x4, start_x = 50 - 2 = 48, start_y = 50 - 2 = 48
    // Pixel (0,0) do sprite vai para tela (48, 48) -> Vermelho
    // Pixel (3,0) do sprite vai para tela (51, 48) -> Verde
    assert_eq!(fb.pixels[48 * 100 + 48], 0xFFFF0000);
    assert_eq!(fb.pixels[48 * 100 + 51], 0xFF00FF00);

    // Agora testa com mirror = true (espelhamento horizontal)
    let mut fb_mirror = SoftwareFramebuffer::new(100, 100);
    let mut act_frame_mirror = ActFrame::default();
    act_frame_mirror.clips.push(ActClip {
        offset_x: 0,
        offset_y: 0,
        spr_index: 0,
        mirror: true,
        ..Default::default()
    });

    fb_mirror.draw_act_frame(50, 50, &act_frame_mirror, &sprite, 1.0);

    // Com espelhamento horizontal:
    // Pixel da esquerda (0, 0) vai para a direita (51, 48) -> Vermelho!
    // Pixel da direita (3, 0) vai para a esquerda (48, 48) -> Verde!
    assert_eq!(fb_mirror.pixels[48 * 100 + 51], 0xFFFF0000);
    assert_eq!(fb_mirror.pixels[48 * 100 + 48], 0xFF00FF00);
}

#[test]
fn test_idle_head_stabilization_no_doridori() {
    // Ação Idle clássica contendo 3 frames (0: Frente, 1: Olhar Esquerda, 2: Olhar Direita - doridori)
    let idle_action = ActAction {
        frames: vec![
            ActFrame { clips: vec![ActClip { spr_index: 0, ..Default::default() }], ..Default::default() },
            ActFrame { clips: vec![ActClip { spr_index: 1, ..Default::default() }], ..Default::default() },
            ActFrame { clips: vec![ActClip { spr_index: 2, ..Default::default() }], ..Default::default() },
        ],
        delay_ms: 100.0,
    };

    // Em Idle, a cabeça e corpo devem permanecer estritamente no frame 0 (estabilidade sem balançar "não")
    let is_walking = false;
    let selected_frame = if is_walking {
        idle_action.frame_by_time(250.0).map(|(f, _)| f)
    } else {
        idle_action.frames.first()
    };

    assert_eq!(selected_frame.unwrap().clips[0].spr_index, 0);
}

#[test]
fn test_camera_relative_facing_prevents_moonwalk() {
    let mut proj = IsometricProjection::default();

    // 1. Câmera padrão (yaw = 0):
    // Personagem anda para o Norte no mundo (afasta-se do observador na tela)
    let world_facing = Direction::North;
    let visual_facing = proj.world_facing_to_camera(world_facing);
    assert_eq!(visual_facing, Direction::North);
    assert_eq!(visual_facing.to_act_dir(), 4); // Costas voltadas para a câmera

    // 2. Câmera girada 180 graus (yaw = PI):
    // O personagem continua andando para o Norte no mundo, mas na tela ele está se aproximando do observador (para baixo).
    // O sprite visual DEVE ser South (frente voltada para o observador), evitando o moonwalk.
    proj.yaw = std::f32::consts::PI;
    let visual_facing_rotated = proj.world_facing_to_camera(world_facing);
    assert_eq!(visual_facing_rotated, Direction::South);
    assert_eq!(visual_facing_rotated.to_act_dir(), 0); // Frente voltada para a câmera (caminha de frente!)

    // 3. Câmera girada 90 graus (yaw = PI / 2):
    // Na tela o personagem caminha para a esquerda. O sprite visual DEVE ser West (olhando e andando para a esquerda).
    proj.yaw = std::f32::consts::PI * 0.5;
    let visual_facing_90 = proj.world_facing_to_camera(world_facing);
    assert_eq!(visual_facing_90, Direction::West);
    assert_eq!(visual_facing_90.to_act_dir(), 2);
}

#[test]
fn test_attach_point_offset_calculation() {
    use berenice::render::{calculate_attach_offset, AttachPoint};

    // Cenário 1: Pontos idênticos (ex: idle standard (1, -56) e (1, -56))
    let p1 = AttachPoint { x: 1, y: -56 };
    let c1 = AttachPoint { x: 1, y: -56 };
    let offset = calculate_attach_offset(Some(&p1), Some(&c1), 1.0);
    assert_eq!(offset, (0, 0));

    // Cenário 2: Bobbing de caminhada (corpo desce para y = -58, cabeça permanece em -56)
    let p_walk = AttachPoint { x: 1, y: -58 };
    let offset_walk = calculate_attach_offset(Some(&p_walk), Some(&c1), 1.0);
    assert_eq!(offset_walk, (0, -2)); // cabeça é deslocada -2px para baixo acompanhando o tronco!

    // Cenário 3: Com escala de zoom 2.0x
    let offset_zoom = calculate_attach_offset(Some(&p_walk), Some(&c1), 2.0);
    assert_eq!(offset_zoom, (0, -4));

    // Cenário 4: Sem pontos de ancoragem definidos
    let offset_none = calculate_attach_offset(None, None, 1.0);
    assert_eq!(offset_none, (0, 0));
}

#[test]
fn test_layer_order_by_camera_facing() {
    use berenice::render::{get_layer_order, VisualLayer};

    // Direções voltadas para trás (3 = NW, 4 = N, 5 = NE):
    // Arma fica por trás do corpo da perspectiva do observador.
    for dir in [3, 4, 5] {
        let order = get_layer_order(dir);
        assert_eq!(
            order,
            [
                VisualLayer::Shadow,
                VisualLayer::Weapon,
                VisualLayer::Body,
                VisualLayer::Head,
                VisualLayer::Headgear,
            ]
        );
    }

    // Direções voltadas para frente e laterais (0 = S, 1 = SW, 2 = W, 6 = E, 7 = SE):
    // Arma fica à frente de todas as camadas.
    for dir in [0, 1, 2, 6, 7] {
        let order = get_layer_order(dir);
        assert_eq!(
            order,
            [
                VisualLayer::Shadow,
                VisualLayer::Body,
                VisualLayer::Head,
                VisualLayer::Headgear,
                VisualLayer::Weapon,
            ]
        );
    }
}

#[test]
fn test_composite_avatar_layering_and_attach_offset_draw() {
    use berenice::render::{AvatarRenderParams, AttachPoint};

    let mut fb = SoftwareFramebuffer::new(60, 60);

    // Sprite 2x2 vermelho para o corpo
    let body_spr = Sprite {
        version: 0x0201,
        frames: vec![SpriteFrame {
            width: 2,
            height: 2,
            pixels: vec![0xFFFF0000; 4], // Vermelho
        }],
    };
    let body_frame = ActFrame {
        clips: vec![ActClip {
            offset_x: 0,
            offset_y: 0,
            spr_index: 0,
            ..Default::default()
        }],
        attach_points: vec![AttachPoint { x: 0, y: -10 }],
        ..Default::default()
    };

    // Sprite 2x2 azul para a cabeça
    let head_spr = Sprite {
        version: 0x0201,
        frames: vec![SpriteFrame {
            width: 2,
            height: 2,
            pixels: vec![0xFF0000FF; 4], // Azul
        }],
    };
    // Cabeça com ponto de ancoragem coincidente
    let head_frame = ActFrame {
        clips: vec![ActClip {
            offset_x: 0,
            offset_y: -10, // Deslocada 10px para cima do pescoço
            spr_index: 0,
            ..Default::default()
        }],
        attach_points: vec![AttachPoint { x: 0, y: -10 }],
        ..Default::default()
    };

    let params = AvatarRenderParams {
        anchor_x: 30,
        anchor_y: 30,
        act_dir: 0, // South (Frente)
        zoom: 1.0,
        shadow_frame: None,
        body: Some((&body_frame, &body_spr)),
        head: Some((&head_frame, &head_spr)),
        headgear: None,
        weapon: None,
        flip_h: false,
    };

    fb.draw_avatar(&params);

    // Corpo desenhado em (30, 30) de tamanho 2x2 -> pixels (29, 29) a (30, 30)
    assert_eq!(fb.pixels[29 * 60 + 29], 0xFFFF0000);
    // Cabeça desenhada em (30, 20) de tamanho 2x2 -> pixels (29, 19) a (30, 20)
    assert_eq!(fb.pixels[19 * 60 + 29], 0xFF0000FF);
}

#[test]
fn test_combo_state_transitions_and_rhythmic_timing_window() {
    use berenice::render::{ComboStage, ComboTracker};
    use std::time::{Duration, Instant};

    let mut tracker = ComboTracker::new();
    let mut current_time = Instant::now();

    // 1. Primeiro ataque -> Inicia Hit 1
    assert!(tracker.trigger_attack(current_time));
    assert_eq!(tracker.current_stage, Some(ComboStage::Hit1));
    assert!(tracker.is_attacking(current_time));

    // 2. Input antecipado (ex: 150ms em Hit 1, antes de min_w=340ms):
    // Não é descartado! É retido no Input Buffer para perdoar pequenos erros de timing
    current_time += Duration::from_millis(150);
    assert!(!tracker.trigger_attack(current_time));
    assert!(tracker.input_buffer.is_some());

    // Ao avançar o tempo até a janela abrir (350ms total) e chamar tick(), o buffer é consumido imediatamente!
    current_time += Duration::from_millis(200);
    tracker.tick(current_time, false, false);
    assert_eq!(tracker.current_stage, Some(ComboStage::Hit2));
    assert!(tracker.input_buffer.is_none());
    assert!(tracker.is_inverted(current_time));

    // 3. Input rítmico perfeito na janela de Hit 2 (ex: 320ms de Hit 2) -> Avança para Hit 3 (Finisher)
    current_time += Duration::from_millis(320);
    assert!(tracker.trigger_attack(current_time));
    assert_eq!(tracker.current_stage, Some(ComboStage::Hit3));
    assert!(!tracker.is_inverted(current_time));

    // 4. Após Hit 3 concluir sua duração (720ms), o ataque cessa e permanece em postura de combate
    current_time += Duration::from_millis(730);
    assert!(!tracker.is_attacking(current_time));
    assert!(tracker.in_combat_stance(current_time));

    // 5. Após 2.5s sem novos ataques, sai da postura de combate e decai para repouso
    current_time += Duration::from_millis(2600);
    assert!(!tracker.in_combat_stance(current_time));
}

#[test]
fn test_hold_to_repeat_combo_chain() {
    use berenice::render::{ComboStage, ComboTracker};
    use std::time::{Duration, Instant};

    let mut tracker = ComboTracker::new();
    let mut current_time = Instant::now();

    // Segurando o gatilho de ataque (R2 / L2 ou Espaço / Z):
    // Frame 0: inicia Hit 1
    tracker.tick(current_time, true, false);
    assert_eq!(tracker.current_stage, Some(ComboStage::Hit1));

    // 390ms depois (dentro da janela de Hit 1): avança automaticamente para Hit 2
    current_time += Duration::from_millis(390);
    tracker.tick(current_time, true, false);
    assert_eq!(tracker.current_stage, Some(ComboStage::Hit2));
    assert!(tracker.is_inverted(current_time));

    // 350ms depois (dentro da janela de Hit 2): avança automaticamente para Hit 3 (Finisher)
    current_time += Duration::from_millis(350);
    tracker.tick(current_time, true, false);
    assert_eq!(tracker.current_stage, Some(ComboStage::Hit3));

    // 730ms depois (conclusão do Finisher com peso): reinicia automaticamente o combo em Hit 1
    current_time += Duration::from_millis(730);
    tracker.tick(current_time, true, false);
    assert_eq!(tracker.current_stage, Some(ComboStage::Hit1));
}

#[test]
fn test_quadrado_failsafe_jab() {
    use berenice::render::{ComboStage, ComboTracker};
    use std::time::{Duration, Instant};

    let mut tracker = ComboTracker::new();
    let mut current_time = Instant::now();

    // Pressionar Quadrado (West) aciona o Jab seguro (Hit 1)
    assert!(tracker.trigger_jab(current_time));
    assert_eq!(tracker.current_stage, Some(ComboStage::Hit1));
    assert!(tracker.is_jab_mode);
    assert_eq!(tracker.current_hud_label(), "SWORDIE [JAB: CORTE RÁPIDO (FAIL-SAFE 1.0x)]");

    // Estando em combo pesado (ex: Hit 2), pressionar Quadrado aciona o fail-safe para jab rápido
    tracker.trigger_attack(current_time);
    current_time += Duration::from_millis(390);
    tracker.tick(current_time, true, false); // Foi para Hit 2
    assert_eq!(tracker.current_stage, Some(ComboStage::Hit2));

    // Usuário aperta Quadrado como fail-safe para cancelar o combo em um Jab rápido
    current_time += Duration::from_millis(250);
    tracker.trigger_jab(current_time);
    assert_eq!(tracker.current_stage, Some(ComboStage::Hit1));
    assert!(tracker.is_jab_mode);
}

#[test]
fn test_training_dummy_hit_resolution_and_damage() {
    use berenice::render::{DummyState, TrainingDummy};
    use hades_core::types::Position;
    use std::time::{Duration, Instant};

    let mut dummy = TrainingDummy::new(Position { x: 158, y: 180 });
    let mut now = Instant::now();

    assert_eq!(dummy.current_hp, 60);
    assert_eq!(dummy.state, DummyState::Idle);

    // Golpe Hit 1: 12 de dano
    let (lethal, dmg) = dummy.take_hit(12, now);
    assert!(!lethal);
    assert_eq!(dmg, 12);
    assert_eq!(dummy.current_hp, 48);
    assert_eq!(dummy.state, DummyState::Hurt);

    // Após 260ms, recupera de Hurt e volta para Idle
    now += Duration::from_millis(260);
    dummy.tick(now);
    assert_eq!(dummy.state, DummyState::Idle);
}

#[test]
fn test_training_dummy_death_and_respawn_cycle() {
    use berenice::render::{DummyState, TrainingDummy};
    use hades_core::types::Position;
    use std::time::{Duration, Instant};

    let mut dummy = TrainingDummy::new(Position { x: 158, y: 180 });
    let mut now = Instant::now();

    // Dano fatal
    let (lethal, _) = dummy.take_hit(60, now);
    assert!(lethal);
    assert_eq!(dummy.current_hp, 0);
    assert_eq!(dummy.state, DummyState::Die);
    assert!(!dummy.is_hittable());

    // Durante a animação de Die (ex: 1s), não renasce ainda
    now += Duration::from_millis(1000);
    dummy.tick(now);
    assert_eq!(dummy.state, DummyState::Die);

    // Após 2s no total, renasce com HP completo
    now += Duration::from_millis(1100);
    dummy.tick(now);
    assert_eq!(dummy.state, DummyState::Idle);
    assert_eq!(dummy.current_hp, 60);
    assert!(dummy.is_hittable());
}

#[test]
fn test_floating_damage_buffer_zero_allocations() {
    use berenice::render::FloatingNumberPool;
    use std::time::{Duration, Instant};

    let mut pool = FloatingNumberPool::new();
    let now = Instant::now();

    pool.spawn(158.0, 180.0, 12, 0xFFF1FA8C, false, now);
    pool.spawn(158.0, 180.0, 15, 0xFFFFB86C, false, now);
    pool.spawn(158.0, 180.0, 24, 0xFFFF5555, true, now);

    let active: Vec<_> = pool.active_numbers(now).collect();
    assert_eq!(active.len(), 3);

    // Após 700ms, números expiram e saem do pool
    let later = now + Duration::from_millis(750);
    let active_later: Vec<_> = pool.active_numbers(later).collect();
    assert_eq!(active_later.len(), 0);
}

#[test]
fn test_evasive_dodge_maneuver_displacement_and_cooldown() {
    use berenice::render::DodgeTracker;
    use std::time::{Duration, Instant};

    let mut tracker = DodgeTracker::new();
    let mut now = Instant::now();

    // 1. Manobra pronta inicialmente
    assert!(tracker.can_dodge(now));
    assert!(tracker.trigger_dodge(now));
    assert!(tracker.is_dodging);
    assert!(tracker.has_iframes(now));

    // 2. Cooldown ativo (não permite esquiva consecutiva)
    now += Duration::from_millis(100);
    assert!(!tracker.can_dodge(now));
    assert!(!tracker.trigger_dodge(now));

    // 3. Fim da translação (180ms)
    now += Duration::from_millis(100);
    tracker.tick(now);
    assert!(!tracker.is_dodging);

    // 4. Retorno após 650ms de cooldown
    now += Duration::from_millis(500);
    assert!(tracker.can_dodge(now));
}

#[test]
fn test_player_hud_hp_sp_ratio_and_regen() {
    use berenice::state::PlayerStats;
    use std::time::{Duration, Instant};

    let mut stats = PlayerStats::new_swordsman("Berenice");
    assert!(stats.current_hp > 0);
    assert!(stats.current_sp > 0);
    assert_eq!(stats.current_hp, stats.derived.max_hp);

    // Consumo de SP (esquiva / habilidade)
    assert!(stats.consume_sp(4));
    assert_eq!(stats.current_sp, stats.derived.max_sp - 4);

    // Dano sofrido
    stats.take_damage(50);
    assert_eq!(stats.current_hp, stats.derived.max_hp - 50);

    // Regeneração periódica a cada 3s
    let now = Instant::now() + Duration::from_millis(3100);
    stats.tick_regen(now);
    assert!(stats.current_hp > stats.derived.max_hp - 50);
    assert!(stats.current_sp > stats.derived.max_sp - 4);
}

#[test]
fn test_inventory_usage_and_weight_limit() {
    use berenice::state::PlayerStats;

    let mut stats = PlayerStats::new_swordsman("Berenice");
    let initial_items = stats.inventory.len();
    let initial_weight = stats.current_weight();
    assert!(initial_weight > 0);
    assert!(initial_weight <= stats.max_weight());

    // Usar Poção Vermelha (slot 0) cura dano
    stats.take_damage(60);
    let hp_before = stats.current_hp;
    assert!(stats.use_item(0));
    assert_eq!(stats.current_hp, hp_before + 45);
    assert_eq!(stats.inventory.len(), initial_items); // 15 -> 14
    assert!(stats.current_weight() < initial_weight);
}

#[test]
fn test_realtime_particle_system_physics_and_zero_alloc() {
    use berenice::render::ParticleSystem;

    let mut ps = ParticleSystem::new();
    assert_eq!(ps.active_count(), 0);

    // 1. Emissão de faíscas de corte
    ps.spawn_slash_sparks(158.5, 180.5, 0.5, 0.0, 16);
    assert_eq!(ps.active_count(), 16);

    // 2. Emissão de explosão crítica
    ps.spawn_critical_burst(158.5, 180.5, 0.5);
    assert_eq!(ps.active_count(), 16 + 28);

    // 3. Emissão de estouro de gelatina (Slime Burst)
    ps.spawn_slime_death_burst(158.5, 180.5, 0.5);
    assert_eq!(ps.active_count(), 16 + 28 + 36);

    // 4. Emissão de poeira de esquiva (Dodge Dust)
    ps.spawn_dodge_dust(158.5, 180.5, 0.1);
    assert_eq!(ps.active_count(), 16 + 28 + 36 + 10);

    // 5. Simulação física ao longo do tempo (arraste e gravidade)
    ps.update(0.016); // 16ms (1 quadro a 60 FPS)
    assert_eq!(ps.active_count(), 90);

    // 6. Decaimento e expiração após passar a vida útil máxima (até 700ms)
    ps.update(1.0);
    assert_eq!(ps.active_count(), 0);
}

#[test]
fn test_poring_dummy_death_animation_single_cycle() {
    use berenice::render::{DummyState, TrainingDummy};
    use hades_core::types::Position;
    use std::time::{Duration, Instant};

    let mut dummy = TrainingDummy::new(Position { x: 158, y: 180 });
    let mut now = Instant::now();

    // Dano fatal de 60 HP
    let (fatal, dmg) = dummy.take_hit(60, now);
    assert!(fatal);
    assert_eq!(dmg, 60);
    assert_eq!(dummy.current_hp, 0);
    assert_eq!(dummy.state, DummyState::Die);
    assert!(!dummy.is_hittable());

    // Durante os primeiros 400ms: animação de morte ativa
    now += Duration::from_millis(350);
    dummy.tick(now);
    assert_eq!(dummy.state, DummyState::Die);
    assert!(!dummy.is_hittable());

    // Entre 400ms e 2000ms: monstro fica oculto (já explodiu), aguardando respawn
    now += Duration::from_millis(500);
    dummy.tick(now);
    assert_eq!(dummy.state, DummyState::Die);
    assert!(!dummy.is_hittable());

    // Após 2000ms: respawn com HP completo
    now += Duration::from_millis(1200);
    dummy.tick(now);
    assert_eq!(dummy.state, DummyState::Idle);
    assert_eq!(dummy.current_hp, 60);
    assert!(dummy.is_hittable());
}

#[test]
fn test_particle_system_render_on_framebuffer() {
    use berenice::render::{ParticleSystem, SoftwareFramebuffer};

    let mut fb = SoftwareFramebuffer::new(200, 200);
    let mut ps = ParticleSystem::new();

    // Spawna faíscas na origem da câmera
    ps.spawn_critical_burst(0.0, 0.0, 0.5);
    assert!(ps.active_count() > 0);

    // Renderiza no framebuffer sem causar panics ou estouros de índice
    ps.render(&mut fb, 0.0, 0.0);
}

#[test]
fn test_nordic_battle_shouts_trigger_and_expiration() {
    use berenice::render::BattleShoutTracker;
    use std::time::{Duration, Instant};

    let mut shouts = BattleShoutTracker::new();
    let mut now = Instant::now();
    assert!(shouts.current_shout.is_none());

    // 1. Grito Nórdico de Jab garantido
    shouts.trigger_jab_guaranteed(now);
    let s = shouts.current_shout.expect("Deveria haver grito ativo");
    assert_eq!(s.color, 0xFF8BE9FD);
    assert_eq!(s.scale, 1);
    assert!(!s.text.is_empty());

    // 2. Grito Nórdico de Hit 1 garantido
    shouts.trigger_hit1_guaranteed(now);
    let s = shouts.current_shout.unwrap();
    assert_eq!(s.color, 0xFFF1FA8C);
    assert_eq!(s.scale, 1);

    // 3. Grito Nórdico de Hit 2 garantido
    shouts.trigger_hit2_guaranteed(now);
    let s = shouts.current_shout.unwrap();
    assert_eq!(s.color, 0xFFFFB86C);
    assert_eq!(s.scale, 1);

    // 4. Grito Nórdico de Hit 3 (Finalizador Crítico: 100% proc, escala 2x, vermelho/dourado)
    shouts.trigger_hit3(now);
    let s = shouts.current_shout.unwrap();
    assert_eq!(s.color, 0xFFFF5555);
    assert_eq!(s.scale, 2);

    // 5. Grito de Esquiva garantido
    shouts.trigger_dodge_guaranteed(now);
    let s = shouts.current_shout.unwrap();
    assert_eq!(s.color, 0xFFBD93F9);

    // 6. Expiração após duração estipulada (500ms)
    now += Duration::from_millis(600);
    shouts.tick(now);
    assert!(shouts.current_shout.is_none());

    // 7. Diálogo de Wake-up ao levantar do descanso
    shouts.trigger_wakeup(now);
    let s = shouts.current_shout.unwrap();
    assert_eq!(s.color, 0xFFF8F8F2);
    assert_eq!(s.scale, 1);

    // 8. Emoticon relaxado em repouso
    shouts.trigger_idle_emoticon(now);
    let s = shouts.current_shout.unwrap();
    assert_eq!(s.color, 0xFF50FA7B);
    assert_eq!(s.scale, 1);
}

#[test]
fn test_auto_idle_tracker_sit_and_wake_up() {
    use berenice::render::AutoIdleTracker;
    use std::time::{Duration, Instant};

    let mut auto_idle = AutoIdleTracker::new();
    let mut now = Instant::now();

    // 1. Estado inicial: em pé ativo
    assert!(!auto_idle.is_sitting);

    // 2. Após 5 segundos: ainda em pé
    now += Duration::from_secs(5);
    assert!(auto_idle.tick(now).is_none());
    assert!(!auto_idle.is_sitting);

    // 3. Após 10 segundos: ativa Auto-Idle Sit (is_sitting == true)
    now += Duration::from_secs(5);
    let _ = auto_idle.tick(now);
    assert!(auto_idle.is_sitting);

    // 4. Após repouso relaxado (6s a 14s): emite o primeiro emoticon nativo
    now += Duration::from_secs(15);
    let emo = auto_idle.tick(now);
    assert!(emo.is_some(), "Deveria ter emitido o primeiro emoticon");
    assert!(auto_idle.active_emotion(now).is_some());

    // 5. Após 5 segundos adicionais: não emite nada (intervalo longo e pausado entre 12s e 28s)
    now += Duration::from_secs(5);
    assert!(auto_idle.tick(now).is_none());
    assert!(auto_idle.is_sitting);

    // 6. Jogador pressiona uma tecla ou move o mouse: acorda imediatamente!
    let woke_up = auto_idle.register_input(now);
    assert!(woke_up, "Deveria ter sinalizado wake-up do repouso");
    assert!(!auto_idle.is_sitting, "Deveria estar em pé novamente");
    assert!(auto_idle.active_emotion(now).is_none());

    // 7. Próximo comando consecutivo enquanto acordado: não é mais wake-up
    assert!(!auto_idle.register_input(now));
    assert!(!auto_idle.is_sitting);
}

#[test]
fn test_mouse_only_combat_targeting_and_range_check() {
    use berenice::input::find_best_attack_cell;
    use hades_core::collision::CollisionGrid;
    use hades_core::types::Position;

    let dummy_pos = Position { x: 158, y: 180 };
    let grid = CollisionGrid::new(200, 200, true);

    // Cenário 1: Jogador a 1 célula de distância (158, 179) -> Em alcance melee (dist <= 1)
    let player_melee = Position { x: 158, y: 179 };
    let dist = player_melee.chebyshev_distance(dummy_pos);
    assert!(dist <= 1, "Deveria estar em alcance melee");

    // Cenário 2: Jogador longe (150, 180) -> Fora de alcance melee (dist = 8 > 1)
    let player_far = Position { x: 150, y: 180 };
    let dist_far = player_far.chebyshev_distance(dummy_pos);
    assert!(dist_far > 1, "Deveria precisar de aproximação automática");

    // Cenário 3: Resolução de aproximação automática para alcance 1 (espada)
    let best_sword = find_best_attack_cell(&grid, player_far, dummy_pos, 1);
    assert_eq!(
        best_sword,
        Some(Position { x: 157, y: 180 }),
        "Deve navegar até a primeira célula adjacente ao alvo"
    );

    // Cenário 4: Resolução de aproximação automática para alcance 2 (lança)
    let best_spear = find_best_attack_cell(&grid, player_far, dummy_pos, 2);
    assert_eq!(
        best_spear,
        Some(Position { x: 156, y: 180 }),
        "Deve navegar até a distância permitida de 2 células para lança"
    );
}

#[test]
fn test_find_emotion_in_grf() {
    let env_grf = std::env::var("HADES_GRF_PATH").ok();
    let grf_candidates = [
        env_grf.as_deref().unwrap_or(""),
        "data.grf",
        "../../data.grf",
    ];
    let mut grf_opt = None;
    for c in &grf_candidates {
        if !c.is_empty() && std::path::Path::new(c).exists() {
            if let Ok(g) = berenice::vfs::GrfArchive::open(c) {
                grf_opt = Some(g);
                break;
            }
        }
    }
    if let Some(grf) = grf_opt {
        let emo_spr_bytes = grf.extract("data/sprite/이팩트/emotion.spr");
        let emo_act_bytes = grf.extract("data/sprite/이팩트/emotion.act");
        assert!(emo_spr_bytes.is_some(), "emotion.spr deve existir no GRF");
        assert!(emo_act_bytes.is_some(), "emotion.act deve existir no GRF");

        let _spr = hades_ro_prere::spr_parser::parse_spr(&emo_spr_bytes.unwrap()).expect("parse emotion.spr");
        let _act = hades_ro_prere::act_parser::parse_act(&emo_act_bytes.unwrap()).expect("parse emotion.act");

        if let (Some(k_act_bytes), Some(k_spr_bytes)) = (
            grf.extract("data/sprite/npc/4_f_kafra1.act"),
            grf.extract("data/sprite/npc/4_f_kafra1.spr"),
        ) {
            let k_act = hades_ro_prere::act_parser::parse_act(&k_act_bytes).expect("parse kafra act");
            let k_spr = hades_ro_prere::spr_parser::parse_spr(&k_spr_bytes).expect("parse kafra spr");
            println!(
                "Kafra NPC loaded! Actions: {}, Sprites: {}",
                k_act.actions.len(),
                k_spr.frames.len()
            );
            assert!(!k_act.actions.is_empty());
        }
    }
}

#[test]
fn test_inspect_gnd_ground() {
    let env_grf = std::env::var("HADES_GRF_PATH").ok();
    let grf_candidates = [
        env_grf.as_deref().unwrap_or(""),
        "data.grf",
        "../../data.grf",
    ];
    let mut grf_opt = None;
    for c in &grf_candidates {
        if !c.is_empty() && std::path::Path::new(c).exists() {
            if let Ok(g) = berenice::vfs::GrfArchive::open(c) {
                grf_opt = Some(g);
                break;
            }
        }
    }
    if let Some(grf) = grf_opt {
        if let Some(bytes) = grf.extract("data/prontera.gnd") {
            println!("prontera.gnd size: {} bytes", bytes.len());
            assert!(bytes.len() >= 16);
            println!("Magic: {:?} ({:?})", &bytes[0..4], std::str::from_utf8(&bytes[0..4]));
            let version_major = bytes[4];
            let version_minor = bytes[5];
            let width = u32::from_le_bytes(bytes[6..10].try_into().unwrap());
            let height = u32::from_le_bytes(bytes[10..14].try_into().unwrap());
            let zoom = f32::from_le_bytes(bytes[14..18].try_into().unwrap());
            let tex_count = u32::from_le_bytes(bytes[18..22].try_into().unwrap());
            println!("GND: version {}.{}, size: {}x{}, zoom: {}, textures: {}", version_major, version_minor, width, height, zoom, tex_count);
            let gnd = hades_ro_prere::parse_gnd(&bytes).expect("parse prontera.gnd");
            println!("prontera.gnd parsed: {}x{} cells, {} textures, {} tiles", gnd.width, gnd.height, gnd.textures.len(), gnd.tiles.len());
            assert_eq!(gnd.width, 156);
            assert_eq!(gnd.height, 196);
            assert_eq!(gnd.textures.len(), 12);
            assert_eq!(gnd.tiles.len(), 28511);
            let tex_sample = gnd.texture_for_cell(78, 98);
            println!("Cell (78, 98) center texture: {:?}", tex_sample);
            for (i, tex) in gnd.textures.iter().enumerate() {
                let found = grf.extract(tex);
                let parsed = found.as_deref().and_then(|b| hades_ro_prere::parse_bmp(b).ok());
                println!("Tex #{}: {} -> found: {}, parsed: {:?}", i, tex, found.is_some(), parsed.as_ref().map(|b| (b.width, b.height)));
            }
        }
    }
}

#[test]
fn test_render_full_frame_with_textured_ground_and_ui() {
    let env_grf = std::env::var("HADES_GRF_PATH").ok();
    let grf_candidates = [
        env_grf.as_deref().unwrap_or(""),
        "data.grf",
        "../../data.grf",
    ];
    let mut grf_opt = None;
    for c in &grf_candidates {
        if !c.is_empty() && std::path::Path::new(c).exists() {
            if let Ok(g) = berenice::vfs::GrfArchive::open(c) {
                grf_opt = Some(g);
                break;
            }
        }
    }
    let grf = match grf_opt {
        Some(g) => g,
        None => return,
    };

    let gat_bytes = match grf.extract("data/prontera.gat") {
        Some(b) => b,
        None => return,
    };
    let grid = hades_ro_prere::gat_parser::parse_gat(&gat_bytes).expect("parse gat");

    let gnd_bytes = match grf.extract("data/prontera.gnd") {
        Some(b) => b,
        None => return,
    };
    let gnd = hades_ro_prere::parse_gnd(&gnd_bytes).expect("parse gnd");

    let mut textures = std::collections::HashMap::new();
    for tex_path in &gnd.textures {
        if let Some(bmp_bytes) = grf.extract(tex_path) {
            if let Ok(bmp) = hades_ro_prere::parse_bmp(&bmp_bytes) {
                textures.insert(tex_path.clone(), bmp);
            }
        }
    }
    assert_eq!(textures.len(), 12);

    let fallback_tex = hades_ro_prere::bmp_parser::BmpImage {
        width: 16,
        height: 16,
        pixels: vec![0xFF888888; 256],
    };

    let width = 800;
    let height = 600;
    let mut fb = SoftwareFramebuffer::new(width, height);
    fb.clear(berenice::render::COLOR_BG);

    let player_pos = hades_core::types::Position::new_unchecked(156, 180);
    let camera_x = (width / 2) as f32;
    let camera_y = (height / 2) as f32;

    let (player_sx, player_sy) = fb.projection.world_to_screen(
        player_pos.x as f32,
        player_pos.y as f32,
        0.0,
        0.0,
    );
    let cam_offset_x = camera_x - player_sx;
    let cam_offset_y = camera_y - player_sy;

    // 1. Terreno Texturizado Real do GND
    fb.render_textured_grid_view(
        &grid,
        player_pos,
        22,
        cam_offset_x,
        cam_offset_y,
        Some(&gnd),
        &textures,
        &fallback_tex,
    );

    // 2. Modelo 3D da Fonte Central
    if let Some(fountain_bytes) = grf.extract("data/model/prontera/prt_k_bunsu_1.rsm") {
        if let Ok(fountain) = hades_ro_prere::rsm_parser::parse_rsm(&fountain_bytes) {
            fb.draw_rsm_model(&fountain, 156.0, 180.0, cam_offset_x, cam_offset_y, 0.22);
        }
    }

    // 3. Footstep Dust Particles
    let mut particle_system = berenice::render::ParticleSystem::new();
    particle_system.spawn_footstep_dust(155.0, 180.0);
    particle_system.spawn_footstep_dust(155.5, 180.0);
    particle_system.render(&mut fb, cam_offset_x, cam_offset_y);

    let shadow_spr = grf.extract("data/sprite/shadow.spr").and_then(|b| hades_ro_prere::parse_spr(&b).ok());

    // 4. Renderiza Personagem Principal (Espadachim com Elmo e Espada em guarda)
    let body_act = grf.extract("data/sprite/인간족/몸통/남/검사_남.act").and_then(|b| hades_ro_prere::parse_act(&b).ok());
    let body_spr = grf.extract("data/sprite/인간족/몸통/남/검사_남.spr").and_then(|b| hades_ro_prere::parse_spr(&b).ok());
    let head_act = grf.extract("data/sprite/인간족/머리통/남/1_남.act").and_then(|b| hades_ro_prere::parse_act(&b).ok());
    let head_spr = grf.extract("data/sprite/인간족/머리통/남/1_남.spr").and_then(|b| hades_ro_prere::parse_spr(&b).ok());
    let headgear_act = grf.extract("data/sprite/악세사리/남/남_본헬름.act").and_then(|b| hades_ro_prere::parse_act(&b).ok());
    let headgear_spr = grf.extract("data/sprite/악세사리/남/남_본헬름.spr").and_then(|b| hades_ro_prere::parse_spr(&b).ok());
    let weapon_act = grf.extract("data/sprite/인간족/검사/검사_남_검.act").and_then(|b| hades_ro_prere::parse_act(&b).ok());
    let weapon_spr = grf.extract("data/sprite/인간족/검사/검사_남_검.spr").and_then(|b| hades_ro_prere::parse_spr(&b).ok());

    if let (Some(b_act), Some(b_spr)) = (body_act.as_ref(), body_spr.as_ref()) {
        let act_dir = 6; // East
        let action_idx = 32 + act_dir; // Combat Stance
        let params = berenice::render::AvatarRenderParams {
            anchor_x: camera_x as i32,
            anchor_y: (camera_y + 8.0) as i32,
            act_dir,
            zoom: 1.0,
            shadow_frame: shadow_spr.as_ref().and_then(|s| s.frames.first()),
            body: b_act.frame(action_idx, 0).map(|f| (f, b_spr)),
            head: head_act.as_ref().and_then(|a| a.frame(action_idx, 0)).zip(head_spr.as_ref()),
            headgear: headgear_act.as_ref().and_then(|a| a.frame(action_idx, 0)).zip(headgear_spr.as_ref()),
            weapon: weapon_act.as_ref().and_then(|a| a.frame(action_idx, 0)).zip(weapon_spr.as_ref()),
            flip_h: false,
        };
        fb.draw_avatar(&params);
    }

    // 5. Slime Dummy e Dano Flutuante
    let dummy_pos = hades_core::types::Position::new_unchecked(158, 180);
    let (dummy_sx, dummy_sy) = fb.projection.world_to_screen(
        dummy_pos.x as f32 + 0.5,
        dummy_pos.y as f32 + 0.5,
        cam_offset_x,
        cam_offset_y,
    );
    let dummy_anchor_x = dummy_sx.round() as i32;
    let dummy_anchor_y = (dummy_sy + 8.0).round() as i32;

    let m_act = grf.extract("data/sprite/몬스터/poring.act").and_then(|b| hades_ro_prere::parse_act(&b).ok());
    let m_spr = grf.extract("data/sprite/몬스터/poring.spr").and_then(|b| hades_ro_prere::parse_spr(&b).ok());

    if let (Some(ref act), Some(ref spr)) = (m_act, m_spr) {
        if let Some(shadow) = shadow_spr.as_ref().and_then(|s| s.frames.first()) {
            fb.draw_sprite_frame(dummy_anchor_x, dummy_anchor_y, shadow, 1.0);
        }
        if let Some(f) = act.frame(0, 0) {
            fb.draw_act_frame(dummy_anchor_x, dummy_anchor_y, f, spr, 1.0);
        }
    }

    // Barra de vida sutil acima do monstro ferido (sem texto cobrindo o sprite)
    let bar_y = dummy_anchor_y - 24;
    fb.draw_hp_bar(dummy_anchor_x, bar_y, 28, 50);
    // Dano flutuante acima
    fb.draw_text(dummy_anchor_x - 14, bar_y - 14, "-42 CRIT!", 0xFFFF5555, 1);

    // 4.5. SPEC-0028: Guide NPC de Prontera com retículo ciano e balão de fala
    let npc_pos = hades_core::types::Position::new_unchecked(150, 178);
    let (npc_sx, npc_sy) = fb.projection.world_to_screen(npc_pos.x as f32 + 0.5, npc_pos.y as f32 + 0.5, cam_offset_x, cam_offset_y);
    let npc_anchor_x = npc_sx.round() as i32;
    let npc_anchor_y = (npc_sy + 8.0).round() as i32;

    // Retículo no chão sob o monstro (vermelho de combate) e sob a Guia (ciano de interação)
    fb.draw_ground_reticle(dummy_pos.x as f32 + 0.5, dummy_pos.y as f32 + 0.5, cam_offset_x, cam_offset_y, 0xFFFF5555, 1.1);
    fb.draw_ground_reticle(npc_pos.x as f32 + 0.5, npc_pos.y as f32 + 0.5, cam_offset_x, cam_offset_y, 0xFF8BE9FD, 1.1);

    let k_act = grf.extract("data/sprite/npc/4_f_kafra1.act").and_then(|b| hades_ro_prere::parse_act(&b).ok());
    let k_spr = grf.extract("data/sprite/npc/4_f_kafra1.spr").and_then(|b| hades_ro_prere::parse_spr(&b).ok());
    if let (Some(ref act), Some(ref spr)) = (k_act, k_spr) {
        if let Some(shadow) = shadow_spr.as_ref().and_then(|s| s.frames.first()) {
            fb.draw_sprite_frame(npc_anchor_x, npc_anchor_y, shadow, 1.0);
        }
        if let Some(f) = act.frame(0, 0) {
            fb.draw_act_frame(npc_anchor_x, npc_anchor_y, f, spr, 1.0);
        }
    }

    // Balão de fala orgânico sobre a Guia NPC
    let guide_npc = berenice::render::GuideNpc::new(npc_pos);
    let (speaker, lines, hint) = guide_npc.current_dialogue();
    fb.draw_speech_bubble(npc_anchor_x, npc_anchor_y - 56, speaker, lines, hint);

    // Sistema de partículas com borrifo da fonte central e respingos
    let mut particle_system = berenice::render::ParticleSystem::new();
    particle_system.spawn_fountain_spray(156.0, 180.0, 1.2, 16);
    particle_system.spawn_water_splash(156.5, 179.5, 0.4);
    particle_system.spawn_ambient_mote(155.0, 179.0, 0.6);
    particle_system.update(0.12);
    particle_system.render(&mut fb, cam_offset_x, cam_offset_y);

    // 5. Badge de Localização no topo esquerdo (docs/VALVE.md)
    fb.draw_rect(10, 10, 195, 24, 0xCC14141E, Some(0xFF6272A4));
    fb.draw_text(18, 16, "PRONTERA [156, 180]", 0xFFF1FA8C, 1);
    fb.draw_text(158, 16, "[F3]", 0xFF8BE9FD, 1);

    // 6. Target Info Bar no topo central
    fb.draw_target_info_bar(width as i32, "PORING DUMMY", 28, 50, 2.0);

    // 7. Player HUD no canto superior direito
    let player_stats = berenice::state::PlayerStats::new_swordsman("Berenice");
    let player_hud_x = (width - 220) as i32;
    fb.draw_player_hud(player_hud_x, 10, &player_stats);

    // 8. Radar Minimapa Dinâmico posicionado abaixo do Player HUD
    let radar_entities = vec![(dummy_pos, 0xFFFF5555), (npc_pos, 0xFF8BE9FD)];
    fb.draw_minimap(width as i32 - 110, 68, 100, &grid, player_pos, &radar_entities);

    // 9. Action Bar / Hotbar no rodapé (sem nenhum painel sobreposto!)
    fb.draw_action_bar(width as i32, height as i32, 15, 8, "SWORDIE [COMBO]");

    // Exportar frame para inspeção
    let ppm = fb.to_ppm();
    let out_dir_buf = std::env::var("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("target"));
    let out_dir = out_dir_buf.as_path();
    let _ = std::fs::create_dir_all(out_dir);
    let out_path = out_dir.join("ground_textures_ui_validated.ppm");
    std::fs::write(&out_path, &ppm).expect("write ppm");
    println!("Exported validated frame to {:?}", out_path);
}

#[test]
fn test_guide_npc_facing_and_dialogue_interaction() {
    use berenice::render::GuideNpc;
    use hades_core::types::{Direction, Position};
    use std::time::Instant;

    let now = Instant::now();
    let mut npc = GuideNpc::new(Position::new_unchecked(153, 178));
    assert_eq!(npc.facing, Direction::SouthEast);
    assert!(!npc.is_talking);

    // Jogador se aproxima pelo Leste (155, 178)
    npc.tick(Position::new_unchecked(155, 178), now);
    assert_eq!(npc.facing, Direction::East);

    // Jogador interage: abre página 0 e dispara emoticon
    npc.interact(now);
    assert!(npc.is_talking);
    assert_eq!(npc.dialogue_page, 0);
    assert!(npc.active_emotion.is_some());
    let (speaker, lines, hint) = npc.current_dialogue();
    assert_eq!(speaker, "GUIA DE PRONTERA");
    assert_eq!(lines.len(), 3);
    assert!(hint.is_some());

    // Próxima interação: avança para página 1
    npc.interact(now);
    assert_eq!(npc.dialogue_page, 1);
    let (s2, _, _) = npc.current_dialogue();
    assert_eq!(s2, "SISTEMA DE COMBATE SUAVE");

    // Próxima interação: avança para página 2
    npc.interact(now);
    assert_eq!(npc.dialogue_page, 2);
    let (s3, _, _) = npc.current_dialogue();
    assert_eq!(s3, "O MUNDO VIVO DE HADES");

    // Jogador se afasta para além de 6 células (ex: 153, 190)
    npc.tick(Position::new_unchecked(153, 190), now);
    assert_eq!(npc.facing, Direction::SouthEast); // Retorna ao default
    assert!(!npc.is_talking); // Diálogo encerra sem travamento modal
}

#[test]
fn test_living_world_particles_and_reticles() {
    use berenice::render::{ParticleSystem, SoftwareFramebuffer};

    let mut ps = ParticleSystem::new();
    assert_eq!(ps.active_count(), 0);

    // Spawn de borrifo de fonte, respingo de água e motes
    ps.spawn_fountain_spray(156.0, 180.0, 1.2, 5);
    ps.spawn_water_splash(156.0, 180.0, 0.4);
    ps.spawn_ambient_mote(156.0, 180.0, 0.5);
    assert!(ps.active_count() >= 15);

    // Física e colisão com o solo sem pânicos
    ps.update(0.1);
    assert!(ps.active_count() >= 15);

    // Renderização no framebuffer com retículo e balão de fala
    let mut fb = SoftwareFramebuffer::new(200, 200);
    ps.render(&mut fb, 0.0, 0.0);

    fb.draw_ground_reticle(100.0, 100.0, 0.0, 0.0, 0xFF8BE9FD, 1.0);
    fb.draw_speech_bubble(100, 80, "GUIA", &["Ola!", "Tudo bem?"], Some("[ESPACO]"));

    // Verifica que pixels foram pintados no framebuffer
    let non_bg = fb.pixels.iter().filter(|&&c| c != berenice::render::COLOR_BG).count();
    assert!(non_bg > 50);
}

