//! SPEC-0002: Atributos Primários e Derivados Clássicos (Pre-Renewal).
//!
//! Cálculos puramente aritméticos, sem alocações na heap, seguros contra overflow
//! e determinísticos.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StatsError {
    #[error("Nível base inválido (deve ser entre 1 e 99): {0}")]
    InvalidBaseLevel(u8),
}

/// Atributos primários do personagem (clássicos de 1 a 99 em jogadores).
/// Exatamente 6 bytes em memória.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct BaseAttributes {
    pub str: u8,
    pub agi: u8,
    pub vit: u8,
    pub int: u8,
    pub dex: u8,
    pub luk: u8,
}

impl BaseAttributes {
    #[inline(always)]
    pub const fn new(str: u8, agi: u8, vit: u8, int: u8, dex: u8, luk: u8) -> Self {
        Self {
            str,
            agi,
            vit,
            int,
            dex,
            luk,
        }
    }
}

/// Classes clássicas da progressão Pré-Renovação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum JobClass {
    // Aprendiz
    Novice = 0,

    // 1-1
    Swordsman = 1,
    Mage = 2,
    Archer = 3,
    Acolyte = 4,
    Thief = 5,
    Merchant = 6,

    // 2-1
    Knight = 7,
    Wizard = 8,
    Hunter = 9,
    Priest = 10,
    Assassin = 11,
    Blacksmith = 12,

    // 2-2
    Crusader = 13,
    Sage = 14,
    Bard = 15,
    Dancer = 16,
    Monk = 17,
    Rogue = 18,
    Alchemist = 19,

    // Transclasses (99/70)
    LordKnight = 20,
    HighWizard = 21,
    Sniper = 22,
    HighPriest = 23,
    AssassinCross = 24,
    Whitesmith = 25,
    Paladin = 26,
    Professor = 27,
    Clown = 28,
    Gypsy = 29,
    Champion = 30,
    Stalker = 31,
    Creator = 32,
}

impl JobClass {
    /// Fator multiplicador base de HP por classe (tabela clássica).
    #[inline]
    pub const fn hp_factor(self) -> u32 {
        match self {
            Self::Novice => 5,
            Self::Swordsman | Self::Knight | Self::LordKnight | Self::Crusader | Self::Paladin => {
                15
            }
            Self::Merchant
            | Self::Blacksmith
            | Self::Whitesmith
            | Self::Alchemist
            | Self::Creator => 11,
            Self::Thief | Self::Assassin | Self::AssassinCross | Self::Rogue | Self::Stalker => 9,
            Self::Acolyte | Self::Priest | Self::HighPriest | Self::Monk | Self::Champion => 10,
            Self::Archer
            | Self::Hunter
            | Self::Sniper
            | Self::Bard
            | Self::Dancer
            | Self::Clown
            | Self::Gypsy => 9,
            Self::Mage | Self::Wizard | Self::HighWizard | Self::Sage | Self::Professor => 6,
        }
    }

    /// Fator multiplicador base de SP por classe.
    #[inline]
    pub const fn sp_factor(self) -> u32 {
        match self {
            Self::Novice => 1,
            Self::Mage | Self::Wizard | Self::HighWizard | Self::Sage | Self::Professor => 9,
            Self::Acolyte | Self::Priest | Self::HighPriest | Self::Monk | Self::Champion => 8,
            Self::Archer
            | Self::Hunter
            | Self::Sniper
            | Self::Bard
            | Self::Dancer
            | Self::Clown
            | Self::Gypsy => 5,
            Self::Thief | Self::Assassin | Self::AssassinCross | Self::Rogue | Self::Stalker => 4,
            Self::Merchant
            | Self::Blacksmith
            | Self::Whitesmith
            | Self::Alchemist
            | Self::Creator => 4,
            Self::Swordsman | Self::Knight | Self::LordKnight | Self::Crusader | Self::Paladin => 3,
        }
    }
}

/// Atributos derivados calculados para simulação de combate e movimento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct DerivedStats {
    pub max_hp: u32,
    pub max_sp: u32,
    pub status_atk: u16,
    pub matk_min: u16,
    pub matk_max: u16,
    pub soft_def: u16,
    pub soft_mdef: u16,
    pub hit: u16,
    pub flee: u16,
    pub crit: u8,
    pub aspd: u8,
    pub cast_reduction_pct: u8,
}

/// Calcula o valor de Status ATK clássico (Pré-Re):
/// STR + floor(STR / 10)^2 + floor(DEX / 5) + floor(LUK / 5)
#[inline]
pub fn calculate_status_atk(str: u8, dex: u8, luk: u8) -> u16 {
    let str_val = str as u16;
    let str_bonus_tens = str_val / 10;
    let str_quadratic = str_bonus_tens * str_bonus_tens;
    let dex_bonus = (dex as u16) / 5;
    let luk_bonus = (luk as u16) / 5;

    str_val + str_quadratic + dex_bonus + luk_bonus
}

/// Calcula o intervalo de MATK clássico (min, max):
/// Min = INT + floor(INT / 7)^2
/// Max = INT + floor(INT / 5)^2
#[inline]
pub fn calculate_matk(int: u8) -> (u16, u16) {
    let int_val = int as u16;
    let min_bonus = int_val / 7;
    let max_bonus = int_val / 5;

    let matk_min = int_val + (min_bonus * min_bonus);
    let matk_max = int_val + (max_bonus * max_bonus);

    (matk_min, matk_max)
}

/// Calcula a velocidade de ataque (ASPD) clássica:
/// ASPD = 200 - floor( (200 - BaseASPD) * (250 - AGI - floor(DEX / 4)) / 250 )
#[inline]
pub fn calculate_aspd(weapon_base_aspd: u8, agi: u8, dex: u8) -> u8 {
    let base = weapon_base_aspd.min(190) as i32;
    let agi_val = agi as i32;
    let dex_mod = (dex as i32) / 4;

    let stat_factor = 250 - agi_val - dex_mod;
    let stat_factor = stat_factor.max(0);

    let penalty = ((200 - base) * stat_factor) / 250;
    let aspd = 200 - penalty;

    // No sistema clássico o teto absoluto de ASPD é 190
    aspd.clamp(0, 190) as u8
}

/// Reduz tempo de conjuração baseado em Destreza (150 DEX = Instant Cast).
#[inline]
pub fn calculate_cast_time(base_cast_ticks: u32, dex: u8) -> u32 {
    if dex >= 150 {
        return 0;
    }
    // Redução linear: ticks * (150 - dex) / 150
    let remaining = 150u32 - (dex as u32);
    (base_cast_ticks * remaining) / 150
}

/// Calcula os atributos derivados completos a partir de atributos base, classe e nível.
#[inline]
pub fn calculate_derived_stats(
    attr: BaseAttributes,
    job: JobClass,
    base_level: u8,
    weapon_base_aspd: u8,
) -> DerivedStats {
    let lvl = (base_level.max(1) as u32).min(99);
    let vit_val = attr.vit as u32;
    let int_val = attr.int as u32;

    // HP clássico aproximado: (JobFactor * Level * (1 + VIT * 0.01)) + base offset
    let base_hp = 35 + (lvl * job.hp_factor());
    let max_hp = base_hp + (base_hp * vit_val / 100);

    // SP clássico: (JobFactor * Level * (1 + INT * 0.01)) + base offset
    let base_sp = 10 + (lvl * job.sp_factor());
    let max_sp = base_sp + (base_sp * int_val / 100);

    let status_atk = calculate_status_atk(attr.str, attr.dex, attr.luk);
    let (matk_min, matk_max) = calculate_matk(attr.int);

    let soft_def = attr.vit as u16;
    let soft_mdef = (attr.int as u16) + ((attr.vit as u16) / 2);

    let hit = (lvl as u16) + (attr.dex as u16);
    let flee = 100 + (lvl as u16) + (attr.agi as u16);
    let crit = (1 + ((attr.luk as u16 * 3) / 10)).min(100) as u8;

    let aspd = calculate_aspd(weapon_base_aspd, attr.agi, attr.dex);
    let cast_reduction_pct = (((attr.dex as u32) * 100) / 150).min(100) as u8;

    DerivedStats {
        max_hp,
        max_sp,
        status_atk,
        matk_min,
        matk_max,
        soft_def,
        soft_mdef,
        hit,
        flee,
        crit,
        aspd,
        cast_reduction_pct,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn test_base_attributes_memory_size() {
        assert_eq!(size_of::<BaseAttributes>(), 6);
    }

    #[test]
    fn test_str_quadratic_bonus_table() {
        // STR 9: bonus tens = 0 -> 9 + 0 = 9
        assert_eq!(calculate_status_atk(9, 0, 0), 9);
        // STR 10: 10 + (1)^2 = 11
        assert_eq!(calculate_status_atk(10, 0, 0), 11);
        // STR 20: 20 + (2)^2 = 24
        assert_eq!(calculate_status_atk(20, 0, 0), 24);
        // STR 50: 50 + (5)^2 = 75
        assert_eq!(calculate_status_atk(50, 0, 0), 75);
        // STR 90: 90 + (9)^2 = 171
        assert_eq!(calculate_status_atk(90, 0, 0), 171);
        // STR 99 + DEX 50 (10) + LUK 25 (5): 99 + 81 + 10 + 5 = 195
        assert_eq!(calculate_status_atk(99, 50, 25), 195);
    }

    #[test]
    fn test_int_matk_min_max_bonus() {
        // INT 70:
        // Min: 70 + (70/7)^2 = 70 + 100 = 170
        // Max: 70 + (70/5)^2 = 70 + 196 = 266
        let (min, max) = calculate_matk(70);
        assert_eq!(min, 170);
        assert_eq!(max, 266);
    }

    #[test]
    fn test_instant_cast_at_150_dex() {
        let base_ticks = 40; // 2 segundos a 20 Hz
        assert_eq!(calculate_cast_time(base_ticks, 150), 0);
        assert_eq!(calculate_cast_time(base_ticks, 160), 0);
        assert_eq!(calculate_cast_time(base_ticks, 75), 20); // 50% de redução
        assert_eq!(calculate_cast_time(base_ticks, 0), 40);
    }

    #[test]
    fn test_hit_and_flee_formulas() {
        let attr = BaseAttributes::new(10, 80, 50, 10, 60, 20);
        let derived = calculate_derived_stats(attr, JobClass::Hunter, 90, 140);

        // HIT = 90 + 60 = 150
        assert_eq!(derived.hit, 150);
        // FLEE = 100 + 90 + 80 = 270
        assert_eq!(derived.flee, 270);
        // CRIT = 1 + (20 * 3) / 10 = 7
        assert_eq!(derived.crit, 7);
    }

    #[test]
    fn test_aspd_curve_cap_190() {
        // ASPD com arma rápida (156 base) e AGI/DEX altos
        let aspd = calculate_aspd(156, 99, 50);
        assert!((170..=190).contains(&aspd));

        // Com valores absurdos nunca deve exceder 190
        let capped = calculate_aspd(180, 255, 255);
        assert_eq!(capped, 190);
    }
}
