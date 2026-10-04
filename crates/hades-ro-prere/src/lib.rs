//! # Hades Pre-Renewal Classic 2.5D Simulation Engine
//!
//! Regras de jogo, cálculos determinísticos de atributos, combate atômico,
//! afinidades elementais, penalidade de tamanho de armas, habilidades clássicas
//! e conversor de mapas .gat baseados em padrões legados de MMORPGs 2.5D.

pub mod act_parser;
pub mod bmp_parser;
pub mod combat;
pub mod elements;
pub mod gat_parser;
pub mod gnd_parser;
pub mod rsm_parser;
pub mod rsw_parser;
pub mod skills;
pub mod spr_parser;
pub mod stats;
pub mod weapons;

pub use act_parser::{parse_act, Act, ActAction, ActClip, ActError, ActFrame, AttachPoint};
pub use bmp_parser::{parse_bmp, BmpError, BmpImage};
pub use gnd_parser::{parse_gnd, GndCell, GndError, GndMesh, GndTile};

pub use combat::{
    aspd_to_attack_delay_ticks, resolve_physical_attack, CombatOutcome, HitResult,
    PhysicalAttackInput,
};
pub use elements::{get_element_modifier, Element, ELEMENTAL_TABLE};
pub use gat_parser::{parse_gat, GatCellType, GatParseError};
pub use rsm_parser::{parse_rsm, RsmError, RsmFace, RsmModel, RsmNode};
pub use rsw_parser::{parse_rsw, RswError, RswModelObject, RswScene};
pub use skills::{
    calculate_heal_amount, get_skill_sp_cost, resolve_bolt_magic_attack,
    resolve_skill_physical_attack, MagicBoltOutcome, SkillId, SkillPhysicalInput,
    SkillPhysicalOutcome,
};
pub use spr_parser::{parse_spr, SprError, Sprite, SpriteFrame};
pub use stats::{
    calculate_aspd, calculate_cast_time, calculate_derived_stats, calculate_matk,
    calculate_status_atk, BaseAttributes, DerivedStats, JobClass, StatsError,
};
pub use weapons::{get_size_modifier, TargetSize, WeaponType, SIZE_MODIFIER_MATRIX};
