# SPEC-0025: Camada de Compatibilidade Mouse-Only/Touchpad e Gritos de Batalha Anime Shounen

## 1. Visão Geral e Motivação
Em jogos clássicos de MMORPG e ARPG isométrico para computadores portáteis e desktops, a possibilidade de jogar utilizando exclusivamente o mouse ou trackpad/touchpad é um pilar essencial de acessibilidade e jogabilidade ergonômica. Além disso, a entrega de combate visceral é grandemente potencializada por efeitos estilísticos de mangá/anime shounen, onde cada golpe, combo e finalizador é acompanhado por balões de fala e gritos de batalha estilizados flutuando sobre o herói.

Esta especificação define:
1. **Camada de Compatibilidade Mouse-Only e Touchpad:**
   - **Left-Click (Segurado sobre Monstro):** Dispara Jabs contínuos de lâmina (corte rápido seguro). Se o jogador estiver fora de alcance corpo-a-corpo ($\le 2$ células), aproxima-se automaticamente do alvo antes de desferir os golpes.
   - **Right-Click (Segurado sobre Monstro):** Dispara o encadeamento rítmico de combos (Hit 1 $\to$ Hit 2 $\to$ Hit 3). Se clicado no chão livre (sem monstro), preserva a rotação de câmera orbital 3D (Yaw e Pitch).
   - **Left-Click (No chão):** Preserva navegação contínua por clique e pathfinding A* (SPEC-0017).
   - **Retículo de Foco do Mouse:** Feedback visual ao posicionar o cursor sobre entidades inimigas.
2. **Sistema de Gritos de Batalha Anime Shounen (Battle Shouts):**
   - Balões ou faixas de texto estilo quadrinhos/mangá posicionados dinamicamente sobre a cabeça do personagem a cada golpe ou manobra de esquiva.
   - Variedade estilizada de gritos conforme o estágio do golpe (Jabs: *"ORA!"*, *"SEIYA!"*; Hit 1: *"ICHINO TACHI: CROSS SLICE!"*; Hit 2: *"NINO TACHI: CRESCENT CLEAVE!"*; Hit 3 / Crítico: *"OUGI: TENCHI KAIPIKU!"*; Evasão: *"OSOI! (TOO SLOW!)"*).
   - Orçamento rígido de recursos (**Potato Budget**) com **zero alocações na heap** a cada frame.

## 2. Princípios de Engenharia e Potato Budget
- **Zero Heap Allocations:** Todas as frases e dados de grito são armazenados como fatias estáticas `&'static str` ou buffers fixos em estrutura estática (`BattleShoutTracker`).
- **Priorização de Alvo vs Câmera:** A detecção do monstro pelo mouse é feita no espaço de projeção de tela e de grade. Ao clicar com o botão direito diretamente sobre um monstro elegível, a rotação de câmera é inibida em favor do início da sequência de combate.
- **Transição Suave de Distância:** Caso o clique esquerdo ou direito ocorra sobre o monstro enquanto o personagem estiver a mais de 2 células de distância, o sistema aciona a condução de movimento em direção à célula adjacente ao monstro e, assim que a distância for atingida, transiciona organicamente para a execução do golpe.

## 3. Mapeamento de Gritos de Batalha (Shounen Archetypes)

| Estágio de Ação | Exemplo de Gritos Aleatorizados | Cor & Estilo |
| :--- | :--- | :--- |
| **Jab (Fail-Safe)** | *"ORA!"*, *"SEIYA!"*, *"HA!"*, *"SHUNSOKU!"* | Ciano elétrico `0xFF8BE9FD`, tamanho 1x |
| **Hit 1 (Slash)** | *"ICHINO TACHI: CROSS SLICE!"*, *"IKUZO!"*, *"SHINKEN!"* | Amarelo lâmina `0xFFF1FA8C`, tamanho 1x |
| **Hit 2 (Cleave)** | *"NINO TACHI: CRESCENT CLEAVE!"*, *"SONIC BURST!"*, *"KURAEEE!"* | Laranja vibrante `0xFFFFB86C`, tamanho 1x |
| **Hit 3 (Finisher/Crit)** | *"OUGI: TENCHI KAIPIKU!"*, *"HISSAATSU: METEOR IMPACT!"*, *"KORE DE OWARI DA!"* | Vermelho crítico / Dourado `0xFFFF5555`, tamanho 2x com contorno preto |
| **Dodge (Evasiva L2)** | *"OSOI! (TOO SLOW!)"*, *"KAGE BUNSHIN!"*, *"MIKIRI!"* | Púrpura místico `0xFFBD93F9`, tamanho 1x |

## 4. Estrutura de Dados

```rust
#[derive(Debug, Clone, Copy)]
pub struct BattleShout {
    pub text: &'static str,
    pub color: u32,
    pub start_time: Instant,
    pub duration_ms: f32,
    pub scale: u8,
}

pub struct BattleShoutTracker {
    pub current_shout: Option<BattleShout>,
    rng_state: u64,
}
```

## 5. Critérios de Aceitação e Testes
1. Segurar o botão esquerdo do mouse sobre o monstro aciona continuamente golpes rápidos do tipo Jab.
2. Segurar o botão direito do mouse sobre o monstro aciona a cadeia rítmica de combos sem rotacionar a câmera.
3. Clicar no chão com o botão esquerdo continua navegando via A* sem atacar no ar.
4. Clicar e arrastar com o botão direito no chão livre continua rotacionando a câmera normalmente.
5. A cada golpe ou esquiva, um grito shounen correspondente é exibido acima do avatar com decaimento suave.
6. Todos os testes unitários do workspace compilam e passam sem erros e sem alocações na heap.
