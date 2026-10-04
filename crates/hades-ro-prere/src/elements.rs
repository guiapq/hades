//! SPEC-0005: Tabela de afinidade elemental (10 elementos x 4 níveis).
//!
//! Tabela estática const clássica em registradores de CPU sem alocações.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum Element {
    #[default]
    Neutral = 0,
    Water = 1,
    Earth = 2,
    Fire = 3,
    Wind = 4,
    Poison = 5,
    Holy = 6,
    Shadow = 7,
    Ghost = 8,
    Undead = 9,
}

impl Element {
    pub const COUNT: usize = 10;
}

/// Matriz de Eficácia Elemental Clássica (% de modificador de dano):
/// Dimensão 0: Elemento Atacante (0..9)
/// Dimensão 1: Elemento Defensor (0..9)
/// Dimensão 2: Nível do Elemento Defensor (0 = Lv 1, 1 = Lv 2, 2 = Lv 3, 3 = Lv 4)
pub const ELEMENTAL_TABLE: [[[i16; 4]; Element::COUNT]; Element::COUNT] = [
    // 0: Neutral Attacker
    [
        [100, 100, 100, 100], // vs Neutral
        [100, 100, 100, 100], // vs Water
        [100, 100, 100, 100], // vs Earth
        [100, 100, 100, 100], // vs Fire
        [100, 100, 100, 100], // vs Wind
        [100, 100, 100, 100], // vs Poison
        [100, 100, 100, 100], // vs Holy
        [100, 100, 100, 100], // vs Shadow
        [70, 50, 25, 0],      // vs Ghost (imune no Lv 4)
        [100, 100, 100, 100], // vs Undead
    ],
    // 1: Water Attacker
    [
        [100, 100, 100, 100], // vs Neutral
        [25, 0, -25, -50],    // vs Water
        [90, 80, 70, 60],     // vs Earth
        [150, 175, 200, 200], // vs Fire (forte!)
        [90, 80, 70, 60],     // vs Wind
        [100, 75, 50, 25],    // vs Poison
        [75, 50, 25, 0],      // vs Holy
        [100, 100, 100, 100], // vs Shadow
        [100, 100, 100, 100], // vs Ghost
        [100, 75, 50, 25],    // vs Undead
    ],
    // 2: Earth Attacker
    [
        [100, 100, 100, 100], // vs Neutral
        [90, 80, 70, 60],     // vs Water
        [25, 0, -25, -50],    // vs Earth
        [90, 80, 70, 60],     // vs Fire
        [150, 175, 200, 200], // vs Wind (forte!)
        [100, 75, 50, 25],    // vs Poison
        [75, 50, 25, 0],      // vs Holy
        [100, 100, 100, 100], // vs Shadow
        [100, 100, 100, 100], // vs Ghost
        [100, 100, 100, 100], // vs Undead
    ],
    // 3: Fire Attacker
    [
        [100, 100, 100, 100], // vs Neutral
        [90, 80, 70, 60],     // vs Water
        [150, 175, 200, 200], // vs Earth (forte!)
        [25, 0, -25, -50],    // vs Fire
        [90, 80, 70, 60],     // vs Wind
        [100, 75, 50, 25],    // vs Poison
        [75, 50, 25, 0],      // vs Holy
        [100, 100, 100, 100], // vs Shadow
        [100, 100, 100, 100], // vs Ghost
        [125, 150, 175, 200], // vs Undead (forte!)
    ],
    // 4: Wind Attacker
    [
        [100, 100, 100, 100], // vs Neutral
        [150, 175, 200, 200], // vs Water (forte!)
        [90, 80, 70, 60],     // vs Earth
        [90, 80, 70, 60],     // vs Fire
        [25, 0, -25, -50],    // vs Wind
        [100, 75, 50, 25],    // vs Poison
        [75, 50, 25, 0],      // vs Holy
        [100, 100, 100, 100], // vs Shadow
        [100, 100, 100, 100], // vs Ghost
        [100, 100, 100, 100], // vs Undead
    ],
    // 5: Poison Attacker
    [
        [100, 100, 100, 100], // vs Neutral
        [100, 75, 50, 25],    // vs Water
        [125, 125, 100, 75],  // vs Earth
        [125, 125, 100, 75],  // vs Fire
        [125, 125, 100, 75],  // vs Wind
        [0, 0, 0, 0],         // vs Poison
        [75, 50, 25, 0],      // vs Holy
        [75, 50, 25, 0],      // vs Shadow
        [75, 50, 25, 0],      // vs Ghost
        [0, 0, 0, 0],         // vs Undead
    ],
    // 6: Holy Attacker
    [
        [100, 100, 100, 100], // vs Neutral
        [100, 75, 50, 25],    // vs Water
        [100, 75, 50, 25],    // vs Earth
        [100, 75, 50, 25],    // vs Fire
        [100, 75, 50, 25],    // vs Wind
        [100, 75, 50, 25],    // vs Poison
        [0, -25, -50, -75],   // vs Holy
        [125, 150, 175, 200], // vs Shadow (forte!)
        [100, 100, 100, 100], // vs Ghost
        [125, 150, 175, 200], // vs Undead (forte!)
    ],
    // 7: Shadow Attacker
    [
        [100, 100, 100, 100], // vs Neutral
        [100, 100, 100, 100], // vs Water
        [100, 100, 100, 100], // vs Earth
        [100, 100, 100, 100], // vs Fire
        [100, 100, 100, 100], // vs Wind
        [50, 25, 0, -25],     // vs Poison
        [125, 150, 175, 200], // vs Holy (forte!)
        [0, -25, -50, -75],   // vs Shadow
        [100, 100, 100, 100], // vs Ghost
        [0, 0, 0, 0],         // vs Undead
    ],
    // 8: Ghost Attacker
    [
        [70, 70, 70, 70],     // vs Neutral
        [100, 100, 100, 100], // vs Water
        [100, 100, 100, 100], // vs Earth
        [100, 100, 100, 100], // vs Fire
        [100, 100, 100, 100], // vs Wind
        [100, 100, 100, 100], // vs Poison
        [75, 50, 25, 0],      // vs Holy
        [75, 50, 25, 0],      // vs Shadow
        [125, 150, 175, 200], // vs Ghost (forte!)
        [100, 100, 100, 100], // vs Undead
    ],
    // 9: Undead Attacker
    [
        [100, 100, 100, 100], // vs Neutral
        [100, 100, 100, 100], // vs Water
        [100, 100, 100, 100], // vs Earth
        [100, 100, 100, 100], // vs Fire
        [100, 100, 100, 100], // vs Wind
        [50, 25, 0, 0],       // vs Poison
        [100, 125, 150, 175], // vs Holy
        [0, 0, 0, 0],         // vs Shadow
        [100, 100, 100, 100], // vs Ghost
        [0, 0, 0, 0],         // vs Undead
    ],
];

/// Retorna o multiplicador de dano elemental em porcentagem (ex: 150 para 150%, 0 para imune).
#[inline(always)]
pub fn get_element_modifier(attacker: Element, defender: Element, defender_level: u8) -> i16 {
    let lv_idx = (defender_level.clamp(1, 4) - 1) as usize;
    let atk_idx = attacker as usize;
    let def_idx = defender as usize;

    ELEMENTAL_TABLE[atk_idx][def_idx][lv_idx]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fire_vs_earth() {
        assert_eq!(get_element_modifier(Element::Fire, Element::Earth, 1), 150);
        assert_eq!(get_element_modifier(Element::Fire, Element::Earth, 2), 175);
        assert_eq!(get_element_modifier(Element::Fire, Element::Earth, 3), 200);
        assert_eq!(get_element_modifier(Element::Fire, Element::Earth, 4), 200);
    }

    #[test]
    fn test_neutral_vs_ghost() {
        assert_eq!(
            get_element_modifier(Element::Neutral, Element::Ghost, 1),
            70
        );
        assert_eq!(get_element_modifier(Element::Neutral, Element::Ghost, 4), 0);
        // Totalmente imune
    }

    #[test]
    fn test_holy_vs_undead_and_shadow() {
        assert_eq!(get_element_modifier(Element::Holy, Element::Undead, 1), 125);
        assert_eq!(get_element_modifier(Element::Holy, Element::Shadow, 4), 200);
    }
}
