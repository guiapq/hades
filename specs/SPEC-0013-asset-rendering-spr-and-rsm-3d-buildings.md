# SPEC-0013: Renderização de Sprites 2D (.spr) e Prédios 3D (.rsm / .rsw)

- **Autor:** Hades Core Team & Berenice Graphics Group
- **Data:** 2026-10-02
- **Status:** Approved
- **Módulo Afetado:** `crates/hades-ro-prere` (parsers de assets) e `crates/berenice` (renderizador 2.5D)

---

## 1. Contexto e Motivação

Atualmente, o cliente **Berenice** renderiza as células do terreno com base no `CollisionGrid` (.gat) e representa entidades como tokens geométricos simples com indicadores vetoriais de face.

Para atingir a fidelidade visual autêntica do motor clássico 2.5D, integramos o pipeline de assets extraídos diretamente do VFS (.grf ou pastas abertas):
1. **Sprites 2D (`.spr`):** Representação rasterizada de personagens, monstros, NPCs e sombras do chão com transparência de paleta (*color key* / alpha).
2. **Modelos 3D de Prédios e Estruturas (`.rsm`):** Malhas poligonais 3D (casas, templos, arcos, muralhas, fontes) posicionadas no espaço tridimensional.
3. **Mapeamento de Objetos do Mundo (`.rsw`):** Localização espacial $(X, Y, Z)$, rotação Euler e escala de cada prédio e objeto espalhado pelo mapa (ex: Prontera possui centenas de prédios, bancos e a fonte central `prt_k_bunsu_1.rsm`).

---

## 2. Especificação do Formato de Sprite (`.spr`)

### 2.1 Cabeçalho
- `magic: [u8; 2]` — Esperado `b"SP"` (0x50, 0x53).
- `version: u16` — Versão do formato (0x0101, 0x0200, 0x0201).
- `indexed_count: u16` — Número de quadros que utilizam a paleta de 256 cores.
- `rgba_count: u16` (se versão $\ge 0x0200$) — Número de quadros em cores diretas RGBA.

### 2.2 Quadros Indexados
Para cada quadro indexado:
- `width: u16`, `height: u16`
- **Dados de Pixels:**
  - Se versão $< 0x0201$: $W \times H$ bytes contíguos de índices na paleta.
  - Se versão $\ge 0x0201$ (RLE): Quando o byte é `0x00`, o próximo byte indica o número de repetições consecutivas do pixel transparente (índice 0).
- **Paleta de Cores (fim do arquivo):**
  - Bloco final de 1.024 bytes contendo 256 entradas RGBA (4 bytes por cor: `[R, G, B, A]`).
  - O índice 0 é reservado como **transparente** (alpha = 0).

---

## 3. Especificação de Modelos 3D (`.rsm`) e Posicionamento (`.rsw`)

### 3.1 Modelo 3D Estático (`.rsm`)
- Cabeçalho mágico: `b"GRSM"` (versão 1.x ou 2.x).
- Texturas: Lista de nomes de arquivos de imagem (.bmp/.tga).
- Nós de Malha (Nodes):
  - Nome do nó (ex: `root`, `body`, `mesh`).
  - Vértices 3D: Array de coordenadas flutuantes $(x, y, z)$.
  - Faces Triangulares: Índices de 3 vértices $(v_0, v_1, v_2)$, vetor normal e mapeamento de textura UV.

### 3.2 Posicionamento no Mundo (`.rsw`)
- Cabeçalho mágico: `b"GRSW"` (versões 1.x a 2.x).
- Contém a lista de objetos do cenário:
  - Tipo 1 (Model): Caminho do modelo `.rsm`, posição $(X, Y, Z)$ em coordenadas de mundo, rotação Euler em graus e escala $(S_x, S_y, S_z)$.

---

## 4. Pipeline de Projeção 3D $\rightarrow$ 2.5D com Câmera Rotacionável

Para cada vértice $V = (x, y, z)$ de um prédio 3D posicionado em $(W_x, W_y, W_z)$:
1. **Transformação de Modelo para Mundo:**
   $$P_{\text{world}} = \text{Rotate}_{\text{model}}(V) \cdot \text{Scale} + (W_x, W_y, W_z)$$
2. **Transformação da Câmera (Yaw e Zoom):**
   Transladamos em relação à câmera e aplicamos a rotação de ângulo da câmera $\theta_{\text{yaw}}$:
   $$\Delta x = P_x - \text{Cam}_x, \quad \Delta y = P_y - \text{Cam}_y, \quad \Delta z = P_z$$
   $$rx = \Delta x \cos(\theta) - \Delta y \sin(\theta)$$
   $$ry = \Delta x \sin(\theta) + \Delta y \cos(\theta)$$
3. **Projeção Isométrica para Tela:**
   $$\text{screen}_x = (rx \cdot \text{tile\_width} \cdot \text{zoom}) + \text{Center}_x$$
   $$\text{screen}_y = (-ry \cdot \text{tile\_height} \cdot \text{zoom}) - (\Delta z \cdot \text{height\_scale} \cdot \text{zoom}) + \text{Center}_y$$
4. **Rasterização Scanline:**
   Triângulos projetados na tela passam pelo rasterizador de preenchimento `draw_triangle` com shading de iluminação ou textura, mantendo **zero alocações no loop de renderização**.

---

## 5. Invariantes de Execução e Performance

- **Zero Crash em Assets Corrompidos:** Parsers seguros com proteção estrita contra overflow de buffers e ponteiros fora dos limites.
- **Potato Budget:** Modelos e sprites distantes fora do raio de visão da tela são descartados sumariamente por culling de frustum 2.5D.
- **Compatibilidade Completa com Rotação da Câmera:** Sprites 2D e prédios 3D giram harmoniosamente com o mouse e analógico regulados no cliente Berenice.
