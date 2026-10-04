# SPEC-0002: Atributos Primários e Derivados Pré-Renovação (Classic 2.5D RPG)

- **Autor:** Hades Core Team
- **Data:** 2026-10-02
- **Status:** Approved
- **Módulo Afetado:** `crates/hades-ro-prere` (Simulation / Gameplay Rules)

---

## 1. Contexto e Motivação

O **Hades Engine** busca oferecer uma plataforma ultrarrápida, escalável e de baixo consumo de recursos para MMORPGs 2D/2.5D. Para as regras canônicas de MMORPGs 2.5D clássicos (Pré-Renovação), as regras de status devem:
1. Ser puramente aritméticas, livres de dependências externas (sem I/O, sem threads, sem GC).
2. Não realizar nenhuma alocação de memória na heap durante a computação de status (`#[inline]`, stack-only, cópia por valor).
3. Reproduzir com 100% de fidelidade matemática as curvas clássicas de atributos pré-re (ex: bônus quadrático de STR a cada 10 pontos, bônus de MATK de INT a cada 5 e 7 pontos, cast instantâneo com 150 de DEX).
4. Suportar classes clássicas (Aprendiz, 1-1, 2-1, 2-2 e Transclasses).

---

## 2. Contrato de Dados

### 2.1 Atributos Primários (`BaseAttributes`)
Armazenados em inteiros compactos de 8 bits (`u8`):
* `str`: Força (1 a 99 em jogadores; até 255 para MVPs).
* `agi`: Agilidade (1 a 99).
* `vit`: Vitalidade (1 a 99).
* `int`: Inteligência (1 a 99).
* `dex`: Destreza (1 a 99).
* `luk`: Sorte (1 a 99).

Tamanho em memória: exatamente **6 bytes** (`size_of::<BaseAttributes>() == 6`).

### 2.2 Classes de Personagens (`JobClass`)
Enum representativo das classes clássicas da era Pré-Renovação:
* **Novice**: Aprendiz.
* **1-1**: Swordsman, Mage, Archer, Acolyte, Thief, Merchant.
* **2-1**: Knight, Wizard, Hunter, Priest, Assassin, Blacksmith.
* **2-2**: Crusader, Sage, Bard, Dancer, Monk, Rogue, Alchemist.
* **Transclasses (99/70)**: Lord Knight, High Wizard, Sniper, High Priest, Assassin Cross, Whitesmith, Paladin, Professor, Clown, Gypsy, Champion, Stalker, Creator.

### 2.3 Atributos Derivados (`DerivedStats`)
Estrutura calculada contendo os sub-atributos derivados:
* `max_hp`: Pontos de vida máximos calculados pela fórmula base da classe + escalonamento por nível e VIT.
* `max_sp`: Pontos de mana máximos calculados pela fórmula base da classe + escalonamento por nível e INT.
* `status_atk`: ATK base pré-equipamentos.
  $$\text{Status ATK} = \text{STR} + \left\lfloor \frac{\text{STR}}{10} \right\rfloor^2 + \left\lfloor \frac{\text{DEX}}{5} \right\rfloor + \left\lfloor \frac{\text{LUK}}{5} \right\rfloor$$
* `matk_min` e `matk_max`:
  $$\text{MATK}_{\min} = \text{INT} + \left\lfloor \frac{\text{INT}}{7} \right\rfloor^2, \quad \text{MATK}_{\max} = \text{INT} + \left\lfloor \frac{\text{INT}}{5} \right\rfloor^2$$
* `soft_def`: Defesa corporal baseada em VIT (redução subtrativa de dano físico).
  $$\text{Soft DEF} = \text{VIT}$$
* `soft_mdef`: Defesa mágica corporal baseada em INT e VIT.
  $$\text{Soft MDEF} = \text{INT} + \left\lfloor \frac{\text{VIT}}{2} \right\rfloor$$
* `hit`: Precisão de acerto.
  $$\text{HIT} = \text{BaseLevel} + \text{DEX}$$
* `flee`: Esquiva base.
  $$\text{FLEE} = 100 + \text{BaseLevel} + \text{AGI}$$
* `crit`: Chance de ataque crítico em porcentagem (escala de 1 a 100).
  $$\text{CRIT} = 1 + \left\lfloor \frac{\text{LUK} \times 3}{10} \right\rfloor$$
* `aspd`: Velocidade de ataque discreta (escala clássica 0 a 190).
* `cast_reduction_pct`: Percentual de redução de tempo de conjuração (0 a 100%).
  $$\text{Redução \%} = \min\left(100, \left\lfloor \frac{\text{DEX} \times 100}{150} \right\rfloor\right)$$

---

## 3. Operações Unitárias

1. `calculate_derived_stats(attr: BaseAttributes, job: JobClass, base_level: u8, weapon_base_aspd: u8) -> DerivedStats`
   * Computação pura e determinística sem alocações.
2. `calculate_cast_time(base_cast_ticks: u32, dex: u8) -> u32`
   * Reduz o tempo de conjuração em ticks (onde 150 DEX resulta em 0 ticks = cast instantâneo).
3. `calculate_aspd(base_weapon_aspd: u8, agi: u8, dex: u8) -> u8`
   * Fórmula clássica:
     $$\text{ASPD} = 200 - \left\lfloor \frac{(200 - \text{BaseASPD}) \times (250 - \text{AGI} - \lfloor \frac{\text{DEX}}{4} \rfloor)}{250} \right\rfloor$$

---

## 4. Invariantes e Regras de Segurança

- [x] **Zero Heap Allocations:** Todas as funções recebem e retornam tipos primitivos ou structs `Copy`.
- [x] **Zero Panic / Overflow-Safe:** Todas as operações usam aritmética saturada ou `checked` para evitar panics em valores limítrofes.
- [x] **Fidelidade às Fórmulas Clássicas:** Bônus quadráticos de STR e INT preservam exatamente os picos clássicos de poder da era Pré-Renovação.
- [x] **Determinismo Estrito:** Mesmos inputs geram rigorosamente os mesmos outputs em qualquer arquitetura (x86_64 ou ARM64).

---

## 5. Suíte de Testes Unitários

1. `test_base_attributes_memory_size`: Garante que `BaseAttributes` possui exatos 6 bytes.
2. `test_str_quadratic_bonus`: Valida os picos de Status ATK nos múltiplos de 10 de STR (10, 20, 30... 90).
3. `test_int_matk_min_max_bonus`: Valida os múltiplos de 5 e 7 para MATK.
4. `test_instant_cast_at_150_dex`: Assegura que 150 de DEX reduz o cast time a 0.
5. `test_hit_and_flee_formulas`: Valida as fórmulas lineares de Hit e Flee.
6. `test_aspd_curve`: Valida a curva assintótica de ASPD sem exceder o teto clássico de 190.
