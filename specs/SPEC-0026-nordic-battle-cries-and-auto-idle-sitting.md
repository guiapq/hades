# SPEC-0026: Gritos de Batalha Nórdicos e Sistema de Descanso Automático (Auto-Idle Sit)

## 1. Visão Geral e Motivação
O universo de fantasia do Projeto Hades é fundamentado na rica mitologia e geografia nórdica/escandinava (Midgard, Valhalla, Yggdrasil, Fjord, Niflheim, Einherjar). Para reforçar a ambientação e a maturidade da experiência de jogo:
1. **Gritos de Batalha Nórdicos Temáticos com Gatilho Probabilístico:**
   - Em vez de disparar falas repetitivas a cada ataque, os brados de combate passam a ter **chances probabilísticas balanceadas** (proc chances: 30% no Jab, 40% no Hit 1, 50% no Hit 2, 100% no Finalizador Crítico Hit 3 e 35% na Evasão).
   - O vocabulário transita do anime genérico para o lore e mitologia nórdica (*"FOR VALHALLA!"*, *"WRATH OF THE FJORD!"*, *"BY TYR!"*, *"SKAL!"*, *"MJOLNIR'S THUNDERCLAP!"*).
2. **Sistema de Descanso Automático (Auto-Idle Sit & Emoticons):**
   - Ao permanecer sem qualquer comando de entrada por mais de **10 segundos**, o personagem transiciona automaticamente para a animação de sentado (Grupo 02 do ACT: `16 + act_dir`).
   - Enquanto descansa, o herói emite periodicamente balões de emoticons relaxados (*"( _ _ )zzZ"*, *"( ´-ω-) ☕"*, *"Camping in Midgard..."*).
   - Ao receber o próximo comando de movimento ou ação, o personagem levanta-se imediatamente e dispara uma fala rápida de prontidão (*"Back to battle!"*, *"Midgard calls!"*, *"Let's move!"*).

## 2. Princípios de Engenharia e Potato Budget
- **Zero Heap Allocations:** Todas as frases, diálogos e emoticons são referências estáticas `&'static str` com zero alocações na heap.
- **Transição de Estados Não-Bloqueante:** O estado de descanso sentado é cancelado no primeiro milissegundo de detecção de input (teclado, mouse ou gamepad), garantindo latência zero de resposta e movimentação fluida.
- **Discretion Policy:** Nomes de divindades e conceitos seguem exclusivamente a mitologia e geografia nórdica histórica de domínio público, sem menção a marcas comerciais ou franquias registradas.

## 3. Mapeamento de Brados Nórdicos e Emoticons

| Gatilho | Chance de Proc | Exemplos de Brados | Cor & Escala |
| :--- | :--- | :--- | :--- |
| **Jab Rápido** | 30% | *"SKAL!"*, *"BY TYR!"*, *"HRAST!"*, *"FROST BITE!"*, *"SWIFT RUNE!"* | Ciano `0xFF8BE9FD`, 1x |
| **Hit 1 (Corte)** | 40% | *"WRATH OF THE FJORD!"*, *"FOR MIDGARD!"*, *"VALKYRIE'S STRIKE!"*, *"RUNE CLEAVE!"* | Amarelo `0xFFF1FA8C`, 1x |
| **Hit 2 (Arco)** | 50% | *"TEMPEST OF THOR!"*, *"EINHERJAR'S FURY!"*, *"WOLF OF FENRIR!"*, *"NORTHERN GALE!"* | Laranja `0xFFFFB86C`, 1x |
| **Hit 3 (Crítico)** | 100% | *"FOR VALHALLA!"*, *"MJOLNIR'S THUNDERCLAP!"*, *"RAGNAROK'S WRATH!"*, *"ODIN'S JUDGMENT!"* | Vermelho/Dourado `0xFFFF5555`, 2x |
| **Evasiva (L2)** | 35% | *"LIKE THE NORTH WIND!"*, *"MIST OF NIFLHEIM!"*, *"SHADOW OF YGGDRASIL!"* | Púrpura `0xFFBD93F9`, 1x |
| **Descanso (Sit)** | 6s..14s inicial, 12s..28s subsequente (irregular) | Sprites nativos animados (.spr/.act): Ação 05 (zzZ), 06 (Música ♪), 03 (...), 19 (Café ☕), 02 (Coração ♥) | Sprite real do GRF renderizado em cima do herói |
| **Acordar (Wakeup)**| 100% (saída) | *"Back to battle!"*, *"Midgard calls!"*, *"Let's move!"*, *"Rest is over!"* | Branco brilhante `0xFFF8F8F2`, 1x |

## 4. Estrutura de Dados e Lógica

```rust
pub struct AutoIdleTracker {
    pub last_input_time: Instant,
    pub is_sitting: bool,
    pub last_emoticon_time: Instant,
    pub next_emoticon_interval_ms: u64,
    pub current_emotion_action: Option<usize>,
    pub emotion_start_time: Instant,
    pub idle_threshold_secs: u64,
    rng_state: u64,
}
```

## 5. Critérios de Aceitação e Testes
1. O personagem permanece ativo e em pé enquanto houver qualquer comando de teclado, mouse ou analógico.
2. Após 10 segundos inativo, transiciona para o grupo 02 de animação (`16 + act_dir`).
3. O intervalo entre emoticons não é fixo/rápido, mas sim aleatorizado entre 8.0 e 18.0 segundos.
4. Quando um emoticon é disparado, renderiza o quadro animado correspondente de `data/sprite/이팩트/emotion.spr` e `emotion.act` sobre a cabeça do avatar até o término da ação.
5. No primeiro comando após o descanso, levanta-se instantaneamente e emite diálogo de retorno à batalha.
6. Ataques e evasivas obedecem às taxas probabilísticas nórdicas e garantem 100% de presença apenas no finalizador crítico.
7. Todos os testes unitários compilam e passam sem alocações dinâmicas na heap.
