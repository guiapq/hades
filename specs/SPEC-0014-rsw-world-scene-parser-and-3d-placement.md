# SPEC-0014: Parser de Cenas de Mundo (.rsw) e Posicionamento 3D de Modelos

- **Autor:** Hades Core Team & Berenice Graphics Group
- **Data:** 2026-10-02
- **Status:** Approved
- **Módulo Afetado:** `crates/hades-ro-prere` (`rsw_parser`) e `crates/berenice` (renderizador de cena 2.5D)

---

## 1. Contexto e Motivação

Nos motores clássicos 2.5D, um mapa completo é composto por uma trindade de arquivos complementares:
1. **`.gat` (Ground Altitude & Type):** Grade discreta de colisão ($X, Y$) e altura de células.
2. **`.gnd` (Ground Mesh):** Geometria de terreno, malhas triangulares de ladrilhos e texturas de chão.
3. **`.rsw` (Resource World):** Descritor mestre da cena. Define as referências aos arquivos `.gat`/`.gnd`, iluminação global, água e, principalmente, a lista e posicionamento tridimensional de todos os objetos e prédios 3D (`.rsm`).

A implementação do parser `.rsw` permite que a **Berenice** posicione automaticamente todos os prédios, muralhas, fontes, barracas, postes e elementos decorativos no mapa sem qualquer coordenada arbitrária ou hardcoded.

---

## 2. Especificação Canônica do Formato `.rsw` (Resource World)

A especificação de referência foi homologada a partir da implementação canônica do *roBrowser* (`Loaders/World.js` e `Loaders/MapLoader.js`).

### 2.1 Cabeçalho
- `magic: [u8; 4]` — Assinatura estrita `b"GRSW"` (`0x47, 0x52, 0x53, 0x57`).
- `major: u8` — Versão maior (ex: 2).
- `minor: u8` — Versão menor (ex: 1 $\rightarrow$ versão 2.1).
- Se versão $\ge 2.5$: `build_number: i32` (4 bytes).
- Se versão $\ge 2.2$: byte reservado (1 byte).

### 2.2 Arquivos Vinculados
- `ini_file: [u8; 40]` — Arquivo de script/ini associado (string terminada em nulo).
- `gnd_file: [u8; 40]` — Arquivo de malha de chão (`.gnd`).
- `gat_file: [u8; 40]` — Arquivo de colisão de células (`.gat`).
- Se versão $\ge 1.4$: `src_file: [u8; 40]` — Arquivo fonte/cenário.

### 2.3 Propriedades Ambientais (Água e Luz)
- **Água (se versão $< 2.6$):**
  - Se versão $\ge 1.3$: `water_level: f32` (dividido por 5.0).
  - Se versão $\ge 1.8$: `water_type: i32`, `wave_height: f32 / 5.0`, `wave_speed: f32`, `wave_pitch: f32`.
  - Se versão $\ge 1.9$: `anim_speed: i32`.
- **Iluminação (se versão $\ge 1.5$):**
  - `longitude: i32`, `latitude: i32`.
  - `diffuse: [f32; 3]`, `ambient: [f32; 3]`.
  - Se versão $\ge 1.7$: `opacity: f32`.
- **Limites de Terreno (se versão $\ge 1.6$):**
  - `top: i32`, `bottom: i32`, `left: i32`, `right: i32`.

### 2.4 Tabela de Objetos
Após os metadados do cabeçalho:
- `object_count: u32` — Quantidade de entidades na cena.
- Itera sobre cada objeto lendo seu tipo identificador `obj_type: u32`:

#### Tipo 1: Modelo 3D (`ModelObject`)
1. Se versão $\ge 1.3$:
   - `name: [u8; 40]` — Nome da instância.
   - `anim_type: i32`, `anim_speed: f32`, `block_type: i32`.
2. Se versão $\ge 2.6$ e `build_number >= 186`: 1 byte reservado.
3. Se versão $\ge 2.7$: 4 bytes reservados.
4. `filename: [u8; 80]` — Caminho relativo do modelo `.rsm` (ex: `\model\prt_k_bunsu_1.rsm`).
5. `nodename: [u8; 80]` — Nó de ancoragem.
6. `position: [f32; 3]` — Posição espacial $(x, y, z)$. Cada coordenada é dividida por 5.0 para converter de unidades brutas para células de grade.
7. `rotation: [f32; 3]` — Ângulos de rotação Euler em graus $(R_x, R_y, R_z)$.
8. `scale: [f32; 3]` — Vetor de escala $(S_x, S_y, S_z)$, dividido por 5.0.

#### Tipos 2, 3 e 4 (Luzes, Sons e Efeitos)
- Tipo 2 (Luz): `name(80) + pos(12) + color(12) + range(4) = 108 bytes`.
- Tipo 3 (Som): `name(80) + file(80) + pos(12) + vol(4) + width(4) + height(4) + range(4) + cycle(4 se >= 2.0)`.
- Tipo 4 (Efeito): `name(80) + pos(12) + type(4) + loop(4) + param(16) = 116 bytes`.

---

## 3. Conversão de Coordenadas para Espaço de Câmera 2.5D

Em mapas clássicos, o ponto de origem $(0, 0, 0)$ do arquivo `.rsw` é ancorado no **centro geométrico** do mapa:
- Centro em células: $(W / 2, H / 2)$, onde $W$ e $H$ são as dimensões do arquivo `.gat` (ex: Prontera possui $W = 312$ e $H = 392$, logo centro em $(156, 196)$).
- A posição em células da grade de um modelo é dada por:
  $$C_x = \frac{W_{\text{gat}}}{2} + \text{pos}_x$$
  $$C_y = \frac{H_{\text{gat}}}{2} + \text{pos}_z$$
  $$C_z = -\text{pos}_y$$

---

## 4. Otimização de Performance e "Potato Budget"

Prontera possui mais de 1.300 modelos 3D registrados no `.rsw`. Para manter 60 FPS contínuos em CPU econômica:
1. **Frustum / Culling por Raio (AoI):** Apenas modelos dentro do raio visível da câmera (ex: $R \le 35$ células da posição do jogador) são submetidos ao renderizador de software.
2. **Cache de Modelos em Memória:** Modelos repetidos (árvores, vasos de flores, postes de luz) são parseados apenas uma vez e armazenados em cache imutável (`Arc<RsmModel>`).
3. **Zero Alocações no Loop:** As listas de vértices transformados reutilizam buffers fixos.
