//! Camada de renderização e projeção 2.5D do cliente Berenice.

pub mod avatar;
pub mod combo;
pub mod dodge;
pub mod dummy;
pub mod font;
pub mod isometric;
pub mod npc;
pub mod particles;
pub mod shout;
pub mod software_framebuffer;

pub use avatar::{
    calculate_attach_offset, get_layer_order, AvatarRenderParams, VisualLayer,
};
pub use combo::{ComboStage, ComboTracker};
pub use dodge::DodgeTracker;
pub use dummy::{DummyState, FloatingNumberPool, TrainingDummy};
pub use hades_ro_prere::act_parser::AttachPoint;
pub use isometric::{
    IsometricProjection, DEFAULT_PITCH_DEG, DEFAULT_TILE_HEIGHT, DEFAULT_TILE_WIDTH,
    MAX_PITCH_DEG, MIN_PITCH_DEG,
};
pub use npc::GuideNpc;
pub use particles::{Particle, ParticleKind, ParticleSystem};
pub use shout::{AutoIdleTracker, BattleShout, BattleShoutTracker};
pub use software_framebuffer::{
    SoftwareFramebuffer, COLOR_BG, COLOR_GRID_LINE, COLOR_LOCAL_PLAYER, COLOR_REMOTE_ENTITY,
    COLOR_WALKABLE, COLOR_WALL,
};

