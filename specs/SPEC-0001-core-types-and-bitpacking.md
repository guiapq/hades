# SPEC-0001: Tipos Fundamentais, Bitpacking de Movimento e Grid de Colisão Compacto

- **Autor:** Hades Core Team
- **Data:** 2026-10-02
- **Status:** Approved
- **Módulo Afetado:** Core / Simulation / Network Protocol

---

## 1. Contexto e Motivação

Para honrar o **Potato Budget** (500 jogadores em 1 vCPU e < 50MB de RAM) e os princípios de **Operações Unitárias**, o motor do Hades necessita de primitivas fundamentais que:
1. Sejam puras, sem dependências de I/O ou banco de dados.
2. Não realizem nenhuma alocação de memória na heap durante a execução.
3. Permitam serialização e desserialização binária compacta de **6 bytes por pacote de movimento**, consumindo o mínimo de largura de banda e zero overhead de parsing (zero-copy).
4. Forneçam verificação de colisão espacial em tempo constante $O(1)$ usando bitsets densos.

---

## 2. Contrato de Dados (Layout & Bitpacking)

### 2.1 Identificadores e Coordenadas
* `EntityId`: Inteiro sem sinal de 16 bits (`u16`). Suporta até 65.535 entidades ativas simultâneas por setor/instância.
* `Position`: Coordenadas discretas $X$ e $Y$, cada uma representada em **12 bits** (intervalo de $0$ a $4095$ tiles):
  * $4096 \times 4096$ tiles cobre com folga os maiores mapas clássicos de MMORPGs 2D/2.5D (que raramente excedem $512 \times 512$).
* `Direction`: 8 direções cardeais e colaterais (N, NE, E, SE, S, SW, W, NW), codificadas em 3 bits (armazenadas em `u8`).

### 2.2 Layout dos 6 Bytes do `MovementDelta`

```
Byte 0..1: [ Entity ID (u16 Little Endian)                                  ]
Byte 2:    [ PosX [0..7] (8 bits mais baixos de X)                          ]
Byte 3:    [ PosX [8..11] (4 bits altos) ] | [ PosY [0..3] (4 bits baixos)  ]
Byte 4:    [ PosY [4..11] (8 bits mais altos de Y)                          ]
Byte 5:    [ Direction (3 bits) | Action Flags (5 bits)                     ]
Total: Exatamente 6 bytes (48 bits).
```

### 2.3 Grid de Colisão por Bitset (`CollisionGrid`)
* Em vez de alocar um array de structs ou inteiros por tile (o que consumiria megabytes), o mapa usa **1 bit por célula** para indicar se o terreno é andável (`1 = walkable`, `0 = blocked`).
* Um mapa de $1024 \times 1024$ consome exatamente:
  $$\frac{1024 \times 1024}{8 \times 1024} = 128\text{ KB de RAM}$$
* A consulta de colisão de uma coordenada $(x, y)$ é resolvida em $O(1)$ por meio de operações de bitwise shift (`>>`) e máscara (`&`).

---

## 3. Operações Unitárias e Métodos

1. `MovementDelta::encode(&self) -> [u8; 6]`
   * Transforma a struct em um buffer contíguo de 6 bytes sem alocações.
2. `MovementDelta::decode(bytes: &[u8; 6]) -> Result<Self, DecodeError>`
   * Reconstitui o delta a partir de 6 bytes brutos recebidos da rede com validação de limites (X < 4096, Y < 4096).
3. `CollisionGrid::is_walkable(&self, pos: Position) -> bool`
   * Checagem puramente aritmética em registradores de CPU.

---

## 4. Invariantes e Regras de Segurança

- [x] **Zero Heap Allocations:** As funções de codificação, decodificação e consulta de colisão não chamam o alocador do sistema operacional (`malloc`/`free`/`Box`).
- [x] **Limites de Coordenadas:** Valores de $X$ e $Y \ge 4096$ retornam erro de validação imediato, prevenindo buffer overflow ou estouro de bitset.
- [x] **Determinismo Estrito:** Codificar e decodificar qualquer tupla de dados válida preserva os dados de forma 100% idêntica e reversível (*lossless round-trip*).

---

## 5. Suíte de Testes Unitários Obrigatórios

1. `test_movement_delta_roundtrip`: Testa se qualquer entidade codificada em 6 bytes decodifica exatamente com os mesmos campos.
2. `test_movement_delta_edge_coordinates`: Testa os limites extremos ($0, 0$), ($4095, 4095$).
3. `test_movement_delta_rejects_out_of_bounds`: Rejeita coordenadas inválidas $\ge 4096$.
4. `test_collision_grid_boundaries`: Valida se células bloqueadas e livres respondem corretamente.
5. `test_potato_memory_size`: Assegura via `std::mem::size_of` que as structs mantêm o tamanho em bytes especificado.
