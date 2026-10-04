# SPEC-0016 — Protocolo do World Server e Movimento em Tempo Real

## Status: Approved
## Revisão: 1
## Data: 2026-10-03
## Módulo: `crates/hades-world`, `crates/hades-net`, `berenice-client`

---

## 1. Contexto e Motivação

O **Hades World Server** é responsável pela simulação espacial, gestão de presença e sincronização em tempo real de avatares no mapa.
Após a seleção de personagem no `LoginServer` (especificado no [SPEC-0015](SPEC-0015-login-auth-webtransport-protocol.md)), o cliente recebe as coordenadas de conexão (`world_ip`, `world_port`) do servidor de mundo.

Este documento especifica:
1. O protocolo de ingresso no mapa (`enter_world` → `enter_world_ok`).
2. O protocolo de movimentação e colisão de entidades (`move_request`, `player_move`, `entity_move`).
3. A integração com o particionamento espacial em grade de buckets $O(1)$ (Área de Interesse — AoI $3 \times 3$ buckets, conforme [SPEC-0006](SPEC-0006-aoi-and-tick-engine.md) e [SPEC-0011](SPEC-0011-multiplayer-world-server-and-aoi-sync.md)).
4. O suporte dual de transporte: **QUIC / WebTransport TLS 1.3** (clientes nativos e browser com suporte a WebTransport) e **WebSocket** transparente (desenvolvimento web).

---

## 2. Ciclo de Vida da Sessão no Mundo

```
Client (Berenice)                                WorldServer (hades-world)
   |                                                        |
   |--- WebTransport / WebSocket Connect (porta 4434) ----->|
   |                                                        |
   |--- STREAM / WS: enter_world { aid, gid, auth_code } -->|
   |<-- STREAM / WS: enter_world_ok { ... } ----------------|  (Spawna avatar no cliente)
   |                                                        |
   |<-- STREAM / WS: entity_spawn { id, name, pos, ... } ---|  (Entidades pré-existentes no AoI 3x3)
   |                                                        |
   |--- DATAGRAM / WS: move_request { to_x, to_y } -------->|  (Jogador clica no chão)
   |                                                        |  [Valida CollisionGrid e Path]
   |<-- DATAGRAM / WS: player_move { from, to, times } -----|  (Confirmação para o jogador)
   |                                                        |
   |                                                        |--- AoI Multicast (3x3 buckets) ---> Outros Clientes
   |                                                        |    (entity_move / MovementDelta 6B)
   |                                                        |
   |--- Desconexão ---------------------------------------->|  [Remove do SpatialGrid]
                                                            |--- AoI Multicast (entity_despawn) -> Outros Clientes
```

---

## 3. Mensagens do Protocolo (Wire Format)

No canal confiável / WebSocket, as mensagens trafegam em JSON delimitado por quebra de linha (`\n`).
No canal não-confiável de baixa latência (QUIC Datagrams), o movimento pode utilizar o bitpacking de 6 bytes (`MovementDelta`, especificado em [SPEC-0001](SPEC-0001-core-types-and-bitpacking.md)).

### 3.1 `enter_world` (Client → Server)
Enviado imediatamente após a abertura do canal de rede com o World Server.

```json
{
  "type": "enter_world",
  "aid": 1,
  "gid": 1,
  "auth_code": 12345
}
```

### 3.2 `enter_world_ok` (Server → Client)
Resposta autorizando a entrada do jogador no mapa. Corresponde ao evento `ACCEPT_ENTER` no cliente gráfico.

```json
{
  "type": "enter_world_ok",
  "aid": 1,
  "gid": 1,
  "name": "Berenice",
  "map_name": "prontera.gat",
  "pos_x": 156,
  "pos_y": 180,
  "dir": 0,
  "sex": 0,
  "speed": 150
}
```

### 3.3 `move_request` (Client → Server)
Solicitação de movimentação gerada pelo clique do usuário ou stick analógico.

```json
{
  "type": "move_request",
  "to_x": 160,
  "to_y": 182
}
```

### 3.4 `player_move` (Server → Client)
Notificação enviada ao jogador confirmando o trajeto validado pelo servidor. Corresponde ao `NOTIFY_PLAYERMOVE` no cliente.

```json
{
  "type": "player_move",
  "from_x": 156,
  "from_y": 180,
  "to_x": 160,
  "to_y": 182,
  "start_time": 1727960000000,
  "end_time": 1727960000750
}
```

### 3.5 `entity_spawn` (Server → Client)
Notificação de que uma entidade remota entrou no raio de visão (AoI $3 \times 3$) ou já estava lá ao entrar no mapa. Corresponde ao `NOTIFY_STANDENTRY` / `NOTIFY_NEWENTRY`.

```json
{
  "type": "entity_spawn",
  "id": 102,
  "name": "OutroJogador",
  "job": 0,
  "pos_x": 158,
  "pos_y": 181,
  "dir": 4,
  "speed": 150
}
```

### 3.6 `entity_move` (Server → Client)
Notificação enviada a observadores no raio de visão quando outra entidade se move. Corresponde a `NOTIFY_MOVE`.

```json
{
  "type": "entity_move",
  "id": 102,
  "from_x": 158,
  "from_y": 181,
  "to_x": 162,
  "to_y": 185,
  "start_time": 1727960000000,
  "end_time": 1727960000800
}
```

### 3.7 `entity_despawn` (Server → Client)
Notificação quando uma entidade sai do raio AoI ou se desconecta. Corresponde a `NOTIFY_VANISH`.

```json
{
  "type": "entity_despawn",
  "id": 102
}
```

---

## 4. Invariantes do Potato Budget & Segurança

1. **Validação Estrita de Colisão:**
   Nenhum movimento é aceito se o ponto de destino ou o passo intermediário cruzar células não caminháveis (`CollisionGrid::is_walkable == false`).
2. **Anti-Warp / Anti-Speedhack:**
   A distância percorrida não pode ultrapassar o limite calculado por tick ($v \times \Delta t$).
3. **Multicast AoI $O(1)$:**
   O broadcast de movimento atinge estritamente conexões nos 9 buckets adjacentes ($3 \times 3$), consumindo zero alocações na heap durante o tick.
4. **Transporte Transparente:**
   O mesmo motor de simulação alimenta tanto conexões WebTransport quanto conexões WebSocket de desenvolvimento.
