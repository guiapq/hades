//! Parser para modelos 3D estáticos (.rsm) de prédios e cenários.
//!
//! Suporta versões 1.x (notadamente 1.4 e 1.5) de modelos poligonais do motor clássico.

use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum RsmError {
    #[error("Assinatura mágica inválida (esperado 'GRSM')")]
    InvalidMagic,
    #[error("Buffer muito curto para conter o cabeçalho RSM")]
    BufferTooShort,
    #[error("Versão RSM não suportada: {0}.{1}")]
    UnsupportedVersion(u8, u8),
    #[error("Erro de formato ao decodificar nós ou faces: {0}")]
    MalformedData(&'static str),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RsmFace {
    pub vertices: [u16; 3],
    pub texcoords: [u16; 3],
    pub tex_id: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RsmNode {
    pub name: String,
    pub parent_name: String,
    pub textures: Vec<u32>,
    pub offset_matrix: [f32; 16],
    pub position: (f32, f32, f32),
    pub scale: (f32, f32, f32),
    pub vertices: Vec<(f32, f32, f32)>,
    pub texcoords: Vec<(f32, f32)>,
    pub faces: Vec<RsmFace>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RsmModel {
    pub major: u8,
    pub minor: u8,
    pub anim_len: u32,
    pub shade_type: u32,
    pub alpha: u8,
    pub textures: Vec<String>,
    pub root_node: String,
    pub nodes: Vec<RsmNode>,
}

/// Realiza o parsing de um modelo 3D `.rsm`.
pub fn parse_rsm(data: &[u8]) -> Result<RsmModel, RsmError> {
    // Cabeçalho básico mínimo: GRSM(4) + ver(2) + anim(4) + shade(4) + alpha(1) + reserved(16) = 31 bytes
    if data.len() < 31 {
        return Err(RsmError::BufferTooShort);
    }

    if &data[0..4] != b"GRSM" {
        return Err(RsmError::InvalidMagic);
    }

    let major = data[4];
    let minor = data[5];
    if major != 1 {
        return Err(RsmError::UnsupportedVersion(major, minor));
    }

    let anim_len = u32::from_le_bytes(data[6..10].try_into().unwrap());
    let shade_type = u32::from_le_bytes(data[10..14].try_into().unwrap());
    let alpha = data[14];

    let mut cur = 31; // após 16 bytes reservados

    // 1. Texturas
    if cur + 4 > data.len() {
        return Err(RsmError::BufferTooShort);
    }
    let tex_count = u32::from_le_bytes(data[cur..cur + 4].try_into().unwrap()) as usize;
    cur += 4;

    if cur + tex_count * 40 > data.len() {
        return Err(RsmError::BufferTooShort);
    }
    let mut textures = Vec::with_capacity(tex_count);
    for _ in 0..tex_count {
        let name_bytes = &data[cur..cur + 40];
        let name = String::from_utf8_lossy(name_bytes)
            .split('\0')
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        textures.push(name);
        cur += 40;
    }

    // 2. Root Node Name
    if cur + 40 > data.len() {
        return Err(RsmError::BufferTooShort);
    }
    let root_node = String::from_utf8_lossy(&data[cur..cur + 40])
        .split('\0')
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    cur += 40;

    // 3. Nós de Malha (Nodes)
    if cur + 4 > data.len() {
        return Err(RsmError::BufferTooShort);
    }
    let node_count = u32::from_le_bytes(data[cur..cur + 4].try_into().unwrap()) as usize;
    cur += 4;

    let mut nodes = Vec::with_capacity(node_count);
    for _ in 0..node_count {
        if cur + 80 > data.len() {
            return Err(RsmError::BufferTooShort);
        }
        let name = String::from_utf8_lossy(&data[cur..cur + 40])
            .split('\0')
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        cur += 40;
        let parent_name = String::from_utf8_lossy(&data[cur..cur + 40])
            .split('\0')
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        cur += 40;

        if cur + 4 > data.len() {
            return Err(RsmError::BufferTooShort);
        }
        let node_tex_count = u32::from_le_bytes(data[cur..cur + 4].try_into().unwrap()) as usize;
        cur += 4;

        if cur + node_tex_count * 4 > data.len() {
            return Err(RsmError::BufferTooShort);
        }
        let mut node_textures = Vec::with_capacity(node_tex_count);
        for _ in 0..node_tex_count {
            node_textures.push(u32::from_le_bytes(data[cur..cur + 4].try_into().unwrap()));
            cur += 4;
        }

        // Matriz de Transformação Offset (16 floats = 64 bytes)
        if cur + 64 > data.len() {
            return Err(RsmError::BufferTooShort);
        }
        let mut offset_matrix = [0.0f32; 16];
        for val in &mut offset_matrix {
            *val = f32::from_le_bytes(data[cur..cur + 4].try_into().unwrap());
            cur += 4;
        }

        // Posição (3 floats = 12 bytes) e Escala (3 floats = 12 bytes)
        if cur + 24 > data.len() {
            return Err(RsmError::BufferTooShort);
        }
        let pos_x = f32::from_le_bytes(data[cur..cur + 4].try_into().unwrap());
        let pos_y = f32::from_le_bytes(data[cur + 4..cur + 8].try_into().unwrap());
        let pos_z = f32::from_le_bytes(data[cur + 8..cur + 12].try_into().unwrap());
        cur += 12;

        let scale_x = f32::from_le_bytes(data[cur..cur + 4].try_into().unwrap());
        let scale_y = f32::from_le_bytes(data[cur + 4..cur + 8].try_into().unwrap());
        let scale_z = f32::from_le_bytes(data[cur + 8..cur + 12].try_into().unwrap());
        cur += 12;

        // Vértices 3D
        if cur + 4 > data.len() {
            return Err(RsmError::BufferTooShort);
        }
        let vert_count = u32::from_le_bytes(data[cur..cur + 4].try_into().unwrap()) as usize;
        cur += 4;

        if cur + vert_count * 12 > data.len() {
            return Err(RsmError::BufferTooShort);
        }
        let mut vertices = Vec::with_capacity(vert_count);
        for _ in 0..vert_count {
            let vx = f32::from_le_bytes(data[cur..cur + 4].try_into().unwrap());
            let vy = f32::from_le_bytes(data[cur + 4..cur + 8].try_into().unwrap());
            let vz = f32::from_le_bytes(data[cur + 8..cur + 12].try_into().unwrap());
            vertices.push((vx, vy, vz));
            cur += 12;
        }

        // Coordenadas de Textura (TexCoords: 12 bytes cada em RSM 1.4+)
        if cur + 4 > data.len() {
            return Err(RsmError::BufferTooShort);
        }
        let tc_count = u32::from_le_bytes(data[cur..cur + 4].try_into().unwrap()) as usize;
        cur += 4;

        if cur + tc_count * 12 > data.len() {
            return Err(RsmError::BufferTooShort);
        }
        let mut texcoords = Vec::with_capacity(tc_count);
        for _ in 0..tc_count {
            // pula 4 bytes de cor/reservado
            let u = f32::from_le_bytes(data[cur + 4..cur + 8].try_into().unwrap());
            let v = f32::from_le_bytes(data[cur + 8..cur + 12].try_into().unwrap());
            texcoords.push((u, v));
            cur += 12;
        }

        // Faces Triangulares (24 bytes cada)
        if cur + 4 > data.len() {
            return Err(RsmError::BufferTooShort);
        }
        let face_count = u32::from_le_bytes(data[cur..cur + 4].try_into().unwrap()) as usize;
        cur += 4;

        if cur + face_count * 24 > data.len() {
            return Err(RsmError::BufferTooShort);
        }
        let mut faces = Vec::with_capacity(face_count);
        for _ in 0..face_count {
            let v0 = u16::from_le_bytes([data[cur], data[cur + 1]]);
            let v1 = u16::from_le_bytes([data[cur + 2], data[cur + 3]]);
            let v2 = u16::from_le_bytes([data[cur + 4], data[cur + 5]]);

            let t0 = u16::from_le_bytes([data[cur + 6], data[cur + 7]]);
            let t1 = u16::from_le_bytes([data[cur + 8], data[cur + 9]]);
            let t2 = u16::from_le_bytes([data[cur + 10], data[cur + 11]]);

            let tex_id = u16::from_le_bytes([data[cur + 12], data[cur + 13]]);
            faces.push(RsmFace {
                vertices: [v0, v1, v2],
                texcoords: [t0, t1, t2],
                tex_id,
            });
            cur += 24;
        }

        // Pula quadros de animação de translação (pos_keys) se presentes
        if cur + 4 <= data.len() {
            let pk_count = u32::from_le_bytes(data[cur..cur + 4].try_into().unwrap()) as usize;
            cur += 4;
            let bytes_to_skip = pk_count.saturating_mul(20);
            if cur + bytes_to_skip <= data.len() {
                cur += bytes_to_skip;
            }
        }

        nodes.push(RsmNode {
            name,
            parent_name,
            textures: node_textures,
            offset_matrix,
            position: (pos_x, pos_y, pos_z),
            scale: (scale_x, scale_y, scale_z),
            vertices,
            texcoords,
            faces,
        });
    }

    Ok(RsmModel {
        major,
        minor,
        anim_len,
        shade_type,
        alpha,
        textures,
        root_node,
        nodes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsm_rejects_invalid_magic() {
        let data = b"NOT_GRSM_HEADER_HERE";
        assert_eq!(parse_rsm(data), Err(RsmError::BufferTooShort));

        let data32 = [0u8; 32];
        assert_eq!(parse_rsm(&data32), Err(RsmError::InvalidMagic));
    }

    #[test]
    fn test_rsm_parse_synthetic_cube() {
        let mut buf = Vec::new();
        // Magic & Version 1.4
        buf.extend_from_slice(b"GRSM");
        buf.push(1); // major
        buf.push(4); // minor
        buf.extend_from_slice(&1000u32.to_le_bytes()); // anim_len
        buf.extend_from_slice(&0u32.to_le_bytes()); // shade_type
        buf.push(255); // alpha
        buf.extend_from_slice(&[0u8; 16]); // reserved

        // Texturas: 1 textura
        buf.extend_from_slice(&1u32.to_le_bytes());
        let mut tex_name = [0u8; 40];
        tex_name[..8].copy_from_slice(b"wall.bmp");
        buf.extend_from_slice(&tex_name);

        // Root node
        let mut root_name = [0u8; 40];
        root_name[..4].copy_from_slice(b"root");
        buf.extend_from_slice(&root_name);

        // Nodes: 1 nó
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&root_name); // node name
        buf.extend_from_slice(&[0u8; 40]); // parent name

        // node textures: 1 texture
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());

        // matrix: 16 floats
        for _ in 0..16 {
            buf.extend_from_slice(&1.0f32.to_le_bytes());
        }

        // pos (3 floats) e scale (3 floats)
        for _ in 0..6 {
            buf.extend_from_slice(&0.0f32.to_le_bytes());
        }

        // Vertices: 3 vertices formando 1 triângulo
        buf.extend_from_slice(&3u32.to_le_bytes());
        // V0 (0, 0, 0)
        buf.extend_from_slice(&0.0f32.to_le_bytes());
        buf.extend_from_slice(&0.0f32.to_le_bytes());
        buf.extend_from_slice(&0.0f32.to_le_bytes());
        // V1 (10, 0, 0)
        buf.extend_from_slice(&10.0f32.to_le_bytes());
        buf.extend_from_slice(&0.0f32.to_le_bytes());
        buf.extend_from_slice(&0.0f32.to_le_bytes());
        // V2 (0, 10, 0)
        buf.extend_from_slice(&0.0f32.to_le_bytes());
        buf.extend_from_slice(&10.0f32.to_le_bytes());
        buf.extend_from_slice(&0.0f32.to_le_bytes());

        // TexCoords: 3
        buf.extend_from_slice(&3u32.to_le_bytes());
        for _ in 0..3 {
            buf.extend_from_slice(&0u32.to_le_bytes()); // color
            buf.extend_from_slice(&0.0f32.to_le_bytes()); // u
            buf.extend_from_slice(&0.0f32.to_le_bytes()); // v
        }

        // Faces: 1 face
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&0u16.to_le_bytes()); // v0
        buf.extend_from_slice(&1u16.to_le_bytes()); // v1
        buf.extend_from_slice(&2u16.to_le_bytes()); // v2
        buf.extend_from_slice(&0u16.to_le_bytes()); // t0
        buf.extend_from_slice(&1u16.to_le_bytes()); // t1
        buf.extend_from_slice(&2u16.to_le_bytes()); // t2
        buf.extend_from_slice(&0u16.to_le_bytes()); // tex_id
        buf.extend_from_slice(&0u16.to_le_bytes()); // pad
        buf.extend_from_slice(&0i32.to_le_bytes()); // two_sided
        buf.extend_from_slice(&0i32.to_le_bytes()); // smooth_group

        let model = parse_rsm(&buf).expect("Deveria parsear modelo sintético com sucesso");
        assert_eq!(model.major, 1);
        assert_eq!(model.minor, 4);
        assert_eq!(model.textures.len(), 1);
        assert_eq!(model.textures[0], "wall.bmp");
        assert_eq!(model.nodes.len(), 1);
        assert_eq!(model.nodes[0].vertices.len(), 3);
        assert_eq!(model.nodes[0].faces.len(), 1);
        assert_eq!(model.nodes[0].faces[0].vertices, [0, 1, 2]);
    }
}
