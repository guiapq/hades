//! Teste de integração do fluxo completo de entrada no mundo e movimento (SPEC-0016).

use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use hades_core::collision::CollisionGrid;
use hades_core::types::Position;
use hades_world::{ClientWorldMsg, ServerWorldMsg, WorldManager, WorldSessionHandler};

#[tokio::test]
async fn test_full_world_entry_and_movement_over_websocket() {
    let col = CollisionGrid::new(256, 256, true);
    let world = Arc::new(WorldManager::new(col));
    let spawn_pos = Position::new_unchecked(156, 180);
    let session_handler = Arc::new(WorldSessionHandler::new(world.clone(), spawn_pos));

    // Sobe listener TCP WebSocket em porta aleatória
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("local_addr");

    let h = session_handler.clone();
    tokio::spawn(async move {
        while let Ok((stream, peer)) = listener.accept().await {
            let handler = h.clone();
            tokio::spawn(async move {
                if let Ok(ws) = tokio_tungstenite::accept_async(stream).await {
                    handler.run_ws(ws, peer).await;
                }
            });
        }
    });

    // Conecta Cliente A (Berenice)
    let ws_url = format!("ws://{addr}");
    let (ws_stream_a, _) = connect_async(&ws_url).await.expect("connect client A");
    let (mut sink_a, mut stream_a) = ws_stream_a.split();

    // Envia EnterWorld
    let enter_msg = ClientWorldMsg::EnterWorld {
        aid: 1,
        gid: 1,
        auth_code: 12345,
        name: Some("Berenice".to_string()),
    };
    sink_a
        .send(Message::Text(serde_json::to_string(&enter_msg).unwrap().into()))
        .await
        .expect("send EnterWorld");

    // Aguarda EnterWorldOk
    let raw_resp = stream_a.next().await.expect("next").expect("read response");
    let text = match raw_resp {
        Message::Text(t) => t.to_string(),
        _ => panic!("Esperado Message::Text"),
    };
    let server_msg: ServerWorldMsg = serde_json::from_str(&text).expect("parse enter_world_ok");

    match server_msg {
        ServerWorldMsg::EnterWorldOk {
            aid,
            gid,
            pos_x,
            pos_y,
            speed,
            ..
        } => {
            assert_eq!(aid, 1);
            assert_eq!(gid, 1);
            assert_eq!(pos_x, 156);
            assert_eq!(pos_y, 180);
            assert_eq!(speed, 150);
        }
        other => panic!("Esperado EnterWorldOk, recebido: {:?}", other),
    }

    // Cliente A solicita movimento para (160, 180)
    let move_msg = ClientWorldMsg::MoveRequest {
        to_x: 160,
        to_y: 180,
    };
    sink_a
        .send(Message::Text(serde_json::to_string(&move_msg).unwrap().into()))
        .await
        .expect("send MoveRequest");

    // Cliente A deve receber PlayerMove
    let raw_move = stream_a.next().await.expect("next").expect("read move response");
    let move_text = match raw_move {
        Message::Text(t) => t.to_string(),
        _ => panic!("Esperado Message::Text"),
    };
    let player_move: ServerWorldMsg = serde_json::from_str(&move_text).expect("parse player_move");

    match player_move {
        ServerWorldMsg::PlayerMove {
            from_x,
            from_y,
            to_x,
            to_y,
            start_time,
            end_time,
        } => {
            assert_eq!(from_x, 156);
            assert_eq!(from_y, 180);
            assert_eq!(to_x, 160);
            assert_eq!(to_y, 180);
            assert!(end_time > start_time);
            // 4 células * 150ms = 600ms de duração esperada
            assert_eq!(end_time - start_time, 4 * 150);
        }
        other => panic!("Esperado PlayerMove, recebido: {:?}", other),
    }
}
