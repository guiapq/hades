# SPEC-0005: Modificadores de Tamanho de Arma, Tabela Elemental e Habilidades Clássicas Pré-Renovação

- **Autor:** Hades Core Team
- **Data:** 2026-10-02
- **Status:** Approved
- **Módulo Afetado:** `crates/hades-ro-prere` (Simulation / Combat Modifiers & Skills)

---

## 1. Contexto e Motivação

No combate canônico de RPGs 2.5D clássicos, o combate é enriquecido por três camadas determinísticas:
1. **Penalidade de Tamanho por Tipo de Arma:** Diferentes armas possuem eficácia distinta contra alvos Pequenos, Médios e Grandes (ex: Adagas cortam 100% de alvos Pequenos, mas apenas 50% de Grandes; Machados causam 100% em Grandes e 50% em Pequenos).
2. **Matriz de Afinidades Elementais (10x10 x 4 Níveis):** O dano varia entre 0% e 200% dependendo do elemento do ataque contra o elemento e nível do defensor (ex: Fogo contra Terra Lv 1 causa 150%, Fantasma Lv 1 é imune a ataques Neutros).
3. **Resolução de Habilidades Atômicas (Skills):** Habilidades icônicas (como *Golpe Fulminante/Bash*, *Impacto Explosivo/Magnum Break*, *Mammonita* e *Lanças Elementais*) operam como transformações puras de entrada e saída.

Para preservar a meta do **Potato Budget**, todas as matrizes devem ser declaradas como `const` arrays contíguos na memória do binário, garantindo lookups $O(1)$ e **zero alocações na heap**.

---

## 2. Contrato de Dados

### 2.1 Tipos de Armas (`WeaponType`) e Tamanhos (`TargetSize`)
* `WeaponType`:
  * `BareFist` (Desarmado), `Dagger` (Adaga), `OneHandSword` (Espada 1M), `TwoHandSword` (Espada 2M),
  * `Spear` (Lança), `Axe` (Machado), `Mace` (Maça), `Bow` (Arco), `Knuckle` (Soqueira),
  * `Staff` (Cajado), `Katar` (Katar).
* `TargetSize`: `Small` (Pequeno), `Medium` (Médio), `Large` (Grande).

#### Matriz de Penalidade de Tamanho (% de Dano):
| Arma | Pequeno | Médio | Grande |
| :--- | :--- | :--- | :--- |
| **BareFist** | 100% | 100% | 100% |
| **Dagger** | 100% | 75% | 50% |
| **OneHandSword** | 75% | 100% | 75% |
| **TwoHandSword** | 75% | 100% | 100% |
| **Spear** | 75% | 75% | 100% |
| **Axe** | 50% | 75% | 100% |
| **Mace** | 75% | 100% | 100% |
| **Bow** | 100% | 100% | 75% |
| **Knuckle** | 100% | 75% | 50% |
| **Staff** | 100% | 100% | 100% |
| **Katar** | 75% | 100% | 75% |

### 2.2 Elementos (`Element`)
* 10 Elementos clássicos: `Neutral` (0), `Water` (1), `Earth` (2), `Fire` (3), `Wind` (4), `Poison` (5), `Holy` (6), `Shadow` (7), `Ghost` (8), `Undead` (9).
* Níveis de Elemento do Alvo: 1 a 4.

### 2.3 Identificadores de Habilidades (`SkillId`)
* `Bash` (ID 5): Dano Físico = $100\% + 30\% \times \text{Nível}$, bônus de Hit = $+5\% \times \text{Nível}$.
* `MagnumBreak` (ID 8): Dano Físico de Fogo = $100\% + 20\% \times \text{Nível}$, knockback = 2 células.
* `Mammonite` (ID 42): Dano Físico = $100\% + 50\% \times \text{Nível}$, consumo de Zeny = $100 \times \text{Nível}$.
* `FireBolt` (ID 19) / `ColdBolt` (ID 14) / `LightningBolt` (ID 20): Dano Mágico = $100\% \text{ MATK}$ por acerto, total de acertos = Nível (1 a 10).
* `Heal` (ID 28): Cura = $\lfloor \frac{\text{BaseLv} + \text{INT}}{8} \rfloor \times (4 + 8 \times \text{Nível})$.

---

## 3. Operações Unitárias

1. `get_size_modifier(weapon: WeaponType, size: TargetSize) -> u8`
   * Lookup puramente indexado $O(1)$.
2. `get_element_modifier(atk_elem: Element, def_elem: Element, def_elem_lv: u8) -> i16`
   * Retorna a porcentagem multiplicadora (ex: 150 para +50%, 0 para imune).
3. `resolve_skill_physical_attack(input: &SkillAttackInput) -> SkillOutcome`
   * Transição pura de cálculo de dano com multiplicadores de habilidade, elemento e tamanho.
4. `calculate_heal_amount(base_level: u8, int: u8, skill_level: u8) -> u32`
   * Cálculo determinístico de cura.

---

## 4. Invariantes do Sistema

- [x] **Zero Heap Allocation:** Tabelas estáticas `const` no segmento de dados e structs passadas por valor/referência.
- [x] **Limites de Nível:** Nível de elemento do defensor limitado estritamente entre 1 e 4.
- [x] **Fidelidade às Tabelas Clássicas:** Casos clássicos (ex: Fogo contra Terra, Neutro contra Fantasma) reproduzem exatamente a matriz clássica de eficácia.
- [x] **Determinismo Estrito:** Mesmos inputs produzem rigorosamente os mesmos deltas de dano.
