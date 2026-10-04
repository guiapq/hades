# Filosofia de Feedback Contínuo e Interatividade Viva (VALVE.md) 🎮⚡

> *"Se o jogador tomou uma ação no mundo e o mundo não respondeu de forma imediata e perceptível, o jogo falhou em validar a presença do jogador."*  
> — **Princípio de Design de Feedback da Valve**

---

## 1. O Manifesto da Interatividade

Em jogos projetados sob a filosofia da Valve (como *Half-Life*, *Portal*, *Team Fortress 2* e *Dota 2*), o pilar fundamental que transforma uma simples simulação em uma experiência imersiva e divertida é: **todo estímulo do jogador deve retornar um feedback tangível**.

Historicamente, os MMORPGs isométricos clássicos da virada dos anos 2000 sofriam de extrema estaticidade:
- **Diálogos com NPCs:** Uma janela de texto modal estática, sem reação física, sem olhar direcionado e sem vida.
- **Ambientes Estéreis:** Água que não reage a passos, grama estática, caixas e barris inanimados que funcionavam apenas como polígonos de colisão mortos.
- **Combate Numérico:** Ataques que se resumiam a trocar números abstratos e pequenas barras vermelhas, sem peso, recuo cinético (*hit-stop*), ressonância sonora ou dinamismo visual.
- **Exploração Sem Recompensa:** O mundo existia apenas como um grid de locomoção entre pontos de grind, sem pequenos loops de descoberta, curiosidade sensorial ou prazer intrínseco na navegação.

No **Project Hades**, o objetivo não é recriar as limitações de motores de 2002, mas sim edificar um **novo jogo com alma clássica**: um mundo vivo, tátil e responsivo, onde **existir e explorar já é divertido em si**.

---

## 2. Os Quatro Círculos de Feedback

Para qualquer mecânica ou elemento do jogo, o design deve passar por quatro camadas obrigatórias de feedback:

```
                  +-----------------------------------+
                  |  4. Feedback Sistêmico / Mundo    |
                  |  (Reações ambientais, IA, clima)  |
                  +-----------------+-----------------+
                                    |
                  +-----------------v-----------------+
                  |  3. Feedback Auditivo             |
                  |  (Camadas sonoras, impacto, eco)  |
                  +-----------------+-----------------+
                                    |
                  +-----------------v-----------------+
                  |  2. Feedback Visual & Cinético    |
                  |  (Partículas, squash/stretch)     |
                  +-----------------+-----------------+
                                    |
                  +-----------------v-----------------+
                  |  1. Feedback Tátil / Input        |
                  |  (Resposta imediata no mesmo tick)|
                  +-----------------------------------+
```

### 1. Camada de Input (Zero Latência Perceptível)
- Qualquer clique, tecla ou comando no joystick deve produzir um resultado instantâneo no mesmo frame (60 FPS / 16.6 ms).
- Se a ação exige tempo de cast ou intervalo de ataque (ASPD), o personagem entra imediatamente em postura preparatória (*wind-up*), sem a sensação de atraso ou botão "morto".

### 2. Camada Visual & Cinética (*Game Feel*)
- **Impacto e Peso:** Golpes melee devem gerar impacto visual visível:
  - *Hit-stop* de micro-frames nos impactos críticos.
  - Partículas temáticas com física (faíscas, corte de runas, fumaça, fragmentos de impacto).
  - Texto de combate com personalidade (tamanho dinâmico, cores contrastantes, flutuação elástica).
- **Linguagem Corporal:** O avatar nunca é uma estátua rígida. Respiração em idle, postura tensa durante combate, inclinação no movimento e descanso automático ao relaxar.

### 3. Camada Auditiva (Ressonância e Clareza)
- Cada material tem som característico: passos em pedra, madeira, terra ou lâmina d'água.
- Ataques variam de tom e intensidade com base no acerto (glance, normal, crítico, esquiva).
- Efeitos sonoros espaciais transmitem vida e alerta sem sobrecarregar a mixagem geral.

### 4. Camada Sistêmica & Ambiental (O Mundo Respira)
- O ambiente não é um papel de parede decorativo.
- Passar perto de pássaros faz com que eles levantem voo; passar perto de tochas projeta reflexos pulsantes; sentar em uma taverna gera murmúrios relaxados.

---

## 3. O Loop de Interação com o Mundo

### A. NPCs Vivos e Expressivos
- **Reconhecimento de Presença:** Ao se aproximar de um NPC, ele gira suavemente a cabeça ou tronco na direção do jogador.
- **Expressão Emotiva:** NPCs usam balões animados e emoticons para indicar seu estado (surpresa, simpatia, cansaço, atenção) antes mesmo de você abrir uma conversa.
- **Diálogo sem Quebra de Fluxo:** Diálogos não devem travar o jogador em uma prisão de interface congelada desnecessária sempre que possível.

### B. Combate Rítmico e Reativo
- O combate não é uma rolagem passiva de dados. O jogador deve sentir cada soco (*Jab*), cada golpe de corte e cada finalizador (*Crit Finisher*).
- O adversário reage com recuo, animação de dano coerente e partículas direcionais baseadas no ângulo do impacto.
- **Gritos e Expressões de Batalha:** Personagens vocalizam e expressam a intensidade da batalha de forma probabilística e climática.

### C. A Alegria da Exploração Pura
- Inspirado no design de exploração da Valve, o jogador deve ser recompensado simplesmente por saciar sua curiosidade:
  - Becos escondidos que possuem pequenos detalhes visuais ou tesouros menores.
  - Interação com elementos do cenário (fontes de água clicáveis que soltam borrifos, altares que ressoam uma luz fraca ao toque, tochas que podem ser atiçadas).
  - Posições estratégicas de câmera que revelam panoramas inspiradores e segredos arquitetônicos.

### D. Descanso e Presença Orgânica
- Mesmo quando o jogador para para tomar um café ou organizar o inventário, seu personagem existe organicamente no mundo:
  - Sentar-se automaticamente após alguns segundos de inatividade.
  - Expressar emoticons calmos e espaçados de forma imprevisível (assobios, sono, bebericadas).
  - Acordar no primeiro toque de tecla com prontidão e energia de combate.

---

## 4. Engenharia de Feedback sob o "Potato Budget"

Incorporar feedback rico e constante **não pode violar** o orçamento de ferro do Hades (500 jogadores simultâneos em 1 vCPU / 512 MB):

1. **Zero Heap Allocations nos Efeitos:**
   - Sistemas de partículas usam *ring buffers* estáticos em memória contígua (`[Particle; N]`).
   - Gritos, emoticons e balões de diálogo usam referências estáticas `&'static str` e pools indexados.
2. **Separação Rígida entre Simulação e Apresentação:**
   - O servidor autoritativo (`hades-world`) calcula a física discreta de eventos em ticks determinísticos (50 ms).
   - O cliente (`berenice`) é livre para extrapolar com interpolação contínua a 60 FPS, gerando *game juice*, tremores de tela, faíscas e partículas puramente locais.
3. **Escalonamento Gracioso de Feedback:**
   - Em aglomerações intensas (ex: 100 jogadores no mesmo campo de visão), partículas e efeitos visuais usam priorização baseada em distância do jogador local, garantindo fluidez ininterrupta.

---

## 5. Acessibilidade Sensorial e Design Neurodivergente (Sensory-Friendly Design) 🧠🌸

Jogadores neurodivergentes (incluindo pessoas no espectro autista, TDAH, com transtorno de processamento sensorial ou sensibilidade a estímulos) frequentemente enfrentam experiências hostis em jogos eletrônicos devido a estímulos excessivos, caóticos ou punitivos. 

**O feedback rico nunca deve ser confundido com agressividade sensorial.** O Hades adota a diretriz de **interações suaves, acolhedoras e previsíveis**:

### A. Assets Visuais Suaves e Anti-Sobrecarga (*Visual Overload Prevention*)
- **Sem Flashes Estroboscópicos:** É terminantemente proibido o uso de clarões brancos repentinos em tela cheia (*white flashes*), estroboscopia ou piscares de alta frequência que possam desencadear crises sensoriais ou enxaquecas.
- **Transições e Dissipação Orgânica:** Partículas e efeitos de luz devem utilizar curvas de atenuação suaves (*ease-out* e *fade-out* graduais), sem cortes abruptos.
- **Paletas Harmônicas e Conforto Ocular:** Uso de paletas balanceadas e tons acolhedores (tons terrosos, cianos suaves, dourados quentes), evitando saturações berrantes que causem fadiga visual.
- **Clareza Sem Poluição:** Em combates ou áreas populosas, o jogo deve manter a hierarquia visual limpa. Efeitos secundários cedem espaço e sofrem atenuação automática para não sufocar a leitura do espaço.

### B. Acústica Suave e Não-Hostil (*Acoustic Comfort*)
- **Sem Ruídos Agudos Perfurantes:** Proibição de frequências estridentes, estalos de clique secos, chiados metálicos ou picos de volume desregulados que gerem sobressalto involuntário (*sensory startle*).
- **Texturas Sonoras Quentes e Orgânicas:** Priorizar frequências médias e graves aveludadas (*warm transients*): o som de passos na grama, o bater macio de madeira, o corte suave do vento e o impacto encorpado de golpes.
- **Mixagem com Teto Dinâmico Seguro:** Nenhum efeito sonoro de combate ou interface pode estourar a mixagem principal ou surpreender o jogador de maneira aversiva.

### C. Previsibilidade e Ritmo Respirável
- **Pausas Naturais:** Elementos automáticos (como o descanso *Auto-Idle* e a emissão de emoticons) utilizam intervalos espaçados, tranquilos e irregulares (12s a 28s), respeitando o tempo de descanso cognitivo do usuário em vez de agir como um bombardeamento contínuo de estímulos.
- **Tolerância e Perdão de Input:** O motor implementa *input buffering* generoso e tolerância de timing, eliminando a frustração de "janelas de um único milissegundo". A experiência deve acolher ritmos motores variados no mouse, teclado ou gamepad.
- **Agência Sensorial Completa:** O cliente disponibiliza alternância para desligar tremores de tela (*screen shake*), atenuar intensidade de partículas e calibrar o brilho geral com um clique.

---

## 6. Checklist de Validação para Novas Especificações (RFCs)

Ao redigir qualquer nova especificação no repositório (`specs/SPEC-XXXX`), o autor deve responder positivamente às seguintes perguntas:

- [ ] **Ação e Reação:** Quando o jogador aciona este recurso, o que ele vê, ouve ou sente imediatamente?
- [ ] **Sensação de Peso:** A ação parece ter massa, impacto e consequência física no mundo?
- [ ] **Interrupção e Cancelamento:** O jogador tem agência para cancelar ou reagir, ou ele fica travado de forma frustrante?
- [ ] **Encanto Visual/Auditivo:** Existe algum pequeno detalhe (micro-animação, partícula, som sutil) que torna a repetição desse loop prazerosa?
- [ ] **Conforto Sensorial & Neurodivergência:** O efeito evita agressividade, flashes estroboscópicos, ruídos agudos ou poluição visual caótica?
- [ ] **Delicadeza dos Assets:** As partículas e sons possuem transições suaves, acolhedoras e confortáveis para sessões prolongadas?
- [ ] **Orçamento Respeitado:** O feedback é produzido com custo zero de alocação dinâmica no gameloop?

---

*Este documento estabelece a diretriz de experiência de usuário e filosofia de jogo para todas as mecânicas, interfaces e sistemas presentes e futuros do Hades.*
