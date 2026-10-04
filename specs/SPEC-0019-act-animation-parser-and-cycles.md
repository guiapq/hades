# SPEC-0019: Parser de Ações/Animações (.act) e Ciclos de Movimento/Repouso

- **Autor:** Hades Core Team & Berenice Client Working Group
- **Data:** 2026-10-03
- **Status:** Approved
- **Módulo Afetado:** `crates/hades-ro-prere` (`act_parser`), `crates/berenice` (Rendering & Animation Engine)

---

## 1. Contexto e Motivação

Em jogos de MMORPG 2.5D clássicos com perspectiva isométrica, a renderização de personagens e monstros baseia-se em uma arquitetura desacoplada em duas partes complementares:
1. **Sprites 2D (`.spr`):** Contêm os bitmaps brutos indexados por paleta de 256 cores (RLE) ou imagens diretas RGBA (especificado na [SPEC-0013](README.md)).
2. **Arquivos de Ação (`.act`):** Metadados estruturados que governam o comportamento dinâmico dos sprites. Definem as sequências de quadros de cada ação (repouso, caminhada, sentar, ataque, sofrer dano), os recortes (clips/layers) a desenhar, espelhamento horizontal (mirroring), escalas, taxas de atraso (delays em milissegundos) e pontos de ancoragem/acoplamento (attach points para cabeça, chapéus e armas).

Até o momento, o cliente nativo `Berenice` renderizava um frame estático de sprite de teste (Fairy Soldier). Esta especificação introduz o parser completo e de alto desempenho de arquivos `.act`, bem como os ciclos contínuos de animação para as 8 direções cardeais e colaterais (Idle e Walk) utilizando o arquétipo do espadachim clássico (Swordsman).

---

## 2. Especificação do Formato Binário `.act`

O arquivo `.act` utiliza formatação Little-Endian e compreende as versões `0x0200` até `0x0205`.

### 2.1 Cabeçalho (16 bytes)
| Campo | Tipo | Descrição |
|---|---|---|
| `magic` | `[u8; 2]` | Assinatura mágica: `b"AC"` (`0x41, 0x43`) |
| `version` | `u16` | Versão empacotada: byte inferior é subversão, byte superior é versão principal (ex: `0x0205` = 2.5) |
| `action_count` | `u16` | Total de ações disponíveis (ex: 104 para corpos jogáveis, 8 a 24 para NPCs) |
| `reserved` | `[u8; 10]` | Bytes reservados de preenchimento |

### 2.2 Estrutura de Ações e Quadros
Para cada uma das `action_count` ações:
- `frame_count`: `u32` (quantidade de quadros da animação nesta ação).
- Para cada quadro `0..frame_count`:
  - `range_bounds`: 32 bytes (delimitadores de colisão/caixa de seleção da ação).
  - `clip_count`: `u32` (número de sub-sprites/recortes sobrepostos que compõem este quadro).
  - Para cada recorte `0..clip_count`:
    - `offset_x`: `i32` (deslocamento horizontal em relação à origem do personagem).
    - `offset_y`: `i32` (deslocamento vertical em relação à origem do personagem).
    - `spr_index`: `i32` (índice do quadro no `.spr`; valor `-1` denota recorte vazio/invisível).
    - `mirror`: `i32` (`1` para inverter horizontalmente/espelhar, `0` para normal).
    - Se `version >= 2.0`:
      - `color`: `[u8; 4]` (modulação de cor RGBA).
      - `scale_x`: `f32` (fator de escala horizontal).
      - `scale_y`: `f32` (se `version <= 2.3`, herda `scale_x`; se `version > 2.3`, lido diretamente).
      - `angle`: `i32` (rotação angular em graus).
      - `spr_type`: `i32` (`0` para indexed palette, `1` para RGBA).
      - Se `version >= 2.5`:
        - `width`: `i32` e `height`: `i32` (dimensões forçadas de renderização).
  - `sound_id`: `i32` (id de efeito sonoro acionado no quadro, ou `-1` se ausente; para `version >= 2.0`).
  - Pontos de Ancoragem (`Attach Points`):
    - Se `version >= 2.3`:
      - `attach_count`: `i32`.
      - Para cada ponto `0..attach_count`: 16 bytes compostos por `4 bytes reservados`, `pos_x: i32`, `pos_y: i32`, `4 bytes reservados`.

### 2.3 Tabela de Delays de Ação
Ao término dos dados de animação:
- Se `version >= 2.1`: tabela de nomes de arquivos de som (`sound_count: i32`, seguido por blocos de 40 bytes).
- Se `version >= 2.2`: para cada ação `0..action_count`, um `f32` multiplicador. A duração base em milissegundos por quadro é expressa por $\text{delay\_ms} = \text{delay\_float} \times 25.0$.

---

## 3. Mapeamento de Direções e Ações

Em conformidade com a convenção clássica de 8 vias do motor e com a [SPEC-0017](SPEC-0017-seamless-mouse-pathfinding-and-navigation.md):

| Direção Hades | Delta $(dx, dy)$ | Índice Clássico ACT |
|---|---|---|
| `Direction::South` | $(0, -1)$ | 0 |
| `Direction::SouthWest` | $(-1, -1)$ | 1 |
| `Direction::West` | $(-1, 0)$ | 2 |
| `Direction::NorthWest` | $(-1, 1)$ | 3 |
| `Direction::North` | $(0, 1)$ | 4 |
| `Direction::NorthEast` | $(1, 1)$ | 5 |
| `Direction::East` | $(1, 0)$ | 6 |
| `Direction::SouthEast` | $(1, -1)$ | 7 |

### 3.1 Seleção de Ação para Corpos Padrão (104 Ações)
- **Modo Parado (Idle / Stand):**
  $$\text{action\_idx} = 0 + \text{act\_dir}$$
- **Modo Em Movimento (Walk):**
  $$\text{action\_idx} = 8 + \text{act\_dir}$$
- **Modo Sentado (Sit):**
  $$\text{action\_idx} = 16 + \text{act\_dir}$$

---

## 4. Renderização Composta (Corpo + Cabeça) e Zero Alocações

1. **Avanço de Quadro:**
   Calculado com base no tempo decorrido $\Delta t$ em milissegundos:
   $$\text{frame\_idx} = \left( \left\lfloor \frac{\text{elapsed\_ms}}{\text{delay\_ms}} \right\rfloor \right) \bmod \text{frame\_count}$$
2. **Composição em Camadas:**
   - **Sombra:** Desenha elipse de sombra base no solo.
   - **Corpo:** Consulta o recorte do quadro atual da ação ativa no `.act` do corpo e blita o pixel buffer do `.spr` com inversão horizontal caso `mirror == 1`.
   - **Cabeça:** Consulta a mesma ação e direção no `.act` da cabeça, sobrepondo os pixels alinhados pelo offset natural do frame.
3. **Potato Budget:**
   - As estruturas `Act` e `Sprite` são mantidas pré-decodificadas em memória (read-only buffers).
   - O loop de desenho a 60 FPS realiza apenas iteração direta e cópia de pixels com teste de alfa/cor transparente sobre o `SoftwareFramebuffer`, sem nenhuma alocação dinâmica (`Box`, `Vec`, `String`).
