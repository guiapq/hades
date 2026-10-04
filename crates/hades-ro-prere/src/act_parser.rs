//! SPEC-0019: Parser de Arquivos de Ação e Animação 2D (.act).
//!
//! Decodifica metadados de quadros, recortes (clips), espelhamento, taxas de delay e
//! pontos de acoplamento (attach points) para o motor de renderização do Hades.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ActError {
    #[error("Arquivo muito curto para conter cabeçalho ACT")]
    TooShort,

    #[error("Assinatura mágica inválida (esperado 'AC')")]
    InvalidMagic,

    #[error("Dados de quadro ou ação corrompidos")]
    CorruptedData,
}

/// Um recorte individual (sub-sprite / layer) dentro de um quadro de animação.
#[derive(Debug, Clone, PartialEq)]
pub struct ActClip {
    pub offset_x: i32,
    pub offset_y: i32,
    pub spr_index: i32,
    pub mirror: bool,
    pub color: [u8; 4],
    pub scale_x: f32,
    pub scale_y: f32,
    pub angle: i32,
    pub spr_type: i32,
    pub width: i32,
    pub height: i32,
}

impl Default for ActClip {
    fn default() -> Self {
        Self {
            offset_x: 0,
            offset_y: 0,
            spr_index: -1,
            mirror: false,
            color: [255, 255, 255, 255],
            scale_x: 1.0,
            scale_y: 1.0,
            angle: 0,
            spr_type: 0,
            width: 0,
            height: 0,
        }
    }
}

/// Ponto de acoplamento para cabeças, armas e acessórios.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AttachPoint {
    pub x: i32,
    pub y: i32,
}

/// Quadro individual que agrega uma ou mais camadas e pontos de acoplamento.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActFrame {
    pub clips: Vec<ActClip>,
    pub sound_id: i32,
    pub attach_points: Vec<AttachPoint>,
}

/// Ação que encapsula uma sequência de quadros e o delay entre eles.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ActAction {
    pub frames: Vec<ActFrame>,
    /// Delay em milissegundos por quadro (padrão: 100.0 ms a 150.0 ms).
    pub delay_ms: f32,
}

impl ActAction {
    /// Obtém um quadro com base no tempo decorrido em milissegundos.
    pub fn frame_by_time(&self, elapsed_ms: f32) -> Option<(&ActFrame, usize)> {
        if self.frames.is_empty() {
            return None;
        }
        let delay = if self.delay_ms <= 0.0 { 100.0 } else { self.delay_ms };
        let frame_idx = ((elapsed_ms / delay) as usize) % self.frames.len();
        Some((&self.frames[frame_idx], frame_idx))
    }
}

/// Conjunto de ações decodificadas de um arquivo .act.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Act {
    pub version: f32,
    pub actions: Vec<ActAction>,
    pub sounds: Vec<String>,
}

impl Act {
    /// Obtém uma ação pelo seu índice.
    pub fn action(&self, action_idx: usize) -> Option<&ActAction> {
        self.actions.get(action_idx)
    }

    /// Obtém um quadro específico de uma ação.
    pub fn frame(&self, action_idx: usize, frame_idx: usize) -> Option<&ActFrame> {
        self.actions.get(action_idx)?.frames.get(frame_idx)
    }

    /// Obtém o quadro de uma ação com base no tempo decorrido.
    pub fn frame_by_time(&self, action_idx: usize, elapsed_ms: f32) -> Option<(&ActFrame, usize)> {
        self.actions.get(action_idx)?.frame_by_time(elapsed_ms)
    }
}

/// Decodifica um arquivo .act em memória.
pub fn parse_act(bytes: &[u8]) -> Result<Act, ActError> {
    if bytes.len() < 16 {
        return Err(ActError::TooShort);
    }

    if &bytes[0..2] != b"AC" {
        return Err(ActError::InvalidMagic);
    }

    let sub_ver = bytes[2];
    let main_ver = bytes[3];
    let version = (main_ver as f32) + (sub_ver as f32) / 10.0;
    let action_count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;

    let mut cursor = 16;
    let mut actions = Vec::with_capacity(action_count);

    for _a in 0..action_count {
        if cursor + 4 > bytes.len() {
            return Err(ActError::CorruptedData);
        }
        let frame_count = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
        cursor += 4;

        let mut frames = Vec::with_capacity(frame_count);

        for _f in 0..frame_count {
            // Pular 32 bytes de delimitadores/range bounds
            if cursor + 32 > bytes.len() {
                return Err(ActError::CorruptedData);
            }
            cursor += 32;

            if cursor + 4 > bytes.len() {
                return Err(ActError::CorruptedData);
            }
            let clip_count = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
            cursor += 4;

            let mut clips = Vec::with_capacity(clip_count);

            for _c in 0..clip_count {
                if cursor + 16 > bytes.len() {
                    return Err(ActError::CorruptedData);
                }
                let offset_x = i32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
                let offset_y = i32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().unwrap());
                let spr_index = i32::from_le_bytes(bytes[cursor + 8..cursor + 12].try_into().unwrap());
                let mirror = i32::from_le_bytes(bytes[cursor + 12..cursor + 16].try_into().unwrap()) != 0;
                cursor += 16;

                let mut color = [255, 255, 255, 255];
                let mut scale_x = 1.0f32;
                let mut scale_y = 1.0f32;
                let mut angle = 0;
                let mut spr_type = 0;
                let mut width = 0;
                let mut height = 0;

                if version >= 2.0 {
                    if cursor + 8 > bytes.len() {
                        return Err(ActError::CorruptedData);
                    }
                    color = [bytes[cursor], bytes[cursor + 1], bytes[cursor + 2], bytes[cursor + 3]];
                    scale_x = f32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().unwrap());
                    cursor += 8;

                    if version > 2.3 {
                        if cursor + 4 > bytes.len() {
                            return Err(ActError::CorruptedData);
                        }
                        scale_y = f32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
                        cursor += 4;
                    } else {
                        scale_y = scale_x;
                    }

                    if cursor + 8 > bytes.len() {
                        return Err(ActError::CorruptedData);
                    }
                    angle = i32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
                    spr_type = i32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().unwrap());
                    cursor += 8;

                    if version >= 2.5 {
                        if cursor + 8 > bytes.len() {
                            return Err(ActError::CorruptedData);
                        }
                        width = i32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
                        height = i32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().unwrap());
                        cursor += 8;
                    }
                }

                clips.push(ActClip {
                    offset_x,
                    offset_y,
                    spr_index,
                    mirror,
                    color,
                    scale_x,
                    scale_y,
                    angle,
                    spr_type,
                    width,
                    height,
                });
            }

            let sound_id = if version >= 2.0 {
                if cursor + 4 > bytes.len() {
                    return Err(ActError::CorruptedData);
                }
                let sid = i32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
                cursor += 4;
                sid
            } else {
                -1
            };

            let mut attach_points = Vec::new();
            if version >= 2.3 {
                if cursor + 4 > bytes.len() {
                    return Err(ActError::CorruptedData);
                }
                let attach_count = i32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
                cursor += 4;

                attach_points.reserve(attach_count);
                for _at in 0..attach_count {
                    if cursor + 16 > bytes.len() {
                        return Err(ActError::CorruptedData);
                    }
                    // 4 bytes desconhecidos + 4 bytes x + 4 bytes y + 4 bytes desconhecidos
                    let x = i32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().unwrap());
                    let y = i32::from_le_bytes(bytes[cursor + 8..cursor + 12].try_into().unwrap());
                    cursor += 16;
                    attach_points.push(AttachPoint { x, y });
                }
            }

            frames.push(ActFrame {
                clips,
                sound_id,
                attach_points,
            });
        }

        actions.push(ActAction {
            frames,
            delay_ms: 150.0, // Delay padrão de fallback
        });
    }

    let mut sounds = Vec::new();
    if version >= 2.1 && cursor + 4 <= bytes.len() {
        let sound_count = i32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
        cursor += 4;

        for _ in 0..sound_count {
            if cursor + 40 > bytes.len() {
                break;
            }
            let sound_bytes = &bytes[cursor..cursor + 40];
            cursor += 40;
            let end = sound_bytes.iter().position(|&b| b == 0).unwrap_or(40);
            let name = String::from_utf8_lossy(&sound_bytes[..end]).to_string();
            sounds.push(name);
        }

        if version >= 2.2 {
            for a in 0..action_count {
                if cursor + 4 > bytes.len() {
                    break;
                }
                let delay_float = f32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
                cursor += 4;
                if let Some(act) = actions.get_mut(a) {
                    act.delay_ms = delay_float * 25.0;
                }
            }
        }
    }

    Ok(Act {
        version,
        actions,
        sounds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_act_parser_too_short() {
        let buf = [0u8; 10];
        assert_eq!(parse_act(&buf), Err(ActError::TooShort));
    }

    #[test]
    fn test_act_parser_invalid_magic() {
        let mut buf = vec![0u8; 32];
        buf[0] = b'X';
        buf[1] = b'Y';
        assert_eq!(parse_act(&buf), Err(ActError::InvalidMagic));
    }

    #[test]
    fn test_act_parser_synthetic_v20() {
        let mut buf = Vec::new();
        // Magic
        buf.extend_from_slice(b"AC");
        // Version 2.0 (sub=0, main=2)
        buf.push(0);
        buf.push(2);
        // Action count: 1
        buf.extend_from_slice(&1u16.to_le_bytes());
        // 10 reserved bytes
        buf.extend_from_slice(&[0u8; 10]);

        // Action 0: 1 frame
        buf.extend_from_slice(&1u32.to_le_bytes());
        // Frame 0: 32 bytes range
        buf.extend_from_slice(&[0u8; 32]);
        // 1 clip
        buf.extend_from_slice(&1u32.to_le_bytes());
        // Clip 0: off_x=10, off_y=-20, spr_idx=5, mirror=1
        buf.extend_from_slice(&10i32.to_le_bytes());
        buf.extend_from_slice(&(-20i32).to_le_bytes());
        buf.extend_from_slice(&5i32.to_le_bytes());
        buf.extend_from_slice(&1i32.to_le_bytes());
        // Color RGBA
        buf.extend_from_slice(&[255, 255, 255, 255]);
        // scale_x
        buf.extend_from_slice(&1.5f32.to_le_bytes());
        // angle (4), spr_type (4)
        buf.extend_from_slice(&0i32.to_le_bytes());
        buf.extend_from_slice(&0i32.to_le_bytes());
        // sound_id
        buf.extend_from_slice(&(-1i32).to_le_bytes());

        let act = parse_act(&buf).expect("Deveria parsear ACT sintético v2.0");
        assert_eq!(act.actions.len(), 1);
        let action = &act.actions[0];
        assert_eq!(action.frames.len(), 1);
        let frame = &action.frames[0];
        assert_eq!(frame.clips.len(), 1);
        let clip = &frame.clips[0];
        assert_eq!(clip.offset_x, 10);
        assert_eq!(clip.offset_y, -20);
        assert_eq!(clip.spr_index, 5);
        assert!(clip.mirror);
        assert_eq!(clip.scale_x, 1.5);
        assert_eq!(clip.scale_y, 1.5); // Herda scale_x para version <= 2.3
    }
}
