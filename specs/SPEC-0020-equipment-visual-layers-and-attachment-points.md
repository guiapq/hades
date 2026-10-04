# SPEC-0020: Camadas Visuais de Equipamentos e Pontos de Ancoragem (Attach Points)

- **Autor:** Hades Core Team & Berenice Client Working Group
- **Data:** 2026-10-03
- **Status:** Approved
- **Módulo Afetado:** `crates/berenice` (`render`, `vfs`, `bin/berenice.rs`), `crates/hades-ro-prere` (`act_parser`)

---

## 1. Contexto e Motivação

No modelo canônico de MMORPGs 2.5D de herança clássica, os avatares dos personagens não são compostos por uma malha monolítica rígida, mas sim por uma composição dinâmica de camadas 2D independentes (multi-layer composite rendering):
1. **Sombra no Solo (`shadow.spr`):** Ancorada na base da célula no chão.
2. **Corpo (`body_act` / `body_spr`):** Define a silhueta principal, o tronco, membros e os ciclos de ação (andar, repouso, combate).
3. **Cabeça (`head_act` / `head_spr`):** Cabelo e feições faciais do personagem.
4. **Equipamentos de Cabeça / Elmos (`headgear_act` / `headgear_spr`):** Chapéus, tiaras e elmos posicionados sobre o topo da cabeça.
5. **Armas (`weapon_act` / `weapon_spr`):** Espadas, machados e adagas sincronizados com os ciclos de ataque e ação da classe.

Para que as camadas não fiquem "flutuando" ou dessincronizadas durante a oscilação natural do corpo ao caminhar (*bobbing*), o motor deve aplicar os **Pontos de Ancoragem (`AttachPoint`)** extraídos dos arquivos `.act` (conforme especificado na [SPEC-0019](SPEC-0019-act-animation-parser-and-cycles.md)).

---

## 2. Modelo Matemático de Ancoragem de Camadas (Attachment Points)

### 2.1 Ancoragem Cabeça-ao-Corpo
Cada quadro do corpo (`ActFrame`) expõe uma lista de pontos de ancoragem `attach_points: Vec<AttachPoint>`.
- `AttachPoint 0` do corpo representa o pescoço/junção da cabeça $(ax_{body}, ay_{body})$.
- `AttachPoint 0` da cabeça representa o pivô de montagem do pescoço $(ax_{head}, ay_{head})$.

O deslocamento relativo para desenhar a cabeça é calculado por:
$$\Delta x_{head} = ax_{body} - ax_{head}$$
$$\Delta y_{head} = ay_{body} - ay_{head}$$

Quando o corpo caminha e seu tronco se inclina ou desce 2 pixels em um determinado frame, $(ax_{body}, ay_{body})$ varia automaticamente, fazendo com que a cabeça acompanhe o movimento com fidelidade pixel-perfect.

### 2.2 Ancoragem Elmo-à-Cabeça (Headgear)
Os elmos e acessórios de cabeça (`headgear`) são ancorados em relação ao pivô da cabeça:
$$\Delta x_{gear} = \Delta x_{head} + (ax_{head} - ax_{gear})$$
$$\Delta y_{gear} = \Delta y_{head} + (ay_{head} - ay_{gear})$$

### 2.3 Sincronização de Armas
Os arquivos `.act` e `.spr` de armas específicas de cada classe (ex: `검사_남_검.act` e `검사_남_양손검.act` para o espadachim masculino) possuem **104 ações**, espelhando exatamente a mesma tabela de índices de ação do corpo:
- **Idle Relaxado (Ações 0..7):** Repouso com arma embainhada e postura relaxada.
- **Walk (Ações 8..15):** Caminhada padrão.
- **Combat Ready / Battle Idle (Ações 32..39):** Postura de combate com arma empunhada em guarda.
- **Attack 1 / Golpe de Espada (Ações 80..87):** Golpe e corte com a espada (sequência dinâmica de 9 quadros com efeito de lâmina/slash).

A arma utiliza a mesma taxa de avanço de tempo e o mesmo índice de ação relativo à câmera (`action_idx`), permitindo composição direta sem necessidade de interpolação complexa.

---

## 3. Ordem de Composição e Z-Ordering

A profundidade relativa das camadas depende da orientação do personagem na perspectiva da câmera:

1. **Visão Frontal / Sul (Direções 0, 1, 7 - South, SouthWest, SouthEast):**
   - Sombra $\rightarrow$ Corpo $\rightarrow$ Cabeça $\rightarrow$ Elmo $\rightarrow$ Arma Frontal.
2. **Visão Traseira / Norte (Direções 3, 4, 5 - North, NorthWest, NorthEast):**
   - Sombra $\rightarrow$ Arma Traseira $\rightarrow$ Corpo $\rightarrow$ Cabeça $\rightarrow$ Elmo.
3. **Visão Lateral (Direções 2, 6 - West, East):**
   - Sombra $\rightarrow$ Corpo $\rightarrow$ Cabeça $\rightarrow$ Elmo $\rightarrow$ Arma (com espelhamento horizontal conforme matriz de rotação da câmera).

---

## 4. Engenharia e Restrições de Desempenho (Potato Budget)

1. **Zero Alocações na Render Loop:**
   - As estruturas de dados das camadas adicionais (`CharacterVisualLayers`) são carregadas uma única vez na inicialização ou quando o equipamento do jogador/entidade é alterado.
   - O cálculo das coordenadas de âncora $(\Delta x, \Delta y)$ opera exclusivamente com aritmética de inteiros primitiva no stack.
2. **Blit Otimizado com Clipping de Tela:**
   - Sub-sprites indexados utilizam a paleta do próprio `.spr` com salto de cor transparente (índice 0).
3. **Persistência de Memória:**
   - O conjunto completo de camadas de um personagem (Corpo + Cabeça + Elmo + Arma) consome menos de 4 MB de RAM na memória de vídeo/framebuffer.

---

## 5. Casos de Teste e Validação

1. **`test_attachment_point_delta_calculation`:**
   - Valida que um quadro com âncora de corpo $(1, -56)$ e âncora de cabeça $(1, -56)$ resulta em delta nulo $(0, 0)$.
   - Valida que um deslocamento de caminhada $(1, -54)$ resulta em delta $(0, 2)$.
2. **`test_headgear_composite_layering`:**
   - Renderiza corpo, cabeça e elmo sintéticos em um framebuffer de teste e valida que os pixels de cada camada se sobrepõem na ordem Z correta.
3. **`test_weapon_action_index_synchronization`:**
   - Valida que o índice de ação da arma acompanha a ação do corpo para todas as 8 direções visuais.
