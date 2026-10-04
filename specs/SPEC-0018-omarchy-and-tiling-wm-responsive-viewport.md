# SPEC-0018: Camada de Compatibilidade Omarchy e Redimensionamento Responsivo para Tiling Window Managers

- **Autor:** Hades Core Team & Berenice Client Working Group
- **Data:** 2026-10-03
- **Status:** Approved
- **Módulo Afetado:** `crates/berenice` (Windowing, Software Framebuffer & UI Engine)

---

## 1. Contexto e Motivação

Ambientes de desenvolvimento e desktop baseados em distribuições como Arch Linux com gerenciadores de janelas tiling (ex: **Hyprland**, **Sway**, **i3**, e ecossistemas como **Omarchy**) organizam os programas em grades dinâmicas de divisão de tela (splits verticais e horizontais). 

Anteriormente, o cliente nativo `Berenice` utilizava dimensões fixas estáticas (`800x600`) com `resize: false` na criação da janela `minifb`. Esse comportamento causava dois grandes problemas em ambientes tiling e fluxos de desenvolvimento com monitor vertical:
1. **Desperdício de Espaço de Tela:** A janela ficava restrita a uma caixa de 800x600 dentro de um split vertical (ex: 540x960 ou 600x1200), deixando áreas pretas ou forçando comportamento flutuante indesejado.
2. **HUD e Viewport Rígidos:** Os painéis de controle e o cálculo de raio de renderização assumiam proporção horizontal 4:3 (800x600), cortando informações ou falhando em preencher a extensão vertical do mapa.

Esta especificação define a camada de compatibilidade com gerenciadores de janelas tiling e o desktop Omarchy, permitindo redimensionamento dinâmico sem perda de performance a 60 FPS.

---

## 2. Requisitos Técnicos e Arquitetura

### 2.1 Redimensionamento Dinâmico de Framebuffer (`SoftwareFramebuffer::resize`)
O buffer de pixels em memória (`SoftwareFramebuffer`) deve suportar redimensionamento instantâneo:
- Quando a janela recebe um evento de resize vindo do Wayland/X11, o tamanho da janela `(win_w, win_h)` é consultado a cada frame.
- Se as dimensões mudarem e forem válidas (`win_w >= 100 && win_h >= 100`), o buffer de pixels `Vec<u32>` é realocado para `win_w * win_h` e as propriedades `width` e `height` são atualizadas.
- O método de renderização limpa e desenha os pixels exatamente na nova proporção 1:1, sem distorção anamórfica ou letterboxing artificial.

### 2.2 Centralização da Câmera e Raio de Visão Responsivo
- **Origem da Câmera:** A câmera isométrica projeta a célula do jogador sempre no centro exato da tela:
  $$\text{camera\_x} = \frac{\text{width}}{2}, \quad \text{camera\_y} = \frac{\text{height}}{2}$$
- **Raio de Visão Dinâmico:** Em layouts verticais (onde `height > width`), o raio de renderização do mapa não pode depender apenas da largura. Deve se basear na maior dimensão da janela:
  $$\text{max\_dim} = \max(\text{width}, \text{height})$$
  $$\text{view\_radius} = \text{clamp}\left(\left\lceil \frac{\text{max\_dim} / 36.0}{\text{zoom}} \right\rceil, 14, 60\right)$$
  Isso garante que todo o comprimento vertical da tela seja preenchido com tiles de terreno e modelos 3D RSW.

### 2.3 HUD Responsivo e Modo Compacto
- **Painel Superior Esquerdo:**
  - Largura adaptativa: $\min(310, \text{width} - 20)$.
- **Barra de Controles Inferior:**
  - Posicionamento vertical: $y = \text{height} - 48$.
  - Largura adaptativa: $w = \text{width} - 20$.
  - **Modo Compacto ($\text{width} < 650$):** Em colunas verticais estreitas (comuns em splits ao lado de terminais ou IDEs), o texto de ajuda se ajusta automaticamente para versões compactas (ex: `"ANDAR: MOUSE/WASD | ZOOM: SCROLL"`).
- **Tela de Login:** O popup de autenticação e seus botões/inputs permanecem centralizados com base em `(fb.width / 2, fb.height / 2)`.

### 2.4 Predefinições e Variáveis de Ambiente
O cliente deve reconhecer parâmetros de inicialização específicos para facilitar testes em ambientes verticais:
1. **Flag CLI:** `--vertical` ou `-v` inicializa a janela com proporção vertical (`480x860`).
2. **Variáveis de Ambiente:**
   - `HADES_VERTICAL=1` ou `HADES_OMARCHY=1`: Ativa predefinição vertical inicial.
   - `HADES_WINDOW_WIDTH` e `HADES_WINDOW_HEIGHT`: Permitem customização arbitrária da resolução inicial.

---

## 3. Conformidade com as Diretrizes do Hades

- **Potato Budget:** O redimensionamento do buffer aloca apenas quando a geometria da janela muda efetivamente (acionada pelo gerenciador de janelas); durante frames estáveis normais a 60 FPS, **zero alocações** dinâmicas ocorrem.
- **Clean-Room & Discretion Policy:** As terminologias e documentações referem-se estritamente aos conceitos de layout de interface gráfica, arquitetura de software e sistemas operacionais abertos.
