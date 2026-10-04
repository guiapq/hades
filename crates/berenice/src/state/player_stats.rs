//! SPEC-0023: Estrutura de Atributos, Progressão, HP/SP e Inventário do Jogador.
//!
//! Integra com as fórmulas de atributos canônicas de `hades-ro-prere::stats`
//! e fornece suporte a regeneração periódica, consumo de recursos e itens.

use hades_ro_prere::stats::{
    calculate_derived_stats, BaseAttributes, DerivedStats, JobClass,
};
use std::time::Instant;

/// Item de inventário simplificado para exibição e consumo local.
#[derive(Debug, Clone)]
pub struct InventoryEntry {
    pub id: u32,
    pub name: &'static str,
    pub amount: u32,
    pub weight: u32,
    pub is_usable: bool,
}

/// Estado completo de atributos, recursos e inventário do jogador local.
#[derive(Debug, Clone)]
pub struct PlayerStats {
    pub name: String,
    pub job: JobClass,
    pub base_level: u8,
    pub job_level: u8,
    pub current_hp: u32,
    pub current_sp: u32,
    pub base_attributes: BaseAttributes,
    pub derived: DerivedStats,
    pub last_regen_time: Instant,
    pub inventory: Vec<InventoryEntry>,
}

impl PlayerStats {
    /// Inicializa um Espadachim nível 14/10 com distribuição clássica de atributos.
    pub fn new_swordsman(name: &str) -> Self {
        let job = JobClass::Swordsman;
        let base_level = 14;
        let job_level = 10;
        let base_attributes = BaseAttributes::new(24, 18, 22, 10, 16, 12);
        let derived = calculate_derived_stats(base_attributes, job, base_level, 145);

        let now = Instant::now();
        Self {
            name: name.to_string(),
            job,
            base_level,
            job_level,
            current_hp: derived.max_hp,
            current_sp: derived.max_sp,
            base_attributes,
            derived,
            last_regen_time: now,
            inventory: vec![
                InventoryEntry { id: 501, name: "Poção Vermelha", amount: 15, weight: 7, is_usable: true },
                InventoryEntry { id: 502, name: "Poção Laranja", amount: 5, weight: 10, is_usable: true },
                InventoryEntry { id: 505, name: "Poção Azul", amount: 8, weight: 15, is_usable: true },
                InventoryEntry { id: 601, name: "Asa de Mosca", amount: 10, weight: 5, is_usable: true },
                InventoryEntry { id: 909, name: "Jellopy", amount: 24, weight: 1, is_usable: false },
                InventoryEntry { id: 1002, name: "Minério de Ferro", amount: 4, weight: 20, is_usable: false },
            ],
        }
    }

    /// Recalcula os atributos derivados (HP max, SP max, ATK, DEF, etc.).
    pub fn recalculate(&mut self) {
        self.derived = calculate_derived_stats(
            self.base_attributes,
            self.job,
            self.base_level,
            145,
        );
        self.current_hp = self.current_hp.min(self.derived.max_hp);
        self.current_sp = self.current_sp.min(self.derived.max_sp);
    }

    /// Tenta consumir uma quantidade de SP (ex: manobra evasiva ou habilidade).
    pub fn consume_sp(&mut self, amount: u32) -> bool {
        if self.current_sp >= amount {
            self.current_sp -= amount;
            true
        } else {
            false
        }
    }

    /// Aplica dano sofrido ao HP.
    pub fn take_damage(&mut self, dmg: u32) {
        self.current_hp = self.current_hp.saturating_sub(dmg);
    }

    /// Cura HP e/ou SP respeitando os valores máximos.
    pub fn heal(&mut self, hp: u32, sp: u32) {
        self.current_hp = (self.current_hp + hp).min(self.derived.max_hp);
        self.current_sp = (self.current_sp + sp).min(self.derived.max_sp);
    }

    /// Ciclo de regeneração natural a cada 3,0 segundos (Potato Budget).
    pub fn tick_regen(&mut self, now: Instant) {
        if now.duration_since(self.last_regen_time).as_millis() >= 3000 {
            self.last_regen_time = now;
            // Regeneração baseada em VIT e INT
            let hp_regen = 1 + (self.base_attributes.vit as u32 / 5);
            let sp_regen = 1 + (self.base_attributes.int as u32 / 6);
            self.heal(hp_regen, sp_regen);
        }
    }

    /// Peso total atual acumulado no inventário.
    pub fn current_weight(&self) -> u32 {
        self.inventory.iter().map(|item| item.amount * item.weight).sum()
    }

    /// Limite de peso do personagem (2000 + STR * 30).
    pub fn max_weight(&self) -> u32 {
        2000 + (self.base_attributes.str as u32 * 30)
    }

    /// Consome um item pelo índice do inventário se for utilizável.
    pub fn use_item(&mut self, index: usize) -> bool {
        if index < self.inventory.len() && self.inventory[index].is_usable && self.inventory[index].amount > 0 {
            let item_id = self.inventory[index].id;
            self.inventory[index].amount -= 1;
            match item_id {
                501 => self.heal(45, 0),  // Poção Vermelha
                502 => self.heal(105, 0), // Poção Laranja
                505 => self.heal(0, 35),  // Poção Azul
                _ => {}
            }
            if self.inventory[index].amount == 0 {
                self.inventory.remove(index);
            }
            true
        } else {
            false
        }
    }
}
