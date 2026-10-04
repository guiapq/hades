//! SPEC-0028: Parser Ultraleve de Texturas de Imagem (.bmp).
//!
//! Decodifica imagens BMP de 8 bits indexadas (com paleta) e 24/32 bits sem dependências
//! externas, convertendo para buffers contíguos de pixels ARGB8888.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BmpError {
    #[error("Arquivo muito curto para conter cabeçalho BMP")]
    TooShort,
    #[error("Assinatura mágica inválida (esperado 'BM')")]
    InvalidMagic,
    #[error("Formato de cabeçalho DIB não suportado: {0}")]
    UnsupportedHeader(u32),
    #[error("Profundidade de cor (bpp) não suportada: {0}")]
    UnsupportedBpp(u16),
    #[error("Compressão BMP não suportada: {0}")]
    UnsupportedCompression(u32),
    #[error("Dados de pixels incompletos ou corrompidos")]
    CorruptedData,
}

/// Imagem decodificada pronta para uso no renderizador.
#[derive(Debug, Clone, PartialEq)]
pub struct BmpImage {
    pub width: u32,
    pub height: u32,
    /// Pixels em formato ARGB8888 (0xAARRGGBB) ordenados de cima para baixo.
    pub pixels: Vec<u32>,
}

impl BmpImage {
    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> u32 {
        if x < self.width && y < self.height {
            self.pixels[(y * self.width + x) as usize]
        } else {
            0
        }
    }

    /// Amostra textura usando coordenadas normalizadas UV (0.0..1.0) com wrapping (clamp/repeat).
    #[inline]
    pub fn sample_uv(&self, u: f32, v: f32) -> u32 {
        if self.width == 0 || self.height == 0 {
            return 0;
        }
        let u_wrapped = u.rem_euclid(1.0);
        let v_wrapped = v.rem_euclid(1.0);
        let px = ((u_wrapped * self.width as f32) as u32).min(self.width - 1);
        let py = ((v_wrapped * self.height as f32) as u32).min(self.height - 1);
        self.pixels[(py * self.width + px) as usize]
    }
}

/// Decodifica um arquivo BMP a partir de um slice de bytes em memória.
pub fn parse_bmp(bytes: &[u8]) -> Result<BmpImage, BmpError> {
    if bytes.len() < 54 {
        return Err(BmpError::TooShort);
    }

    if &bytes[0..2] != b"BM" {
        return Err(BmpError::InvalidMagic);
    }

    let data_offset = u32::from_le_bytes(bytes[10..14].try_into().unwrap()) as usize;
    let dib_header_size = u32::from_le_bytes(bytes[14..18].try_into().unwrap());

    if dib_header_size < 40 {
        return Err(BmpError::UnsupportedHeader(dib_header_size));
    }

    let raw_w = i32::from_le_bytes(bytes[18..22].try_into().unwrap());
    let raw_h = i32::from_le_bytes(bytes[22..26].try_into().unwrap());

    if raw_w <= 0 || raw_h == 0 {
        return Err(BmpError::CorruptedData);
    }

    let width = raw_w as u32;
    let is_bottom_up = raw_h > 0;
    let height = raw_h.unsigned_abs();

    let planes = u16::from_le_bytes(bytes[26..28].try_into().unwrap());
    let bpp = u16::from_le_bytes(bytes[28..30].try_into().unwrap());
    let compression = u32::from_le_bytes(bytes[30..34].try_into().unwrap());

    if planes != 1 {
        return Err(BmpError::CorruptedData);
    }

    // 0 = BI_RGB (sem compressão)
    if compression != 0 {
        return Err(BmpError::UnsupportedCompression(compression));
    }

    let total_pixels = (width * height) as usize;
    let mut pixels = vec![0u32; total_pixels];

    match bpp {
        8 => {
            // Imagem indexada com paleta de 256 cores (típica das texturas de chão do GRF)
            let palette_offset = 14 + dib_header_size as usize;
            let palette_colors = if data_offset > palette_offset {
                (data_offset - palette_offset) / 4
            } else {
                256
            };

            let mut palette = [0u32; 256];
            for i in 0..palette_colors.min(256) {
                let p_idx = palette_offset + i * 4;
                if p_idx + 4 <= bytes.len() {
                    let b = bytes[p_idx] as u32;
                    let g = bytes[p_idx + 1] as u32;
                    let r = bytes[p_idx + 2] as u32;
                    palette[i] = 0xFF00_0000 | (r << 16) | (g << 8) | b;
                }
            }

            // Cada linha de pixels no BMP é alinhada a múltiplos de 4 bytes
            let row_stride = (width as usize + 3) & !3;

            for y in 0..height {
                let target_y = if is_bottom_up { height - 1 - y } else { y } as usize;
                let row_start = data_offset + (y as usize) * row_stride;
                if row_start + width as usize > bytes.len() {
                    break;
                }

                for x in 0..width as usize {
                    let color_idx = bytes[row_start + x] as usize;
                    pixels[target_y * (width as usize) + x] = palette[color_idx];
                }
            }
        }
        24 => {
            // BGR 24-bit
            let row_stride = (width as usize * 3 + 3) & !3;

            for y in 0..height {
                let target_y = if is_bottom_up { height - 1 - y } else { y } as usize;
                let row_start = data_offset + (y as usize) * row_stride;
                if row_start + (width as usize) * 3 > bytes.len() {
                    break;
                }

                for x in 0..width as usize {
                    let px_idx = row_start + x * 3;
                    let b = bytes[px_idx] as u32;
                    let g = bytes[px_idx + 1] as u32;
                    let r = bytes[px_idx + 2] as u32;
                    pixels[target_y * (width as usize) + x] = 0xFF00_0000 | (r << 16) | (g << 8) | b;
                }
            }
        }
        32 => {
            // BGRA 32-bit
            let row_stride = width as usize * 4;

            for y in 0..height {
                let target_y = if is_bottom_up { height - 1 - y } else { y } as usize;
                let row_start = data_offset + (y as usize) * row_stride;
                if row_start + row_stride > bytes.len() {
                    break;
                }

                for x in 0..width as usize {
                    let px_idx = row_start + x * 4;
                    let b = bytes[px_idx] as u32;
                    let g = bytes[px_idx + 1] as u32;
                    let r = bytes[px_idx + 2] as u32;
                    let a = bytes[px_idx + 3] as u32;
                    let alpha = if a == 0 { 0xFF } else { a };
                    pixels[target_y * (width as usize) + x] = (alpha << 24) | (r << 16) | (g << 8) | b;
                }
            }
        }
        unsupported => return Err(BmpError::UnsupportedBpp(unsupported)),
    }

    Ok(BmpImage {
        width,
        height,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bmp_synthetic_24bit_rgb() {
        let mut buf = Vec::new();
        // File Header (14 bytes)
        buf.extend_from_slice(b"BM");
        buf.extend_from_slice(&58u32.to_le_bytes()); // File size: 54 header + 4 bytes row
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&54u32.to_le_bytes()); // Data offset: 54

        // DIB Header (40 bytes)
        buf.extend_from_slice(&40u32.to_le_bytes());
        buf.extend_from_slice(&1i32.to_le_bytes()); // width = 1
        buf.extend_from_slice(&1i32.to_le_bytes()); // height = 1
        buf.extend_from_slice(&1u16.to_le_bytes()); // planes = 1
        buf.extend_from_slice(&24u16.to_le_bytes()); // bpp = 24
        buf.extend_from_slice(&0u32.to_le_bytes()); // comp = 0
        buf.extend_from_slice(&4u32.to_le_bytes()); // img size
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());

        // Pixel row: 1 pixel BGR (0x00, 0xFF, 0x00) + 1 byte pad = 4 bytes
        buf.extend_from_slice(&[0x00, 0xFF, 0x00, 0x00]);

        let img = parse_bmp(&buf).expect("parse bmp");
        assert_eq!(img.width, 1);
        assert_eq!(img.height, 1);
        assert_eq!(img.get_pixel(0, 0), 0xFF00_FF00); // Verde puro ARGB
    }

    #[test]
    fn test_bmp_reject_invalid_magic() {
        let mut buf = vec![0u8; 60];
        buf[0] = b'X';
        buf[1] = b'Y';
        assert_eq!(parse_bmp(&buf), Err(BmpError::InvalidMagic));
    }
}
