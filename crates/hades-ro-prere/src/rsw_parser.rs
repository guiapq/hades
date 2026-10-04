//! # Parser de Arquivos de Cena de Mundo (.rsw)
//!
//! SPEC-0014: Parser binário seguro para arquivos de mundo (.rsw),
//! permitindo a reconstrução espacial e posicionamento tridimensional
//! de modelos (.rsm), luzes e água em mapas clássicos 2.5D.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RswError {
    #[error("Arquivo muito curto para conter cabeçalho RSW válido")]
    UnexpectedEof,
    #[error("Assinatura mágica inválida (esperado 'GRSW')")]
    InvalidMagic,
    #[error("Versão RSW não suportada")]
    UnsupportedVersion,
}

/// Representa um modelo 3D posicionado no mundo (Tipo 1 do RSW).
#[derive(Debug, Clone, PartialEq)]
pub struct RswModelObject {
    /// Nome da instância (ex: "prt_fountain1").
    pub name: String,
    /// Caminho normalizado do arquivo de modelo (ex: "data/model/prontera/prt_k_bunsu_1.rsm").
    pub filename: String,
    /// Nó de ancoragem.
    pub node_name: String,
    /// Posição 3D dividida por 5.0 (espaço de células de grade).
    pub position: [f32; 3],
    /// Rotação Euler em graus (pitch, yaw, roll).
    pub rotation: [f32; 3],
    /// Escala 3D dividida por 5.0.
    pub scale: [f32; 3],
}

/// Cena completa do mundo carregada do arquivo .rsw.
#[derive(Debug, Clone, PartialEq)]
pub struct RswScene {
    pub major: u8,
    pub minor: u8,
    pub ini_file: String,
    pub gnd_file: String,
    pub gat_file: String,
    pub src_file: String,
    pub water_level: f32,
    pub water_type: i32,
    pub models: Vec<RswModelObject>,
}

fn read_cstring(slice: &[u8]) -> String {
    let clean_len = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
    let s = String::from_utf8_lossy(&slice[..clean_len]);
    s.trim().to_string()
}

fn normalize_model_path(slice: &[u8]) -> String {
    let clean = read_cstring(slice);
    let mut path = clean.replace('\\', "/");
    while path.starts_with('/') {
        path.remove(0);
    }
    if !path.starts_with("data/model/") && !path.starts_with("model/") {
        format!("data/model/{}", path)
    } else if path.starts_with("model/") {
        format!("data/{}", path)
    } else {
        path
    }
}

/// Realiza o parse completo de um arquivo .rsw em memória.
pub fn parse_rsw(data: &[u8]) -> Result<RswScene, RswError> {
    if data.len() < 4 {
        return Err(RswError::UnexpectedEof);
    }

    if &data[0..4] != b"GRSW" {
        return Err(RswError::InvalidMagic);
    }

    if data.len() < 46 {
        return Err(RswError::UnexpectedEof);
    }

    let major = data[4];
    let minor = data[5];
    let version = major as f32 + minor as f32 / 10.0;

    let mut cur = 6;

    let build_number = if version >= 2.5 {
        if cur + 4 > data.len() {
            return Err(RswError::UnexpectedEof);
        }
        let bn = i32::from_le_bytes(data[cur..cur + 4].try_into().unwrap());
        cur += 4;
        bn
    } else {
        0
    };

    if version >= 2.2 {
        if cur + 1 > data.len() {
            return Err(RswError::UnexpectedEof);
        }
        cur += 1;
    }

    if cur + 120 > data.len() {
        return Err(RswError::UnexpectedEof);
    }

    let ini_file = read_cstring(&data[cur..cur + 40]);
    cur += 40;
    let gnd_file = read_cstring(&data[cur..cur + 40]);
    cur += 40;
    let gat_file = read_cstring(&data[cur..cur + 40]);
    cur += 40;

    let src_file = if version >= 1.4 {
        if cur + 40 > data.len() {
            return Err(RswError::UnexpectedEof);
        }
        let src = read_cstring(&data[cur..cur + 40]);
        cur += 40;
        src
    } else {
        String::new()
    };

    let mut water_level = 0.0f32;
    let mut water_type = 0i32;

    if version < 2.6 {
        if version >= 1.3 {
            if cur + 4 > data.len() {
                return Err(RswError::UnexpectedEof);
            }
            water_level = f32::from_le_bytes(data[cur..cur + 4].try_into().unwrap()) / 5.0;
            cur += 4;
        }
        if version >= 1.8 {
            if cur + 16 > data.len() {
                return Err(RswError::UnexpectedEof);
            }
            water_type = i32::from_le_bytes(data[cur..cur + 4].try_into().unwrap());
            cur += 16; // type (4) + waveHeight (4) + waveSpeed (4) + wavePitch (4)
        }
        if version >= 1.9 {
            if cur + 4 > data.len() {
                return Err(RswError::UnexpectedEof);
            }
            cur += 4; // animSpeed (4)
        }
    }

    // Light info
    if version >= 1.5 {
        if cur + 32 > data.len() {
            return Err(RswError::UnexpectedEof);
        }
        cur += 32; // longitude (4) + latitude (4) + diffuse (12) + ambient (12)
        if version >= 1.7 {
            if cur + 4 > data.len() {
                return Err(RswError::UnexpectedEof);
            }
            cur += 4; // opacity (4)
        }
    }

    // Ground bounds
    if version >= 1.6 {
        if cur + 16 > data.len() {
            return Err(RswError::UnexpectedEof);
        }
        cur += 16; // top, bottom, left, right (4 * 4)
    }

    // Version >= 2.7 skip quadtree / additional counts
    if version >= 2.7 {
        if cur + 4 > data.len() {
            return Err(RswError::UnexpectedEof);
        }
        let skip_count = i32::from_le_bytes(data[cur..cur + 4].try_into().unwrap()) as usize;
        cur += 4 + 4 * skip_count;
        if cur > data.len() {
            return Err(RswError::UnexpectedEof);
        }
    }

    if cur + 4 > data.len() {
        return Err(RswError::UnexpectedEof);
    }

    let obj_count = u32::from_le_bytes(data[cur..cur + 4].try_into().unwrap()) as usize;
    cur += 4;

    let mut models = Vec::new();

    for _ in 0..obj_count {
        if cur + 4 > data.len() {
            break;
        }
        let obj_type = u32::from_le_bytes(data[cur..cur + 4].try_into().unwrap());
        cur += 4;

        match obj_type {
            1 => {
                // Model object
                let name = if version >= 1.3 {
                    if cur + 52 > data.len() {
                        break;
                    }
                    let n = read_cstring(&data[cur..cur + 40]);
                    cur += 52; // name(40) + animType(4) + animSpeed(4) + blockType(4)
                    n
                } else {
                    String::new()
                };

                if version >= 2.6 && build_number >= 186 {
                    if cur + 1 > data.len() {
                        break;
                    }
                    cur += 1;
                }
                if version >= 2.7 {
                    if cur + 4 > data.len() {
                        break;
                    }
                    cur += 4;
                }

                if cur + 80 + 80 + 12 + 12 + 12 > data.len() {
                    break;
                }

                let filename = normalize_model_path(&data[cur..cur + 80]);
                cur += 80;

                let node_name = read_cstring(&data[cur..cur + 80]);
                cur += 80;

                let px = f32::from_le_bytes(data[cur..cur + 4].try_into().unwrap()) / 5.0;
                let py = f32::from_le_bytes(data[cur + 4..cur + 8].try_into().unwrap()) / 5.0;
                let pz = f32::from_le_bytes(data[cur + 8..cur + 12].try_into().unwrap()) / 5.0;
                cur += 12;

                let rx = f32::from_le_bytes(data[cur..cur + 4].try_into().unwrap());
                let ry = f32::from_le_bytes(data[cur + 4..cur + 8].try_into().unwrap());
                let rz = f32::from_le_bytes(data[cur + 8..cur + 12].try_into().unwrap());
                cur += 12;

                let sx = f32::from_le_bytes(data[cur..cur + 4].try_into().unwrap()) / 5.0;
                let sy = f32::from_le_bytes(data[cur + 4..cur + 8].try_into().unwrap()) / 5.0;
                let sz = f32::from_le_bytes(data[cur + 8..cur + 12].try_into().unwrap()) / 5.0;
                cur += 12;

                models.push(RswModelObject {
                    name,
                    filename,
                    node_name,
                    position: [px, py, pz],
                    rotation: [rx, ry, rz],
                    scale: [sx, sy, sz],
                });
            }
            2 => {
                // Light: name(80) + pos(12) + color(12) + range(4) = 108 bytes
                if cur + 108 > data.len() {
                    break;
                }
                cur += 108;
            }
            3 => {
                // Sound: name(80) + file(80) + pos(12) + vol(4) + width(4) + height(4) + range(4) = 188 bytes (+ 4 if version >= 2.0)
                let sound_len = if version >= 2.0 { 192 } else { 188 };
                if cur + sound_len > data.len() {
                    break;
                }
                cur += sound_len;
            }
            4 => {
                // Effect: name(80) + pos(12) + id(4) + delay(4) + param(16) = 116 bytes
                if cur + 116 > data.len() {
                    break;
                }
                cur += 116;
            }
            _ => break,
        }
    }

    Ok(RswScene {
        major,
        minor,
        ini_file,
        gnd_file,
        gat_file,
        src_file,
        water_level,
        water_type,
        models,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rejects_invalid_magic() {
        let bad_magic = b"NOPE\x02\x01";
        assert_eq!(parse_rsw(bad_magic), Err(RswError::InvalidMagic));
    }

    #[test]
    fn test_rejects_truncated_data() {
        let short = b"GRSW\x02";
        assert_eq!(parse_rsw(short), Err(RswError::UnexpectedEof));
    }

    #[test]
    fn test_synthetic_rsw_parsing() {
        let mut buf = Vec::new();
        // Magic + version 2.1
        buf.extend_from_slice(b"GRSW\x02\x01");
        // ini (40)
        let mut ini = [0u8; 40];
        ini[0..7].copy_from_slice(b"map.ini");
        buf.extend_from_slice(&ini);
        // gnd (40)
        let mut gnd = [0u8; 40];
        gnd[0..7].copy_from_slice(b"map.gnd");
        buf.extend_from_slice(&gnd);
        // gat (40)
        let mut gat = [0u8; 40];
        gat[0..7].copy_from_slice(b"map.gat");
        buf.extend_from_slice(&gat);
        // src (40)
        let mut src = [0u8; 40];
        src[0..7].copy_from_slice(b"map.src");
        buf.extend_from_slice(&src);

        // Water: level (4) + type (4) + waveHeight (4) + waveSpeed (4) + wavePitch (4) + animSpeed (4) = 24 bytes
        buf.extend_from_slice(&(-25.0f32).to_le_bytes()); // level: -25 / 5 = -5
        buf.extend_from_slice(&1i32.to_le_bytes()); // type
        buf.extend_from_slice(&[0u8; 16]); // wave params + animSpeed

        // Light: 32 bytes + opacity 4 bytes = 36 bytes
        buf.extend_from_slice(&[0u8; 36]);
        // Ground: 16 bytes
        buf.extend_from_slice(&[0u8; 16]);

        // Object count = 1
        buf.extend_from_slice(&1u32.to_le_bytes());

        // Object 1: Model (type = 1)
        buf.extend_from_slice(&1u32.to_le_bytes());
        // name(40) + animType(4) + animSpeed(4) + blockType(4) = 52
        let mut model_name = [0u8; 40];
        model_name[0..8].copy_from_slice(b"fountain");
        buf.extend_from_slice(&model_name);
        buf.extend_from_slice(&[0u8; 12]); // animType, animSpeed, blockType
        // filename (80)
        let mut filename = [0u8; 80];
        let fname = b"prontera\\fount.rsm\0";
        filename[0..fname.len()].copy_from_slice(fname);
        buf.extend_from_slice(&filename);
        // nodename (80)
        let mut node = [0u8; 80];
        node[0..4].copy_from_slice(b"root");
        buf.extend_from_slice(&node);
        // pos (12)
        buf.extend_from_slice(&50.0f32.to_le_bytes()); // x = 50 / 5 = 10
        buf.extend_from_slice(&0.0f32.to_le_bytes());  // y = 0
        buf.extend_from_slice(&100.0f32.to_le_bytes()); // z = 100 / 5 = 20
        // rot (12)
        buf.extend_from_slice(&0.0f32.to_le_bytes());
        buf.extend_from_slice(&45.0f32.to_le_bytes());
        buf.extend_from_slice(&0.0f32.to_le_bytes());
        // scale (12)
        buf.extend_from_slice(&5.0f32.to_le_bytes()); // sx = 5 / 5 = 1
        buf.extend_from_slice(&5.0f32.to_le_bytes()); // sy = 5 / 5 = 1
        buf.extend_from_slice(&5.0f32.to_le_bytes()); // sz = 5 / 5 = 1

        let scene = parse_rsw(&buf).expect("Falha ao parsear cena sintética");
        assert_eq!(scene.major, 2);
        assert_eq!(scene.minor, 1);
        assert_eq!(scene.gnd_file, "map.gnd");
        assert_eq!(scene.gat_file, "map.gat");
        assert_eq!(scene.water_level, -5.0);
        assert_eq!(scene.water_type, 1);
        assert_eq!(scene.models.len(), 1);

        let m = &scene.models[0];
        assert_eq!(m.name, "fountain");
        assert_eq!(m.filename, "data/model/prontera/fount.rsm");
        assert_eq!(m.node_name, "root");
        assert_eq!(m.position, [10.0, 0.0, 20.0]);
        assert_eq!(m.rotation, [0.0, 45.0, 0.0]);
        assert_eq!(m.scale, [1.0, 1.0, 1.0]);
    }
}
