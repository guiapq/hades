# SPEC-0028: Texturização de Solo (.gnd + .bmp), Interface Despoluída e Mundo Vivo (docs/VALVE.md)

## 1. Contexto e Motivação
Em conformidade com a filosofia entronada em `docs/VALVE.md` (Game Feel, clareza visual sem poluição, loop de exploração, feedback contínuo em toda interação e design não-hostil a neurodivergentes) e a política de discrição técnica de `AGENTS.md`, o cliente Berenice foi expandido em quatro pilares fundamentais:
1. **Piso e Terreno Fiel com Texturas Reais:** Substituição da grade plana por texturas autênticas de pedra e calçamento de vila extraídas de arquivos `.gnd` e imagens `.bmp` do VFS.
2. **Interface Despoluída e Respirável (Zero Overlap):** Eliminação de sobreposições visuais entre painéis. Métricas de desenvolvedor foram movidas para ativação sob demanda via `[F3]`, substituídas por um badge discreto de localização no topo esquerdo. HUD do Jogador e Minimapa foram organizados verticalmente no topo direito, a Target Info Bar centralizada no topo e a Hotbar posicionada na margem inferior.
3. **Mundo Vivo e Interativo (Living World):** Borrifos de água contínuos da fonte central, respingos ao interagir com água, partículas sutis de luz ambiente (motes) e Guia NPC que gira a face acompanhando o jogador por proximidade.
4. **Comunicação Orgânica Não-Modal:** Balões de fala estilizados que flutuam sobre entidades sem criar pausas ou travas modais de tela.

## 2. Decodificação de Texturas BMP (.bmp)
O módulo `bmp_parser` implementa suporte limpo e sem dependências externas para formatos BMP comumente encontrados nos pacotes legados:
- **BMP de 8 bits indexado (Paleta):** 256 cores ARGB, com suporte à conversão de índice e detecção de cor magenta (`#FF00FF`) para transparência.
- **BMP de 24 bits (RGB888):** Com padding de 4 bytes por scanline e inversão de linhas (bottom-up / top-down).
- **Amostragem UV Clamped:** Função `sample_uv(u, v)` com normalização contínua `0.0..=1.0`.

## 3. Parser de Malha de Terreno (.gnd)
O formato binário `.gnd` (versão 1.7) contém:
- Cabeçalho `GRGN`, dimensões do grid `(width, height)` e escala de zoom.
- Tabela de nomes de texturas codificadas em EUC-KR / CP949.
- Lista de ladrilhos (`GndTile`), cada um com 4 coordenadas `u` e `v` que determinam o recorte da textura.
- Matriz de células `(x, y)` com índices de superfície superior (chão), frontal e lateral.
- Mapeamento direto com o grid GAT: cada célula GND equivale exatamente a um bloco 2x2 do grid de colisão GAT.

## 4. Mapeamento de Textura Isométrica e Renderização Afim
Como a projeção isométrica é afim/ortográfica no plano da tela:
- O mapeamento de textura não requer divisão de perspectiva por scanline.
- As coordenadas UV são interpoladas linearmente nas bordas dos triângulos e rasterizadas em buffers ARGB8888 de 60 FPS com alocação zero de memória no loop de desenho.
- Células não-andáveis recebem coloração suave avermelhada de segurança, enquanto o solo andável se conecta de forma fluida sem linhas de grade artificiais.

## 5. Elementos de Interface Expandidos e Mundo Vivo
- **Minimapa Dinâmico (100x100):** Janela translúcida com radar local (raio de 24 células), indicando colisão, localização do herói em tempo real e pontos coloridos para entidades na AoI (verde: jogador/aliados, vermelho: monstros, ciano: NPCs).
- **Target Info Bar (Topo Central):** Caixa escura suave com cantos arredondados, barra de progresso de HP e distância do alvo, mantendo o espaço sobre o monstro livre de poluição de texto.
- **Action Bar (460x44):** 6 slots com contagem de itens em tempo real (Poções de HP, SP), teclas de atalho e indicador de postura rítmica.
- **Retículos Isométricos de Solo (`draw_ground_reticle`):** Elipse projetada no chão (Z=0) com marcas táteis (vermelho para combate, ciano para interação amigável), provendo feedback visual imediato ao passar o mouse ou travar a mira.
- **Balões de Fala Orgânicos Não-Modais (`draw_speech_bubble`):** Caixa de diálogo curva com sombra, cabeçalho e dicas de avanço sem pausar a movimentação nem travar o jogo.
- **Partículas Ambientais e Hidráulicas:**
  - `FountainSpray`: Borrifo ascendente e parabólico de água cristalina da fonte.
  - `WaterSplash`: Efeito de ondulação e respingos ao clicar ou andar sobre a água.
  - `AmbientMote`: Motes de luz flutuando no ar para ambientação viva e acolhedora.

## 6. Critérios de Aceite
- [x] Todas as 12 texturas do mapa central de Prontera decodificadas do GRF com 100% de sucesso.
- [x] Rasterizador afim de triângulos texturizados implementado sem alocações dinâmicas no framebuffer.
- [x] Minimapa, Target Info Bar e Action Bar renderizados responsivamente em 60 FPS sem sobreposições.
- [x] Guia NPC com tracking de olhar por proximidade, emoticons e diálogo não-modal funcional.
- [x] Partículas de água e solo integradas com colisão física no plano do chão.
- [x] Conformidade estrita com o Potato Budget (1 vCPU, 512MB RAM), `docs/VALVE.md` e `AGENTS.md`.
