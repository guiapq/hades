//! SPEC-0013: Parser de Sprites 2D (.spr) com suporte a paleta de 256 cores e RLE.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SprError {
    #[error("Arquivo muito curto para conter cabeçalho SPR")]
    TooShort,

    #[error("Assinatura mágica inválida (esperado 'SP')")]
    InvalidMagic,

    #[error("Versão de sprite não suportada: 0x{0:04X}")]
    UnsupportedVersion(u16),

    #[error("Dados de quadro corrompidos ou incompletos")]
    CorruptedData,
}

/// Um quadro individual de sprite decodificado em formato 0xAARRGGBB.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteFrame {
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<u32>,
}

impl SpriteFrame {
    /// Obtém a cor de um pixel no quadro (retorna 0 para fora dos limites).
    #[inline(always)]
    pub fn get_pixel(&self, x: u16, y: u16) -> u32 {
        if x < self.width && y < self.height {
            self.pixels[(y as usize) * (self.width as usize) + (x as usize)]
        } else {
            0
        }
    }
}

/// Conjunto de quadros de um arquivo .spr decodificado.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Sprite {
    pub version: u16,
    pub frames: Vec<SpriteFrame>,
}

/// Decodifica um arquivo .spr em memória.
pub fn parse_spr(bytes: &[u8]) -> Result<Sprite, SprError> {
    if bytes.len() < 6 {
        return Err(SprError::TooShort);
    }

    if &bytes[0..2] != b"SP" {
        return Err(SprError::InvalidMagic);
    }

    let version = u16::from_le_bytes([bytes[2], bytes[3]]);
    let indexed_count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    let rgba_count = if version >= 0x0200 && bytes.len() >= 8 {
        u16::from_le_bytes([bytes[6], bytes[7]]) as usize
    } else {
        0
    };

    let mut cursor = if version >= 0x0200 { 8 } else { 6 };
    let mut frames = Vec::with_capacity(indexed_count + rgba_count);

    // 1. Extrai a paleta de 256 cores (1024 bytes) se houver quadros indexados
    let palette: [u32; 256] = if indexed_count > 0 {
        if bytes.len() < 1024 {
            return Err(SprError::TooShort);
        }
        let pal_offset = bytes.len() - 1024;
        let pal_bytes = &bytes[pal_offset..];
        let mut pal = [0u32; 256];
        for i in 0..256 {
            let r = pal_bytes[i * 4] as u32;
            let g = pal_bytes[i * 4 + 1] as u32;
            let b = pal_bytes[i * 4 + 2] as u32;
            // Índice 0 é estritamente transparente
            let a = if i == 0 { 0u32 } else { 255u32 };
            pal[i] = (a << 24) | (r << 16) | (g << 8) | b;
        }
        pal
    } else {
        [0u32; 256]
    };

    let pal_limit = if indexed_count > 0 {
        bytes.len() - 1024
    } else {
        bytes.len()
    };

    // 2. Decodifica quadros indexados (Paleta)
    for _ in 0..indexed_count {
        if version >= 0x0201 {
            if cursor + 6 > pal_limit {
                return Err(SprError::CorruptedData);
            }
            let width = u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]);
            let height = u16::from_le_bytes([bytes[cursor + 2], bytes[cursor + 3]]);
            let encoded_size = u16::from_le_bytes([bytes[cursor + 4], bytes[cursor + 5]]) as usize;
            cursor += 6;

            let frame_end = (cursor + encoded_size).min(pal_limit);
            let total_pixels = (width as usize) * (height as usize);
            let mut pixels = Vec::with_capacity(total_pixels);

            // Decodificação RLE (Run-Length Encoding)
            while pixels.len() < total_pixels && cursor < frame_end {
                let b = bytes[cursor];
                cursor += 1;
                if b == 0 {
                    if cursor < frame_end {
                        let count = bytes[cursor] as usize;
                        cursor += 1;
                        let to_add = count.min(total_pixels - pixels.len());
                        pixels.extend(std::iter::repeat_n(palette[0], to_add));
                    }
                } else {
                    pixels.push(palette[b as usize]);
                }
            }
            cursor = frame_end;

            if pixels.len() < total_pixels {
                pixels.resize(total_pixels, palette[0]);
            }

            frames.push(SpriteFrame {
                width,
                height,
                pixels,
            });
        } else {
            if cursor + 4 > pal_limit {
                return Err(SprError::CorruptedData);
            }
            let width = u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]);
            let height = u16::from_le_bytes([bytes[cursor + 2], bytes[cursor + 3]]);
            cursor += 4;

            let total_pixels = (width as usize) * (height as usize);
            if cursor + total_pixels > pal_limit {
                return Err(SprError::CorruptedData);
            }
            let mut pixels = Vec::with_capacity(total_pixels);
            for i in 0..total_pixels {
                let idx = bytes[cursor + i] as usize;
                pixels.push(palette[idx]);
            }
            cursor += total_pixels;

            frames.push(SpriteFrame {
                width,
                height,
                pixels,
            });
        }
    }

    // 3. Decodifica quadros RGBA diretos (se houver)
    let rgba_limit = bytes.len();
    for _ in 0..rgba_count {
        if cursor + 4 > rgba_limit {
            break;
        }
        let width = u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]);
        let height = u16::from_le_bytes([bytes[cursor + 2], bytes[cursor + 3]]);
        cursor += 4;

        let total_pixels = (width as usize) * (height as usize);
        if cursor + total_pixels * 4 > rgba_limit {
            break;
        }

        let mut pixels = Vec::with_capacity(total_pixels);
        for _ in 0..total_pixels {
            let r = bytes[cursor] as u32;
            let g = bytes[cursor + 1] as u32;
            let b = bytes[cursor + 2] as u32;
            let a = bytes[cursor + 3] as u32;
            cursor += 4;
            pixels.push((a << 24) | (r << 16) | (g << 8) | b);
        }

        frames.push(SpriteFrame {
            width,
            height,
            pixels,
        });
    }

    Ok(Sprite { version, frames })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reject_invalid_magic_or_short() {
        assert_eq!(parse_spr(&[]), Err(SprError::TooShort));
        assert_eq!(parse_spr(b"INVALID"), Err(SprError::InvalidMagic));
    }

    #[test]
    fn test_parse_synthetic_spr_rle() {
        let mut data = Vec::new();
        // Magic
        data.extend_from_slice(b"SP");
        // Version 0x0201
        data.extend_from_slice(&0x0201u16.to_le_bytes());
        // 1 indexed frame
        data.extend_from_slice(&1u16.to_le_bytes());
        // 0 rgba frames
        data.extend_from_slice(&0u16.to_le_bytes());

        // Frame dims: 4x2 and encoded_size (6 bytes payload)
        data.extend_from_slice(&4u16.to_le_bytes());
        data.extend_from_slice(&2u16.to_le_bytes());
        data.extend_from_slice(&6u16.to_le_bytes());

        // RLE: 2 transparent (0, 2), then index 1, then index 2, then 4 transparent (0, 4)
        data.extend_from_slice(&[0, 2, 1, 2, 0, 4]);

        // Palette (1024 bytes)
        let mut pal = [0u8; 1024];
        // Index 1 = Red
        pal[4] = 255;
        pal[5] = 0;
        pal[6] = 0;
        // Index 2 = Green
        pal[8] = 0;
        pal[9] = 255;
        pal[10] = 0;

        data.extend_from_slice(&pal);

        let sprite = parse_spr(&data).expect("deve decodificar sprite sintético");
        assert_eq!(sprite.frames.len(), 1);
        let f0 = &sprite.frames[0];
        assert_eq!(f0.width, 4);
        assert_eq!(f0.height, 2);
        assert_eq!(f0.pixels.len(), 8);

        // Pixel 0 e 1: transparente (alpha = 0)
        assert_eq!(f0.pixels[0] >> 24, 0);
        assert_eq!(f0.pixels[1] >> 24, 0);
        // Pixel 2: Vermelho opaco (0xFFFF0000)
        assert_eq!(f0.pixels[2], 0xFFFF0000);
        // Pixel 3: Verde opaco (0xFF00FF00)
        assert_eq!(f0.pixels[3], 0xFF00FF00);
    }
}
