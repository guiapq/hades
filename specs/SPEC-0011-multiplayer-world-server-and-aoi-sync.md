# SPEC-0011: Servidor Mundial Multi-Sessão e Sincronização AoI em Tempo Real (QUIC TLS 1.3)

- **Autor:** Hades Core Team
- **Data:** 2026-10-02
- **Status:** Approved
- **Módulo Afetado:** `crates/hades-net` e `crates/berenice`

---

## 1. Contexto e Motivação

Com o cliente **Berenice** renderizando a 60 FPS, colidindo com a grade do mapa real de Prontera e com a câmera ajustável funcionando, o próximo passo arquitetural é fechar o ciclo multiplayer em tempo real.

O servidor Hades deve suportar múltiplos clientes conectados simultaneamente sob o **Potato Budget** (500 CCU em 1 vCPU / 512MB RAM):
1. **Atribuição Atômica de `EntityId`:** Cada cliente autenticado recebe um ID único incremental ($[1..2^{32}-1]$).
2. **Registro no Grid Espacial de Buckets ($32 \times 32$ tiles):** O jogador é alocado no seu respectivo bucket no [`SpatialGrid`](../crates/hades-core/src/aoi.rs).
3. **Ingestão Contínua de Datagramas Não-Bloqueantes:** Datagramas de 6 bytes (`MovementDelta`) são recebidos a taxas de até 20 Hz por jogador.
4. **Multicast de Área de Interesse (AoI $3 \times 3$ Buckets):** O servidor despacha o pacote de 6 bytes apenas para clientes cujos avatares estão no raio de $3 \times 3$ buckets (bloco de $96 \times 96$ tiles). Jogadores em outros quadrantes não recebem tráfego ($O(1)$ multicast).
5. **Recepção e Renderização no Cliente:** O cliente Berenice recebe datagramas de entidades remotas em background, atualiza seu `WorldView` e renderiza os avatares remotos na tela com cores diferenciadas e indicador de direção.

---

## 2. Arquitetura do `WorldServer`

```
                                  [Hades WorldServer]
                                           │
             ┌─────────────────────────────┼─────────────────────────────┐
             ▼                             ▼                             ▼
       [Cliente A]                   [Cliente B]                   [Cliente C]
   (Pos: 156, 180)               (Pos: 158, 182)               (Pos: 20, 20)
    [Bucket (4, 5)]               [Bucket (4, 5)]               [Bucket (0, 0)]
             │                             │                             │
             └── (Mesmo AoI 3x3: Sincronizam) ──┘                             └── (AoI Distante: 0 tráfego)
```

### 2.1 Estrutura da Sessão do Jogador (`PlayerSession`)
Cada conexão ativa mantém:
- `entity_id: EntityId`
- `position: Position`
- `facing: Direction`
- `connection: quinn::Connection`
- `username: String`

### 2.2 Ciclo de Vida da Conexão
1. **Handshake & Auth:** Conexão QUIC TLS 1.3 estabelecida; troca de `AuthRequest` / `AuthResponse` em stream confiável.
2. **Spawn Notifier:** O servidor envia via stream inicial o `EntityId` atribuído ao cliente e as entidades já presentes no seu AoI $3 \times 3$.
3. **Loop de Datagramas (20 Hz):**
   - Cliente envia `MovementDelta` (6 bytes).
   - Servidor valida limites e colisão com o `CollisionGrid`.
   - Servidor atualiza a posição do jogador no `SpatialGrid`.
   - Servidor localiza todos os clientes nos 9 buckets do AoI e despacha o datagrama de 6 bytes.
4. **Desconexão:** Ao encerrar conexão, o jogador é removido do `SpatialGrid`, e um datagrama de despawn (ou encerramento) é transmitido aos observadores locais.

---

## 3. Invariantes de Desempenho e Segurança

1. **Zero Heap Allocations no Multicast:** Buffers fixos na pilha para iterar sobre os 9 buckets ($3 \times 3$) e despachar datagramas.
2. **Potato Budget:** Consumo máximo de memória por sessão de jogador $\le 10$ KB.
3. **Resistência a Packet Loss:** Por usar datagramas não-confiáveis para movimento, perdas de pacotes intermediários não causam head-of-line blocking (HOLB) na transmissão da simulação.
4. **Discretion Policy:** Nenhuma menção a marcas ou títulos comerciais em comentários ou documentação.

---

## 4. Plano de Implementação

1. **`hades-net::world`:**
   - Criar `WorldSessionManager` gerenciando sessões ativas, mapeamento de `EntityId -> Connection` e integração com `SpatialGrid`.
   - Adicionar método `broadcast_aoi_movement(&self, delta: &MovementDelta, source_id: EntityId)` enviando datagramas para conexões no mesmo bloco $3 \times 3$.
2. **`berenice::network`:**
   - Adicionar receptor de datagramas em background (`receive_datagrams`) que alimenta o `WorldView`.
3. **`berenice::render`:**
   - Renderizar entidades remotas no `SoftwareFramebuffer` (`draw_entity_token` com `is_local_player: false`).
4. **Testes Unitários e de Integração:**
   - Testar 2 clientes simultâneos conectados ao `WorldServer`: Cliente A se move, Cliente B recebe o `MovementDelta` de 6 bytes e atualiza seu `WorldView`.
   - Testar cliente fora do raio de AoI: zero pacotes recebidos.
