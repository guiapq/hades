# SPEC-0009: Cliente Berenice — Entrada Multiplataforma (Gamepads, Teclado/Mouse ToS) e Motor Leve

- **Autor:** Hades Core Team & Persephone Working Group
- **Data:** 2026-10-02
- **Status:** Draft / Proposed
- **Módulo Afetado:** `crates/berenice` (Client Engine & Multi-Input Subsystem)

---

## 1. Contexto e Visão Geral

O **Berenice** é o cliente leve, responsivo e multiplataforma do ecossistema Hades. Construído em Rust puro sem dependências pesadas de ecossistemas externos (sem Node.js, sem npm, sem interpretadores Lua embutidos na interface), o Berenice visa:

1. **Acessibilidade Universal:** Executar nativamente em Linux, Windows, macOS, BSDs/Haiku e em navegadores web modernos via WebAssembly.
2. **Suporte Amplo a Controles (Gamepads):** Compatibilidade pronta para uso com controles de Xbox (XInput), PlayStation (DualShock 4, DualSense), Nintendo Switch (Pro Controller / Joy-Cons) e gamepads USB genéricos via banco de dados SDL GameControllerDB (`gilrs`).
3. **Esquema de Entrada Híbrido Estilo *Tree of Savior*:**
   - **Modo Gamepad:** Movimentação fluida em 8 direções com analógico/D-Pad, mira frontal direcional automática, botões de ação com camadas de atalho via gatilhos/bumpers (estilo Cross-Hotbar).
   - **Modo Teclado/Mouse:** Movimentação direta (WASD / Setas) ou navegação por clique (ponto a ponto), com hotkeys clássicas de 1 a 0 e F1 a F9.
4. **Integração Direta com Protocolos Hades:**
   - Consumo e envio direto do pacote compacto de 6 bytes [`MovementDelta`](../crates/hades-core/src/bitpacking.rs).
   - Conexão nativa via QUIC / WebTransport ([SPEC-0007](SPEC-0007-quic-webtransport-network-layer.md)) com zero alocações por frame de renderização.

---

## 2. Contrato de Entrada e Abstração de Controles

### 2.1 Modelo de Intenção do Jogador (`PlayerIntent`)

Para desacoplar os periféricos físicos da simulação visual, toda ação do usuário é convertida para uma estrutura de intenção:

```rust
use hades_core::types::Direction;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MovementIntent {
    pub is_moving: bool,
    pub direction: Direction,
    pub running: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionIntent {
    None,
    BasicAttack,
    Interact,
    UseSkill { slot: u8, target_direction: Direction },
    UseItem { slot: u8 },
    Cancel,
}
```

### 2.2 Mapeamento de Gamepads (Estilo *Tree of Savior*)

| Botão Físico | Ação Padrão | Com Gatilho L1/L2 Pressionado | Com Gatilho R1/R2 Pressionado |
| :--- | :--- | :--- | :--- |
| **Analógico Esquerdo / D-Pad** | Movimento em 8 direções (`Direction`) | - | - |
| **Botão Sul (A / Cruz)** | Ataque Básico / Interagir | Habilidade Slot 1 | Habilidade Slot 5 |
| **Botão Leste (B / Círculo)** | Cancelar / Salto ou Esquiva | Habilidade Slot 2 | Habilidade Slot 6 |
| **Botão Oeste (X / Quadrado)** | Habilidade Rápida 1 | Habilidade Slot 3 | Habilidade Slot 7 |
| **Botão Norte (Y / Triângulo)** | Habilidade Rápida 2 | Habilidade Slot 4 | Habilidade Slot 8 |
| **L1 / LB** | Modificador de Paleta A | - | - |
| **R1 / RB** | Modificador de Paleta B | - | - |
| **L3 (Click Stick)** | Alternar Alvo Próximo | - | - |
| **R3 (Click Stick)** | Centralizar Câmera | - | - |

---

## 3. Arquitetura do Crate `crates/berenice`

```text
crates/berenice/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── input/
    │   ├── mod.rs          // Hub de entrada unificado
    │   ├── gamepad.rs      // Abstração de controles físicos (gilrs / web gamepad)
    │   ├── keyboard.rs     // Mapeamento teclado & mouse estilo ToS
    │   └── intent.rs       // Estruturas de intenção puras
    ├── network/
    │   └── transport.rs    // Cliente WebTransport/QUIC (usando hades-net)
    ├── render/
    │   ├── mod.rs          // Backend gráfico desacoplado (wgpu/miniquad/canvas)
    │   └── grid_view.rs    // Projeção isométrica 2.5D do CollisionGrid e entidades
    └── state/
        └── world_view.rs   // Estado local da simulação (jogadores vizinhos, monstros, chão)
```

---

## 4. Invariantes de Design

- [x] **Zero Stack Maluca:** Dependências mínimas em Rust estático. Nenhum runtime Node.js, compilação de assets em tempo de execução ou interpretadores embutidos.
- [x] **Portabilidade Máxima:** Capaz de compilar tanto como binário nativo para desktop (Linux, Windows, macOS, Haiku) quanto para WebAssembly puro (navegadores com WebTransport e WebGL/WebGPU).
- [x] **Reaproveitamento do Hades:** Utilização direta dos tipos [`Position`](../crates/hades-core/src/types.rs), [`CollisionGrid`](../crates/hades-core/src/collision.rs), [`MovementDelta`](../crates/hades-core/src/bitpacking.rs) e tabelas clássicas de [`hades-ro-prere`](../crates/hades-ro-prere/src/lib.rs).
- [x] **Sensibilidade de Controle ToS:** Suporte a deadzones configuráveis em sticks analógicos e snap instantâneo para os 8 ângulos ortogonais e diagonais da grade de células.

---

## 5. Plano de Testes

1. `test_gamepad_stick_to_8_directions`: Garantir que vetores $(X, Y)$ em 360° convertem precisamente para as 8 direções do `Direction` do Hades sem zonas cegas.
2. `test_cross_hotbar_action_selection`: Validar ativação correta de habilidades sob combinações de botões modificadores (L1/R1).
3. `test_movement_intent_to_delta_packet`: Verificar a geração contínua de deltas de 6 bytes a partir da entrada do controle.
