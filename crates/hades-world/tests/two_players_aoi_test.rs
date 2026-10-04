//! SPEC-0016 / M3: Teste de integração de dois jogadores sincronizando movimento em tempo real no AoI 3x3.

use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use hades_core::collision::CollisionGrid;
use hades_core::types::Position;
use hades_world::{ClientWorldMsg, ServerWorldMsg, WorldManager, WorldSessionHandler};

#[tokio::test]
async fn test_two_players_see_each_other_and_sync_movement() {
    let mut col = CollisionGrid::new(312, 392, true);
    // Garante que o centro de Prontera é caminhável
    for x in 140..180 {
        for y in 160..200 {
            col.set_walkable(Position::new_unchecked(x, y), true);
        }
    }

    let world = Arc::new(WorldManager::new(col));
    let spawn_pos = Position::new_unchecked(156, 180);
    let session_handler = Arc::new(WorldSessionHandler::new(world.clone(), spawn_pos));

    // Sobe o servidor WebSocket em porta dinâmica
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

    let ws_url = format!("ws://{addr}");

    // 1. Conecta Jogador A (Berenice)
    let (ws_a, _) = connect_async(&ws_url).await.expect("connect A");
    let (mut sink_a, mut stream_a) = ws_a.split();

    sink_a
        .send(Message::Text(
            serde_json::to_string(&ClientWorldMsg::EnterWorld {
                aid: 1,
                gid: 1,
                auth_code: 100,
                name: Some("Berenice".to_string()),
            })
            .unwrap()
            .into(),
        ))
        .await
        .expect("send EnterWorld A");

    // A recebe EnterWorldOk
    let a_enter_resp = stream_a.next().await.unwrap().unwrap();
    let a_enter_msg: ServerWorldMsg =
        serde_json::from_str(&a_enter_resp.to_text().unwrap()).unwrap();
    assert!(matches!(a_enter_msg, ServerWorldMsg::EnterWorldOk { gid: 1, pos_x: 156, pos_y: 180, .. }));

    // 2. Conecta Jogador B (OutroJogador) no mesmo AoI
    let (ws_b, _) = connect_async(&ws_url).await.expect("connect B");
    let (mut sink_b, mut stream_b) = ws_b.split();

    sink_b
        .send(Message::Text(
            serde_json::to_string(&ClientWorldMsg::EnterWorld {
                aid: 2,
                gid: 2,
                auth_code: 200,
                name: Some("OutroJogador".to_string()),
            })
            .unwrap()
            .into(),
        ))
        .await
        .expect("send EnterWorld B");

    // B recebe EnterWorldOk
    let b_enter_resp = stream_b.next().await.unwrap().unwrap();
    let b_enter_msg: ServerWorldMsg =
        serde_json::from_str(&b_enter_resp.to_text().unwrap()).unwrap();
    assert!(matches!(b_enter_msg, ServerWorldMsg::EnterWorldOk { gid: 2, .. }));

    // B recebe EntitySpawn do Jogador A (que já estava no mapa)
    let b_spawn_a = stream_b.next().await.unwrap().unwrap();
    let b_spawn_a_msg: ServerWorldMsg =
        serde_json::from_str(&b_spawn_a.to_text().unwrap()).unwrap();
    assert!(matches!(b_spawn_a_msg, ServerWorldMsg::EntitySpawn { id, .. } if id == 100));

    // A recebe EntitySpawn do Jogador B (que acabou de entrar)
    let a_spawn_b = stream_a.next().await.unwrap().unwrap();
    let a_spawn_b_msg: ServerWorldMsg =
        serde_json::from_str(&a_spawn_b.to_text().unwrap()).unwrap();
    assert!(matches!(a_spawn_b_msg, ServerWorldMsg::EntitySpawn { id, .. } if id == 101));

    // 3. Jogador A anda de (156, 180) para (160, 180)
    sink_a
        .send(Message::Text(
            serde_json::to_string(&ClientWorldMsg::MoveRequest {
                to_x: 160,
                to_y: 180,
            })
            .unwrap()
            .into(),
        ))
        .await
        .expect("send MoveRequest A");

    // A recebe PlayerMove
    let a_move_resp = stream_a.next().await.unwrap().unwrap();
    let a_move_msg: ServerWorldMsg =
        serde_json::from_str(&a_move_resp.to_text().unwrap()).unwrap();
    assert!(matches!(
        a_move_msg,
        ServerWorldMsg::PlayerMove { from_x: 156, from_y: 180, to_x: 160, to_y: 180, .. }
    ));

    // B recebe EntityMove do Jogador A!
    let b_recv_a_move = stream_b.next().await.unwrap().unwrap();
    let b_recv_a_move_msg: ServerWorldMsg =
        serde_json::from_str(&b_recv_a_move.to_text().unwrap()).unwrap();
    assert!(matches!(
        b_recv_a_move_msg,
        ServerWorldMsg::EntityMove { id: 100, from_x: 156, from_y: 180, to_x: 160, to_y: 180, .. }
    ));

    // 4. Jogador B desconecta
    drop(sink_b);
    drop(stream_b);

    // A recebe EntityDespawn do Jogador B!
    let a_despawn_b = stream_a.next().await.unwrap().unwrap();
    let a_despawn_b_msg: ServerWorldMsg =
        serde_json::from_str(&a_despawn_b.to_text().unwrap()).unwrap();
    assert!(matches!(
        a_despawn_b_msg,
        ServerWorldMsg::EntityDespawn { id: 101 }
    ));
}
