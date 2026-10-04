# SPEC-0007: Camada de Transporte de Rede QUIC / WebTransport e Multiplexação de Canais (`hades-net`)

- **Autor:** Hades Core Team
- **Data:** 2026-10-02
- **Status:** Approved
- **Módulo Afetado:** `crates/hades-net` (Transport & Network I/O)

---

## 1. Contexto e Motivação

Conforme o [Documento de Arquitetura](../../docs/ARCHITECTURE.md), os servidores de MMORPGs legados dos anos 2000 foram construídos sobre TCP síncrono, sofrendo severamente de **Head-of-Line (HoL) Blocking**: quando um único pacote de movimento é perdido em uma rota com perda de pacotes, toda a fila do TCP congela até a confirmação de retransmissão.

O Hades rompe com essa arquitetura adotando **QUIC e WebTransport (sobre UDP)**:
1. **Zero Head-of-Line Blocking:** A perda de um pacote de movimento não atrasa uma mensagem de chat ou uma compra em NPC.
2. **Datagramas Não-Confiáveis (Unreliable Datagrams):** Utilizados para posições e deltas de movimento de 6 bytes ([SPEC-0001](SPEC-0001-core-types-and-bitpacking.md)). Perdas são descartadas, pois o próximo tick (50ms) enviará o estado mais recente.
3. **Streams Bidirecionais Confiáveis (Reliable Streams):** Utilizados para login, inventário, troca de itens e chat.
4. **Criptografia TLS 1.3 Integrada:** Conexões autenticadas e seguras nativamente, compatíveis tanto com clientes Desktop (Godot/Unity) quanto Web (WebAssembly/WebTransport via navegador).

---

## 2. Contrato de Dados e Estrutura de Mensagens

### 2.1 Separação Rígida de Canais
```
+-------------------------------------------------------------+
|                   Hades Net Transport                       |
|                                                             |
|   +-----------------------+     +-----------------------+   |
|   |   Reliable Streams    |     | Unreliable Datagrams  |   |
|   | (Auth, Chat, Items)   |     | (MovementDelta - 6B)  |   |
|   +-----------------------+     +-----------------------+   |
|               \                             /               |
|                v                           v                |
|                  QUIC / WebTransport Endpoint               |
|                           (UDP Socket)                      |
+-------------------------------------------------------------+
```

### 2.2 Mensagens Confiáveis de Controle (`ReliablePacket`)
Codificação binária simples com discriminador de tipo (`u8`):
* `0x01 - AuthRequest`: Identificação de sessão e conta.
* `0x02 - AuthResponse`: Confirmação de login e permissões.
* `0x03 - ChatMessage`: Mensagens de chat (canal global, grupo ou privado).
* `0x04 - SystemNotification`: Alertas de servidor e mensagens de erro.

### 2.3 Datagramas Não-Confiáveis de Gameplay (`DatagramPayload`)
* Empacotamento direto do `MovementDelta` de 6 bytes.
* Suporte a envio em lote (*batching*): múltiplos deltas consolidados em um único datagrama UDP quando necessário ($6 \times N$ bytes).

---

## 3. Operações e Métodos do Servidor

1. `HadesServer::bind(addr: SocketAddr) -> Result<Self, NetError>`
   * Inicializa o socket UDP QUIC com TLS 1.3 e configuração de datagramas habilitada.
2. `HadesServer::generate_self_signed_cert() -> (CertificateDer, PrivateKeyDer)`
   * Geração de certificados X.509 em memória com `rcgen` para desenvolvimento local sem dependências de OpenSSL.
3. `HadesConnection::send_movement_datagram(delta: MovementDelta) -> Result<(), NetError>`
   * Envio imediato do delta de 6 bytes no canal não-bloqueante.
4. `HadesConnection::send_reliable_message(packet: &ReliablePacket) -> Result<(), NetError>`
   * Envio garantido e ordenado através de stream multiplexada.

---

## 4. Invariantes e Regras de Segurança

- [x] **Zero Head-of-Line Blocking no Movimento:** O canal de datagramas nunca bloqueia aguardando confirmação (ACK) do cliente.
- [x] **Criptografia Obrigatória:** 100% do tráfego trafega criptografado via TLS 1.3 nativo do QUIC.
- [x] **Desacoplamento do Tick:** A rede recebe pacotes e os enfileira em filas atômicas. O tick de simulação nunca espera por I/O de rede.
- [x] **Compatibilidade com o Potato Budget:** Sem threads concorrentes por jogador; o runtime assíncrono consome recursos mínimos.

---

## 5. Suíte de Testes Obrigatórios

1. `test_tls_certificate_generation`: Valida a geração em memória de certificados autoassinados para QUIC.
2. `test_server_bind_and_endpoint_creation`: Assegura que o servidor sobe o socket UDP com datagramas habilitados.
3. `test_loopback_connection_and_datagram_exchange`: Conecta um cliente de teste ao servidor em `127.0.0.1` e envia o pacote de 6 bytes de `MovementDelta`.
4. `test_reliable_stream_multiplexing`: Valida o envio e recebimento de mensagens de chat ou autenticação no canal confiável.
