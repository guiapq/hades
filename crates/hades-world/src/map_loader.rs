//! SPEC-0016: Carregador de mapas e colisão (.gat / GRF) para o World Server.

use flate2::read::ZlibDecoder;
use hades_core::collision::CollisionGrid;
use hades_ro_prere::gat_parser::parse_gat;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Leitor leve de arquivos .grf para extração de mapas server-side.
pub struct SimpleGrf {
    file: File,
    entries: HashMap<String, (u32, u32, u8, u32)>, // (comp_len, uncomp_len, flags, offset)
}

impl SimpleGrf {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(path)?;
        let mut header = [0u8; 46];
        file.read_exact(&mut header)?;

        if &header[0..16] != b"Master of Magic\0" {
            return Err("Assinatura mágica inválida de GRF".into());
        }

        let ft_offset = u32::from_le_bytes(header[30..34].try_into()?);
        let raw_count = u32::from_le_bytes(header[38..42].try_into()?);
        let version = u32::from_le_bytes(header[42..46].try_into()?);

        if version != 0x200 {
            return Err(format!("Versão não suportada de GRF: 0x{:X}", version).into());
        }

        file.seek(SeekFrom::Start(ft_offset as u64 + 46))?;
        let mut ft_sizes = [0u8; 8];
        file.read_exact(&mut ft_sizes)?;
        let comp_size = u32::from_le_bytes(ft_sizes[0..4].try_into()?) as usize;

        let mut comp_data = vec![0u8; comp_size];
        file.read_exact(&mut comp_data)?;

        let mut decoder = ZlibDecoder::new(&comp_data[..]);
        let mut table = Vec::new();
        decoder.read_to_end(&mut table)?;

        let mut entries = HashMap::with_capacity(raw_count as usize);
        let mut pos = 0;
        let len = table.len();

        while pos < len {
            let start = pos;
            while pos < len && table[pos] != 0 {
                pos += 1;
            }
            if pos >= len {
                break;
            }

            let raw_name = &table[start..pos];
            pos += 1;

            if pos + 17 > len {
                break;
            }

            let comp_len = u32::from_le_bytes(table[pos..pos + 4].try_into()?);
            let uncomp_len = u32::from_le_bytes(table[pos + 8..pos + 12].try_into()?);
            let flags = table[pos + 12];
            let offset = u32::from_le_bytes(table[pos + 13..pos + 17].try_into()?);
            pos += 17;

            let norm_name = String::from_utf8_lossy(raw_name)
                .to_lowercase()
                .replace('\\', "/");
            entries.insert(norm_name, (comp_len, uncomp_len, flags, offset));
        }

        Ok(Self { file, entries })
    }

    pub fn extract(&mut self, relative_path: &str) -> Option<Vec<u8>> {
        let norm = relative_path.to_lowercase().replace('\\', "/");
        let &(comp_len, uncomp_len, flags, offset) = self.entries.get(&norm)?;

        if flags != 1 {
            return None;
        }

        if self.file.seek(SeekFrom::Start(offset as u64 + 46)).is_err() {
            return None;
        }

        let mut comp = vec![0u8; comp_len as usize];
        if self.file.read_exact(&mut comp).is_err() {
            return None;
        }

        let mut decoder = ZlibDecoder::new(&comp[..]);
        let mut uncomp = Vec::with_capacity(uncomp_len as usize);
        if decoder.read_to_end(&mut uncomp).is_err() {
            return None;
        }

        Some(uncomp)
    }
}

/// Carrega a grade de colisão de um mapa (.gat).
pub struct MapLoader;

impl MapLoader {
    /// Tenta carregar o mapa do filesystem direto ou do arquivo .grf configurado.
    /// Se não encontrar, retorna uma grade padrão de teste transitável.
    pub fn load_or_fallback(map_name: &str) -> CollisionGrid {
        let clean_name = map_name.trim_end_matches(".gat");
        let gat_file = format!("{clean_name}.gat");

        // 1. Tenta carregar arquivo .gat direto se existir
        let candidate_paths = [
            PathBuf::from(&gat_file),
            PathBuf::from(format!("assets/maps/{gat_file}")),
            PathBuf::from(format!("../assets/maps/{gat_file}")),
        ];

        for path in &candidate_paths {
            if path.exists() {
                if let Ok(bytes) = std::fs::read(path) {
                    if let Ok(grid) = parse_gat(&bytes) {
                        info!("Grade de colisão carregada de {:?} (dimensões {}x{})", path, grid.width, grid.height);
                        return grid;
                    }
                }
            }
        }

        // 2. Tenta extrair do GRF via HADES_GRF_PATH
        if let Ok(grf_path) = std::env::var("HADES_GRF_PATH") {
            if Path::new(&grf_path).exists() {
                match SimpleGrf::open(&grf_path) {
                    Ok(mut grf) => {
                        let inner_path = format!("data/{gat_file}");
                        if let Some(bytes) = grf.extract(&inner_path) {
                            if let Ok(grid) = parse_gat(&bytes) {
                                info!(
                                    "Grade de colisão extraída de {} via [{}] (dimensões {}x{})",
                                    grf_path, inner_path, grid.width, grid.height
                                );
                                return grid;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Falha ao abrir GRF em {grf_path}: {e}");
                    }
                }
            }
        }

        // 3. Fallback: cria mapa padrão 400x400 com todas as células caminháveis
        info!("Usando CollisionGrid sintético 400x400 para {gat_file}");
        CollisionGrid::new(400, 400, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hades_core::types::Position;

    #[test]
    fn test_map_loader_fallback_creates_walkable_grid() {
        let grid = MapLoader::load_or_fallback("prontera.gat");
        assert!(grid.width >= 256);
        assert!(grid.height >= 256);
        assert!(grid.is_walkable(Position::new_unchecked(156, 180)));

        println!("--- Walkable tiles around (156, 180) ---");
        for y in (175..=185).rev() {
            let mut line = format!("Y={y:3}: ");
            for x in 150..=162 {
                let p = Position::new_unchecked(x, y);
                line.push(if grid.is_walkable(p) { '.' } else { '#' });
            }
            println!("{line}");
        }
    }

}
