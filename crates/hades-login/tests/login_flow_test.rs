//! Teste de integração ponta-a-ponta do Hades Login Server:
//! Fluxo: Connect -> Login -> CharList -> CharCreate -> CharSelect

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncReadExt;

use hades_login::db::Db;
use hades_login::messages::{ClientMsg, ServerMsg};
use hades_login::session::{LoginSession, WorldServerConfig};
use hades_net::{connect_to_server, HadesServer};

#[tokio::test]
async fn test_full_login_and_char_flow_over_quic() {
    // 1. Inicializa DB em memória e cria conta de teste
    let db = Db::open("sqlite::memory:").await.expect("DB in-memory");
    db.migrate().await.expect("Migrate DB");

    let test_user = "hercules";
    let test_pass_hash = "5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8"; // sha256("password")
    let aid = db.create_account_dev(test_user, test_pass_hash, 0).await.expect("create account");
    assert!(aid > 0);

    let db = Arc::new(db);
    let session = Arc::new(LoginSession::new(
        db.clone(),
        WorldServerConfig {
            ip: "127.0.0.1".to_string(),
            port: 4434,
            name: "Hades Midgard Test".to_string(),
        },
    ));

    // 2. Sobe servidor em porta efêmera
    let bind_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = HadesServer::bind(bind_addr).expect("Server bind");
    let server_addr = server.local_addr().expect("local addr");
    let cert_der = server.cert_der().clone();

    // 3. Spawna task do servidor para aceitar a conexão
    let server_session = session.clone();
    tokio::spawn(async move {
        if let Some(conn) = server.accept().await {
            server_session.run(conn).await;
        }
    });

    // 4. Cliente conecta via QUIC
    let client_conn = connect_to_server(server_addr, "localhost", cert_der)
        .await
        .expect("Client connect");

    // 5. Abre stream bidirecional
    let (mut send, mut recv) = client_conn.open_bi().await.expect("Open bi stream");

    // Helper para ler uma linha JSON
    async fn read_server_msg<R: AsyncReadExt + Unpin>(reader: &mut R) -> ServerMsg {
        let mut line = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            let n = reader.read(&mut byte).await.expect("read byte");
            if n == 0 {
                panic!("EOF inesperado lendo ServerMsg");
            }
            if byte[0] == b'\n' {
                break;
            }
            line.push(byte[0]);
        }
        let s = String::from_utf8(line).expect("utf8 string");
        serde_json::from_str(&s).expect("parse ServerMsg")
    }

    // 6. Teste Login
    let login_cmd = serde_json::to_string(&ClientMsg::Login {
        username: test_user.to_string(),
        password: test_pass_hash.to_string(),
    }).unwrap() + "\n";
    send.write_all(login_cmd.as_bytes()).await.unwrap();

    let resp = read_server_msg(&mut recv).await;
    match resp {
        ServerMsg::LoginOk { aid: got_aid, sex, servers, .. } => {
            assert_eq!(got_aid, aid);
            assert_eq!(sex, 0);
            assert_eq!(servers.len(), 1);
            assert_eq!(servers[0].port, 4434);
        }
        other => panic!("Esperava LoginOk, recebeu: {:?}", other),
    }

    // 7. Teste CharList (deve estar vazia)
    let char_list_cmd = serde_json::to_string(&ClientMsg::CharList { aid: None, auth_code: None }).unwrap() + "\n";
    send.write_all(char_list_cmd.as_bytes()).await.unwrap();

    let resp = read_server_msg(&mut recv).await;
    match resp {
        ServerMsg::CharList { chars } => {
            assert!(chars.is_empty());
        }
        other => panic!("Esperava CharList vazia, recebeu: {:?}", other),
    }

    // 8. Teste CharCreate (Cria Novice com stats somando 6)
    let create_cmd = serde_json::to_string(&ClientMsg::CharCreate {
        name: "TestBerenice".to_string(),
        str: 1, agi: 1, vit: 1, int_stat: 1, dex: 1, luk: 1,
        hair_style: 2,
        hair_color: 1,
    }).unwrap() + "\n";
    send.write_all(create_cmd.as_bytes()).await.unwrap();

    let created_gid;
    let resp = read_server_msg(&mut recv).await;
    match resp {
        ServerMsg::CharCreated { char } => {
            assert_eq!(char.name, "TestBerenice");
            assert_eq!(char.slot, 0);
            assert_eq!(char.level, 1);
            assert_eq!(char.hair_style, 2);
            assert_eq!(char.body_palette, 0);
            created_gid = char.gid;
        }
        other => panic!("Esperava CharCreated, recebeu: {:?}", other),
    }
    assert!(created_gid > 0);

    // 9. Teste CharSelect
    let select_cmd = serde_json::to_string(&ClientMsg::CharSelect {
        gid: created_gid,
    }).unwrap() + "\n";
    send.write_all(select_cmd.as_bytes()).await.unwrap();

    let resp = read_server_msg(&mut recv).await;
    match resp {
        ServerMsg::CharSelected { gid, map_name, world_port, .. } => {
            assert_eq!(gid, created_gid);
            assert_eq!(map_name, "prontera.gat");
            assert_eq!(world_port, 4434);
        }
        other => panic!("Esperava CharSelected, recebeu: {:?}", other),
    }
}
