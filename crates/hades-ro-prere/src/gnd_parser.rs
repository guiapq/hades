//! SPEC-0028: Parser de Malhas de Terreno e Texturização de Chão (.gnd).
//!
//! Decodifica a geometria de terreno, coordenadas UV de ladrilhos (tiles),
//! referências de texturas de piso e matrizes de células em mapas clássicos.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GndError {
    #[error("Arquivo muito curto para conter cabeçalho GND")]
    TooShort,
    #[error("Assinatura mágica inválida (esperado 'GRGN')")]
    InvalidMagic,
    #[error("Dados de terreno ou células corrompidos")]
    CorruptedData,
}

/// Um ladrilho (superfície texturizada individual com coordenadas UV).
#[derive(Debug, Clone, PartialEq)]
pub struct GndTile {
    pub u: [f32; 4],
    pub v: [f32; 4],
    pub texture_index: u16,
    pub lightmap_index: u16,
    pub color: [u8; 4],
}

/// Uma célula ou cubo do grid de terreno GND.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GndCell {
    /// Elevações dos 4 vértices da célula: [top-left, top-right, bottom-left, bottom-right].
    pub heights: [f32; 4],
    /// Índice do ladrilho da face superior (chão/piso) na lista `tiles` (-1 se ausente).
    pub top_surface: i32,
    /// Índice do ladrilho da face frontal (degraus norte/sul).
    pub front_surface: i32,
    /// Índice do ladrilho da face lateral direita (degraus leste/oeste).
    pub right_surface: i32,
}

/// Malha completa de terreno decodificada a partir de um arquivo .gnd.
#[derive(Debug, Clone, PartialEq)]
pub struct GndMesh {
    pub version_major: u8,
    pub version_minor: u8,
    pub width: u32,
    pub height: u32,
    pub zoom: f32,
    pub textures: Vec<String>,
    pub tiles: Vec<GndTile>,
    pub cells: Vec<GndCell>,
}

impl GndMesh {
    /// Obtém a célula do grid de terreno em coordenadas GND `(x, y)`.
    #[inline]
    pub fn cell(&self, x: u32, y: u32) -> Option<&GndCell> {
        if x < self.width && y < self.height {
            self.cells.get((y * self.width + x) as usize)
        } else {
            None
        }
    }

    /// Obtém o ladrilho da superfície superior (chão) de uma célula.
    #[inline]
    pub fn tile_for_cell(&self, x: u32, y: u32) -> Option<&GndTile> {
        let cell = self.cell(x, y)?;
        if cell.top_surface >= 0 {
            self.tiles.get(cell.top_surface as usize)
        } else {
            None
        }
    }

    /// Obtém o caminho relativo da textura para a célula especificada.
    #[inline]
    pub fn texture_for_cell(&self, x: u32, y: u32) -> Option<&str> {
        let tile = self.tile_for_cell(x, y)?;
        self.textures.get(tile.texture_index as usize).map(|s| s.as_str())
    }
}

/// Decodifica um arquivo .gnd em memória.
pub fn parse_gnd(bytes: &[u8]) -> Result<GndMesh, GndError> {
    if bytes.len() < 26 {
        return Err(GndError::TooShort);
    }

    if &bytes[0..4] != b"GRGN" {
        return Err(GndError::InvalidMagic);
    }

    let version_major = bytes[4];
    let version_minor = bytes[5];
    let width = u32::from_le_bytes(bytes[6..10].try_into().unwrap());
    let height = u32::from_le_bytes(bytes[10..14].try_into().unwrap());
    let zoom = f32::from_le_bytes(bytes[14..18].try_into().unwrap());
    let tex_count = u32::from_le_bytes(bytes[18..22].try_into().unwrap()) as usize;
    let tex_name_len = u32::from_le_bytes(bytes[22..26].try_into().unwrap()) as usize;

    let mut cur = 26;
    let mut textures = Vec::with_capacity(tex_count);

    for _ in 0..tex_count {
        if cur + tex_name_len > bytes.len() {
            return Err(GndError::CorruptedData);
        }
        let name_bytes = &bytes[cur..cur + tex_name_len];
        let null_pos = name_bytes.iter().position(|&b| b == 0).unwrap_or(tex_name_len);
        let (decoded, _, _) = encoding_rs::EUC_KR.decode(&name_bytes[..null_pos]);
        let normalized = format!("data/texture/{}", decoded.replace('\\', "/").to_lowercase());
        textures.push(normalized);
        cur += tex_name_len;
    }

    // Lightmaps
    if cur + 16 > bytes.len() {
        return Err(GndError::CorruptedData);
    }
    let lightmap_count = u32::from_le_bytes(bytes[cur..cur + 4].try_into().unwrap()) as usize;
    let lm_w = u32::from_le_bytes(bytes[cur + 4..cur + 8].try_into().unwrap()) as usize;
    let lm_h = u32::from_le_bytes(bytes[cur + 8..cur + 12].try_into().unwrap()) as usize;
    let _lm_cell_size = u32::from_le_bytes(bytes[cur + 12..cur + 16].try_into().unwrap());
    cur += 16;

    let lm_bytes_per_map = lm_w * lm_h * 4;
    cur += lightmap_count * lm_bytes_per_map;

    // Tiles (Superfícies)
    if cur + 4 > bytes.len() {
        return Err(GndError::CorruptedData);
    }
    let tile_count = u32::from_le_bytes(bytes[cur..cur + 4].try_into().unwrap()) as usize;
    cur += 4;

    let mut tiles = Vec::with_capacity(tile_count);
    let tile_byte_size = 40; // 16 (u) + 16 (v) + 2 (tex) + 2 (lm) + 4 (color)

    for _ in 0..tile_count {
        if cur + tile_byte_size > bytes.len() {
            return Err(GndError::CorruptedData);
        }
        let u = [
            f32::from_le_bytes(bytes[cur..cur + 4].try_into().unwrap()),
            f32::from_le_bytes(bytes[cur + 4..cur + 8].try_into().unwrap()),
            f32::from_le_bytes(bytes[cur + 8..cur + 12].try_into().unwrap()),
            f32::from_le_bytes(bytes[cur + 12..cur + 16].try_into().unwrap()),
        ];
        let v = [
            f32::from_le_bytes(bytes[cur + 16..cur + 20].try_into().unwrap()),
            f32::from_le_bytes(bytes[cur + 20..cur + 24].try_into().unwrap()),
            f32::from_le_bytes(bytes[cur + 24..cur + 28].try_into().unwrap()),
            f32::from_le_bytes(bytes[cur + 28..cur + 32].try_into().unwrap()),
        ];
        let texture_index = u16::from_le_bytes(bytes[cur + 32..cur + 34].try_into().unwrap());
        let lightmap_index = u16::from_le_bytes(bytes[cur + 34..cur + 36].try_into().unwrap());
        let color = [bytes[cur + 36], bytes[cur + 37], bytes[cur + 38], bytes[cur + 39]];

        tiles.push(GndTile {
            u,
            v,
            texture_index,
            lightmap_index,
            color,
        });
        cur += tile_byte_size;
    }

    // Grid de Células
    let total_cells = (width * height) as usize;
    let mut cells = Vec::with_capacity(total_cells);
    let cell_byte_size = 28; // 16 (heights) + 4 (top) + 4 (front) + 4 (right)

    for _ in 0..total_cells {
        if cur + cell_byte_size > bytes.len() {
            break;
        }
        let heights = [
            f32::from_le_bytes(bytes[cur..cur + 4].try_into().unwrap()),
            f32::from_le_bytes(bytes[cur + 4..cur + 8].try_into().unwrap()),
            f32::from_le_bytes(bytes[cur + 8..cur + 12].try_into().unwrap()),
            f32::from_le_bytes(bytes[cur + 12..cur + 16].try_into().unwrap()),
        ];
        let top_surface = i32::from_le_bytes(bytes[cur + 16..cur + 20].try_into().unwrap());
        let front_surface = i32::from_le_bytes(bytes[cur + 20..cur + 24].try_into().unwrap());
        let right_surface = i32::from_le_bytes(bytes[cur + 24..cur + 28].try_into().unwrap());

        cells.push(GndCell {
            heights,
            top_surface,
            front_surface,
            right_surface,
        });
        cur += cell_byte_size;
    }

    Ok(GndMesh {
        version_major,
        version_minor,
        width,
        height,
        zoom,
        textures,
        tiles,
        cells,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gnd_reject_invalid_magic() {
        let buf = [0u8; 30];
        assert_eq!(parse_gnd(&buf), Err(GndError::InvalidMagic));
    }

    #[test]
    fn test_gnd_synthetic_minimal() {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"GRGN");
        buf.push(1); // major
        buf.push(7); // minor
        buf.extend_from_slice(&1u32.to_le_bytes()); // w=1
        buf.extend_from_slice(&1u32.to_le_bytes()); // h=1
        buf.extend_from_slice(&10.0f32.to_le_bytes()); // zoom=10.0
        buf.extend_from_slice(&1u32.to_le_bytes()); // tex_count=1
        buf.extend_from_slice(&80u32.to_le_bytes()); // tex_name_len=80

        // 1 texture name (80 bytes)
        let mut name = [0u8; 80];
        name[..8].copy_from_slice(b"tile.bmp");
        buf.extend_from_slice(&name);

        // Lightmaps: count=0, w=0, h=0, cell_size=0
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());

        // Tiles: count=1 (40 bytes)
        buf.extend_from_slice(&1u32.to_le_bytes());
        for _ in 0..8 {
            buf.extend_from_slice(&0.0f32.to_le_bytes());
        }
        buf.extend_from_slice(&0u16.to_le_bytes()); // tex_idx = 0
        buf.extend_from_slice(&0u16.to_le_bytes());
        buf.extend_from_slice(&[255, 255, 255, 255]); // color

        // Cells: 1 cell (28 bytes)
        for _ in 0..4 {
            buf.extend_from_slice(&0.0f32.to_le_bytes());
        }
        buf.extend_from_slice(&0i32.to_le_bytes()); // top_surface = 0
        buf.extend_from_slice(&(-1i32).to_le_bytes());
        buf.extend_from_slice(&(-1i32).to_le_bytes());

        let gnd = parse_gnd(&buf).expect("parse minimal gnd");
        assert_eq!(gnd.width, 1);
        assert_eq!(gnd.height, 1);
        assert_eq!(gnd.textures.len(), 1);
        assert_eq!(gnd.textures[0], "data/texture/tile.bmp");
        assert_eq!(gnd.texture_for_cell(0, 0), Some("data/texture/tile.bmp"));
    }
}
