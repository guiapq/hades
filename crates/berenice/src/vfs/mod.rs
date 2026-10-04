//! Camada de Sistema de Arquivos Virtual (VFS) desacoplada de contêineres legados.

pub mod dir;
pub mod grf;

pub use dir::DirectoryArchive;
pub use grf::{GrfArchive, GrfError};

/// Interface comum para provedores de assets.
pub trait AssetSource: Send + Sync {
    fn load_file(&self, path: &str) -> Option<Vec<u8>>;
    fn has_file(&self, path: &str) -> bool;
}

impl AssetSource for GrfArchive {
    fn load_file(&self, path: &str) -> Option<Vec<u8>> {
        self.extract(path)
    }

    fn has_file(&self, path: &str) -> bool {
        self.contains(path)
    }
}

impl AssetSource for DirectoryArchive {
    fn load_file(&self, path: &str) -> Option<Vec<u8>> {
        self.load(path)
    }

    fn has_file(&self, path: &str) -> bool {
        self.exists(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_directory_archive_loads_file() {
        let dir = DirectoryArchive::new(env!("CARGO_MANIFEST_DIR"));
        let toml_bytes = dir
            .load_file("Cargo.toml")
            .expect("Cargo.toml should exist");
        assert!(toml_bytes.starts_with(b"[package]"));
    }
}
