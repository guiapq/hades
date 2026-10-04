//! SPEC-0010: Leitor de arquivos em diretórios normais para migração pós-GRF.

use std::fs;
use std::path::{Path, PathBuf};

/// Fonte de assets baseada em uma pasta regular do sistema de arquivos.
pub struct DirectoryArchive {
    root_dir: PathBuf,
}

impl DirectoryArchive {
    pub fn new<P: AsRef<Path>>(root_dir: P) -> Self {
        Self {
            root_dir: root_dir.as_ref().to_path_buf(),
        }
    }

    /// Carrega os bytes de um arquivo relativo à pasta base.
    pub fn load(&self, relative_path: &str) -> Option<Vec<u8>> {
        let clean_path = relative_path.replace('\\', "/");
        let full_path = self.root_dir.join(clean_path);
        fs::read(full_path).ok()
    }

    /// Verifica a existência do arquivo.
    pub fn exists(&self, relative_path: &str) -> bool {
        let clean_path = relative_path.replace('\\', "/");
        self.root_dir.join(clean_path).exists()
    }
}
