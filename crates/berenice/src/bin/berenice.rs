//! # Berenice Client - Executável Interativo a 60 FPS
//!
//! Aplicação nativa com janela gráfica, suporte a controles físicos (Xbox/PS5/Switch/USB),
//! teclado e mouse estilo Tree of Savior, carregamento de mapas reais (.gat via GRF)
//! e comunicação QUIC/WebTransport em tempo real com o servidor Hades.

use berenice::input::{find_best_attack_cell, DirectionalKeys, InputHub};
use berenice::network::BereniceNetwork;
use berenice::render::{
    AutoIdleTracker, AvatarRenderParams, BattleShoutTracker, ComboStage, ComboTracker, DodgeTracker,
    DummyState, FloatingNumberPool, GuideNpc, ParticleSystem, SoftwareFramebuffer, TrainingDummy, COLOR_BG,
    DEFAULT_PITCH_DEG,
};
use berenice::state::{PlayerStats, WorldView};
use berenice::ui::LoginScene;
use berenice::vfs::GrfArchive;
use gilrs::{Axis, Button, EventType, Gilrs};
use hades_core::collision::CollisionGrid;
use hades_core::types::{Direction, EntityId, Position};

use hades_ro_prere::act_parser::{parse_act, Act};
use hades_ro_prere::bmp_parser::{parse_bmp, BmpImage};
use hades_ro_prere::gat_parser::parse_gat;
use hades_ro_prere::gnd_parser::parse_gnd;
use hades_ro_prere::rsm_parser::{parse_rsm, RsmModel};
use hades_ro_prere::rsw_parser::parse_rsw;
use hades_ro_prere::spr_parser::{parse_spr, Sprite};
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

/// Duração padrão de travessia de 1 célula (passo) a pé (ms). Sincronizado 1:1 com animação .act.
const WALK_STEP_MS: u128 = 150;

#[derive(Clone)]
struct CharacterAnimationSet {
    body_act: Arc<Act>,
    body_spr: Arc<Sprite>,
    head_act: Option<Arc<Act>>,
    head_spr: Option<Arc<Sprite>>,
    headgear_act: Option<Arc<Act>>,
    headgear_spr: Option<Arc<Sprite>>,
    weapon_act: Option<Arc<Act>>,
    weapon_spr: Option<Arc<Sprite>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MouseCombatMode {
    Jab,
    Combo,
}

/// Textura procedural de solo com estilo de calçamento de pedra rústica para fallback.
fn create_fallback_ground_texture() -> BmpImage {
    let width = 32u32;
    let height = 32u32;
    let mut pixels = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            let is_edge = x == 0 || x == width - 1 || y == 0 || y == height - 1;
            let checker = ((x / 4) + (y / 4)) % 2 == 0;
            let noise = (x * 37 + y * 73) % 19;
            let base = if is_edge { 110 } else if checker { 140 + noise } else { 128 + noise };
            let r = base.min(255);
            let g = ((base as f32 * 0.95) as u32).min(255);
            let b = ((base as f32 * 0.88) as u32).min(255);
            pixels.push(0xFF000000 | (r << 16) | (g << 8) | b);
        }
    }
    BmpImage {
        width,
        height,
        pixels,
    }
}

const DEFAULT_WINDOW_WIDTH: usize = 800;
const DEFAULT_WINDOW_HEIGHT: usize = 600;

/// Níveis de zoom discretos e graduais para a câmera (17 níveis, 16 passos de scroll).
const ZOOM_LEVELS: [f32; 17] = [
    0.45, 0.52, 0.60, 0.68, 0.76, 0.84, 0.92, 1.00,
    1.10, 1.20, 1.32, 1.45, 1.60, 1.75, 1.90, 2.05, 2.20
];
const DEFAULT_ZOOM_INDEX: usize = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppScreen {
    Login,
    World,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🏛️⚡ [BERENICE] Iniciando Cliente 2.5D a 60 FPS...");

    // 1. Conectar ao Hades World Server (ou fallback offline)
    let _ = dotenvy::dotenv();
    let styx_config = hades_styx::StyxConfig::from_args_and_env();

    // SPEC-0027: Suporte a flags de linha de comando (--host <IP>, --verbose, --heartbeat)
    let args: Vec<String> = std::env::args().collect();
    let mut custom_host: Option<String> = None;
    let mut is_verbose = false;
    let mut enable_heartbeat = false;
    let mut i = 1;
    while i < args.len() {
        if (args[i] == "--host" || args[i] == "-h") && i + 1 < args.len() {
            custom_host = Some(args[i + 1].clone());
            i += 2;
        } else if args[i].starts_with("--host=") {
            custom_host = Some(args[i].trim_start_matches("--host=").to_string());
            i += 1;
        } else if args[i].starts_with("--world-server=") {
            custom_host = Some(args[i].trim_start_matches("--world-server=").to_string());
            i += 1;
        } else if args[i] == "--verbose" || args[i] == "-v" {
            is_verbose = true;
            i += 1;
        } else if args[i] == "--heartbeat" || args[i] == "-b" || args[i] == "--ping" {
            enable_heartbeat = true;
            i += 1;
        } else {
            i += 1;
        }
    }

    if is_verbose {
        println!(" 🔍 [Berenice] Modo Verbose ATIVADO: logando troca de pacotes de rede.");
    }
    if enable_heartbeat {
        println!(" 💓 [Berenice] Heartbeat ATIVADO: enviando pings periódicos a cada 1s.");
    }

    let world_addr_str = if let Some(ref h) = custom_host {
        if h.contains(':') {
            h.clone()
        } else {
            format!("{h}:4434")
        }
    } else {
        std::env::var("HADES_WORLD_SERVER")
            .or_else(|_| std::env::var("HADES_SERVER_HOST").map(|h| format!("{h}:4434")))
            .unwrap_or_else(|_| "127.0.0.1:4434".to_string())
    };

    let world_addr: std::net::SocketAddr = world_addr_str
        .parse()
        .unwrap_or_else(|_| "127.0.0.1:4434".parse().unwrap());

    let client_aid: u32 = std::env::var("HADES_CLIENT_AID")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let client_gid: u32 = std::env::var("HADES_CLIENT_GID")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let client_auth_code: i32 = std::env::var("HADES_CLIENT_AUTH_CODE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(42);
    let char_name = std::env::var("HADES_CLIENT_CHAR_NAME")
        .or_else(|_| std::env::var("HADES_DEV_CHAR_NAME"))
        .unwrap_or_else(|_| "Berenice".to_string());

    let live_rtt = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));

    println!(" 🌐 [Berenice] Conectando ao Hades World Server em {world_addr}...");
    let (cmd_tx, mut cmd_rx) =
        tokio::sync::mpsc::unbounded_channel::<hades_world::ClientWorldMsg>();
    let (server_tx, mut server_rx) =
        tokio::sync::mpsc::unbounded_channel::<hades_world::ServerWorldMsg>();

    let host_only = if let Some(ref h) = custom_host {
        h.split(':').next().unwrap_or("localhost").to_string()
    } else {
        std::env::var("HADES_SERVER_HOST")
            .or_else(|_| {
                std::env::var("HADES_WORLD_SERVER")
                    .map(|s| s.split(':').next().unwrap_or("localhost").to_string())
            })
            .unwrap_or_else(|_| "localhost".to_string())
    };

    let server_sni = std::env::var("HADES_SERVER_SNI")
        .unwrap_or(host_only);

    let client = match BereniceNetwork::connect_insecure(world_addr, &server_sni).await {
        Ok(network) => {
            println!(" 🏛️⚡ [Berenice] Conexão QUIC TLS 1.3 estabelecida com sucesso!");
            if let Ok(mut world_client) = network.open_world_client().await {
                // Handshake EnterWorld
                let _ = world_client
                    .send_msg(&hades_world::ClientWorldMsg::EnterWorld {
                        aid: client_aid,
                        gid: client_gid,
                        auth_code: client_auth_code,
                        name: Some(char_name.clone()),
                    })
                    .await;

                // Task em background para streams de rede
                let verbose_mode = is_verbose;
                tokio::spawn(async move {
                    loop {
                        tokio::select! {
                            Some(cmd) = cmd_rx.recv() => {
                                if verbose_mode {
                                    println!(" 📤 [NET-TX] Enviando para World Server: {cmd:?}");
                                }
                                if world_client.send_msg(&cmd).await.is_err() {
                                    if verbose_mode {
                                        println!(" ⚠️ [NET-TX] Conexão encerrada pelo servidor.");
                                    }
                                    break;
                                }
                            }
                            msg_res = world_client.read_msg() => {
                                match msg_res {
                                    Ok(Some(msg)) => {
                                        if verbose_mode {
                                            println!(" 📥 [NET-RX] Recebido do World Server: {msg:?}");
                                        }
                                        let _ = server_tx.send(msg);
                                    }
                                    Ok(None) => {
                                        if verbose_mode {
                                            println!(" 🔌 [NET-RX] Conexão QUIC finalizada pelo servidor.");
                                        }
                                        break;
                                    }
                                    Err(e) => {
                                        if verbose_mode {
                                            println!(" ⚠️ [NET-RX] Erro na stream QUIC: {e}");
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    }
                });

                // SPEC-0027: Monitoramento periódico de RTT (Ping em milissegundos)
                let rtt_updater = live_rtt.clone();
                let net_clone = network.clone();
                tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        let ms = net_clone.rtt_ms();
                        rtt_updater.store(ms, std::sync::atomic::Ordering::Relaxed);
                    }
                });

                // Task periódica de Heartbeat (--heartbeat ou --verbose)
                if enable_heartbeat || is_verbose {
                    let cmd_tx_heartbeat = cmd_tx.clone();
                    let net_heartbeat = network.clone();
                    let host_display = world_addr_str.clone();
                    tokio::spawn(async move {
                        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
                        interval.tick().await; // ignora tick imediato inicial
                        loop {
                            interval.tick().await;
                            let now_ms = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64;
                            let quic_rtt = net_heartbeat.rtt_ms();
                            println!(" 💓 [Heartbeat] Servidor: {host_display} | RTT QUIC: {quic_rtt} ms");
                            let _ = cmd_tx_heartbeat.send(hades_world::ClientWorldMsg::Ping {
                                timestamp: now_ms,
                            });
                        }
                    });
                }
            }
            Some(network)
        }
        Err(e) => {
            println!(" ⚠️ [Berenice] Hades World Server offline ({e}); iniciando em modo offline local.");
            None
        }
    };


    // 3. Inicializar VFS e carregar Mapa Real
    let grf_path = std::env::var("HADES_GRF_PATH").unwrap_or_else(|_| "data.grf".to_string());
    let map_name_env = std::env::var("HADES_MAP_NAME").unwrap_or_else(|_| "prontera.gat".to_string());
    let map_stem = map_name_env
        .trim_end_matches(".gat")
        .trim_end_matches(".rsw")
        .trim_end_matches(".gnd");
    let spawn_x: u16 = std::env::var("HADES_SPAWN_X")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(156);
    let spawn_y: u16 = std::env::var("HADES_SPAWN_Y")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(180);
    let default_spawn_pos = Position::new_unchecked(spawn_x, spawn_y);
    let map_rsw_path = format!("data/{map_stem}.rsw");
    let map_gnd_path = format!("data/{map_stem}.gnd");
    let map_gat_path = format!("data/{map_stem}.gat");

    let (world_grid, mut player_pos, shadow_spr, player_spr, char_anims, monster_anims, npc_anims, emotion_anims, fountain_model, pot_model, rsw_scene, rsm_cache, gnd_mesh, ground_textures) = if Path::new(&grf_path).exists() {
        println!(" [VFS] Carregando arquivo de assets: {}", grf_path);
        let grf = GrfArchive::open(&grf_path)?;
        println!(" [VFS] Total de arquivos indexados: {}", grf.file_count());

        let s = grf
            .extract("data/sprite/shadow.spr")
            .and_then(|b| parse_spr(&b).ok());
        let p = grf
            .extract("data/sprite/npc/4_m_fairysoldier2.spr")
            .and_then(|b| parse_spr(&b).ok());
        if s.is_some() && p.is_some() {
            println!(" [VFS] Sprites 2D de Sombra e Personagem carregados com sucesso!");
        }

        // SPEC-0019 & SPEC-0020: Carrega Sprites e Animações (.act) do Espadachim (Corpo + Cabeça + Elmo + Espada)
        let anims = if let (Some(b_act_bytes), Some(b_spr_bytes)) = (
            grf.extract("data/sprite/인간족/몸통/남/검사_남.act"),
            grf.extract("data/sprite/인간족/몸통/남/검사_남.spr"),
        ) {
            match (parse_act(&b_act_bytes), parse_spr(&b_spr_bytes)) {
                (Ok(body_act), Ok(body_spr)) => {
                    let head_act = grf
                        .extract("data/sprite/인간족/머리통/남/1_남.act")
                        .and_then(|b| parse_act(&b).ok())
                        .map(Arc::new);
                    let head_spr = grf
                        .extract("data/sprite/인간족/머리통/남/1_남.spr")
                        .and_then(|b| parse_spr(&b).ok())
                        .map(Arc::new);
                    let headgear_act = grf
                        .extract("data/sprite/악세사리/남/남_본헬름.act")
                        .and_then(|b| parse_act(&b).ok())
                        .map(Arc::new);
                    let headgear_spr = grf
                        .extract("data/sprite/악세사리/남/남_본헬름.spr")
                        .and_then(|b| parse_spr(&b).ok())
                        .map(Arc::new);
                    let weapon_act = grf
                        .extract("data/sprite/인간족/검사/검사_남_검.act")
                        .and_then(|b| parse_act(&b).ok())
                        .map(Arc::new);
                    let weapon_spr = grf
                        .extract("data/sprite/인간족/검사/검사_남_검.spr")
                        .and_then(|b| parse_spr(&b).ok())
                        .map(Arc::new);

                    println!(" [VFS] Sprites 2D e Ações .act do Espadachim (Corpo, Cabeça, Elmo e Espada) carregados com sucesso!");
                    Some(CharacterAnimationSet {
                        body_act: Arc::new(body_act),
                        body_spr: Arc::new(body_spr),
                        head_act,
                        head_spr,
                        headgear_act,
                        headgear_spr,
                        weapon_act,
                        weapon_spr,
                    })
                }
                _ => None,
            }
        } else {
            None
        };

        // SPEC-0022: Carrega Sprites e Ações (.act/.spr) do Monstro Gelatinoso de Treino (Slime)
        let monster_anims = if let (Some(m_act_bytes), Some(m_spr_bytes)) = (
            grf.extract("data/sprite/몬스터/poring.act"),
            grf.extract("data/sprite/몬스터/poring.spr"),
        ) {
            match (parse_act(&m_act_bytes), parse_spr(&m_spr_bytes)) {
                (Ok(act), Ok(spr)) => {
                    println!(" [VFS] Sprites 2D e Ações .act do Monstro de Treino (Slime) carregados com sucesso!");
                    Some((Arc::new(act), Arc::new(spr)))
                }
                _ => None,
            }
        } else {
            None
        };

        // SPEC-0028 / VALVE.md: Carrega Sprites e Ações (.act/.spr) da Guia NPC de Prontera
        let npc_anims = if let (Some(k_act_bytes), Some(k_spr_bytes)) = (
            grf.extract("data/sprite/npc/4_f_kafra1.act"),
            grf.extract("data/sprite/npc/4_f_kafra1.spr"),
        ) {
            match (parse_act(&k_act_bytes), parse_spr(&k_spr_bytes)) {
                (Ok(act), Ok(spr)) => {
                    println!(" [VFS] Sprites 2D e Ações .act da Guia de Prontera carregados com sucesso!");
                    Some((Arc::new(act), Arc::new(spr)))
                }
                _ => None,
            }
        } else {
            None
        };

        // SPEC-0026: Carrega Sprites e Ações (.act/.spr) dos Emoticons Nativos do GRF
        let emotion_anims = if let (Some(emo_act_bytes), Some(emo_spr_bytes)) = (
            grf.extract("data/sprite/이팩트/emotion.act"),
            grf.extract("data/sprite/이팩트/emotion.spr"),
        ) {
            match (parse_act(&emo_act_bytes), parse_spr(&emo_spr_bytes)) {
                (Ok(act), Ok(spr)) => {
                    println!(" [VFS] Sprites Nativos de Emoticons (.spr/.act) carregados com sucesso ({} ações)!", act.actions.len());
                    Some((Arc::new(act), Arc::new(spr)))
                }
                _ => None,
            }
        } else {
            None
        };

        let f = grf
            .extract("data/model/prontera/prt_k_bunsu_1.rsm")
            .and_then(|b| parse_rsm(&b).ok());
        let pot = grf
            .extract("data/model/prontera/flowerpot_01.rsm")
            .and_then(|b| parse_rsm(&b).ok());
        if f.is_some() {
            println!(" [VFS] Modelo 3D da Fonte Central de Prontera (.rsm) carregado com sucesso!");
        }

        let rsw = grf
            .extract(&map_rsw_path)
            .and_then(|b| parse_rsw(&b).ok());

        let mut cache: HashMap<String, Arc<RsmModel>> = HashMap::new();
        if let Some(ref scene) = rsw {
            println!(
                " [VFS] Cena {} carregada: {} objetos 3D encontrados!",
                map_rsw_path,
                scene.models.len()
            );
            let mut cached_count = 0;
            let center_x_f = spawn_x as f32;
            let center_y_f = (spawn_y as f32) + 16.0;
            for m in &scene.models {
                let wx = center_x_f + m.position[0];
                let wy = center_y_f + m.position[2];
                let dist = ((wx - center_x_f).powi(2) + (wy - (spawn_y as f32)).powi(2)).sqrt();
                if dist <= 40.0 && !cache.contains_key(&m.filename) {
                    if let Some(bytes) = grf.extract(&m.filename) {
                        if let Ok(model) = parse_rsm(&bytes) {
                            cache.insert(m.filename.clone(), Arc::new(model));
                            cached_count += 1;
                        }
                    }
                }
            }
            if cached_count > 0 {
                println!(
                    " [VFS] {} modelos 3D do cenário da praça central carregados em cache!",
                    cached_count
                );
            }
        }

        // SPEC-0028: Carregamento do Terreno e Texturização de Solo (.gnd + .bmp)
        let (gnd_mesh, ground_textures) = if let Some(gnd_bytes) = grf.extract(&map_gnd_path) {
            match parse_gnd(&gnd_bytes) {
                Ok(mesh) => {
                    let mut textures = HashMap::new();
                    for tex_name in &mesh.textures {
                        if let Some(bmp_bytes) = grf.extract(tex_name) {
                            if let Ok(bmp) = parse_bmp(&bmp_bytes) {
                                textures.insert(tex_name.clone(), bmp);
                            }
                        }
                    }
                    println!(
                        " [VFS] Terreno de {} (.gnd) carregado: {}x{} células, {} texturas decodificadas!",
                        map_gnd_path, mesh.width, mesh.height, textures.len()
                    );
                    (Some(mesh), textures)
                }
                Err(e) => {
                    println!(" ⚠️ [VFS] Falha ao parsear {}: {:?}", map_gnd_path, e);
                    (None, HashMap::new())
                }
            }
        } else {
            (None, HashMap::new())
        };

        if let Some(gat_bytes) = grf.extract(&map_gat_path) {
            println!(
                " [VFS] Extraído {} ({} bytes) com sucesso!",
                map_gat_path,
                gat_bytes.len()
            );
            let grid = parse_gat(&gat_bytes)?;
            println!(
                " [Mapa] {} carregado: {}x{} células!",
                map_gat_path, grid.width, grid.height
            );
            (grid, default_spawn_pos, s, p, anims, monster_anims, npc_anims, emotion_anims, f, pot, rsw, cache, gnd_mesh, ground_textures)
        } else {
            (
                CollisionGrid::new(64, 64, true),
                Position::new_unchecked(spawn_x.min(63), spawn_y.min(63)),
                s,
                p,
                anims,
                monster_anims,
                npc_anims,
                emotion_anims,
                f,
                pot,
                rsw,
                cache,
                gnd_mesh,
                ground_textures,
            )
        }
    } else {
        println!(" [VFS] data.grf não encontrado; utilizando arena padrão.");
        (
            CollisionGrid::new(64, 64, true),
            Position::new_unchecked(spawn_x.min(63), spawn_y.min(63)),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            HashMap::new(),
            None,
            HashMap::new(),
        )
    };

    let fallback_ground_texture = create_fallback_ground_texture();

    // 4. Inicializar subsistemas do cliente
    let auto_login = std::env::var("HADES_AUTO_LOGIN")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let mut current_screen = if auto_login { AppScreen::World } else { AppScreen::Login };
    let mut login_scene = LoginScene::from_env();
    let player_id = EntityId::new(client_aid as u16);
    let mut world_view = WorldView::new(player_id, player_pos);
    let mut input_hub = InputHub::new();
    let mut gilrs = Gilrs::new().ok();

    // SPEC-0018: Suporte ao Omarchy e Tiling Window Managers (Hyprland / Splits Verticais)
    let is_vertical = args.iter().any(|a| a == "--vertical" || a == "-v")
        || std::env::var("HADES_VERTICAL").map(|v| v == "1" || v == "true").unwrap_or(false)
        || std::env::var("HADES_OMARCHY").map(|v| v == "1" || v == "true").unwrap_or(false);

    let default_w = if is_vertical { 540 } else { DEFAULT_WINDOW_WIDTH };
    let default_h = if is_vertical { 960 } else { DEFAULT_WINDOW_HEIGHT };

    let mut window_width: usize = std::env::var("HADES_WINDOW_WIDTH")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default_w);
    let mut window_height: usize = std::env::var("HADES_WINDOW_HEIGHT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default_h);

    if is_vertical {
        println!(" 📱 [Omarchy/Tiling] Modo vertical ativado: {}x{}", window_width, window_height);
    }

    let mut fb = SoftwareFramebuffer::new(window_width, window_height);

    struct WindowInputReceiver {
        chars: std::sync::Arc<std::sync::Mutex<Vec<char>>>,
        keys_down: std::sync::Arc<std::sync::Mutex<[bool; 256]>>,
    }

    impl minifb::InputCallback for WindowInputReceiver {
        fn add_char(&mut self, uni_char: u32) {
            if let Some(c) = char::from_u32(uni_char) {
                if !c.is_control() {
                    if let Ok(mut q) = self.chars.lock() {
                        q.push(c);
                    }
                }
            }
        }

        fn set_key_state(&mut self, key: minifb::Key, state: bool) {
            let idx = key as usize;
            if idx < 256 {
                if let Ok(mut k) = self.keys_down.lock() {
                    k[idx] = state;
                }
            }
        }
    }

    // 5. Criar Janela Nativa 60 FPS com Redimensionamento Dinâmico (resize: true)
    let mut window = Window::new(
        "Berenice - Hades 2.5D Client",
        window_width,
        window_height,
        WindowOptions {
            resize: true,
            scale: minifb::Scale::X1,
            ..WindowOptions::default()
        },
    )?;
    window.set_target_fps(60);

    let char_queue = std::sync::Arc::new(std::sync::Mutex::new(Vec::<char>::new()));
    let keys_down = std::sync::Arc::new(std::sync::Mutex::new([false; 256]));
    window.set_input_callback(Box::new(WindowInputReceiver {
        chars: char_queue.clone(),
        keys_down: keys_down.clone(),
    }));

    let mut last_move_time = Instant::now();
    let anim_start_time = Instant::now();
    let mut combo_tracker = ComboTracker::new();
    let mut prev_attack_down = false;
    let mut prev_jab_down = false;
    let mut gamepad_trigger_held = false;
    let mut gamepad_west_held = false;
    let mut prev_mouse_down = false;
    let mut last_right_mouse_pos: Option<(f32, f32)> = None;
    let mut zoom_idx = DEFAULT_ZOOM_INDEX;
    let mut dummy_attack_target: Option<MouseCombatMode> = None;
    let mut combat_target_locked = false;

    // SPEC-0022 / SPEC-0024 / SPEC-0025 / SPEC-0026: Alvo, Partículas, Gritos Nórdicos e Auto-Idle
    let mut training_dummy = TrainingDummy::new(Position::new_unchecked(spawn_x.saturating_add(2), spawn_y));
    let mut floating_numbers = FloatingNumberPool::new();
    let mut particle_system = ParticleSystem::new();
    let mut shout_tracker = BattleShoutTracker::new();
    let mut auto_idle = AutoIdleTracker::new();
    let mut last_connected_strike_time = Instant::now() - std::time::Duration::from_secs(10);
    let mut last_shout_stage_time = Instant::now() - std::time::Duration::from_secs(10);
    let mut last_frame_time = Instant::now();
    let mut cam_center_x = player_pos.x as f32;
    let mut cam_center_y = player_pos.y as f32;
    let mut step_index: usize = 0;

    // SPEC-0028 / VALVE.md: Guia NPC Interativa de Prontera e Simulação do Mundo Vivo
    let mut guide_npc = GuideNpc::new(Position::new_unchecked(spawn_x.saturating_sub(6), spawn_y.saturating_sub(2)));
    let mut is_mouse_over_npc = false;
    let mut last_fountain_spray_time = Instant::now();

    // SPEC-0023: Manobra Evasiva (L2), Atributos/HP/SP do Jogador e Janelas
    let mut dodge_tracker = DodgeTracker::new();
    let mut player_stats = PlayerStats::new_swordsman(&char_name);
    let mut show_status_window = false;
    let mut show_inventory_window = false;
    let mut trigger_evasive_dodge = false;
    let mut prev_dodge_key_down = false;
    let mut prev_c_down = false;
    let mut prev_i_down = false;
    let mut prev_num1_down = false;
    let mut prev_num2_down = false;
    let mut show_debug_overlay = false;
    let mut prev_f3_down = false;

    println!("\n🎮 Janela aberta! Pronto para jogar com Teclado, Mouse ou Gamepad!");
    println!("   - Na tela de login: digite seu nome, use o Mouse para clicar ou Enter/A.");
    println!("   - No mapa: use Analógico Esquerdo ou WASD para andar!");
    println!("   - Ataque Normal (Combo): Gatilhos R2/L2 ou Barra de Espaço/Z (Segure ou aperte no ritmo)!");
    println!("   - Jab Fail-Safe: Botão Quadrado ou tecla X (Corte rápido seguro com buffer de timing)!");
    println!("   - Câmera: segure Botão Direito do Mouse (X: Rotação Yaw, Y: Ângulo Pitch 30°..150°)!");
    println!("   - Teclado Câmera: Q/E (Yaw horizontal), R/F (Pitch vertical 30°..150°).");
    println!("   - Zoom: Roda do Mouse (Scroll gradual em 17 níveis), PageUp/PageDown ou Analógico Direito Y.");
    println!("   - Reset da Câmera: Botão do Meio do Mouse, tecla Home ou clique no Analógico Direito (R3).\n");

    // 6. Loop Principal a 60 FPS
    while window.is_open() && !window.is_key_down(Key::Escape) {
        // SPEC-0018: Redimensionamento Dinâmico para Tiling Window Managers (Omarchy/Hyprland)
        let (cur_w, cur_h) = window.get_size();
        if cur_w >= 100 && cur_h >= 100 && (cur_w != window_width || cur_h != window_height) {
            window_width = cur_w;
            window_height = cur_h;
            fb.resize(window_width, window_height);
        }
        // --- DRENAGEM DE MENSAGENS DO HADES WORLD SERVER ---
        while let Ok(msg) = server_rx.try_recv() {
            match msg {
                hades_world::ServerWorldMsg::EnterWorldOk {
                    pos_x,
                    pos_y,
                    name,
                    ..
                } => {
                    println!(" 🏛️ [World] Entrada confirmada! Personagem: {name} em ({pos_x}, {pos_y})");
                    player_pos = Position::new_unchecked(pos_x, pos_y);
                    cam_center_x = pos_x as f32;
                    cam_center_y = pos_y as f32;
                }
                hades_world::ServerWorldMsg::PlayerMove { to_x, to_y, .. } => {
                    let server_pos = Position::new_unchecked(to_x, to_y);
                    let dist = player_pos.chebyshev_distance(server_pos);
                    // Client-Side Prediction: o servidor confirma passos passados com latência (~200ms).
                    // Só reconcilia a posição se a divergência for severa (> 3 tiles) por colisão ou teleporte.
                    if dist > 3 {
                        player_pos = server_pos;
                    }
                }
                hades_world::ServerWorldMsg::EntitySpawn {
                    id,
                    name,
                    pos_x,
                    pos_y,
                    dir,
                    ..
                } => {
                    println!(
                        " 👥 [AoI] Entidade avistada: {name} (ID {id}) em ({pos_x}, {pos_y})"
                    );
                    world_view.update_remote_entity(berenice::state::ObservedEntity {
                        entity_id: EntityId::new(id),
                        position: Position::new_unchecked(pos_x, pos_y),
                        facing: Direction::from_u8(dir).unwrap_or(Direction::South),
                        is_moving: false,
                    });
                }
                hades_world::ServerWorldMsg::EntityMove { id, to_x, to_y, .. } => {
                    world_view.update_remote_entity(berenice::state::ObservedEntity {
                        entity_id: EntityId::new(id),
                        position: Position::new_unchecked(to_x, to_y),
                        facing: Direction::South,
                        is_moving: true,
                    });
                }
                hades_world::ServerWorldMsg::EntityDespawn { id } => {
                    println!(" 💨 [AoI] Entidade saiu do alcance: ID {id}");
                    world_view.remove_remote_entity(EntityId::new(id));
                }
                hades_world::ServerWorldMsg::Pong { timestamp } => {
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    let latency = if timestamp > 0 && now_ms >= timestamp {
                        now_ms - timestamp
                    } else {
                        live_rtt.load(std::sync::atomic::Ordering::Relaxed)
                    };
                    live_rtt.store(latency, std::sync::atomic::Ordering::Relaxed);
                    if enable_heartbeat || is_verbose {
                        println!(" 🏓 [Pong] Resposta do Servidor em {latency} ms (Ping ponta-a-ponta)");
                    }
                }
                _ => {}
            }
        }

        // --- POLLING DE STATUS DA JANELA E MOUSE ---
        let is_window_active = window.is_active();
        if !is_window_active {
            // Em tiling WMs (ex: Omarchy/Hyprland), o foco segue o cursor.
            // Quando a janela perde foco, desativa imediatamente comandos de movimento
            // para prevenir que o personagem caminhe eternamente.
            if let Ok(mut k) = keys_down.lock() {
                *k = [false; 256];
            }
            input_hub.keyboard_keys = DirectionalKeys::default();
            input_hub.shift_pressed = false;
            input_hub.mouse_nav.is_dragging = false;
            last_right_mouse_pos = None;
            dummy_attack_target = None;
            combat_target_locked = false;
        }

        let now = Instant::now();
        let mouse_pos = window.get_mouse_pos(minifb::MouseMode::Discard);
        if mouse_pos.is_none() {
            last_right_mouse_pos = None;
            if input_hub.mouse_nav.is_dragging {
                input_hub.mouse_nav.on_mouse_up();
            }
        }
        let mouse_down = is_window_active && window.get_mouse_down(minifb::MouseButton::Left);
        let mouse_right_down = is_window_active && window.get_mouse_down(minifb::MouseButton::Right);
        let mouse_middle_down = is_window_active && window.get_mouse_down(minifb::MouseButton::Middle);
        let mouse_clicked = mouse_down && !prev_mouse_down;
        let mouse_released = !mouse_down && prev_mouse_down;
        prev_mouse_down = mouse_down;

        let mut is_mouse_over_dummy = false;
        let mut mouse_attack_jab = false;
        let mut mouse_attack_combo = false;

        if current_screen == AppScreen::Login && mouse_clicked {
            if let Some((mx, my)) = mouse_pos {
                if login_scene.handle_click(mx, my, window_width, window_height) {
                    if let Some(ref c) = client {
                        let _ = login_scene.submit(c).await;
                    }
                    current_screen = AppScreen::World;
                }
            }
        }

        // Controle de Câmera e Navegação por Mouse no Mundo
        if current_screen == AppScreen::World {
            let camera_x = (window_width / 2) as f32;
            let camera_y = (window_height / 2) as f32;
            let (player_sx, player_sy) = fb.projection.world_to_screen(
                cam_center_x,
                cam_center_y,
                0.0,
                0.0,
            );
            let cam_offset_x = camera_x - player_sx;
            let cam_offset_y = camera_y - player_sy;

            // SPEC-0025: Detecção de foco do mouse sobre o monstro para camada Mouse-Only / Touchpad
            let (dummy_sx, dummy_sy) = fb.projection.world_to_screen(
                training_dummy.position.x as f32 + 0.5,
                training_dummy.position.y as f32 + 0.5,
                cam_offset_x,
                cam_offset_y,
            );

            let hovered_tile = mouse_pos.and_then(|(mx, my)| {
                fb.projection.screen_to_world(mx, my, cam_offset_x, cam_offset_y)
            });

            is_mouse_over_dummy = if training_dummy.is_hittable() {
                if let Some((mx, my)) = mouse_pos {
                    let inside_hud = my >= (window_height.saturating_sub(48)) as f32
                        || (mx >= 10.0 && mx <= 320.0 && my >= 10.0 && my <= 135.0);
                    if inside_hud {
                        false
                    } else {
                        let dx = mx - dummy_sx;
                        let dy = my - (dummy_sy - 12.0 * fb.projection.zoom);
                        let dist = (dx * dx + dy * dy).sqrt();
                        let radius = 44.0 * fb.projection.zoom;
                        let tile_match = hovered_tile.map_or(false, |t| {
                            (t.x as i32 - training_dummy.position.x as i32).abs() <= 1
                                && (t.y as i32 - training_dummy.position.y as i32).abs() <= 1
                        });
                        dist <= radius || tile_match
                    }
                } else {
                    false
                }
            } else {
                false
            };

            // SPEC-0028: Detecção de foco do mouse sobre o Guia NPC de Prontera
            let (npc_sx, npc_sy) = fb.projection.world_to_screen(
                guide_npc.position.x as f32 + 0.5,
                guide_npc.position.y as f32 + 0.5,
                cam_offset_x,
                cam_offset_y,
            );
            is_mouse_over_npc = if let Some((mx, my)) = mouse_pos {
                let inside_hud = my >= (window_height.saturating_sub(48)) as f32
                    || (mx >= (window_width.saturating_sub(220)) as f32 && my <= 170.0);
                if inside_hud {
                    false
                } else {
                    let dx = mx - npc_sx;
                    let dy = my - (npc_sy - 16.0 * fb.projection.zoom);
                    let dist = (dx * dx + dy * dy).sqrt();
                    let radius = 34.0 * fb.projection.zoom;
                    let tile_match = hovered_tile.map_or(false, |t| {
                        (t.x as i32 - guide_npc.position.x as i32).abs() <= 1
                            && (t.y as i32 - guide_npc.position.y as i32).abs() <= 1
                    });
                    dist <= radius || tile_match
                }
            } else {
                false
            };

            let is_mouse_over_target = is_mouse_over_dummy && training_dummy.is_hittable();

            if is_mouse_over_npc {
                // Interação com o Guia NPC por clique do mouse (docs/VALVE.md)
                if mouse_clicked {
                    last_right_mouse_pos = None;
                    dummy_attack_target = None;
                    combat_target_locked = false;
                    input_hub.mouse_nav.interrupt();
                    guide_npc.interact(now);
                }
            } else if (is_mouse_over_target && (mouse_clicked || mouse_down || mouse_right_down))
                || (combat_target_locked && (mouse_down || mouse_right_down))
            {
                // SPEC-0025: Combate por mouse com aproximação automática e alcance de arma
                if mouse_right_down {
                    last_right_mouse_pos = None;
                    combat_target_locked = true;
                    dummy_attack_target = Some(MouseCombatMode::Combo);
                } else if mouse_down {
                    last_right_mouse_pos = None;
                    combat_target_locked = true;
                    dummy_attack_target = Some(MouseCombatMode::Jab);
                }
            } else {
                combat_target_locked = false;

                // Rotação da câmera pelo Botão Direito do Mouse no chão livre
                if mouse_right_down {
                    if let Some((mx, my)) = mouse_pos {
                        if let Some((last_x, last_y)) = last_right_mouse_pos {
                            let delta_x = mx - last_x;
                            let delta_y = my - last_y;
                            fb.projection.yaw += delta_x * 0.008;
                            // Arraste vertical ajusta o ângulo de ataque (pitch 30° a 150°)
                            let current_pitch = fb.projection.pitch_deg();
                            fb.projection.set_pitch_deg(current_pitch - delta_y * 0.25);
                        }
                        last_right_mouse_pos = Some((mx, my));
                    }
                } else {
                    last_right_mouse_pos = None;

                    // SPEC-0017: Seamless Mouse Navigation & Pathfinding por Clique / Arraste
                    if let Some((mx, my)) = mouse_pos {
                        let inside_hud = my >= (window_height.saturating_sub(48)) as f32
                            || (mx >= 10.0 && mx <= 320.0 && my >= 10.0 && my <= 135.0);

                        if !inside_hud {
                            if let Some(target_tile) = hovered_tile {
                                if mouse_clicked {
                                    dummy_attack_target = None;
                                    // Feedback tátil com a fonte ao clicar na água (docs/VALVE.md)
                                    let f_dx = (target_tile.x as i32 - 156).abs();
                                    let f_dy = (target_tile.y as i32 - 180).abs();
                                    if f_dx <= 2 && f_dy <= 2 {
                                        particle_system.spawn_water_splash(
                                            target_tile.x as f32 + 0.5,
                                            target_tile.y as f32 + 0.5,
                                            0.5,
                                        );
                                    }

                                    input_hub.mouse_nav.on_mouse_down(
                                        &world_grid,
                                        player_pos,
                                        target_tile,
                                    );
                                } else if mouse_down {
                                    dummy_attack_target = None;
                                    input_hub.mouse_nav.on_mouse_drag(
                                        &world_grid,
                                        player_pos,
                                        target_tile,
                                    );
                                }
                            }
                        }
                    }
                }
            }

            // SPEC-0025: Resolução de Alcance de Arma e Aproximação Automática para Ataque
            let weapon_range: u16 = 1; // Espadachim com espada/adaga: 1 célula adjacente

            if !training_dummy.is_hittable() {
                dummy_attack_target = None;
                combat_target_locked = false;
            }

            if let Some(mode) = dummy_attack_target {
                let dist = player_pos.chebyshev_distance(training_dummy.position);
                if dist <= weapon_range {
                    // Já atingiu a distância permitida para a arma: interrompe corrida e ataca
                    input_hub.mouse_nav.interrupt();

                    let p_dx = training_dummy.position.x as i32 - player_pos.x as i32;
                    let p_dy = training_dummy.position.y as i32 - player_pos.y as i32;
                    if let Some(dir) = Direction::from_delta(p_dx, p_dy) {
                        world_view.local_facing = dir;
                    }

                    let is_mouse_holding = match mode {
                        MouseCombatMode::Jab => mouse_down,
                        MouseCombatMode::Combo => mouse_right_down,
                    };

                    match mode {
                        MouseCombatMode::Jab => {
                            mouse_attack_jab = is_mouse_holding;
                            if mouse_clicked || !combo_tracker.is_attacking(now) {
                                combo_tracker.trigger_jab(now);
                            }
                        }
                        MouseCombatMode::Combo => {
                            mouse_attack_combo = is_mouse_holding;
                            if mouse_clicked || !combo_tracker.is_attacking(now) {
                                combo_tracker.trigger_attack(now);
                            }
                        }
                    }

                    if !is_mouse_holding && combo_tracker.is_attacking(now) {
                        dummy_attack_target = None;
                    }
                } else {
                    // Fora de alcance: NUNCA balança a arma nem ataca de longe.
                    // Navega até a primeira célula adjacente/válida para a arma.
                    mouse_attack_jab = false;
                    mouse_attack_combo = false;

                    let target_cell = find_best_attack_cell(
                        &world_grid,
                        player_pos,
                        training_dummy.position,
                        weapon_range,
                    )
                    .unwrap_or(training_dummy.position);

                    if input_hub.mouse_nav.target_pos != Some(target_cell) {
                        input_hub.mouse_nav.set_target(
                            &world_grid,
                            player_pos,
                            target_cell,
                        );
                        input_hub.mouse_nav.click_marker_pos = None;
                    }
                }
            }

            if mouse_released {
                combat_target_locked = false;
                input_hub.mouse_nav.on_mouse_up();
            }

            // Atualiza temporizador do anel de clique visual
            input_hub.mouse_nav.tick(1.0 / 60.0);

            // Zoom da câmera com a Roda do Mouse (Scroll Wheel gradual: 17 níveis discretos)
            if let Some((_scroll_x, scroll_y)) = window.get_scroll_wheel() {
                if scroll_y > 0.0 {
                    zoom_idx = (zoom_idx + 1).min(ZOOM_LEVELS.len() - 1);
                } else if scroll_y < 0.0 {
                    zoom_idx = zoom_idx.saturating_sub(1);
                }
            }

            // Reset da câmera com Botão do Meio (Scroll click)
            if mouse_middle_down {
                fb.projection.yaw = 0.0;
                fb.projection.set_pitch_deg(DEFAULT_PITCH_DEG);
                zoom_idx = DEFAULT_ZOOM_INDEX;
            }

            // SPEC-0026: Registro de atividade por mouse (cancela Auto-Idle Sit)
            if mouse_down || mouse_right_down || mouse_clicked || mouse_middle_down {
                if auto_idle.register_input(Instant::now()) {
                    shout_tracker.trigger_wakeup(Instant::now());
                }
            }
        }

        // --- POLING DE ENTRADA: GAMEPAD (gilrs) ---
        if let Some(ref mut g) = gilrs {
            while let Some(event) = g.next_event() {
                match event.event {
                    EventType::AxisChanged(Axis::LeftStickX, val, _) => {
                        input_hub.gamepad.left_stick_x = val;
                    }
                    EventType::AxisChanged(Axis::LeftStickY, val, _) => {
                        // Analógico para cima = +Y
                        input_hub.gamepad.left_stick_y = val;
                    }
                    EventType::AxisChanged(Axis::RightStickX, val, _) => {
                        input_hub.gamepad.right_stick_x = val;
                    }
                    EventType::AxisChanged(Axis::RightStickY, val, _) => {
                        input_hub.gamepad.right_stick_y = val;
                    }
                    EventType::ButtonPressed(Button::RightThumb, _) => {
                        input_hub.gamepad.btn_right_thumb = true;
                    }
                    EventType::ButtonReleased(Button::RightThumb, _) => {
                        input_hub.gamepad.btn_right_thumb = false;
                    }
                    EventType::ButtonPressed(Button::South, _) => {
                        input_hub.gamepad.btn_south = true;
                        if current_screen == AppScreen::Login {
                            if let Some(ref c) = client {
                                let _ = login_scene.submit(c).await;
                            }
                            current_screen = AppScreen::World;
                        }
                    }
                    EventType::ButtonReleased(Button::South, _) => {
                        input_hub.gamepad.btn_south = false;
                    }
                    // SPEC-0021: Gatilho R2 (ou R1) para Ataque Normal com Cadência Fixa / Combo
                    EventType::ButtonPressed(Button::RightTrigger2, _)
                    | EventType::ButtonPressed(Button::RightTrigger, _) => {
                        gamepad_trigger_held = true;
                        if current_screen == AppScreen::World {
                            combo_tracker.trigger_attack(Instant::now());
                        }
                    }
                    EventType::ButtonReleased(Button::RightTrigger2, _)
                    | EventType::ButtonReleased(Button::RightTrigger, _) => {
                        gamepad_trigger_held = false;
                    }
                    EventType::ButtonChanged(Button::RightTrigger2, val, _) => {
                        if val > 0.3 {
                            if !gamepad_trigger_held {
                                gamepad_trigger_held = true;
                                if current_screen == AppScreen::World {
                                    combo_tracker.trigger_attack(Instant::now());
                                }
                            }
                        } else if val < 0.15 {
                            gamepad_trigger_held = false;
                        }
                    }
                    // SPEC-0023: Gatilho L2 para Manobra Evasiva (Dodge Dash / Esquiva)
                    EventType::ButtonPressed(Button::LeftTrigger2, _)
                    | EventType::ButtonPressed(Button::LeftTrigger, _) => {
                        if current_screen == AppScreen::World {
                            trigger_evasive_dodge = true;
                        }
                    }
                    EventType::ButtonChanged(Button::LeftTrigger2, val, _) => {
                        if val > 0.4 && current_screen == AppScreen::World {
                            trigger_evasive_dodge = true;
                        }
                    }
                    // SPEC-0023: Botões Select / Start para alternar Janelas de Atributos e Inventário
                    EventType::ButtonPressed(Button::Select, _) => {
                        if current_screen == AppScreen::World {
                            show_status_window = !show_status_window;
                        }
                    }
                    EventType::ButtonPressed(Button::Start, _) => {
                        if current_screen == AppScreen::World {
                            show_inventory_window = !show_inventory_window;
                        }
                    }
                    // Quadrado (West): Fail-safe dedicado para Jabs rápidos com input buffer
                    EventType::ButtonPressed(Button::West, _) => {
                        gamepad_west_held = true;
                        if current_screen == AppScreen::World {
                            combo_tracker.trigger_jab(Instant::now());
                        }
                    }
                    EventType::ButtonReleased(Button::West, _) => {
                        gamepad_west_held = false;
                    }
                    EventType::ButtonPressed(Button::DPadDown, _)
                        if current_screen == AppScreen::Login =>
                    {
                        login_scene.next_field();
                    }
                    EventType::ButtonPressed(Button::DPadUp, _)
                        if current_screen == AppScreen::Login =>
                    {
                        login_scene.prev_field();
                    }
                    _ => {}
                }

                // SPEC-0026: Cancela descanso se houver atividade no Gamepad
                if current_screen == AppScreen::World {
                    if auto_idle.register_input(Instant::now()) {
                        shout_tracker.trigger_wakeup(Instant::now());
                    }
                }
            }
        }

        // Câmera pelo Analógico Direito do Gamepad
        if current_screen == AppScreen::World {
            let rx = input_hub.gamepad.right_stick_x;
            let ry = input_hub.gamepad.right_stick_y;
            let deadzone = 0.15;
            if rx.abs() > deadzone {
                fb.projection.yaw += rx * 0.04;
            }
            if ry.abs() > deadzone {
                let current_pitch = fb.projection.pitch_deg();
                fb.projection.set_pitch_deg(current_pitch - ry * 0.8);
            }
            if input_hub.gamepad.btn_right_thumb {
                fb.projection.yaw = 0.0;
                fb.projection.set_pitch_deg(DEFAULT_PITCH_DEG);
                zoom_idx = DEFAULT_ZOOM_INDEX;
            }
        }

        // --- ENTRADA DE TECLADO: TELA DE LOGIN ---
        if current_screen == AppScreen::Login && is_window_active {
            // 1. Processa caracteres recebidos do sistema operacional (Wayland/X11 UTF-32)
            if let Ok(mut q) = char_queue.lock() {
                for c in q.drain(..) {
                    login_scene.handle_char(c);
                }
            }

            // 2. Teclas de controle e atalhos
            for key in window.get_keys_pressed(KeyRepeat::Yes) {
                match key {
                    Key::Tab => login_scene.next_field(),
                    Key::Enter => {
                        if let Some(ref c) = client {
                            let _ = login_scene.submit(c).await;
                        }
                        current_screen = AppScreen::World;
                    }
                    Key::Backspace => login_scene.handle_backspace(),
                    Key::Up => login_scene.prev_field(),
                    Key::Down => login_scene.next_field(),
                    _ => {}
                }
            }
        }

        // --- ENTRADA DE TECLADO: MUNDO (WASD + Atalhos de Câmera) ---
        if current_screen == AppScreen::World {
            let mut dir_keys = DirectionalKeys::default();
            if is_window_active {
                if let Ok(k) = keys_down.lock() {
                    let is_down = |key: Key| {
                        let idx = key as usize;
                        idx < 256 && k[idx]
                    };

                    if is_down(Key::W) || is_down(Key::Up) {
                        dir_keys.up = true;
                    }
                    if is_down(Key::S) || is_down(Key::Down) {
                        dir_keys.down = true;
                    }
                    if is_down(Key::A) || is_down(Key::Left) {
                        dir_keys.left = true;
                    }
                    if is_down(Key::D) || is_down(Key::Right) {
                        dir_keys.right = true;
                    }
                    input_hub.shift_pressed =
                        is_down(Key::LeftShift) || is_down(Key::RightShift);

                    // Ataque por Barra de Espaço ou tecla Z (Golpe de espada encadeado com cadência)
                    let attack_key_down = is_down(Key::Space) || is_down(Key::Z);
                    if attack_key_down && !prev_attack_down {
                        let npc_dx = (player_pos.x as i32 - guide_npc.position.x as i32).abs();
                        let npc_dy = (player_pos.y as i32 - guide_npc.position.y as i32).abs();
                        if is_down(Key::Space) && npc_dx <= 2 && npc_dy <= 2 && !combo_tracker.in_combat_stance(now) {
                            guide_npc.interact(now);
                        } else {
                            combo_tracker.trigger_attack(now);
                        }
                    }
                    prev_attack_down = attack_key_down;

                    // Jab fail-safe por tecla X (Corte rápido seguro com buffer)
                    let jab_key_down = is_down(Key::X);
                    if jab_key_down && !prev_jab_down {
                        combo_tracker.trigger_jab(Instant::now());
                    }
                    prev_jab_down = jab_key_down;

                    // SPEC-0023: Manobra Evasiva por tecla V ou Shift
                    let dodge_key_down = is_down(Key::V) || is_down(Key::LeftShift);
                    if dodge_key_down && !prev_dodge_key_down {
                        trigger_evasive_dodge = true;
                    }
                    prev_dodge_key_down = dodge_key_down;

                    // SPEC-0023: Janela de Atributos por tecla C
                    let c_down = is_down(Key::C);
                    if c_down && !prev_c_down {
                        show_status_window = !show_status_window;
                    }
                    prev_c_down = c_down;

                    // SPEC-0023: Janela de Inventário por tecla I
                    let i_down = is_down(Key::I);
                    if i_down && !prev_i_down {
                        show_inventory_window = !show_inventory_window;
                    }
                    prev_i_down = i_down;

                    // SPEC-0023: Atalhos de Uso de Itens (1: Poção Vermelha, 2: Poção Laranja, 3: Poção Azul)
                    let num1_down = is_down(Key::Key1);
                    if num1_down && !prev_num1_down {
                        player_stats.use_item(0);
                    }
                    prev_num1_down = num1_down;

                    let num2_down = is_down(Key::Key2);
                    if num2_down && !prev_num2_down {
                        player_stats.use_item(1);
                    }
                    prev_num2_down = num2_down;

                    // Alterna Painel de Informações Técnicas / Métricas de Desenvolvedor
                    let f3_down = is_down(Key::F3);
                    if f3_down && !prev_f3_down {
                        show_debug_overlay = !show_debug_overlay;
                    }
                    prev_f3_down = f3_down;

                    // Rotação de câmera por teclas Q / E
                    if is_down(Key::Q) {
                        fb.projection.yaw -= 0.035;
                    }
                    if is_down(Key::E) {
                        fb.projection.yaw += 0.035;
                    }
                    // Ângulo de ataque (pitch) vertical por teclas R / F (30° a 150°)
                    if is_down(Key::R) {
                        let current_pitch = fb.projection.pitch_deg();
                        fb.projection.set_pitch_deg(current_pitch + 0.75);
                    }
                    if is_down(Key::F) {
                        let current_pitch = fb.projection.pitch_deg();
                        fb.projection.set_pitch_deg(current_pitch - 0.75);
                    }

                    // SPEC-0026: Registro de atividade de teclado no mundo
                    if dir_keys.up || dir_keys.down || dir_keys.left || dir_keys.right
                        || is_down(Key::Space) || is_down(Key::Z) || is_down(Key::X)
                        || is_down(Key::V) || is_down(Key::LeftShift) || is_down(Key::C)
                        || is_down(Key::I) || is_down(Key::Key1) || is_down(Key::Key2)
                        || is_down(Key::Q) || is_down(Key::E) || is_down(Key::R) || is_down(Key::F)
                    {
                        if auto_idle.register_input(Instant::now()) {
                            shout_tracker.trigger_wakeup(Instant::now());
                        }
                    }
                }

                // Zoom gradual por PageUp / PageDown / + / -
                for key in window.get_keys_pressed(KeyRepeat::Yes) {
                    match key {
                        Key::PageUp | Key::Equal => {
                            zoom_idx = (zoom_idx + 1).min(ZOOM_LEVELS.len() - 1);
                        }
                        Key::PageDown | Key::Minus => {
                            zoom_idx = zoom_idx.saturating_sub(1);
                        }
                        Key::Home => {
                            fb.projection.yaw = 0.0;
                            fb.projection.set_pitch_deg(DEFAULT_PITCH_DEG);
                            zoom_idx = DEFAULT_ZOOM_INDEX;
                        }
                        _ => {}
                    }
                }
            } else {
                input_hub.shift_pressed = false;
            }
            input_hub.keyboard_keys = dir_keys;
        }

        // Interpolação suave em direção ao nível de zoom alvo (17 níveis graduais)
        let target_zoom = ZOOM_LEVELS[zoom_idx];
        fb.projection.zoom += (target_zoom - fb.projection.zoom) * 0.25;
        if (fb.projection.zoom - target_zoom).abs() < 0.001 {
            fb.projection.zoom = target_zoom;
        }

        // Normaliza o ângulo de yaw da câmera para [0..2PI)
        fb.projection.yaw = fb.projection.yaw.rem_euclid(std::f32::consts::TAU);

        // --- ATUALIZAÇÃO DA SIMULAÇÃO (WORLD SCREEN) ---
        if current_screen == AppScreen::World {
            let dt = last_frame_time.elapsed().as_secs_f32().min(0.1);
            last_frame_time = now;
            particle_system.update(dt);

            // Suavização da câmera (lerp exponencial) para amortecer transição de células e latência WAN
            let lerp_factor = (14.0 * dt).min(1.0);
            cam_center_x += (player_pos.x as f32 - cam_center_x) * lerp_factor;
            cam_center_y += (player_pos.y as f32 - cam_center_y) * lerp_factor;

            // SPEC-0021: Processa hold-to-repeat e drena input buffer a cada frame
            let is_holding_combo = prev_attack_down || gamepad_trigger_held || mouse_attack_combo;
            let is_holding_jab = prev_jab_down || gamepad_west_held || mouse_attack_jab;
            combo_tracker.tick(now, is_holding_combo, is_holding_jab);
            shout_tracker.tick(now);

            // SPEC-0026: Ciclo de descanso automático e emoticons relaxados periódicos (8s a 18s)
            if auto_idle.tick(now).is_some() && emotion_anims.is_none() {
                shout_tracker.trigger_idle_emoticon(now);
            }

            // SPEC-0025 & SPEC-0026: Gritos de Batalha Nórdicos acionados com proc balanceado
            if combo_tracker.is_attacking(now) && combo_tracker.stage_start_time != last_shout_stage_time {
                last_shout_stage_time = combo_tracker.stage_start_time;
                if combo_tracker.is_jab_mode {
                    shout_tracker.trigger_jab(now);
                } else {
                    match combo_tracker.current_stage {
                        Some(ComboStage::Hit1) => shout_tracker.trigger_hit1(now),
                        Some(ComboStage::Hit2) => shout_tracker.trigger_hit2(now),
                        Some(ComboStage::Hit3) => shout_tracker.trigger_hit3(now),
                        None => {}
                    }
                }
            }

            // SPEC-0023: Manobra Evasiva e Regeneração Periódica de HP/SP
            dodge_tracker.tick(now);
            player_stats.tick_regen(now);

            // SPEC-0022: Atualiza ciclo de vida e respawn do Monstro de Treino
            training_dummy.tick(now);

            // O monstro sempre direciona o olhar em direção ao jogador enquanto salta
            let m_dx = player_pos.x as i32 - training_dummy.position.x as i32;
            let m_dy = player_pos.y as i32 - training_dummy.position.y as i32;
            training_dummy.facing = match (m_dx.signum(), m_dy.signum()) {
                (0, 1) => Direction::North,
                (1, 1) => Direction::NorthEast,
                (1, 0) => Direction::East,
                (1, -1) => Direction::SouthEast,
                (0, -1) => Direction::South,
                (-1, -1) => Direction::SouthWest,
                (-1, 0) => Direction::West,
                (-1, 1) => Direction::NorthWest,
                _ => Direction::South,
            };

            // SPEC-0028 / VALVE.md: Atualiza o Guia NPC de Prontera e Simulação do Mundo Vivo
            guide_npc.tick(player_pos, now);

            if now.duration_since(last_fountain_spray_time).as_millis() >= 140 {
                last_fountain_spray_time = now;
                // Borrifo suave da água da fonte central
                particle_system.spawn_fountain_spray(156.0, 180.0, 1.3, 2);
                // Poeira de luz / motes sutis flutuando no ar
                particle_system.spawn_ambient_mote(player_pos.x as f32, player_pos.y as f32, 0.4);
            }

            // SPEC-0022: Resolução de impacto corpo-a-corpo e feedback de golpe
            if combo_tracker.is_attacking(now)
                && combo_tracker.stage_start_time != last_connected_strike_time
                && training_dummy.is_hittable()
            {
                let dx = training_dummy.position.x as i32 - player_pos.x as i32;
                let dy = training_dummy.position.y as i32 - player_pos.y as i32;
                let dist_sq = dx * dx + dy * dy;

                // Alcance corpo-a-corpo clássico: até 2 células de distância
                if dist_sq <= 5 {
                    last_connected_strike_time = combo_tracker.stage_start_time;

                    let (dmg, color, is_crit) = if combo_tracker.is_jab_mode {
                        (10, 0xFF8BE9FD, false)
                    } else {
                        match combo_tracker.current_stage {
                            Some(ComboStage::Hit1) => (12, 0xFFF1FA8C, false),
                            Some(ComboStage::Hit2) => (15, 0xFFFFB86C, false),
                            Some(ComboStage::Hit3) => (24, 0xFFFF5555, true),
                            None => (10, 0xFFF1FA8C, false),
                        }
                    };

                    let (was_fatal, _) = training_dummy.take_hit(dmg, now);
                    floating_numbers.spawn(
                        training_dummy.position.x as f32 + 0.5,
                        training_dummy.position.y as f32 + 0.5,
                        dmg,
                        color,
                        is_crit,
                        now,
                    );

                    let impact_x = training_dummy.position.x as f32 + 0.5;
                    let impact_y = training_dummy.position.y as f32 + 0.5;

                    // SPEC-0024: Emissão de partículas em tempo real
                    if was_fatal {
                        dummy_attack_target = None;
                        combat_target_locked = false;
                        particle_system.spawn_slime_death_burst(impact_x, impact_y, 0.4);
                    } else if combo_tracker.is_jab_mode {
                        particle_system.spawn_jab_sparks(impact_x, impact_y, 0.5, 14);
                    } else if is_crit {
                        particle_system.spawn_critical_burst(impact_x, impact_y, 0.5);
                    } else {
                        let slash_angle = (dy as f32).atan2(dx as f32);
                        particle_system.spawn_slash_sparks(impact_x, impact_y, 0.5, slash_angle, 16);
                    }

                    // Finalizador pesado (Hit 3): Recuo / Knockback sutil de 1 célula
                    if is_crit {
                        let push_x = (training_dummy.position.x as i32 + dx.signum())
                            .clamp(0, world_grid.width as i32 - 1) as u16;
                        let push_y = (training_dummy.position.y as i32 + dy.signum())
                            .clamp(0, world_grid.height as i32 - 1) as u16;
                        let push_pos = Position::new_unchecked(push_x, push_y);
                        if world_grid.is_walkable(push_pos) {
                            training_dummy.position = push_pos;
                        }
                    }
                }
            }

            // Movimento relativo ao ângulo atual da câmera
            let move_intent = input_hub.poll_movement_relative(fb.projection.yaw);

            // SPEC-0023: Execução da Manobra Evasiva (L2 / Esquiva / Dash de 2 células)
            if trigger_evasive_dodge {
                trigger_evasive_dodge = false;
                if dodge_tracker.trigger_dodge(now) {
                    player_stats.consume_sp(4);
                    dummy_attack_target = None;
                    combat_target_locked = false;
                    input_hub.mouse_nav.interrupt();

                    let dodge_dir = if move_intent.is_moving {
                        move_intent.direction
                    } else {
                        world_view.local_facing
                    };

                    let (dx, dy) = match dodge_dir {
                        Direction::North => (0i32, 1i32),
                        Direction::NorthEast => (1, 1),
                        Direction::East => (1, 0),
                        Direction::SouthEast => (1, -1),
                        Direction::South => (0, -1),
                        Direction::SouthWest => (-1, -1),
                        Direction::West => (-1, 0),
                        Direction::NorthWest => (-1, 1),
                    };

                    let prev_player_pos = player_pos;
                    // Deslocamento de até 2 células com checagem de colisão
                    let s1_x = (player_pos.x as i32 + dx).clamp(0, world_grid.width as i32 - 1) as u16;
                    let s1_y = (player_pos.y as i32 + dy).clamp(0, world_grid.height as i32 - 1) as u16;
                    let p1 = Position::new_unchecked(s1_x, s1_y);

                    if world_grid.is_walkable(p1) {
                        let s2_x = (player_pos.x as i32 + 2 * dx).clamp(0, world_grid.width as i32 - 1) as u16;
                        let s2_y = (player_pos.y as i32 + 2 * dy).clamp(0, world_grid.height as i32 - 1) as u16;
                        let p2 = Position::new_unchecked(s2_x, s2_y);

                        if world_grid.is_walkable(p2) {
                            player_pos = p2;
                            step_index = step_index.wrapping_add(2);
                        } else {
                            player_pos = p1;
                            step_index = step_index.wrapping_add(1);
                        }
                        world_view.local_facing = dodge_dir;
                        last_move_time = now;

                        particle_system.spawn_dodge_dust(
                            prev_player_pos.x as f32 + 0.5,
                            prev_player_pos.y as f32 + 0.5,
                            0.1,
                        );
                        shout_tracker.trigger_dodge(now);

                        let _ = cmd_tx.send(hades_world::ClientWorldMsg::MoveRequest {
                            to_x: player_pos.x,
                            to_y: player_pos.y,
                        });
                        if let Some(ref c) = client {
                            if let Some(delta) = world_view.create_movement_delta(
                                player_pos,
                                dodge_dir,
                                true,
                            ) {
                                let _ = c.send_movement(&delta);
                            }
                        }
                    }
                }
            }
            if move_intent.is_moving {
                // Interrompe imediatamente a navegação por mouse e combate se o jogador usar WASD ou analógico
                dummy_attack_target = None;
                combat_target_locked = false;
                input_hub.mouse_nav.interrupt();

                if last_move_time.elapsed().as_millis() >= WALK_STEP_MS {
                    last_move_time = Instant::now();
                    world_view.local_facing = move_intent.direction;

                    // Calcula próxima posição baseada na direção 8-way (Norte = +Y, Sul = -Y)
                    let (dx, dy) = match move_intent.direction {
                        Direction::North => (0i32, 1i32),
                        Direction::NorthEast => (1, 1),
                        Direction::East => (1, 0),
                        Direction::SouthEast => (1, -1),
                        Direction::South => (0, -1),
                        Direction::SouthWest => (-1, -1),
                        Direction::West => (-1, 0),
                        Direction::NorthWest => (-1, 1),
                    };

                    let target_x =
                        (player_pos.x as i32 + dx).clamp(0, world_grid.width as i32 - 1) as u16;
                    let target_y =
                        (player_pos.y as i32 + dy).clamp(0, world_grid.height as i32 - 1) as u16;
                    let target_pos = Position::new_unchecked(target_x, target_y);

                    // Verificação de colisão pura no CollisionGrid
                    if world_grid.is_walkable(target_pos) {
                        if player_pos != target_pos {
                            particle_system.spawn_footstep_dust(
                                player_pos.x as f32 + 0.5,
                                player_pos.y as f32 + 0.5,
                            );
                        }
                        player_pos = target_pos;
                        step_index = step_index.wrapping_add(1);
                        let _ = cmd_tx.send(hades_world::ClientWorldMsg::MoveRequest {
                            to_x: target_pos.x,
                            to_y: target_pos.y,
                        });
                        if let Some(ref c) = client {
                            if let Some(delta) = world_view.create_movement_delta(
                                player_pos,
                                move_intent.direction,
                                move_intent.running,
                            ) {
                                let _ = c.send_movement(&delta);
                            }
                        }
                    }
                }
            } else if input_hub.mouse_nav.is_active {
                // SPEC-0017: Condução contínua e passo a passo guiada pelo mouse pathfinding
                if last_move_time.elapsed().as_millis() >= WALK_STEP_MS {
                    last_move_time = Instant::now();
                    if let Some(next_pos) = input_hub.mouse_nav.advance_step(player_pos) {
                        let dx = next_pos.x as i32 - player_pos.x as i32;
                        let dy = next_pos.y as i32 - player_pos.y as i32;
                        if let Some(dir) = Direction::from_delta(dx, dy) {
                            world_view.local_facing = dir;
                        }

                        if world_grid.is_walkable(next_pos) {
                            if player_pos != next_pos {
                                particle_system.spawn_footstep_dust(
                                    player_pos.x as f32 + 0.5,
                                    player_pos.y as f32 + 0.5,
                                );
                            }
                            player_pos = next_pos;
                            step_index = step_index.wrapping_add(1);
                            let _ = cmd_tx.send(hades_world::ClientWorldMsg::MoveRequest {
                                to_x: next_pos.x,
                                to_y: next_pos.y,
                            });
                            if let Some(ref c) = client {
                                if let Some(delta) = world_view.create_movement_delta(
                                    player_pos,
                                    world_view.local_facing,
                                    false,
                                ) {
                                    let _ = c.send_movement(&delta);
                                }
                            }
                        } else {
                            // Se encontrar obstáculo imprevisto no caminho, interrompe o trajeto
                            input_hub.mouse_nav.interrupt();
                        }
                    }
                }
            }
        }

        // --- RENDERIZAÇÃO NO FRAMEBUFFER ---
        fb.clear(COLOR_BG);

        let camera_x = (window_width / 2) as f32;
        let camera_y = (window_height / 2) as f32;

        match current_screen {
            AppScreen::Login => {
                // Renderiza fundo texturizado e diálogo interativo com hover de mouse
                fb.render_textured_grid_view(
                    &world_grid,
                    player_pos,
                    12,
                    camera_x,
                    camera_y,
                    gnd_mesh.as_ref(),
                    &ground_textures,
                    &fallback_ground_texture,
                );
                login_scene.render_with_mouse(&mut fb, mouse_pos);
            }
            AppScreen::World => {
                // Centraliza a câmera diretamente no centro visual suave do jogador
                let (player_sx, player_sy) = fb.projection.world_to_screen(
                    cam_center_x,
                    cam_center_y,
                    0.0,
                    0.0,
                );
                let cam_offset_x = camera_x - player_sx;
                let cam_offset_y = camera_y - player_sy;

                // SPEC-0018: Raio de renderização dinâmico baseado na maior dimensão da janela
                let max_dim = (window_width.max(window_height) as f32).max(600.0);
                let view_radius = (((max_dim / 36.0) / fb.projection.zoom).ceil() as u16).clamp(16, 60);

                // SPEC-0028: Renderiza o mapa de terreno texturizado real ao redor do jogador
                fb.render_textured_grid_view(
                    &world_grid,
                    player_pos,
                    view_radius,
                    cam_offset_x,
                    cam_offset_y,
                    gnd_mesh.as_ref(),
                    &ground_textures,
                    &fallback_ground_texture,
                );

                // Marcador visual no chão de clique de destino (SPEC-0017)
                if let Some(marker_pos) = input_hub.mouse_nav.click_marker_pos {
                    let timer_norm = input_hub.mouse_nav.click_marker_timer
                        / berenice::input::MouseNavigation::MARKER_DURATION;
                    fb.draw_ground_target_marker(marker_pos, timer_norm, cam_offset_x, cam_offset_y);
                }

                // Renderiza Prédios e Modelos 3D do Cenário RSW
                let mut rendered_models = 0;
                if let Some(ref scene) = rsw_scene {
                    let radius = (view_radius as f32) + 8.0;
                    for obj in &scene.models {
                        let wx = 156.0 + obj.position[0];
                        let wy = 196.0 + obj.position[2];
                        if (wx - player_pos.x as f32).abs() <= radius
                            && (wy - player_pos.y as f32).abs() <= radius
                        {
                            if let Some(model) = rsm_cache.get(&obj.filename) {
                                let scale = (obj.scale[0] * 0.45).clamp(0.12, 1.2);
                                fb.draw_rsm_model(
                                    model,
                                    wx,
                                    wy,
                                    cam_offset_x,
                                    cam_offset_y,
                                    scale,
                                );
                                rendered_models += 1;
                            }
                        }
                    }
                }

                // Renderiza Fonte Central e Vasos de Flores
                if let Some(ref fountain) = fountain_model {
                    fb.draw_rsm_model(
                        fountain,
                        156.0,
                        180.0,
                        cam_offset_x,
                        cam_offset_y,
                        0.22,
                    );
                }
                if rendered_models == 0 {
                    if let Some(ref pot) = pot_model {
                        fb.draw_rsm_model(pot, 149.0, 172.0, cam_offset_x, cam_offset_y, 0.40);
                        fb.draw_rsm_model(pot, 163.0, 172.0, cam_offset_x, cam_offset_y, 0.40);
                    }
                }

                // SPEC-0019, SPEC-0020 & SPEC-0021: Animação e Renderização 2D Multi-Camada com Combos Rítmicos
                let now = Instant::now();
                let is_attacking = combo_tracker.is_attacking(now);
                let in_combat_stance = combo_tracker.in_combat_stance(now);
                let is_walking = !is_attacking && (last_move_time.elapsed().as_millis() <= (WALK_STEP_MS + 40));
                let anim_elapsed_ms = if is_attacking {
                    combo_tracker.anim_elapsed_ms(now)
                } else {
                    anim_start_time.elapsed().as_millis() as f32
                };

                let anchor_x = camera_x as i32;
                let anchor_y = (camera_y + 8.0 * fb.projection.zoom) as i32;

                if let Some(ref anims) = char_anims {
                    let visual_facing = fb.projection.world_facing_to_camera(world_view.local_facing);
                    let act_dir = visual_facing.to_act_dir();
                    let action_idx = if is_attacking {
                        80 + act_dir // Grupo 10: Golpe de Espada real (9 quadros)
                    } else if is_walking {
                        8 + act_dir // Grupo 01: Caminhada (8 quadros)
                    } else if in_combat_stance {
                        32 + act_dir // Grupo 04: Postura de Combate / Guarda (6 quadros)
                    } else if auto_idle.is_sitting {
                        16 + act_dir // SPEC-0026 Grupo 02: Sentado / Descanso Automático (Auto-Idle Sit)
                    } else {
                        act_dir // Grupo 00: Descanso / Idle Relaxado em Pé (3 quadros)
                    };

                    // Sincronização oficial de cadência (roBrowser / .act delay_ms = 75ms):
                    // Grupo 01 possui 8 quadros a 75ms/quadro = ciclo completo de 600ms.
                    // A 150ms/célula, um ciclo de 600ms cobre exatamente 4 células (2 células por passada).
                    // Portanto, cada célula percorrida avança exatamente 2 quadros de animação (150ms / 75ms = 2 quadros/célula).
                    let walk_frame_override = if is_walking {
                        let step_elapsed = last_move_time.elapsed().as_millis() as f32;
                        let step_t = (step_elapsed / WALK_STEP_MS as f32).clamp(0.0, 0.999);
                        let sub_frame = (step_t * 2.0) as usize; // 0 ou 1
                        let cell_in_cycle = step_index % 4;
                        Some((cell_in_cycle * 2 + sub_frame).min(7))
                    } else {
                        None
                    };

                    let body_frame = if is_attacking || in_combat_stance {
                        anims.body_act.frame_by_time(action_idx, anim_elapsed_ms).map(|(f, _)| f)
                    } else if let Some(f_idx) = walk_frame_override {
                        anims.body_act.frame(action_idx, f_idx)
                    } else {
                        anims.body_act.frame(action_idx, 0)
                    };

                    let head_frame = if let Some(ref h_act) = anims.head_act {
                        if is_attacking || in_combat_stance {
                            h_act.frame_by_time(action_idx, anim_elapsed_ms).map(|(f, _)| f)
                        } else if let Some(f_idx) = walk_frame_override {
                            h_act.frame(action_idx, f_idx)
                        } else {
                            h_act.frame(action_idx, 0)
                        }
                    } else {
                        None
                    };

                    let headgear_frame = if let Some(ref hg_act) = anims.headgear_act {
                        if is_attacking || in_combat_stance {
                            hg_act.frame_by_time(action_idx, anim_elapsed_ms).map(|(f, _)| f)
                        } else if let Some(f_idx) = walk_frame_override {
                            hg_act.frame(action_idx, f_idx)
                        } else {
                            hg_act.frame(action_idx, 0)
                        }
                    } else {
                        None
                    };

                    let weapon_frame = if let Some(ref w_act) = anims.weapon_act {
                        if is_attacking || in_combat_stance {
                            w_act.frame_by_time(action_idx, anim_elapsed_ms).map(|(f, _)| f)
                        } else if let Some(f_idx) = walk_frame_override {
                            w_act.frame(action_idx, f_idx)
                        } else {
                            w_act.frame(action_idx, 0)
                        }
                    } else {
                        None
                    };

                    let params = AvatarRenderParams {
                        anchor_x,
                        anchor_y,
                        act_dir,
                        zoom: fb.projection.zoom,
                        shadow_frame: shadow_spr.as_ref().and_then(|s| s.frames.first()),
                        body: body_frame.map(|f| (f, anims.body_spr.as_ref())),
                        head: head_frame.zip(anims.head_spr.as_deref()),
                        headgear: headgear_frame.zip(anims.headgear_spr.as_deref()),
                        weapon: weapon_frame.zip(anims.weapon_spr.as_deref()),
                        flip_h: combo_tracker.is_inverted(now),
                    };
                    fb.draw_avatar(&params);
                } else if let Some(ref ch) = player_spr {
                    if let Some(ref shadow) = shadow_spr {
                        if let Some(f0) = shadow.frames.first() {
                            fb.draw_sprite_frame(anchor_x, anchor_y, f0, fb.projection.zoom);
                        }
                    }
                    if let Some(f0) = ch.frames.first() {
                        fb.draw_sprite_frame(
                            camera_x as i32,
                            (camera_y + 4.0 * fb.projection.zoom) as i32,
                            f0,
                            fb.projection.zoom,
                        );
                    }
                } else {
                    fb.draw_entity_token(
                        player_pos,
                        world_view.local_facing,
                        true,
                        cam_offset_x,
                        cam_offset_y,
                    );
                }

                // SPEC-0026: Renderiza Emoticon Nativo Animado do GRF sobre a cabeça do herói ao descansar
                if auto_idle.is_sitting {
                    if let Some((action_idx, elapsed_ms)) = auto_idle.active_emotion(now) {
                        if let Some((ref emo_act, ref emo_spr)) = emotion_anims {
                            if let Some((f, _)) = emo_act.frame_by_time(action_idx, elapsed_ms) {
                                let emo_x = camera_x as i32;
                                let emo_y = (camera_y - 42.0 * fb.projection.zoom).round() as i32;
                                fb.draw_act_frame(emo_x, emo_y, f, emo_spr, fb.projection.zoom);
                            }
                        }
                    }
                }

                // SPEC-0025: Renderiza Grito de Batalha Estilo Anime Shounen sobre o Herói
                shout_tracker.render(&mut fb, camera_x as i32, camera_y as i32, now);

                // SPEC-0028 / VALVE.md: Retículos no Chão para Feedback Tátil Visual Imediato
                if (is_mouse_over_dummy || dummy_attack_target.is_some()) && training_dummy.is_hittable() {
                    fb.draw_ground_reticle(
                        training_dummy.position.x as f32 + 0.5,
                        training_dummy.position.y as f32 + 0.5,
                        cam_offset_x,
                        cam_offset_y,
                        0xFFFF5555,
                        1.1,
                    );
                }
                if is_mouse_over_npc {
                    fb.draw_ground_reticle(
                        guide_npc.position.x as f32 + 0.5,
                        guide_npc.position.y as f32 + 0.5,
                        cam_offset_x,
                        cam_offset_y,
                        0xFF8BE9FD,
                        1.1,
                    );
                }

                // Renderiza Entidades no Campo de Visão (AoI 3x3)
                for entity in world_view.nearby_entities.values() {
                    if let Some(ref anims) = char_anims {
                        let (sx, sy) = fb.projection.world_to_screen(
                            entity.position.x as f32 + 0.5,
                            entity.position.y as f32 + 0.5,
                            cam_offset_x,
                            cam_offset_y,
                        );
                        let anchor_x = sx.round() as i32;
                        let anchor_y = (sy + 8.0 * fb.projection.zoom).round() as i32;

                        let visual_facing = fb.projection.world_facing_to_camera(entity.facing);
                        let act_dir = visual_facing.to_act_dir();
                        let action_idx = act_dir; // Idle

                        let body_frame = anims.body_act.frame(action_idx, 0);
                        let head_frame = anims.head_act.as_ref().and_then(|a| a.frame(action_idx, 0));
                        let headgear_frame = anims.headgear_act.as_ref().and_then(|a| a.frame(action_idx, 0));
                        let weapon_frame = anims.weapon_act.as_ref().and_then(|a| a.frame(action_idx, 0));

                        let params = AvatarRenderParams {
                            anchor_x,
                            anchor_y,
                            act_dir,
                            zoom: fb.projection.zoom,
                            shadow_frame: shadow_spr.as_ref().and_then(|s| s.frames.first()),
                            body: body_frame.map(|f| (f, anims.body_spr.as_ref())),
                            head: head_frame.zip(anims.head_spr.as_deref()),
                            headgear: headgear_frame.zip(anims.headgear_spr.as_deref()),
                            weapon: weapon_frame.zip(anims.weapon_spr.as_deref()),
                            flip_h: false,
                        };
                        fb.draw_avatar(&params);
                    } else {
                        fb.draw_entity_token(
                            entity.position,
                            entity.facing,
                            false,
                            cam_offset_x,
                            cam_offset_y,
                        );
                    }
                }

                // SPEC-0022: Renderização da Entidade Alvo Gelatinosa de Treino (Bouncy Slime Dummy)
                let (dummy_sx, dummy_sy) = fb.projection.world_to_screen(
                    training_dummy.position.x as f32 + 0.5,
                    training_dummy.position.y as f32 + 0.5,
                    cam_offset_x,
                    cam_offset_y,
                );
                let dummy_anchor_x = dummy_sx.round() as i32;
                let dummy_anchor_y = (dummy_sy + 8.0 * fb.projection.zoom).round() as i32;

                if let Some((ref m_act, ref m_spr)) = monster_anims {
                    let visual_facing = fb.projection.world_facing_to_camera(training_dummy.facing);
                    let act_dir = visual_facing.to_act_dir();
                    let action_idx = training_dummy.action_index(act_dir);
                    let elapsed_ms = training_dummy.anim_elapsed_ms(now);

                    let frame = if training_dummy.state == DummyState::Die {
                        // SPEC-0024: A animação de derrota toca uma única vez (não faz looping).
                        // Após a conclusão da animação de estouro, o monstro fica oculto até o respawn.
                        if let Some(action) = m_act.action(action_idx) {
                            let delay = if action.delay_ms <= 0.0 { 100.0 } else { action.delay_ms };
                            let total_dur = action.frames.len() as f32 * delay;
                            if elapsed_ms < total_dur {
                                let frame_idx = (elapsed_ms / delay) as usize;
                                action.frames.get(frame_idx)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        m_act.frame_by_time(action_idx, elapsed_ms).map(|(f, _)| f)
                    };

                    // Sombra do monstro (somente desenhada enquanto o monstro estiver visível)
                    if training_dummy.state != DummyState::Die || frame.is_some() {
                        if let Some(shadow) = shadow_spr.as_ref().and_then(|s| s.frames.first()) {
                            fb.draw_sprite_frame(dummy_anchor_x, dummy_anchor_y, shadow, fb.projection.zoom);
                        }
                    }
                    // Sprite animado do monstro
                    if let Some(f) = frame {
                        fb.draw_act_frame(dummy_anchor_x, dummy_anchor_y, f, m_spr, fb.projection.zoom);
                    }
                } else if training_dummy.state != DummyState::Die {
                    fb.draw_entity_token(
                        training_dummy.position,
                        training_dummy.facing,
                        false,
                        cam_offset_x,
                        cam_offset_y,
                    );
                }

                // Em combate com o monstro: feedback visual limpo e não-poluído (docs/VALVE.md)
                // Se o monstro estiver ferido, desenha apenas uma barra sutil de vida acima dele.
                // As informações detalhadas (nome, HP e distância) ficam na Target Info Bar no topo da tela.
                if training_dummy.is_hittable() && training_dummy.current_hp < training_dummy.max_hp {
                    let bar_y = dummy_anchor_y - (24.0 * fb.projection.zoom) as i32;
                    fb.draw_hp_bar(dummy_anchor_x, bar_y, training_dummy.current_hp, training_dummy.max_hp);
                }

                // SPEC-0028: Renderização da Guia NPC de Prontera (Living World & Feedback)
                let (npc_sx, npc_sy) = fb.projection.world_to_screen(
                    guide_npc.position.x as f32 + 0.5,
                    guide_npc.position.y as f32 + 0.5,
                    cam_offset_x,
                    cam_offset_y,
                );
                let npc_anchor_x = npc_sx.round() as i32;
                let npc_anchor_y = (npc_sy + 8.0 * fb.projection.zoom).round() as i32;

                if let Some((ref n_act, ref n_spr)) = npc_anims {
                    let visual_facing = fb.projection.world_facing_to_camera(guide_npc.facing);
                    let act_dir = visual_facing.to_act_dir() as usize;
                    let action_idx = act_dir % n_act.actions.len();
                    let elapsed_ms = (now.duration_since(anim_start_time).as_millis() % 2400) as f32;

                    if let Some(shadow) = shadow_spr.as_ref().and_then(|s| s.frames.first()) {
                        fb.draw_sprite_frame(npc_anchor_x, npc_anchor_y, shadow, fb.projection.zoom);
                    }
                    if let Some((f, _)) = n_act.frame_by_time(action_idx, elapsed_ms) {
                        fb.draw_act_frame(npc_anchor_x, npc_anchor_y, f, n_spr, fb.projection.zoom);
                    }
                } else {
                    fb.draw_entity_token(
                        guide_npc.position,
                        guide_npc.facing,
                        false,
                        cam_offset_x,
                        cam_offset_y,
                    );
                }

                // Emoticon da Guia NPC (se houver emoção ativa)
                if let Some((emo_idx, emo_start)) = guide_npc.active_emotion {
                    let emo_elapsed = now.duration_since(emo_start).as_millis() as f32;
                    if let Some((ref emo_act, ref emo_spr)) = emotion_anims {
                        if let Some((f, _)) = emo_act.frame_by_time(emo_idx, emo_elapsed) {
                            let emo_y = npc_anchor_y - (54.0 * fb.projection.zoom).round() as i32;
                            fb.draw_act_frame(npc_anchor_x, emo_y, f, emo_spr, fb.projection.zoom);
                        }
                    }
                }

                // Balão de Diálogo Orgânico e Não-Modal (docs/VALVE.md)
                if guide_npc.is_talking {
                    let (speaker, lines, hint) = guide_npc.current_dialogue();
                    let bubble_y = npc_anchor_y - (56.0 * fb.projection.zoom).round() as i32;
                    fb.draw_speech_bubble(npc_anchor_x, bubble_y, speaker, lines, hint);
                }

                // SPEC-0024: Renderização do Sistema de Partículas em Tempo Real
                particle_system.render(&mut fb, cam_offset_x, cam_offset_y);

                // SPEC-0022: Desenha Números Flutuantes de Dano de Combate
                for (wx, wy, dmg, color, is_crit, float_y) in floating_numbers.active_numbers(now) {
                    let (num_sx, num_sy) = fb.projection.world_to_screen(wx, wy, cam_offset_x, cam_offset_y);
                    let px = num_sx.round() as i32 - 14;
                    let py = (num_sy - 30.0 * fb.projection.zoom - float_y).round() as i32;
                    let dmg_text = if is_crit {
                        format!("-{} CRIT!", dmg)
                    } else {
                        format!("-{}", dmg)
                    };
                    // Sombra preta para alto contraste
                    fb.draw_text(px + 1, py + 1, &dmg_text, 0xFF000000, 1);
                    fb.draw_text(px, py, &dmg_text, color, 1);
                }

                let anim_status = if char_anims.is_some() {
                    if is_attacking {
                        combo_tracker.current_hud_label()
                    } else if is_walking {
                        "SWORDIE [WALK]"
                    } else if in_combat_stance {
                        "SWORDIE [READY]"
                    } else if auto_idle.is_sitting {
                        "SWORDIE [SIT]"
                    } else {
                        "SWORDIE [IDLE]"
                    }
                } else {
                    "FALLBACK"
                };

                // SPEC-0028 / SPEC-0027: Badge de Localização e Ping no Topo Esquerdo (docs/VALVE.md)
                let ping_ms = live_rtt.load(std::sync::atomic::Ordering::Relaxed);
                let badge_w = if client.is_some() { 245 } else { 195 };
                fb.draw_rect(10, 10, badge_w, 24, 0xCC14141E, Some(0xFF6272A4));
                fb.draw_text(18, 16, "PRONTERA [156, 180]", 0xFFF1FA8C, 1);
                if client.is_some() {
                    let ping_color = if ping_ms <= 80 {
                        0xFF50FA7B // Verde (< 80ms)
                    } else if ping_ms <= 150 {
                        0xFFFFB86C // Laranja
                    } else {
                        0xFFFF5555 // Vermelho (> 150ms)
                    };
                    fb.draw_text(152, 16, &format!("{}ms", ping_ms), ping_color, 1);
                    fb.draw_text(205, 16, "[F3]", 0xFF8BE9FD, 1);
                } else {
                    fb.draw_text(158, 16, "[F3]", 0xFF8BE9FD, 1);
                }

                // Painel de métricas e telemetria para desenvolvedores (somente visível sob demanda com F3)
                if show_debug_overlay {
                    let debug_w = 260;
                    fb.draw_rect(10, 38, debug_w, 106, 0xEE14141E, Some(0xFF6272A4));
                    let cam_degrees = fb.projection.yaw.to_degrees().rem_euclid(360.0);
                    fb.draw_text(18, 44, &format!("CAM: {:03.0}° YAW | {:03.0}° PITCH", cam_degrees, fb.projection.pitch_deg()), 0xFFFFB86C, 1);
                    fb.draw_text(18, 58, &format!("ZOOM: {:.2}x [NIVEL {:02}/17]", fb.projection.zoom, zoom_idx + 1), 0xFFFF79C6, 1);
                    let net_status = if client.is_some() {
                        format!("REDE: QUIC | {}ms | {}", ping_ms, styx_config.profile.tag())
                    } else {
                        format!("REDE: OFFLINE | {}", styx_config.profile.tag())
                    };
                    fb.draw_text(18, 72, &net_status, 0xFF8BE9FD, 1);
                    fb.draw_text(18, 86, &format!("CENARIO: {} OBJETOS (.RSW)", rendered_models + 1), 0xFF50FA7B, 1);
                    fb.draw_text(18, 100, &format!("SPRITE: {}", anim_status), 0xFFFF79C6, 1);
                    fb.draw_text(18, 114, &format!("ALVO: PORING [HP {}/{}]", training_dummy.current_hp, training_dummy.max_hp), 0xFFF1FA8C, 1);
                    fb.draw_text(18, 128, "CLIENTE: BERENICE 2.5D @ 60 FPS", 0xFF6272A4, 1);
                }

                // SPEC-0028: Barra de Informações do Alvo no Topo Central (Target Info Bar)
                if is_mouse_over_dummy || dummy_attack_target.is_some() || training_dummy.current_hp < training_dummy.max_hp {
                    let target_dist = (((training_dummy.position.x as f32 - player_pos.x as f32).powi(2)
                        + (training_dummy.position.y as f32 - player_pos.y as f32).powi(2))
                    .sqrt())
                    .max(0.5);
                    fb.draw_target_info_bar(
                        window_width as i32,
                        "PORING DUMMY",
                        training_dummy.current_hp,
                        training_dummy.max_hp,
                        target_dist,
                    );
                }

                // SPEC-0023: Desenha HUD do Jogador com Barras de HP e SP no Canto Superior Direito
                let player_hud_x = (window_width.saturating_sub(220)) as i32;
                fb.draw_player_hud(player_hud_x, 10, &player_stats);

                // SPEC-0028: Minimapa Dinâmico posicionado exatamente abaixo do Player HUD
                let minimap_size = 100;
                let minimap_x = (window_width as i32) - minimap_size - 10;
                let minimap_y = 68;
                let mut radar_entities = Vec::with_capacity(6);
                if training_dummy.is_hittable() {
                    radar_entities.push((training_dummy.position, 0xFFFF5555));
                }
                radar_entities.push((guide_npc.position, 0xFF8BE9FD));
                for entity in world_view.nearby_entities.values() {
                    radar_entities.push((entity.position, 0xFF50FA7B));
                }
                fb.draw_minimap(
                    minimap_x,
                    minimap_y,
                    minimap_size,
                    &world_grid,
                    player_pos,
                    &radar_entities,
                );

                // SPEC-0023: Desenha Janelas Flutuantes de Status e Inventário
                if show_status_window {
                    fb.draw_status_window(10, 175, &player_stats);
                }
                if show_inventory_window {
                    let inv_x = (window_width.saturating_sub(275)) as i32;
                    fb.draw_inventory_window(inv_x, 75, &player_stats);
                }

                // SPEC-0028: Hotbar / Barra de Ação de Habilidades e Consumíveis (Centro Inferior Único)
                let red_pots = player_stats.inventory.iter().find(|i| i.id == 501).map(|i| i.amount).unwrap_or(0);
                let blue_pots = player_stats.inventory.iter().find(|i| i.id == 505).map(|i| i.amount).unwrap_or(0);
                fb.draw_action_bar(
                    window_width as i32,
                    window_height as i32,
                    red_pots,
                    blue_pots,
                    combo_tracker.current_hud_label(),
                );
            }
        }

        // SPEC-0018: Atualiza a janela nativa com os pixels na resolução dinâmica
        window.update_with_buffer(&fb.pixels, window_width, window_height)?;
    }

    println!("👋 Janela encerrada. Sessão finalizada!");
    Ok(())
}
