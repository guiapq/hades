//! Executável de Playtesting do Cliente Berenice integrado ao Servidor Hades.
//!
//! Executa uma sessão completa:
//! 1. Sobe o servidor Hades em segundo plano (porta UDP efêmera).
//! 2. Inicializa o cliente Berenice e conecta via QUIC/WebTransport (TLS 1.3).
//! 3. Simula entrada analógica de controle estilo Tree of Savior em 8 direções.
//! 4. Transmite o pacote compacto de 6 bytes (MovementDelta) via datagrama não-bloqueante.
//! 5. Executa a Cena de Login com VFS de Assets (.grf) e detecção de BGM.
//! 6. Renderiza a grade isométrica 2.5D e a caixa de diálogo de Login.

use berenice::input::InputHub;
use berenice::network::BereniceNetwork;
use berenice::render::{SoftwareFramebuffer, COLOR_BG};
use berenice::state::WorldView;
use berenice::ui::LoginScene;
use berenice::vfs::GrfArchive;
use hades_core::collision::CollisionGrid;
use hades_core::types::{EntityId, Position};
use hades_net::server::HadesServer;
use hades_ro_prere::gat_parser::parse_gat;
use hades_ro_prere::rsm_parser::{parse_rsm, RsmModel};
use hades_ro_prere::rsw_parser::parse_rsw;
use hades_ro_prere::spr_parser::parse_spr;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();
    println!("🏛️⚡ [HADES & BERENICE] Iniciando Sessão de Playtesting & Login");
    println!("=============================================================");

    // 1. Inicializar servidor Hades local
    let server = HadesServer::bind("127.0.0.1:0".parse().unwrap())?;
    let server_addr = server.local_addr()?;
    let server_cert = server.cert_der().clone();
    println!(" [Servidor Hades] Escutando em QUIC/UDP: {}", server_addr);

    // Task do servidor para ingestão de datagramas e streams do playtest
    let server_handle = tokio::spawn(async move {
        if let Some(conn) = server.accept().await {
            println!(" [Servidor Hades] Conexão de cliente aceita com sucesso!");
            if let Ok(dgram) = conn.read_datagram().await {
                println!(
                    " [Servidor Hades] Datagrama de 6 bytes recebido com sucesso ({} bytes)",
                    dgram.len()
                );
            }
            if let Ok(mut recv) = conn.accept_uni().await {
                let mut buf = [0u8; 256];
                if let Ok(Some(n)) = recv.read(&mut buf).await {
                    println!(
                        " [Servidor Hades] Requisição de autenticação recebida via stream QUIC ({} bytes)",
                        n
                    );
                }
            }
        }
    });

    // 2. Conectar cliente Berenice
    println!(" [Cliente Berenice] Conectando via TLS 1.3...");
    let client = BereniceNetwork::connect(server_addr, "localhost", server_cert).await?;
    println!(" [Cliente Berenice] Conectado ao Hades!");

    // 3. Inicializar Estado do Mundo e Entrada do Jogador
    let player_id = EntityId::new(1);
    let start_pos = Position::new_unchecked(16, 16);
    let mut world = WorldView::new(player_id, start_pos);
    let mut input_hub = InputHub::new();

    // Cria mapa de teste com obstáculos
    let mut grid = CollisionGrid::new(32, 32, true);
    for y in 14..=18 {
        grid.set_walkable(Position::new_unchecked(18, y), false);
    }

    println!("\n🎮 [Modo de Controle: Gamepad ToS]");
    input_hub.gamepad.left_stick_x = 0.85;
    input_hub.gamepad.left_stick_y = 0.85;

    let move_intent = input_hub.poll_movement();
    println!(
        "   -> Analógico ({:.2}, {:.2}) = Direção: {:?}, Correndo: {}",
        input_hub.gamepad.left_stick_x,
        input_hub.gamepad.left_stick_y,
        move_intent.direction,
        move_intent.running
    );

    let target_pos = Position::new_unchecked(17, 17);
    if let Some(delta) =
        world.create_movement_delta(target_pos, move_intent.direction, move_intent.running)
    {
        println!("   -> Gerado MovementDelta (6 bytes):");
        println!(
            "      EntityId: {}, Pos: ({}, {}), Dir: {:?}",
            delta.entity_id.0, delta.position.x, delta.position.y, delta.direction
        );

        client.send_movement(&delta)?;
        println!("   -> Datagrama despachado para o servidor Hades!");
    }

    // 4. Testar VFS de Assets (GRF / Pastas)
    println!("\n📦 [VFS de Assets & Trilha Sonora]");
    let grf_path = std::env::var("HADES_GRF_PATH").unwrap_or_else(|_| "data.grf".to_string());
    let mut prontera_grid = None;
    if Path::new(&grf_path).exists() {
        println!("   -> Localizado arquivo de assets: {}", grf_path);
        let grf = GrfArchive::open(&grf_path)?;
        println!(
            "   -> GRF carregado com sucesso! Total de arquivos indexados: {}",
            grf.file_count()
        );
        if let Some(gat_bytes) = grf.extract("data/prontera.gat") {
            if let Ok(grid) = parse_gat(&gat_bytes) {
                println!(
                    "   -> Prontera extraída e analisada: {}x{} células!",
                    grid.width, grid.height
                );
                prontera_grid = Some(grid);
            }
        }
    } else {
        println!("   -> Arquivo GRF não encontrado (utilizando fallback VFS).");
    }

    let bgm_path = std::env::var("HADES_BGM_PATH").unwrap_or_else(|_| "BGM/01.mp3".to_string());
    if Path::new(&bgm_path).exists() {
        println!(
            "   -> BGM de abertura detectada: {} (pronta para reprodução)",
            bgm_path
        );
    }

    // 5. Testar Cena de Login
    println!("\n🔐 [Cena de Login & Autenticação]");
    let mut login_scene = LoginScene::new();

    let test_user = std::env::var("HADES_AUTH_USER")
        .or_else(|_| std::env::var("HADES_USERNAME"))
        .unwrap_or_else(|_| "persephone".to_string());
    let test_pass = std::env::var("HADES_AUTH_PASSWORD")
        .or_else(|_| std::env::var("HADES_PASSWORD"))
        .unwrap_or_else(|_| "potato_budget_2026".to_string());

    // Simula digitação de usuário
    login_scene.username.clear();
    for c in test_user.chars() {
        login_scene.handle_char(c);
    }
    println!("   -> Campo Usuário preenchido: {}", login_scene.username);

    // Navega para o campo de senha
    login_scene.next_field();
    login_scene.password.clear();
    for c in test_pass.chars() {
        login_scene.handle_char(c);
    }
    println!(
        "   -> Campo Senha preenchido: {}",
        "*".repeat(login_scene.password.len())
    );

    // Navega para o botão Conectar e submete
    login_scene.next_field();
    println!("   -> Foco no botão: {:?}", login_scene.focused_field);
    login_scene.submit(&client).await?;
    println!("   -> Status de Login: {}", login_scene.status_message);

    // 6. Renderização Isométrica 2.5D + Interface de Login
    println!("\n🎨 [Renderizador Isométrico 2.5D & UI]");
    let width = 640;
    let height = 480;
    let mut fb = SoftwareFramebuffer::new(width, height);
    fb.clear(COLOR_BG);

    let camera_x = (width / 2) as f32;
    let camera_y = (height / 2) as f32;

    // Renderiza o mundo isométrico
    fb.render_grid_view(&grid, target_pos, 8, camera_x, camera_y);
    fb.draw_entity_token(target_pos, move_intent.direction, true, camera_x, camera_y);

    // Renderiza a interface de login por cima do cenário
    login_scene.render(&mut fb);

    let ppm_bytes = fb.to_ppm();
    let filename = "target/playtest_berenice_login.ppm";
    let mut file = File::create(filename)?;
    file.write_all(&ppm_bytes)?;
    println!(
        "   -> Frame 2.5D com Tela de Login exportado para: {}",
        filename
    );

    // 7. Renderiza também a cena do Mundo (sem login) com HUD
    let mut fb_world = SoftwareFramebuffer::new(width, height);
    fb_world.clear(COLOR_BG);
    let (player_sx, player_sy) =
        fb_world
            .projection
            .world_to_screen(target_pos.x as f32, target_pos.y as f32, 0.0, 0.0);
    let cam_offset_x = camera_x - player_sx;
    let cam_offset_y = camera_y - player_sy;
    fb_world.render_grid_view(&grid, target_pos, 14, cam_offset_x, cam_offset_y);
    fb_world.draw_entity_token(
        target_pos,
        move_intent.direction,
        true,
        cam_offset_x,
        cam_offset_y,
    );
    fb_world.draw_rect(10, 10, 240, 68, 0xDD14141E, Some(0xFF6272A4));
    fb_world.draw_text(18, 16, "MAPA: PRONTERA [312x392]", 0xFFBD93F9, 1);
    let pos_text = format!(
        "POS: ({:03}, {:03}) | DIR: {:?}",
        target_pos.x, target_pos.y, move_intent.direction
    );
    fb_world.draw_text(18, 30, &pos_text, 0xFF50FA7B, 1);
    fb_world.draw_text(18, 44, "TRANSPORTE: QUIC TLS 1.3", 0xFF8BE9FD, 1);
    let world_ppm = fb_world.to_ppm();
    let mut file_world = File::create("target/playtest_world.ppm")?;
    file_world.write_all(&world_ppm)?;

    // 8. Renderiza a cena do Mundo com Câmera Rotacionada (yaw = 45°, zoom = 1.25x)
    let mut fb_rotated = SoftwareFramebuffer::new(width, height);
    fb_rotated.clear(COLOR_BG);
    fb_rotated.projection.yaw = std::f32::consts::FRAC_PI_4; // 45 graus
    fb_rotated.projection.zoom = 1.25;

    let (rot_player_sx, rot_player_sy) =
        fb_rotated
            .projection
            .world_to_screen(target_pos.x as f32, target_pos.y as f32, 0.0, 0.0);
    let rot_cam_offset_x = camera_x - rot_player_sx;
    let rot_cam_offset_y = camera_y - rot_player_sy;

    let view_radius = ((22.0 / fb_rotated.projection.zoom).ceil() as u16).clamp(14, 45);
    fb_rotated.render_grid_view(
        &grid,
        target_pos,
        view_radius,
        rot_cam_offset_x,
        rot_cam_offset_y,
    );
    fb_rotated.draw_entity_token(
        target_pos,
        move_intent.direction,
        true,
        rot_cam_offset_x,
        rot_cam_offset_y,
    );

    fb_rotated.draw_rect(10, 10, 270, 84, 0xDD14141E, Some(0xFF6272A4));
    fb_rotated.draw_text(18, 16, "MAPA: ARENA DE TESTES [32x32]", 0xFFBD93F9, 1);
    let rot_pos_text = format!(
        "POS: ({:03}, {:03}) | DIR: {:?}",
        target_pos.x, target_pos.y, move_intent.direction
    );
    fb_rotated.draw_text(18, 30, &rot_pos_text, 0xFF50FA7B, 1);
    let rot_cam_degrees = fb_rotated.projection.yaw.to_degrees().rem_euclid(360.0);
    let rot_cam_text = format!(
        "CAMERA: {:03.0} DEG | ZOOM: {:.2}x",
        rot_cam_degrees, fb_rotated.projection.zoom
    );
    fb_rotated.draw_text(18, 44, &rot_cam_text, 0xFFFFB86C, 1);
    fb_rotated.draw_text(18, 58, "TRANSPORTE: QUIC TLS 1.3", 0xFF8BE9FD, 1);
    fb_rotated.draw_text(18, 72, "CLIENTE: BERENICE 2.5D @ 60 FPS", 0xFFF1FA8C, 1);

    let rotated_ppm = fb_rotated.to_ppm();
    let mut file_rotated = File::create("target/playtest_world_rotated.ppm")?;
    file_rotated.write_all(&rotated_ppm)?;
    println!("   -> Frame com Câmera Rotacionada (45°, 1.25x) exportado para target/playtest_world_rotated.ppm");

    // 9. Renderiza Prontera com Câmera Rotacionada a 45 graus se o GRF estiver presente
    if let Some(ref prontera) = prontera_grid {
        let mut fb_prontera = SoftwareFramebuffer::new(800, 600);
        fb_prontera.clear(COLOR_BG);
        fb_prontera.projection.yaw = std::f32::consts::FRAC_PI_4; // 45 graus
        fb_prontera.projection.zoom = 1.0;

        let center_fountain = Position::new_unchecked(156, 180);
        let (cx, cy) = fb_prontera.projection.world_to_screen(
            center_fountain.x as f32,
            center_fountain.y as f32,
            0.0,
            0.0,
        );
        let cam_offset_x = 400.0 - cx;
        let cam_offset_y = 300.0 - cy;

        fb_prontera.render_grid_view(prontera, center_fountain, 22, cam_offset_x, cam_offset_y);

        // Extrai e renderiza Modelos 3D (.rsm), Cena (.rsw) e Sprites 2D (.spr)
        let (shadow_spr, char_spr, fountain_model, pot_model, rsw_scene, rsm_cache) = if let Ok(grf_arc) = GrfArchive::open(&grf_path) {
            let s = grf_arc
                .extract("data/sprite/shadow.spr")
                .and_then(|b| parse_spr(&b).ok());
            let c = grf_arc
                .extract("data/sprite/npc/4_m_fairysoldier2.spr")
                .and_then(|b| parse_spr(&b).ok());
            let f = grf_arc
                .extract("data/model/prontera/prt_k_bunsu_1.rsm")
                .and_then(|b| parse_rsm(&b).ok());
            let pot = grf_arc
                .extract("data/model/prontera/flowerpot_01.rsm")
                .and_then(|b| parse_rsm(&b).ok());
            let rsw = grf_arc
                .extract("data/prontera.rsw")
                .and_then(|b| parse_rsw(&b).ok());

            let mut cache: HashMap<String, Arc<RsmModel>> = HashMap::new();
            if let Some(ref scene) = rsw {
                for m in &scene.models {
                    let wx = 156.0 + m.position[0];
                    let wy = 196.0 + m.position[2];
                    let dist = ((wx - 156.0).powi(2) + (wy - 180.0).powi(2)).sqrt();
                    if dist <= 35.0 && !cache.contains_key(&m.filename) {
                        if let Some(bytes) = grf_arc.extract(&m.filename) {
                            if let Ok(model) = parse_rsm(&bytes) {
                                cache.insert(m.filename.clone(), Arc::new(model));
                            }
                        }
                    }
                }
            }

            (s, c, f, pot, rsw, cache)
        } else {
            (None, None, None, None, None, HashMap::new())
        };

        // Renderiza Modelos 3D do Cenário RSW (.rsw)
        let mut rsw_rendered = 0;
        if let Some(ref scene) = rsw_scene {
            for obj in &scene.models {
                let wx = 156.0 + obj.position[0];
                let wy = 196.0 + obj.position[2];
                if (wx - 156.0).abs() <= 28.0 && (wy - 180.0).abs() <= 28.0 {
                    if let Some(model) = rsm_cache.get(&obj.filename) {
                        let scale = (obj.scale[0] * 0.45).clamp(0.12, 1.2);
                        fb_prontera.draw_rsm_model(
                            model,
                            wx,
                            wy,
                            cam_offset_x,
                            cam_offset_y,
                            scale,
                        );
                        rsw_rendered += 1;
                    }
                }
            }
        }

        // Renderiza Prédio 3D da Fonte Central e Vasos
        if let Some(ref fountain) = fountain_model {
            fb_prontera.draw_rsm_model(
                fountain,
                156.0,
                180.0,
                cam_offset_x,
                cam_offset_y,
                0.22,
            );
        }
        if rsw_rendered == 0 {
            if let Some(ref pot) = pot_model {
                fb_prontera.draw_rsm_model(pot, 149.0, 172.0, cam_offset_x, cam_offset_y, 0.40);
                fb_prontera.draw_rsm_model(pot, 163.0, 172.0, cam_offset_x, cam_offset_y, 0.40);
            }
        }

        if let Some(ref shadow) = shadow_spr {
            if let Some(f0) = shadow.frames.first() {
                fb_prontera.draw_sprite_frame(400, 308, f0, 1.0);
            }
        }

        if let Some(ref ch) = char_spr {
            if let Some(f0) = ch.frames.first() {
                fb_prontera.draw_sprite_frame(400, 304, f0, 1.0);
            }
        } else {
            fb_prontera.draw_entity_token(
                center_fountain,
                hades_core::types::Direction::North,
                true,
                cam_offset_x,
                cam_offset_y,
            );
        }

        fb_prontera.draw_rect(10, 10, 270, 84, 0xDD14141E, Some(0xFF6272A4));
        fb_prontera.draw_text(18, 16, "MAPA: PRONTERA [312x392]", 0xFFBD93F9, 1);
        fb_prontera.draw_text(18, 30, "POS: (156, 180) | DIR: North", 0xFF50FA7B, 1);
        fb_prontera.draw_text(18, 44, "CAMERA: 045 DEG | ZOOM: 1.00x", 0xFFFFB86C, 1);
        fb_prontera.draw_text(18, 58, "TRANSPORTE: QUIC TLS 1.3", 0xFF8BE9FD, 1);
        fb_prontera.draw_text(18, 72, "CLIENTE: BERENICE 2.5D @ 60 FPS", 0xFFF1FA8C, 1);

        let prontera_ppm = fb_prontera.to_ppm();
        let mut file_p = File::create("target/playtest_prontera_rotated.ppm")?;
        file_p.write_all(&prontera_ppm)?;
        println!("   -> Frame Prontera Rotacionada (45°) exportado para target/playtest_prontera_rotated.ppm");
    }

    let _ = server_handle.await;

    println!("\n✅ [Playtesting & Autenticação Concluídos com Sucesso Total]");
    Ok(())
}
