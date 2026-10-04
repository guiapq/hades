# SPEC-0024: Sistema de Partículas em Tempo Real e Impactos de Combate

## 1. Visão Geral e Motivação
Em jogos clássicos de MMORPG 2.5D, os efeitos visuais (VFX) costumam ser atrelados a animações estáticas pré-renderizadas extraídas dos arquivos de arquivo (.grf). No entanto, um motor moderno e responsivo beneficia-se imensamente de um **sistema de partículas em tempo real** simulado fisicamente em espaço de mundo 3D/isométrico.

Esta especificação define a arquitetura do motor de partículas em tempo real do cliente Berenice, projetado sob o orçamento estrito de recursos do Projeto Hades (**Potato Budget**), com **zero alocações dinâmicas na heap** durante a execução a 60 FPS, além de corrigir o ciclo de reprodução da animação de derrota da entidade de treino gelatinosa (Poring Dummy) para evitar repetições indesejadas (*looping* contínuo).

## 2. Princípios de Engenharia e Orçamento de Recursos
1. **Zero Heap Allocations no Tick Loop:** O subsistema de partículas opera sobre um pool estático circular pré-alocado de 512 partículas (`ParticlePool`). Nenhuma estrutura dinâmica (`Vec`, `Box`, `String`) é alocada no loop de renderização ou simulação de física.
2. **Física Espacial 2.5D / 3D Isométrica:** Cada partícula possui coordenadas no espaço de mundo `(world_x, world_y, world_z)` e vetores de velocidade tridimensionais `(vx, vy, vz)`. A projeção visual obedece rigorosamente às transformações de câmera (rotação de yaw, inclinação de pitch e nível de zoom).
3. **Composição Visual Aditiva & Alpha Blending:** Suporte a mesclagem aditiva com saturação (efeito luminoso de faíscas e impactos metálicos) e interpolação linear com canal alfa (gotas translúcidas de gelatina e poeira).
4. **Reprodução Não-Cíclica da Animação de Derrota:** A animação de derrota (*Die*) de entidades deve ser executada exatamente uma vez (`clamp` no último quadro ou ocultação após o término da sequência), impedindo que o monstro reproduza repetidamente a sequência de estouro antes do respawn.

## 3. Tipos de Partículas e Efeitos de Combate

| Tipo de Partícula | Mecânica / Gatilho | Comportamento Físico e Visual |
| :--- | :--- | :--- |
| `Spark` | Golpes normais de espada (Hit 1 e Hit 2) | Feixe direcional de faíscas de alta velocidade com arraste (*drag*) e gravidade; transição cromática de branco/amarelo brilhante para laranja e vermelho. |
| `ElectricMicroSpark` | Jab rápido de emergência (botão Quadrado) | Microfaíscas ciano/azul elétrico (`0x8BE9FD`) de dissipação ultra-rápida (150ms). |
| `CriticalBurst` | Finalizador pesado (Hit 3 / Acerto Crítico) | Explosão radial de faíscas densas combinada com estrela de impacto de 4 pontas luminescente. |
| `SlimeBurst` | Derrota da entidade gelatinosa (`current_hp == 0`) | Estouro aquoso de 32 a 48 partículas esféricas translúcidas cor-de-rosa/magenta (`0xFFFF79C6`) e água brilhante lançadas em parábola com gravidade acentuada. |
| `DodgeDust` | Manobra evasiva (L2 / Rolamento) | Puffs suaves de poeira e vento emitidos na posição de saída do jogador com expansão de raio e decaimento suave. |

## 4. Estrutura de Dados (Data-Oriented Design)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleKind {
    Spark,
    ElectricMicroSpark,
    CriticalBurst,
    SlimeBurst,
    DodgeDust,
}

#[derive(Debug, Clone, Copy)]
pub struct Particle {
    pub world_x: f32,
    pub world_y: f32,
    pub world_z: f32,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
    pub drag: f32,
    pub gravity: f32,
    pub color: u32,
    pub size: u8,
    pub age_ms: f32,
    pub max_age_ms: f32,
    pub kind: ParticleKind,
    pub additive: bool,
    pub active: bool,
}

pub struct ParticleSystem {
    particles: [Particle; 512],
    next_idx: usize,
}
```

## 5. Algoritmo de Renderização e Blending
- **Conversão de Coordenadas:**
  $$\text{screen\_x}, \text{screen\_y} = \text{projection.world\_to\_screen}(x, y)$$
  $$\text{final\_screen\_y} = \text{screen\_y} - z \times \text{tile\_height} \times \text{zoom}$$
- **Blending Aditivo (Sparks/Crits):**
  $$R_{final} = \min(255, R_{bg} + R_{p})$$
  $$G_{final} = \min(255, G_{bg} + G_{p})$$
  $$B_{final} = \min(255, B_{bg} + B_{p})$$

## 6. Critérios de Aceitação e Testes
1. O pool de partículas aloca estritamente em tempo de compilação/inicialização, com zero alocações na heap a cada frame.
2. Ao acertar golpes com a espada ou botão quadrado, partículas são emitidas na posição de impacto e decaem suavemente com gravidade e arraste.
3. Ao derrotar a entidade alvo gelatinosa:
   - Uma explosão aquosa de partículas rosa é gerada em tempo real.
   - A animação do monstro toca exatamente uma vez e desaparece até o momento do respawn (2000ms), sem repetir o ciclo de estouro.
4. Ao acionar a manobra evasiva L2, partículas de poeira são geradas no rastro do personagem.
5. Todos os testes unitários do workspace compilam e passam sem erros.
