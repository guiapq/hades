//! SPEC-0003: Resolução Atômica de Combate Físico e Mágico Pré-Renovação.
//!
//! Operação Unitária Pura: Sem dependência de estado global, I/O ou alocações na heap.

use crate::stats::DerivedStats;
use hades_core::types::EntityId;

/// Resultado de acerto do ataque.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum HitResult {
    #[default]
    Miss = 0,
    Hit = 1,
    Critical = 2,
}

/// Parâmetros de entrada para resolução atômica de um ataque físico corpo-a-corpo / à distância.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalAttackInput {
    pub attacker_id: EntityId,
    pub defender_id: EntityId,
    pub attacker_stats: DerivedStats,
    pub defender_stats: DerivedStats,
    pub weapon_atk: u16,
    pub hard_def: u8,
    pub attack_count_on_defender: u8,
    /// Rolagem aleatória normalizada entre 0 e 99 (fornecida externamente para determinismo)
    pub rng_roll: u8,
}

/// Desfecho do ataque computado pelo motor de simulação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CombatOutcome {
    pub hit_result: HitResult,
    pub damage: u32,
    pub attack_delay_ticks: u32,
    pub flinch_ticks: u32,
}

/// Converte o valor de ASPD clássico (0 a 190) para o delay de ataque em ticks de simulação (a 20 Hz / 50ms por tick).
#[inline]
pub fn aspd_to_attack_delay_ticks(aspd: u8) -> u32 {
    let bounded_aspd = aspd.min(190) as u32;
    // Fórmula clássica: delay_seg = (200 - aspd) / 50
    // Em ticks a 20 Hz (20 ticks/seg): delay_ticks = ((200 - aspd) * 20) / 50 = ((200 - aspd) * 2) / 5
    let ticks = ((200 - bounded_aspd) * 2) / 5;
    ticks.max(1)
}

/// Operação Unitária: Resolução de ataque físico puro (sem efeitos colaterais).
#[inline]
pub fn resolve_physical_attack(input: &PhysicalAttackInput) -> CombatOutcome {
    let attack_delay_ticks = aspd_to_attack_delay_ticks(input.attacker_stats.aspd);
    let base_atk = (input.attacker_stats.status_atk as u32) + (input.weapon_atk as u32);

    // 1. Verificação de Crítico (ignora Flee e DEF)
    if input.rng_roll < input.attacker_stats.crit {
        // Dano crítico clássico: 1.4x (140%)
        let crit_damage = (base_atk * 14) / 10;
        return CombatOutcome {
            hit_result: HitResult::Critical,
            damage: crit_damage.max(1),
            attack_delay_ticks,
            flinch_ticks: 4, // 200ms a 20 Hz
        };
    }

    // 2. Penalidade de esquiva clássica contra múltiplos atacantes (-10% a cada atacante além de 2)
    let effective_flee = if input.attack_count_on_defender > 2 {
        let penalty_steps = (input.attack_count_on_defender - 2) as u32;
        let penalty_pct = (penalty_steps * 10).min(80); // mínimo de 20% da esquiva original mantida
        let keep_factor = 100 - penalty_pct;
        ((input.defender_stats.flee as u32) * keep_factor) / 100
    } else {
        input.defender_stats.flee as u32
    };

    // 3. Verificação de Hit vs. Flee: Hit% = clamp(80 + Attacker.HIT - Defender.FLEE, 5, 100)
    let hit_base = 80i32 + (input.attacker_stats.hit as i32) - (effective_flee as i32);
    let hit_chance = hit_base.clamp(5, 100) as u8;

    if input.rng_roll >= hit_chance {
        return CombatOutcome {
            hit_result: HitResult::Miss,
            damage: 0,
            attack_delay_ticks,
            flinch_ticks: 0,
        };
    }

    // 4. Redução de Dano Normal: Hard DEF (% de equipamentos) + Soft DEF (linear de VIT)
    let hard_def = input.hard_def.min(100) as u32;
    let after_hard_def = (base_atk * (100 - hard_def)) / 100;
    let soft_def = input.defender_stats.soft_def as u32;

    let final_damage = after_hard_def.saturating_sub(soft_def).max(1);

    CombatOutcome {
        hit_result: HitResult::Hit,
        damage: final_damage,
        attack_delay_ticks,
        flinch_ticks: 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_stats(
        atk: u16,
        hit: u16,
        flee: u16,
        soft_def: u16,
        crit: u8,
        aspd: u8,
    ) -> DerivedStats {
        DerivedStats {
            status_atk: atk,
            hit,
            flee,
            soft_def,
            crit,
            aspd,
            ..Default::default()
        }
    }

    #[test]
    fn test_critical_hit_ignores_def_and_flee() {
        let attacker = make_test_stats(100, 10, 10, 0, 30, 150); // 30% crit, 10 hit (muito baixo)
        let defender = make_test_stats(0, 0, 500, 100, 0, 150); // 500 flee (esquiva infinita), 100 soft def

        let input = PhysicalAttackInput {
            attacker_id: EntityId::new(1),
            defender_id: EntityId::new(2),
            attacker_stats: attacker,
            defender_stats: defender,
            weapon_atk: 50,
            hard_def: 50, // 50% hard def
            attack_count_on_defender: 1,
            rng_roll: 10, // Menor que 30 -> Crítico!
        };

        let outcome = resolve_physical_attack(&input);
        assert_eq!(outcome.hit_result, HitResult::Critical);
        // Base ATK = 100 + 50 = 150. Dano crítico = 150 * 1.4 = 210 (sem redução de def nem miss)
        assert_eq!(outcome.damage, 210);
        assert_eq!(outcome.flinch_ticks, 4);
    }

    #[test]
    fn test_miss_when_flee_beats_hit() {
        let attacker = make_test_stats(100, 100, 100, 0, 0, 150);
        let defender = make_test_stats(0, 0, 200, 0, 0, 150);

        // Hit% = clamp(80 + 100 - 200, 5, 100) = clamp(-20, 5, 100) = 5%
        let input = PhysicalAttackInput {
            attacker_id: EntityId::new(1),
            defender_id: EntityId::new(2),
            attacker_stats: attacker,
            defender_stats: defender,
            weapon_atk: 0,
            hard_def: 0,
            attack_count_on_defender: 1,
            rng_roll: 50, // 50 >= 5 -> Miss!
        };

        let outcome = resolve_physical_attack(&input);
        assert_eq!(outcome.hit_result, HitResult::Miss);
        assert_eq!(outcome.damage, 0);
        assert_eq!(outcome.flinch_ticks, 0);
    }

    #[test]
    fn test_normal_hit_with_hard_and_soft_def() {
        let attacker = make_test_stats(100, 200, 100, 0, 0, 150);
        let defender = make_test_stats(0, 0, 100, 20, 0, 150); // Soft DEF = 20

        // Base ATK = 100 + 100 = 200
        // Hard DEF = 30% -> 200 * 0.7 = 140
        // Soft DEF = 20 -> 140 - 20 = 120
        let input = PhysicalAttackInput {
            attacker_id: EntityId::new(1),
            defender_id: EntityId::new(2),
            attacker_stats: attacker,
            defender_stats: defender,
            weapon_atk: 100,
            hard_def: 30,
            attack_count_on_defender: 1,
            rng_roll: 10,
        };

        let outcome = resolve_physical_attack(&input);
        assert_eq!(outcome.hit_result, HitResult::Hit);
        assert_eq!(outcome.damage, 120);
    }

    #[test]
    fn test_aspd_to_delay_ticks() {
        // ASPD 150: (200 - 150) * 2 / 5 = 50 * 2 / 5 = 20 ticks (1 segundo)
        assert_eq!(aspd_to_attack_delay_ticks(150), 20);

        // ASPD 190 (teto máximo pré-re): (200 - 190) * 2 / 5 = 10 * 2 / 5 = 4 ticks (200ms)
        assert_eq!(aspd_to_attack_delay_ticks(190), 4);
    }
}
