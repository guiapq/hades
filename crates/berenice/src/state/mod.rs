//! Módulos de estado da simulação do cliente Berenice.

pub mod player_stats;
pub mod world_view;

pub use player_stats::{InventoryEntry, PlayerStats};
pub use world_view::{ObservedEntity, WorldView};
