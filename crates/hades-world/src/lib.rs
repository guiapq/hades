//! hades-world: Servidor de Mundo (World Server) do Hades.
//!
//! Implementa a simulação espacial, gestão de presença em grade O(1) de buckets
//! e difusão de movimento em tempo real sob o Potato Budget.

pub mod map_loader;
pub mod messages;
pub mod session;
pub mod world;

pub use map_loader::MapLoader;
pub use messages::{ClientWorldMsg, ServerWorldMsg};
pub use session::WorldSessionHandler;
pub use world::{WorldManager, WorldSession};
