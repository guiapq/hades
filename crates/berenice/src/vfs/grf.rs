//! SPEC-0010: Leitor de arquivos de pacote .grf (versão 0x200) com streaming sob demanda.

use flate2::read::ZlibDecoder;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Mutex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GrfError {
    #[error("Erro de I/O ao acessar o arquivo GRF: {0}")]
    Io(#[from] std::io::Error),

    #[error("Assinatura mágica inválida (esperado 'Master of Magic\\0')")]
    InvalidMagic,

    #[error("Versão de GRF não suportada: 0x{0:X} (esperado 0x200)")]
    UnsupportedVersion(u32),

    #[error("Falha ao descompactar tabela de arquivos Zlib: {0}")]
    Decompress(std::io::Error),
}

/// Metadados de um arquivo individual dentro do GRF.
#[derive(Debug, Clone, Copy)]
pub struct GrfEntry {
    pub compressed_len: u32,
    pub uncompressed_len: u32,
    pub flags: u8,
    pub offset: u32,
}

/// Leitor indexado de arquivo .grf.
pub struct GrfArchive {
    file: Mutex<File>,
    entries: HashMap<String, GrfEntry>,
}

impl GrfArchive {
    /// Abre o arquivo .grf, valida o cabeçalho e lê a tabela de entradas em memória.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, GrfError> {
        let mut file = File::open(path)?;

        // 1. Ler cabeçalho de 46 bytes
        let mut header = [0u8; 46];
        file.read_exact(&mut header)?;

        if &header[0..16] != b"Master of Magic\0" {
            return Err(GrfError::InvalidMagic);
        }

        let ft_offset = u32::from_le_bytes(header[30..34].try_into().unwrap());
        let _seed = u32::from_le_bytes(header[34..38].try_into().unwrap());
        let raw_count = u32::from_le_bytes(header[38..42].try_into().unwrap());
        let version = u32::from_le_bytes(header[42..46].try_into().unwrap());

        if version != 0x200 {
            return Err(GrfError::UnsupportedVersion(version));
        }

        // 2. Posicionar na tabela de arquivos (ft_offset + 46)
        file.seek(SeekFrom::Start(ft_offset as u64 + 46))?;

        let mut ft_sizes = [0u8; 8];
        file.read_exact(&mut ft_sizes)?;
        let comp_size = u32::from_le_bytes(ft_sizes[0..4].try_into().unwrap()) as usize;
        let _uncomp_size = u32::from_le_bytes(ft_sizes[4..8].try_into().unwrap()) as usize;

        // Ler bloco compactado da tabela
        let mut comp_data = vec![0u8; comp_size];
        file.read_exact(&mut comp_data)?;

        // Descompactar com Zlib
        let mut decoder = ZlibDecoder::new(&comp_data[..]);
        let mut decomp_table = Vec::new();
        decoder
            .read_to_end(&mut decomp_table)
            .map_err(GrfError::Decompress)?;

        // 3. Parsear entradas da tabela descompactada
        let mut entries = HashMap::with_capacity(raw_count as usize);
        let mut pos = 0;
        let table_len = decomp_table.len();

        while pos < table_len {
            // Localiza null byte terminator do nome
            let start = pos;
            while pos < table_len && decomp_table[pos] != 0 {
                pos += 1;
            }
            if pos >= table_len {
                break;
            }

            let name_slice = &decomp_table[start..pos];
            pos += 1; // Pular \0

            if pos + 17 > table_len {
                break;
            }

            let comp_len = u32::from_le_bytes(decomp_table[pos..pos + 4].try_into().unwrap());
            let _comp_aligned =
                u32::from_le_bytes(decomp_table[pos + 4..pos + 8].try_into().unwrap());
            let uncomp_len =
                u32::from_le_bytes(decomp_table[pos + 8..pos + 12].try_into().unwrap());
            let flags = decomp_table[pos + 12];
            let offset = u32::from_le_bytes(decomp_table[pos + 13..pos + 17].try_into().unwrap());
            pos += 17;

            let normalized_name = Self::normalize_path(name_slice);
            entries.insert(
                normalized_name,
                GrfEntry {
                    compressed_len: comp_len,
                    uncompressed_len: uncomp_len,
                    flags,
                    offset,
                },
            );
        }

        Ok(Self {
            file: Mutex::new(file),
            entries,
        })
    }

    /// Normaliza caminhos trocando '\\' por '/' e aplicando lowercase, decodificando EUC-KR (CP949).
    pub fn normalize_path(bytes: &[u8]) -> String {
        let (decoded, _, _) = encoding_rs::EUC_KR.decode(bytes);
        decoded.replace('\\', "/").to_lowercase()
    }

    /// Quantidade total de arquivos indexados.
    pub fn file_count(&self) -> usize {
        self.entries.len()
    }

    /// Itera sobre todos os nomes de arquivos normalizados indexados no arquivo.
    pub fn file_names(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }

    /// Verifica se um arquivo existe no índice.
    pub fn contains(&self, path: &str) -> bool {
        let norm = path.replace('\\', "/").to_lowercase();
        self.entries.contains_key(&norm)
    }

    /// Extrai sob demanda os bytes descompactados de um arquivo do GRF.
    pub fn extract(&self, path: &str) -> Option<Vec<u8>> {
        let norm = path.replace('\\', "/").to_lowercase();
        let entry = *self.entries.get(&norm)?;

        let mut file = self.file.lock().ok()?;
        file.seek(SeekFrom::Start(entry.offset as u64 + 46)).ok()?;

        let mut comp_data = vec![0u8; entry.compressed_len as usize];
        file.read_exact(&mut comp_data).ok()?;

        if entry.flags & 1 != 0 {
            // Arquivo compactado com Zlib
            let mut decoder = ZlibDecoder::new(&comp_data[..]);
            let mut decomp = Vec::with_capacity(entry.uncompressed_len as usize);
            decoder.read_to_end(&mut decomp).ok()?;
            Some(decomp)
        } else {
            // Arquivo armazenado cru
            Some(comp_data)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_grf_path_normalization() {
        let path = b"data\\texture\\UI\\Title.tga";
        assert_eq!(
            GrfArchive::normalize_path(path),
            "data/texture/ui/title.tga"
        );
    }

    #[test]
    fn test_grf_monster_assets_extractable() {
        let path = if Path::new("data.grf").exists() {
            "data.grf"
        } else if Path::new("../../data.grf").exists() {
            "../../data.grf"
        } else {
            return;
        };
        let grf = GrfArchive::open(path).unwrap();
        let act_data = grf.extract("data/sprite/몬스터/poring.act").unwrap();
        let spr_data = grf.extract("data/sprite/몬스터/poring.spr").unwrap();
        let act = hades_ro_prere::act_parser::parse_act(&act_data).unwrap();
        let spr = hades_ro_prere::spr_parser::parse_spr(&spr_data).unwrap();
        assert_eq!(act.actions.len(), 72);
        assert!(!spr.frames.is_empty());
    }
}
