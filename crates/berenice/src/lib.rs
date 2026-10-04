//! # Berenice Client Engine
//!
//! Cliente leve, ágil e multiplataforma para o ecossistema Hades.
//! Focado em portabilidade máxima (Linux, Windows, macOS, BSDs, Haiku e WebAssembly),
//! suporte abrangente a controles físicos (Xbox, PS5, Switch, USB genérico) e
//! esquema de controle híbrido estilo Tree of Savior.

pub mod input;
pub mod network;
pub mod render;
pub mod state;
pub mod ui;
pub mod vfs;

pub use input::{
    analog_stick_to_direction, resolve_action_input, resolve_keyboard_action,
    resolve_keyboard_movement, ActionIntent, DirectionalKeys, FaceButton, GamepadState,
    HotbarModifier, InputHub, MovementIntent,
};
pub use network::{BereniceNetwork, NetworkClientError, WorldClient};

pub use render::{
    IsometricProjection, SoftwareFramebuffer, COLOR_BG, COLOR_LOCAL_PLAYER, COLOR_REMOTE_ENTITY,
    COLOR_WALKABLE, COLOR_WALL, DEFAULT_TILE_HEIGHT, DEFAULT_TILE_WIDTH,
};
pub use state::{ObservedEntity, WorldView};
pub use ui::{LoginField, LoginScene, LoginState};
pub use vfs::{AssetSource, DirectoryArchive, GrfArchive, GrfError};
