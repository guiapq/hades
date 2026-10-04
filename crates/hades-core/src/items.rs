//! SPEC-0008: Transação Atômica de Itens do Chão (Floor Items), UUIDs Universais
//! e Prevenção Definitiva de Duplicação por Desync/Rollback/DoS.

use crate::tick::Tick;
use crate::types::{EntityId, Position};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ItemError {
    #[error("Item não encontrado no chão: ID={0}")]
    ItemNotFound(u32),

    #[error("Inventário cheio (capacidade máxima de slots atingida)")]
    InventoryFull,

    #[error("Capacidade máxima de peso excedida")]
    Overweight,

    #[error("Alvo muito distante para recolher: distância={0} células (máximo permitido 2)")]
    TooFarAway(u16),

    #[error("Item protegido por prioridade de saque do jogador {0:?}")]
    LootPriorityActive(EntityId),

    #[error("Tentativa de injeção de item com UUID duplicado detectada: {0:?}")]
    DuplicateInstanceId(ItemInstanceId),

    #[error("Slot de inventário inválido: {0}")]
    InvalidSlot(usize),

    #[error("Quantidade insuficiente no inventário: solicitado {0}, disponível {1}")]
    InsufficientAmount(u32, u32),
}

/// Identificador único universal de cada item gerado no mundo (UUID sequencial de 64 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ItemInstanceId(pub u64);

impl ItemInstanceId {
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Dados completos de uma instância individual de item em memória.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemInstance {
    pub instance_id: ItemInstanceId,
    pub item_id: u32,
    pub amount: u32,
    pub weight: u32,
    pub refine: u8,
    pub cards: [u16; 4],
}

impl ItemInstance {
    pub fn new(instance_id: ItemInstanceId, item_id: u32, amount: u32, weight: u32) -> Self {
        Self {
            instance_id,
            item_id,
            amount,
            weight,
            refine: 0,
            cards: [0; 4],
        }
    }
}

/// Entidade de item descansando no chão do mapa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FloorItem {
    pub floor_id: u32,
    pub item: ItemInstance,
    pub position: Position,
    pub owner_id: Option<EntityId>,
    pub priority_until_tick: Tick,
    pub despawn_tick: Tick,
}

/// Gerenciador atômico de itens caídos no chão de um mapa.
#[derive(Debug, Clone, Default)]
pub struct FloorItemManager {
    next_floor_id: u32,
    items: HashMap<u32, FloorItem>,
}

impl FloorItemManager {
    pub fn new() -> Self {
        Self {
            next_floor_id: 1,
            items: HashMap::new(),
        }
    }

    /// Spawna um novo item no chão do mapa com prioridade e tempo de vida em ticks.
    pub fn spawn_floor_item(
        &mut self,
        item: ItemInstance,
        position: Position,
        owner_id: Option<EntityId>,
        current_tick: Tick,
        priority_ticks: u64,
        despawn_ticks: u64,
    ) -> u32 {
        let floor_id = self.next_floor_id;
        self.next_floor_id = self.next_floor_id.wrapping_add(1).max(1);

        let floor_item = FloorItem {
            floor_id,
            item,
            position,
            owner_id,
            priority_until_tick: Tick::new(current_tick.0 + priority_ticks),
            despawn_tick: Tick::new(current_tick.0 + despawn_ticks),
        };

        self.items.insert(floor_id, floor_item);
        floor_id
    }

    /// Remove ATOMICAMENTE o item do chão e retorna sua posse.
    /// Se outro pacote/thread já retirou, retorna None instantaneamente (idempotente).
    #[inline]
    pub fn take_floor_item(&mut self, floor_id: u32) -> Option<FloorItem> {
        self.items.remove(&floor_id)
    }

    /// Consulta dados do item no chão sem removê-lo.
    #[inline]
    pub fn get_floor_item(&self, floor_id: u32) -> Option<&FloorItem> {
        self.items.get(&floor_id)
    }

    /// Quantidade de itens atualmente no chão.
    #[inline]
    pub fn count(&self) -> usize {
        self.items.len()
    }
}

/// Inventário do jogador com limite de slots, peso e garantia de UUIDs únicos.
#[derive(Debug, Clone)]
pub struct Inventory {
    pub max_slots: usize,
    pub max_weight: u32,
    pub current_weight: u32,
    items: Vec<ItemInstance>,
    /// Rastreamento em O(1) de todos os UUIDs ativos neste inventário
    active_uuids: HashSet<ItemInstanceId>,
}

impl Inventory {
    pub fn new(max_slots: usize, max_weight: u32) -> Self {
        Self {
            max_slots,
            max_weight,
            current_weight: 0,
            items: Vec::with_capacity(max_slots),
            active_uuids: HashSet::with_capacity(max_slots),
        }
    }

    #[inline]
    pub fn items(&self) -> &[ItemInstance] {
        &self.items
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    #[inline]
    pub fn contains_instance_id(&self, uuid: ItemInstanceId) -> bool {
        self.active_uuids.contains(&uuid)
    }

    /// Insere um item validando capacidade de slots, sobrepeso e garantia anti-dupe de UUID.
    pub fn add_item(&mut self, item: ItemInstance) -> Result<(), ItemError> {
        // 1. Verificação Anti-Duplicação de UUID
        if self.contains_instance_id(item.instance_id) {
            return Err(ItemError::DuplicateInstanceId(item.instance_id));
        }

        // 2. Capacidade de slots
        if self.items.len() >= self.max_slots {
            return Err(ItemError::InventoryFull);
        }

        // 3. Capacidade de peso
        let item_total_weight = item.weight * item.amount;
        if self.current_weight.saturating_add(item_total_weight) > self.max_weight {
            return Err(ItemError::Overweight);
        }

        self.current_weight = self.current_weight.saturating_add(item_total_weight);
        self.active_uuids.insert(item.instance_id);
        self.items.push(item);

        Ok(())
    }

    /// Remove um item pelo índice do slot do inventário.
    pub fn remove_item_by_slot(
        &mut self,
        slot: usize,
        amount: u32,
    ) -> Result<ItemInstance, ItemError> {
        if slot >= self.items.len() {
            return Err(ItemError::InvalidSlot(slot));
        }

        let current_amount = self.items[slot].amount;
        if amount > current_amount || amount == 0 {
            return Err(ItemError::InsufficientAmount(amount, current_amount));
        }

        let weight_per_unit = self.items[slot].weight;
        let removed_weight = weight_per_unit * amount;
        self.current_weight = self.current_weight.saturating_sub(removed_weight);

        if amount == current_amount {
            let removed = self.items.remove(slot);
            self.active_uuids.remove(&removed.instance_id);
            Ok(removed)
        } else {
            self.items[slot].amount -= amount;
            let mut split_item = self.items[slot].clone();
            split_item.amount = amount;
            Ok(split_item)
        }
    }
}

/// OPERAÇÃO UNITÁRIA ATÔMICA: Descarta um item do inventário diretamente no chão.
pub fn drop_item_to_floor(
    inventory: &mut Inventory,
    slot_idx: usize,
    amount: u32,
    drop_pos: Position,
    floor_manager: &mut FloorItemManager,
    current_tick: Tick,
) -> Result<u32, ItemError> {
    let item_to_drop = inventory.remove_item_by_slot(slot_idx, amount)?;
    let floor_id = floor_manager.spawn_floor_item(
        item_to_drop,
        drop_pos,
        None,
        current_tick,
        0,    // Sem prioridade de saque (drop manual)
        1200, // 60 segundos a 20 Hz (1200 ticks)
    );
    Ok(floor_id)
}

/// OPERAÇÃO UNITÁRIA ATÔMICA: Coleta um item do chão para o inventário do jogador.
///
/// Imune a DoS/Spam: se 100 requisições simultâneas chegarem para o mesmo `floor_id`,
/// apenas a primeira transição atômica remove o item do chão; todas as outras 99
/// recebem imediatamente `Err(ItemError::ItemNotFound)` com zero efeitos colaterais.
pub fn pickup_floor_item(
    inventory: &mut Inventory,
    picker_id: EntityId,
    picker_pos: Position,
    floor_id: u32,
    floor_manager: &mut FloorItemManager,
    current_tick: Tick,
) -> Result<ItemInstance, ItemError> {
    // 1. Verificação prévia de existência
    let Some(floor_item) = floor_manager.get_floor_item(floor_id) else {
        return Err(ItemError::ItemNotFound(floor_id));
    };

    // 2. Validação de distância de coleta (máximo 2 células via Chebyshev)
    let dist = picker_pos.chebyshev_distance(floor_item.position);
    if dist > 2 {
        return Err(ItemError::TooFarAway(dist));
    }

    // 3. Validação de prioridade de saque
    if current_tick < floor_item.priority_until_tick {
        if let Some(owner) = floor_item.owner_id {
            if owner != picker_id {
                return Err(ItemError::LootPriorityActive(owner));
            }
        }
    }

    // 4. RETIRADA ATÔMICA DO CHÃO
    // Apenas UMA execução conseguirá retirar o item do mapa!
    let Some(taken_item) = floor_manager.take_floor_item(floor_id) else {
        return Err(ItemError::ItemNotFound(floor_id));
    };

    // 5. Inserção no inventário do jogador
    if let Err(e) = inventory.add_item(taken_item.item.clone()) {
        // Se a mochila estiver cheia ou com sobrepeso, devolve o item com o mesmo UUID ao chão
        floor_manager.items.insert(floor_id, taken_item);
        return Err(e);
    }

    Ok(taken_item.item)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_floor_item_atomic_pickup_happy_path() {
        let mut inventory = Inventory::new(100, 5000);
        let mut floor = FloorItemManager::new();

        let uuid = ItemInstanceId::new(123456789);
        let item = ItemInstance::new(uuid, 501, 10, 5); // 10 Poções Vermelhas, peso 50
        let pos = Position::new_unchecked(100, 100);

        let floor_id = floor.spawn_floor_item(item, pos, None, Tick::ZERO, 0, 1000);
        assert_eq!(floor.count(), 1);

        let player_id = EntityId::new(42);
        let player_pos = Position::new_unchecked(101, 101); // 1 célula de distância

        let picked = pickup_floor_item(
            &mut inventory,
            player_id,
            player_pos,
            floor_id,
            &mut floor,
            Tick::ZERO,
        )
        .expect("Coleta atômica deve ter sucesso");

        assert_eq!(picked.instance_id, uuid);
        assert_eq!(
            floor.count(),
            0,
            "Item deve ter sumido do chão imediatamente"
        );
        assert_eq!(inventory.len(), 1);
        assert_eq!(inventory.current_weight, 50);
        assert!(inventory.contains_instance_id(uuid));
    }

    #[test]
    fn test_prevent_dos_spam_pickup_duplication() {
        let mut inventory = Inventory::new(100, 5000);
        let mut floor = FloorItemManager::new();

        let uuid = ItemInstanceId::new(99999);
        let item = ItemInstance::new(uuid, 1101, 1, 100); // 1 Espada
        let pos = Position::new_unchecked(50, 50);

        let floor_id = floor.spawn_floor_item(item, pos, None, Tick::ZERO, 0, 1000);

        let player_id = EntityId::new(1);
        let player_pos = Position::new_unchecked(50, 50);

        // Simula 100 requisições simultâneas de clique de coleta para o exato mesmo floor_id
        let mut successes = 0;
        let mut failures = 0;

        for _ in 0..100 {
            match pickup_floor_item(
                &mut inventory,
                player_id,
                player_pos,
                floor_id,
                &mut floor,
                Tick::ZERO,
            ) {
                Ok(_) => successes += 1,
                Err(ItemError::ItemNotFound(_)) => failures += 1,
                Err(e) => panic!("Erro inesperado: {:?}", e),
            }
        }

        // INVARIANTE INVIOLÁVEL: Exatamente 1 sucesso e 99 rejeições!
        assert_eq!(successes, 1, "Exatamente um clique deve ter sucesso");
        assert_eq!(failures, 99, "99 tentativas de spam devem falhar");
        assert_eq!(
            inventory.len(),
            1,
            "Inventário deve conter apenas 1 cópia do item"
        );
        assert_eq!(
            floor.count(),
            0,
            "O chão deve estar vazio sem item fantasma"
        );
    }

    #[test]
    fn test_reject_pickup_beyond_range() {
        let mut inventory = Inventory::new(100, 5000);
        let mut floor = FloorItemManager::new();

        let item = ItemInstance::new(ItemInstanceId::new(1), 501, 1, 5);
        let item_pos = Position::new_unchecked(10, 10);
        let floor_id = floor.spawn_floor_item(item, item_pos, None, Tick::ZERO, 0, 1000);

        // Jogador a 5 células de distância
        let distant_player_pos = Position::new_unchecked(15, 10);

        let res = pickup_floor_item(
            &mut inventory,
            EntityId::new(2),
            distant_player_pos,
            floor_id,
            &mut floor,
            Tick::ZERO,
        );

        assert_eq!(res, Err(ItemError::TooFarAway(5)));
        assert_eq!(floor.count(), 1, "Item deve permanecer intacto no chão");
    }

    #[test]
    fn test_loot_priority_protection() {
        let mut inventory = Inventory::new(100, 5000);
        let mut floor = FloorItemManager::new();

        let item = ItemInstance::new(ItemInstanceId::new(1), 501, 1, 5);
        let pos = Position::new_unchecked(10, 10);
        let killer_id = EntityId::new(100);

        // Protegido por 100 ticks (5 segundos) para killer_id
        let floor_id = floor.spawn_floor_item(item, pos, Some(killer_id), Tick::ZERO, 100, 1000);

        let thief_id = EntityId::new(200);

        // Ladrão tenta pegar no tick 50 (dentro da prioridade): REJEITADO
        let res_thief = pickup_floor_item(
            &mut inventory,
            thief_id,
            pos,
            floor_id,
            &mut floor,
            Tick::new(50),
        );
        assert_eq!(res_thief, Err(ItemError::LootPriorityActive(killer_id)));

        // Dono legítimo pega no tick 60: SUCESSO
        let res_owner = pickup_floor_item(
            &mut inventory,
            killer_id,
            pos,
            floor_id,
            &mut floor,
            Tick::new(60),
        );
        assert!(res_owner.is_ok());
    }

    #[test]
    fn test_anti_dupe_rejects_duplicate_instance_id() {
        let mut inventory = Inventory::new(100, 5000);
        let uuid = ItemInstanceId::new(777);

        let item1 = ItemInstance::new(uuid, 501, 1, 5);
        let item2 = ItemInstance::new(uuid, 501, 1, 5); // Mesmo UUID clonado

        assert!(inventory.add_item(item1).is_ok());
        // Segunda inserção com o mesmo UUID deve ser rejeitada!
        assert_eq!(
            inventory.add_item(item2),
            Err(ItemError::DuplicateInstanceId(uuid))
        );
        assert_eq!(inventory.len(), 1);
    }
}
