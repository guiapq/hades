//! SPEC-0005: Resolução Atômica de Habilidades Clássicas (Skills) Pre-Renewal.
//!
//! Operações Unitárias Puras: Sem alocações na heap, sem efeitos colaterais e determinísticas.

use crate::elements::{get_element_modifier, Element};
use crate::stats::DerivedStats;
use crate::weapons::{get_size_modifier, TargetSize, WeaponType};
use hades_core::types::EntityId;

/// Identificadores das habilidades clássicas suportadas nesta especificação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum SkillId {
    Bash = 5,
    MagnumBreak = 8,
    ColdBolt = 14,
    FireBolt = 19,
    LightningBolt = 20,
    Heal = 28,
    Mammonite = 42,
}

/// Parâmetros de entrada para cálculo de uma habilidade física ofensiva.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillPhysicalInput {
    pub attacker_id: EntityId,
    pub defender_id: EntityId,
    pub skill_id: SkillId,
    pub skill_level: u8,
    pub attacker_stats: DerivedStats,
    pub defender_stats: DerivedStats,
    pub weapon_type: WeaponType,
    pub weapon_atk: u16,
    pub attack_element: Element,
    pub defender_element: Element,
    pub defender_element_level: u8,
    pub target_size: TargetSize,
    pub hard_def: u8,
    pub rng_roll: u8,
}

/// Desfecho do ataque por habilidade física.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SkillPhysicalOutcome {
    pub damage: u32,
    pub sp_cost: u16,
    pub zeny_cost: u32,
    pub knockback_cells: u8,
    pub is_hit: bool,
}

/// Desfecho do ataque por magia de lanças elementais.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MagicBoltOutcome {
    pub damage_per_hit: u32,
    pub hits: u8,
    pub total_damage: u32,
    pub sp_cost: u16,
}

/// Retorna o custo de SP clássico de habilidades físicas.
#[inline]
pub fn get_skill_sp_cost(skill: SkillId, level: u8) -> u16 {
    let lvl = level.clamp(1, 10);
    match skill {
        SkillId::Bash => {
            // Bash: 8 SP nos níveis 1 a 5; 15 SP nos níveis 6 a 10
            if lvl <= 5 {
                8
            } else {
                15
            }
        }
        SkillId::MagnumBreak => 30,
        SkillId::Mammonite => 5,
        SkillId::FireBolt | SkillId::ColdBolt | SkillId::LightningBolt => {
            // Bolts: 12 + 2 * lvl
            12 + (lvl as u16 * 2)
        }
        SkillId::Heal => {
            // Heal: 13 + 3 * lvl
            13 + (lvl as u16 * 3)
        }
    }
}

/// Operação Unitária: Resolução de ataque por habilidade física clássica.
#[inline]
pub fn resolve_skill_physical_attack(input: &SkillPhysicalInput) -> SkillPhysicalOutcome {
    let sp_cost = get_skill_sp_cost(input.skill_id, input.skill_level);
    let lvl = input.skill_level.clamp(1, 10) as u32;

    // 1. Modificador de Dano da Habilidade (% de ATK)
    let (skill_mult_pct, hit_bonus, knockback_cells, zeny_cost) = match input.skill_id {
        SkillId::Bash => {
            // Bash: 100% + 30% * lvl (400% no lvl 10), bônus de Hit = +5% * lvl
            (100 + (30 * lvl), (5 * lvl) as i32, 0u8, 0u32)
        }
        SkillId::MagnumBreak => {
            // Magnum Break: 100% + 20% * lvl, 2 células de empurrão
            (100 + (20 * lvl), 0i32, 2u8, 0u32)
        }
        SkillId::Mammonite => {
            // Mammonite: 100% + 50% * lvl (600% no lvl 10), zeny = 100 * lvl
            (100 + (50 * lvl), 0i32, 0u8, 100 * lvl)
        }
        _ => (100, 0i32, 0u8, 0u32),
    };

    // 2. Verificação de Hit vs. Flee (com bônus de Hit da skill)
    let total_hit = (input.attacker_stats.hit as i32) + hit_bonus;
    let hit_chance = (80i32 + total_hit - (input.defender_stats.flee as i32)).clamp(5, 100) as u8;

    if input.rng_roll >= hit_chance {
        return SkillPhysicalOutcome {
            damage: 0,
            sp_cost,
            zeny_cost,
            knockback_cells: 0,
            is_hit: false,
        };
    }

    // 3. Modificador de Tamanho da Arma (50% a 100%)
    let size_mult = get_size_modifier(input.weapon_type, input.target_size) as u32;

    // 4. Modificador Elemental (-50% a 200%)
    let elem_mult_i16 = get_element_modifier(
        input.attack_element,
        input.defender_element,
        input.defender_element_level,
    );
    let elem_mult = elem_mult_i16.max(0) as u32;

    // 5. Cálculo de Dano Base
    let base_atk = (input.attacker_stats.status_atk as u32) + (input.weapon_atk as u32);
    let with_skill = (base_atk * skill_mult_pct) / 100;
    let with_size = (with_skill * size_mult) / 100;
    let with_elem = (with_size * elem_mult) / 100;

    // 6. Mitigação por DEF (Hard DEF % + Soft DEF linear)
    let hard_def = input.hard_def.min(100) as u32;
    let after_hard = (with_elem * (100 - hard_def)) / 100;
    let final_damage = after_hard
        .saturating_sub(input.defender_stats.soft_def as u32)
        .max(1);

    SkillPhysicalOutcome {
        damage: final_damage,
        sp_cost,
        zeny_cost,
        knockback_cells,
        is_hit: true,
    }
}

/// Operação Unitária: Resolução de Lanças Mágicas (Fire Bolt, Cold Bolt, Lightning Bolt).
#[inline]
pub fn resolve_bolt_magic_attack(
    bolt_skill: SkillId,
    skill_level: u8,
    attacker_matk: u16,
    target_element: Element,
    target_element_level: u8,
    hard_mdef: u8,
    soft_mdef: u16,
) -> MagicBoltOutcome {
    let hits = skill_level.clamp(1, 10);
    let sp_cost = get_skill_sp_cost(bolt_skill, hits);

    let bolt_elem = match bolt_skill {
        SkillId::FireBolt => Element::Fire,
        SkillId::ColdBolt => Element::Water,
        SkillId::LightningBolt => Element::Wind,
        _ => Element::Neutral,
    };

    let elem_mult_i16 = get_element_modifier(bolt_elem, target_element, target_element_level);
    let elem_mult = elem_mult_i16.max(0) as u32;

    let base_per_hit = (attacker_matk as u32 * elem_mult) / 100;
    let hard_mdef_val = hard_mdef.min(100) as u32;
    let after_hard_mdef = (base_per_hit * (100 - hard_mdef_val)) / 100;
    let damage_per_hit = after_hard_mdef.saturating_sub(soft_mdef as u32).max(1);

    let total_damage = damage_per_hit * (hits as u32);

    MagicBoltOutcome {
        damage_per_hit,
        hits,
        total_damage,
        sp_cost,
    }
}

/// Fórmula clássica de Cura (Heal):
/// Cura = floor((BaseLevel + INT) / 8) * (4 + 8 * SkillLevel)
#[inline(always)]
pub fn calculate_heal_amount(base_level: u8, int: u8, skill_level: u8) -> u32 {
    let stat_factor = (base_level as u32 + int as u32) / 8;
    let skill_factor = 4 + (8 * (skill_level.clamp(1, 10) as u32));
    stat_factor * skill_factor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bash_level_10_damage_multiplier() {
        let attacker = DerivedStats {
            status_atk: 100,
            hit: 150,
            ..Default::default()
        };
        let defender = DerivedStats {
            flee: 100,
            soft_def: 0,
            ..Default::default()
        };

        // Bash Lv 10: 400% ATK
        // Base ATK = 100 (status) + 100 (weapon) = 200
        // Tamanho Médio com Espada 1M = 100%
        // Elemento Neutro vs Neutro = 100%
        // Hard DEF = 0
        // Dano esperado = 200 * 4 = 800
        let input = SkillPhysicalInput {
            attacker_id: EntityId::new(1),
            defender_id: EntityId::new(2),
            skill_id: SkillId::Bash,
            skill_level: 10,
            attacker_stats: attacker,
            defender_stats: defender,
            weapon_type: WeaponType::OneHandSword,
            weapon_atk: 100,
            attack_element: Element::Neutral,
            defender_element: Element::Neutral,
            defender_element_level: 1,
            target_size: TargetSize::Medium,
            hard_def: 0,
            rng_roll: 0,
        };

        let outcome = resolve_skill_physical_attack(&input);
        assert!(outcome.is_hit);
        assert_eq!(outcome.damage, 800);
        assert_eq!(outcome.sp_cost, 15);
    }

    #[test]
    fn test_fire_bolt_vs_earth_monster() {
        // Fire Bolt Lv 5 (5 hits)
        // MATK = 100
        // Fogo contra Terra Lv 1 = 150%
        // Dano por hit = 100 * 1.5 = 150
        // 5 hits = 750 de dano total
        let outcome = resolve_bolt_magic_attack(SkillId::FireBolt, 5, 100, Element::Earth, 1, 0, 0);

        assert_eq!(outcome.hits, 5);
        assert_eq!(outcome.damage_per_hit, 150);
        assert_eq!(outcome.total_damage, 750);
        assert_eq!(outcome.sp_cost, 22); // 12 + 2 * 5 = 22
    }

    #[test]
    fn test_classic_heal_formula() {
        // BaseLevel 80, INT 80, Heal Lv 10:
        // (80 + 80) / 8 = 160 / 8 = 20
        // (4 + 8 * 10) = 84
        // Cura = 20 * 84 = 1680
        assert_eq!(calculate_heal_amount(80, 80, 10), 1680);
    }
}
