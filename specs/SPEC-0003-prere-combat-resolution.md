# SPEC-0003: Resolução Atômica de Combate Físico e Mágico Pré-Renovação

- **Autor:** Hades Core Team
- **Data:** 2026-10-02
- **Status:** Approved
- **Módulo Afetado:** `crates/hades-ro-prere` (Simulation / Combat Pipeline)

---

## 1. Contexto e Motivação

No combate clássico 2.5D Pré-Renovação, o combate possui regras estritas e bem delimitadas de mitigação de dano, verificação de acerto (Hit vs. Flee) e dano crítico.
Na filosofia do **Hades**:
1. O combate é uma **Operação Unitária Pura**:
   $$\text{resolve\_attack}(\text{Attacker}, \text{Defender}, \text{RngSeed}) \rightarrow \text{CombatOutcome}$$
2. Zero alocação na heap durante a resolução do golpe.
3. Não existem acessos a banco de dados ou estado global dentro da função de combate.
4. Determinismo garantido por passagem explícita de semente pseudoaleatória (XorShift / splitmix) para rolagens de acerto e variação de dano de arma.

---

## 2. Contrato de Dados

### 2.1 Entrada do Combate (`AttackInput`)
* `attacker_id`: `EntityId` do atacante.
* `defender_id`: `EntityId` do defensor.
* `attacker_stats`: `DerivedStats` do atacante.
* `defender_stats`: `DerivedStats` do defensor.
* `weapon_atk`: ATK fornecido pela arma (0 se desarmado).
* `hard_def`: DEF de equipamentos do defensor (0 a 100, redução percentual).
* `hard_mdef`: MDEF de equipamentos do defensor (0 a 100, redução percentual).
* `attack_count_on_defender`: Quantidade de entidades atacando o defensor simultaneamente (para penalidade clássica de Flee: -10% de esquiva a cada atacante além de 2).
* `rng_seed`: Inteiro `u64` para geração determinística de números aleatórios.

### 2.2 Resultado do Combate (`CombatOutcome`)
* `hit_result`: Enum `HitResult` (`Hit`, `Critical`, `Miss`).
* `damage`: Quantidade final de dano a ser subtraído do HP do defensor.
* `is_dead`: Booleano indicando se o dano excedeu o HP atual.
* `attack_delay_ticks`: Quantidade de ticks que o atacante deve esperar antes do próximo ataque (baseado em ASPD).
* `flinch_ticks`: Quantidade de ticks de hit-stun que o defensor sofre ao receber dano (exceto se tiver cartas/habilidades que anulem o stun, como Carta Endure/Eddga).

---

## 3. Fórmulas Matemáticas Clássicas (Pré-Renovação)

### 3.1 Verificação de Acerto (Hit vs. Flee)
1. **Critical Roll:**
   * Se $\text{Rng}(0..100) < \text{Attacker.CRIT}$, o golpe é **Crítico** automático (ignora Flee e ignora Hard/Soft DEF).
2. **Hit Rate Roll:**
   * Caso não seja crítico:
     $$\text{Effective Flee} = \text{Defender.FLEE} \times \max\left(0.2, 1.0 - 0.1 \times \max(0, \text{Atacantes} - 2)\right)$$
     $$\text{Hit Chance \%} = \text{clamp}(80 + \text{Attacker.HIT} - \text{Effective Flee}, 5, 100)$$
   * Se $\text{Rng}(0..100) \ge \text{Hit Chance}$, o golpe é um **Miss** ($\text{damage} = 0$).

### 3.2 Redução de Dano Físico (Hard DEF % + Soft DEF linear)
1. Dano Bruto:
   $$\text{Base ATK} = \text{Status ATK} + \text{Weapon ATK}$$
2. Se Crítico:
   $$\text{Final Damage} = \left\lfloor \text{Base ATK} \times 1.4 \right\rfloor$$
3. Se Ataque Normal com Sucesso:
   $$\text{After Hard DEF} = \left\lfloor \text{Base ATK} \times \frac{100 - \min(100, \text{Hard DEF})}{100} \right\rfloor$$
   $$\text{Final Damage} = \max(1, \text{After Hard DEF} - \text{Soft DEF})$$

### 3.3 Mapeamento de ASPD para Ticks do Servidor
A 20 Hz (50 ms por tick):
$$\text{Attack Delay (segundos)} = \frac{200 - \text{ASPD}}{50}$$
$$\text{Delay Ticks} = \max\left(1, \left\lfloor \text{Attack Delay (seg)} \times 20 \right\rfloor\right)$$

---

## 4. Invariantes do Sistema

- [x] O dano nunca é negativo.
- [x] Se for Miss, o dano é estritamente 0 e o defensor não entra em flinch.
- [x] Se for Crítico, nunca erra (Miss é impossível) e ignora as defesas do alvo.
- [x] Resolução puramente em registradores de CPU, sem I/O nem heap.
