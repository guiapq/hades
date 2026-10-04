# SPEC-0015 — Protocolo de Autenticação Hades (WebTransport)

## Status: DRAFT
## Revisão: 1
## Data: 2026-10-02

---

## 1. Motivação

O Hades utiliza WebTransport sobre QUIC/TLS 1.3 como transporte exclusivo.
Este documento especifica o protocolo de autenticação (handshake → login →
seleção de personagem) entre o `berenice-client` (JavaScript, browser) e o
`hades-login` (Rust).

Não há compatibilidade com o protocolo rAthena nem com TCP legado.

---

## 2. Visão Geral do Fluxo

```
Client                          LoginServer
  |                                 |
  |-- WebTransport CONNECT -------->|  (TLS 1.3, ALPN "hades/1")
  |<-- 200 OK ----------------------|
  |                                 |
  |-- STREAM: AuthRequest --------->|  (reliable, bidirecional)
  |<-- STREAM: AuthResponse --------|
  |                                 |
  |-- STREAM: CharListRequest ----->|
  |<-- STREAM: CharListResponse ----|
  |                                 |
  |-- STREAM: CharSelectRequest --->|
  |<-- STREAM: CharSelectResponse --|  (contém WorldToken p/ WorldServer)
  |                                 |
  ·  (fecha a sessão de login)      ·
  |                                 |
  |-- WebTransport CONNECT -------->|  WorldServer (porta separada)
  |  (usa WorldToken no header)     |
```

---

## 3. Encoding de Pacotes

Todos os pacotes em streams confiáveis usam o mesmo framing:

```
┌────────────┬────────────┬─────────────────────────────┐
│  type: u8  │  len: u16  │  payload: [u8; len]         │
└────────────┴────────────┴─────────────────────────────┘
```

- **type**: identificador do pacote (1 byte)
- **len**: tamanho do payload em bytes (u16 LE)
- **payload**: corpo do pacote (variável)

---

## 4. Pacotes

### 4.1 AuthRequest (Client → Server) — type `0x01`

| Campo          | Tipo     | Descrição                        |
|----------------|----------|----------------------------------|
| username_len   | u8       | Tamanho do username (1..=32)     |
| username       | [u8]     | Username em UTF-8                |
| password_hash  | [u8; 32] | SHA-256 da senha (sem sal aqui)  |

> **Nota de segurança:** O sal é aplicado server-side usando Argon2id.
> O hash SHA-256 enviado é apenas ofuscação de transporte — o TLS 1.3 já
> protege o canal. Em v2 migrar para SRP-6a.

### 4.2 AuthResponse (Server → Client) — type `0x02`

| Campo       | Tipo   | Descrição                              |
|-------------|--------|----------------------------------------|
| result      | u8     | `0x00` = OK, `0x01` = credencial errada, `0x02` = ban, `0x03` = servidor cheio |
| account_id  | u32 LE | Presente apenas se result == 0x00      |
| session_key | [u8; 16] | Token de sessão (128-bit random)     |

### 4.3 CharListRequest (Client → Server) — type `0x10`

Sem payload. Enviado após AuthResponse OK.

### 4.4 CharListResponse (Server → Client) — type `0x11`

| Campo      | Tipo   | Descrição                    |
|------------|--------|------------------------------|
| count      | u8     | Número de personagens (0..9) |
| chars      | Char[] | Lista de personagens         |

**Struct Char:**

| Campo      | Tipo     | Descrição                     |
|------------|----------|-------------------------------|
| char_id    | u32 LE   | ID único                      |
| name_len   | u8       | Tamanho do nome               |
| name       | [u8]     | Nome em UTF-8 (1..=23 chars)  |
| class      | u16 LE   | JobID (0=Novice, 1=Swordsman…)|
| level      | u16 LE   | Base level (1..=99)           |
| job_level  | u16 LE   | Job level (1..=50)            |
| str        | u16 LE   | STR base                      |
| agi        | u16 LE   | AGI base                      |
| vit        | u16 LE   | VIT base                      |
| int        | u16 LE   | INT base                      |
| dex        | u16 LE   | DEX base                      |
| luk        | u16 LE   | LUK base                      |
| hp         | u32 LE   | HP atual                      |
| max_hp     | u32 LE   | HP máximo                     |
| sp         | u32 LE   | SP atual                      |
| max_sp     | u32 LE   | SP máximo                     |
| map_len    | u8       | Tamanho do nome do mapa       |
| map        | [u8]     | Nome do mapa (ex: "prontera") |
| pos_x      | u16 LE   | Posição X no mapa             |
| pos_y      | u16 LE   | Posição Y no mapa             |
| sprite     | u16 LE   | ID do sprite (hair/body)      |

### 4.5 CharSelectRequest (Client → Server) — type `0x12`

| Campo   | Tipo   | Descrição        |
|---------|--------|------------------|
| char_id | u32 LE | ID do personagem |

### 4.6 CharSelectResponse (Server → Client) — type `0x13`

| Campo        | Tipo     | Descrição                              |
|--------------|----------|----------------------------------------|
| result       | u8       | `0x00` = OK, `0x01` = erro            |
| world_host   | [u8; 64] | Endereço do WorldServer (null-padded)  |
| world_port   | u16 LE   | Porta do WorldServer                   |
| world_token  | [u8; 32] | Token JWT compacto para o WorldServer  |

### 4.7 CharCreateRequest (Client → Server) — type `0x14`

| Campo    | Tipo     | Descrição                    |
|----------|----------|------------------------------|
| name_len | u8       | Tamanho do nome              |
| name     | [u8]     | Nome em UTF-8                |
| class    | u16 LE   | JobID inicial (sempre 0)     |
| str      | u8       | Ponto inicial em STR (1..=9) |
| agi      | u8       | Ponto inicial em AGI         |
| vit      | u8       | Ponto inicial em VIT         |
| int      | u8       | Ponto inicial em INT         |
| dex      | u8       | Ponto inicial em DEX         |
| luk      | u8       | Ponto inicial em LUK         |
| sprite   | u16 LE   | Hair style/color ID          |

> Total de pontos distribuídos deve ser exatamente 6 (validado server-side).

### 4.8 CharCreateResponse (Server → Client) — type `0x15`

| Campo  | Tipo | Descrição                                   |
|--------|------|---------------------------------------------|
| result | u8   | `0x00` = OK, `0x01` = nome duplicado, etc.  |
| char   | Char | Struct Char completa (se result == 0x00)    |

---

## 5. Implementação

### 5.1 Server (`crates/hades-login`)

- Binário: `hades-login`
- Porta padrão: `6900` (configurável via `HADES_LOGIN_PORT`)
- DB: SQLite via `sqlx` (dev), Postgres (prod)
- Tabelas: `accounts(id, username, password_argon2, created_at, banned_until)`,
  `characters(id, account_id, name, class, level, ...)`

### 5.2 Client (`berenice-client/src/Network/Hades/`)

- `HadesTransport.js` — abstrai WebTransport
- `HadesProtocol.js` — encode/decode de todos os pacotes deste spec
- `LoginHandler.js` — orquestra o fluxo login → charlist → select

---

## 6. Testes Requeridos

- [ ] `test_auth_request_roundtrip` — encode/decode AuthRequest
- [ ] `test_auth_response_roundtrip` — encode/decode AuthResponse
- [ ] `test_charlist_roundtrip` — encode/decode lista de 3 personagens
- [ ] `test_charselect_roundtrip` — encode/decode CharSelectResponse
- [ ] `test_login_flow_integration` — login completo via loopback QUIC
- [ ] `test_invalid_credentials_returns_0x01`
- [ ] `test_char_creation_with_stat_overflow_rejected`
