//! SPEC-0005: Tipos de armas e penalidade de dano por tamanho de alvo (Small, Medium, Large).
//!
//! Tabela estática const em registradores de CPU sem alocações.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum TargetSize {
    Small = 0,
    #[default]
    Medium = 1,
    Large = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum WeaponType {
    #[default]
    BareFist = 0,
    Dagger = 1,
    OneHandSword = 2,
    TwoHandSword = 3,
    Spear = 4,
    Axe = 5,
    Mace = 6,
    Bow = 7,
    Knuckle = 8,
    Staff = 9,
    Katar = 10,
}

impl WeaponType {
    pub const COUNT: usize = 11;
}

/// Matriz clássica de eficácia de tamanho (% de dano causado):
/// Linha = WeaponType (0..10)
/// Coluna = TargetSize (0 = Small, 1 = Medium, 2 = Large)
pub const SIZE_MODIFIER_MATRIX: [[u8; 3]; WeaponType::COUNT] = [
    // BareFist: 100% / 100% / 100%
    [100, 100, 100],
    // Dagger: 100% / 75% / 50%
    [100, 75, 50],
    // OneHandSword: 75% / 100% / 75%
    [75, 100, 75],
    // TwoHandSword: 75% / 100% / 100%
    [75, 100, 100],
    // Spear: 75% / 75% / 100%
    [75, 75, 100],
    // Axe: 50% / 75% / 100%
    [50, 75, 100],
    // Mace: 75% / 100% / 100%
    [75, 100, 100],
    // Bow: 100% / 100% / 75%
    [100, 100, 75],
    // Knuckle: 100% / 75% / 50%
    [100, 75, 50],
    // Staff: 100% / 100% / 100%
    [100, 100, 100],
    // Katar: 75% / 100% / 75%
    [75, 100, 75],
];

/// Retorna a porcentagem multiplicadora de dano (50 a 100%) da arma contra o tamanho do alvo.
#[inline(always)]
pub const fn get_size_modifier(weapon: WeaponType, size: TargetSize) -> u8 {
    let w_idx = weapon as usize;
    let s_idx = size as usize;
    SIZE_MODIFIER_MATRIX[w_idx][s_idx]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dagger_penalties() {
        assert_eq!(
            get_size_modifier(WeaponType::Dagger, TargetSize::Small),
            100
        );
        assert_eq!(
            get_size_modifier(WeaponType::Dagger, TargetSize::Medium),
            75
        );
        assert_eq!(get_size_modifier(WeaponType::Dagger, TargetSize::Large), 50);
    }

    #[test]
    fn test_axe_penalties() {
        assert_eq!(get_size_modifier(WeaponType::Axe, TargetSize::Small), 50);
        assert_eq!(get_size_modifier(WeaponType::Axe, TargetSize::Medium), 75);
        assert_eq!(get_size_modifier(WeaponType::Axe, TargetSize::Large), 100);
    }

    #[test]
    fn test_two_handed_sword_penalties() {
        assert_eq!(
            get_size_modifier(WeaponType::TwoHandSword, TargetSize::Small),
            75
        );
        assert_eq!(
            get_size_modifier(WeaponType::TwoHandSword, TargetSize::Medium),
            100
        );
        assert_eq!(
            get_size_modifier(WeaponType::TwoHandSword, TargetSize::Large),
            100
        );
    }
}
