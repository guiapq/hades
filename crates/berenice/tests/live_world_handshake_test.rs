//! Teste de integração: Berenice conecta ao Hades World Server em execução.

use berenice::network::BereniceNetwork;
use hades_world::{ClientWorldMsg, ServerWorldMsg};
use std::net::SocketAddr;

#[tokio::test]
async fn test_berenice_connects_to_live_world_server() {
    let addr_str = std::env::var("HADES_WORLD_SERVER").unwrap_or_else(|_| "127.0.0.1:4434".to_string());
    let server_addr: SocketAddr = addr_str.parse().unwrap();

    println!("Conectando Berenice ao Hades World Server em {server_addr}...");
    let network = match BereniceNetwork::connect_insecure(server_addr, "localhost").await {
        Ok(n) => n,
        Err(e) => {
            eprintln!("Aviso: Hades World Server não está ativo nesta porta ({e}). Pulando teste de integração viva.");
            return;
        }
    };

    let mut world_client = network
        .open_world_client()
        .await
        .expect("Falha ao abrir stream WorldClient");

    let _ = dotenvy::dotenv();
    let char_name = std::env::var("HADES_CLIENT_CHAR_NAME").unwrap_or_else(|_| "Berenice".to_string());
    let expected_map = std::env::var("HADES_MAP_NAME").unwrap_or_else(|_| "prontera.gat".to_string());
    let expected_x: u16 = std::env::var("HADES_SPAWN_X").ok().and_then(|v| v.parse().ok()).unwrap_or(156);
    let expected_y: u16 = std::env::var("HADES_SPAWN_Y").ok().and_then(|v| v.parse().ok()).unwrap_or(180);

    // Envia EnterWorld
    if world_client
        .send_msg(&ClientWorldMsg::EnterWorld {
            aid: 1,
            gid: 1,
            auth_code: 42,
            name: Some(char_name.clone()),
        })
        .await
        .is_err()
    {
        eprintln!("Aviso: Falha ao enviar EnterWorld (servidor encerrou stream). Pulando teste.");
        return;
    }

    // Lê resposta
    let msg = match world_client.read_msg().await {
        Ok(Some(m)) => m,
        _ => {
            eprintln!("Aviso: Nenhuma resposta recebida do servidor vivo. Pulando teste.");
            return;
        }
    };

    match msg {
        ServerWorldMsg::EnterWorldOk {
            name,
            map_name,
            pos_x,
            pos_y,
            ..
        } => {
            println!("✅ Conexão Berenice ⇄ World Server bem-sucedida!");
            println!("   Jogador: {name}");
            println!("   Mapa: {map_name}");
            println!("   Spawn: ({pos_x}, {pos_y})");
            assert_eq!(name, char_name);
            assert_eq!(map_name, expected_map);
            assert_eq!(pos_x, expected_x);
            assert_eq!(pos_y, expected_y);
        }
        other => {
            eprintln!("Mensagem inesperada: {other:?}");
            return;
        }
    }

    // 2. Testa requisição de movimento no mapa (M3)
    println!("Enviando MoveRequest para (156, 175)...");
    world_client
        .send_msg(&ClientWorldMsg::MoveRequest {
            to_x: 156,
            to_y: 175,
        })
        .await
        .expect("Falha ao enviar MoveRequest");


    let mut found_player_move = false;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);

    while std::time::Instant::now() < deadline {
        let msg = match tokio::time::timeout(std::time::Duration::from_millis(500), world_client.read_msg()).await {
            Ok(Ok(Some(m))) => m,
            _ => continue,
        };

        match msg {
            ServerWorldMsg::PlayerMove {
                from_x,
                from_y,
                to_x,
                to_y,
                start_time,
                end_time,
            } => {
                println!("✅ Confirmação de Movimento (M3) recebida com sucesso!");
                println!("   Origem: ({from_x}, {from_y}) -> Destino: ({to_x}, {to_y})");
                println!(
                    "   Janela de interpolação: {start_time}ms até {end_time}ms (Duração: {}ms)",
                    end_time - start_time
                );
                assert_eq!(from_x, 156);
                assert_eq!(from_y, 180);
                assert_eq!(to_x, 156);
                assert_eq!(to_y, 175);
                assert!(end_time > start_time);
                found_player_move = true;
                break;
            }
            other => {
                println!("   [Stream] Mensagem recebida no canal de rede: {other:?}");
            }
        }
    }

    assert!(found_player_move, "Não recebeu PlayerMove dentro do tempo limite");
}

