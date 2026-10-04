//! Hades World Server: servidor de simulação de mundo e movimento em tempo real.

use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::EnvFilter;

use hades_core::types::Position;
use hades_net::HadesServer;
use hades_world::{MapLoader, WorldManager, WorldSessionHandler};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();

    let args: Vec<String> = std::env::args().collect();
    let is_verbose = args.iter().any(|a| a == "--verbose" || a == "-v")
        || std::env::var("HADES_VERBOSE").map(|v| v == "1" || v == "true").unwrap_or(false);

    let default_filter = if is_verbose {
        "debug,hades_world=debug,hades_net=debug"
    } else {
        "info,hades_world=info,hades_net=info"
    };

    // 1. Inicializa logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(default_filter)),
        )
        .init();

    info!("Iniciando Hades World Server...");
    if is_verbose {
        info!("🔍 [Verbose Mode ATIVADO] Exibindo tráfego detalhado de pacotes QUIC e WebSocket");
    }

    // 2. Configurações via variáveis de ambiente
    let ws_bind_str = std::env::var("HADES_WORLD_BIND").unwrap_or_else(|_| "0.0.0.0:4434".to_string());
    let ws_bind_addr: SocketAddr = ws_bind_str.parse().expect("HADES_WORLD_BIND inválido");

    let quic_bind_str = std::env::var("HADES_WORLD_QUIC_BIND").unwrap_or_else(|_| "0.0.0.0:4434".to_string());
    let quic_bind_addr: SocketAddr = quic_bind_str.parse().expect("HADES_WORLD_QUIC_BIND inválido");

    let map_name = std::env::var("HADES_MAP_NAME").unwrap_or_else(|_| "prontera.gat".to_string());
    let spawn_x: u16 = std::env::var("HADES_SPAWN_X")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(156);
    let spawn_y: u16 = std::env::var("HADES_SPAWN_Y")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(180);

    // 3. Carrega grade de colisão do mapa
    info!("Carregando grade de colisão para mapa '{map_name}'...");
    let collision_grid = MapLoader::load_or_fallback(&map_name);
    let spawn_pos = Position::new_unchecked(spawn_x, spawn_y);

    if !collision_grid.is_walkable(spawn_pos) {
        tracing::warn!("Atenção: Posição de spawn ({spawn_x}, {spawn_y}) está marcada como não caminhável!");
    } else {
        info!("Posição de spawn inicial ({spawn_x}, {spawn_y}) validada como caminhável.");
    }

    // 4. Inicializa o WorldManager
    let world = Arc::new(WorldManager::new(collision_grid));
    let session_handler = Arc::new(WorldSessionHandler::new(world.clone(), spawn_pos).with_verbose(is_verbose));

    // 5. Inicia o listener WebSocket (porta 4434 TCP)
    let ws_listener = tokio::net::TcpListener::bind(ws_bind_addr).await.expect("Falha ao abrir socket TCP WebSocket World");
    info!("Hades World WebSocket Server escutando em {ws_bind_addr} (ws://)");

    let ws_handler = session_handler.clone();
    tokio::spawn(async move {
        while let Ok((stream, peer)) = ws_listener.accept().await {
            let h = ws_handler.clone();
            tokio::spawn(async move {
                match tokio_tungstenite::accept_async(stream).await {
                    Ok(ws) => h.run_ws(ws, peer).await,
                    Err(e) => tracing::warn!("Falha no handshake WebSocket World de {peer}: {e}"),
                }
            });
        }
    });

    // 6. Inicia o servidor QUIC (porta 4434 UDP)
    info!("Vinculando socket QUIC World em {quic_bind_addr}...");
    let quic_server = HadesServer::bind(quic_bind_addr).expect("Falha ao subir HadesServer QUIC World");
    info!("Hades World QUIC Server escutando em {quic_bind_addr} (QUIC/WebTransport)");

    // 7. Loop de conexões QUIC
    while let Some(conn) = quic_server.accept().await {
        let h = session_handler.clone();
        tokio::spawn(async move {
            h.run_quic(conn).await;
        });
    }

    Ok(())
}
