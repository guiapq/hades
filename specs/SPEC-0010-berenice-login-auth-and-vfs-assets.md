# SPEC-0010: Subsistema de Autenticação, Interface de Login e Camada VFS (Assets & GRF)

- **Autor:** Hades Core Team & Persephone Working Group
- **Data:** 2026-10-02
- **Status:** Approved
- **Módulo Afetado:** `crates/berenice` (UI, Auth & Asset Loading Subsystems)

---

## 1. Contexto e Motivação

Para permitir a inicialização completa do cliente **Berenice** com tela de entrada interativa:
1. **Desacoplamento de Formatos de Asset (VFS):** O cliente não deve se prender ao formato legado `.grf`. Criamos uma interface genérica de sistema de arquivos virtual (`AssetSource`) com implementações para:
   - `GrfArchive`: Leitura sob demanda (zero-copy / streaming com Zlib `flate2`) de arquivos `.grf` legados (versão 0x200).
   - `DirectoryArchive`: Leitura de arquivos regulares em disco (permitindo migrar 100% para pastas comuns no futuro com zero alteração no código do cliente).
2. **Protocolo de Autenticação sobre QUIC:** Utilização da stream confiável multiplexada definida na [SPEC-0007](SPEC-0007-quic-webtransport-network-layer.md) para troca de pacotes `AuthRequest` e `AuthResponse`.
3. **Cena de Login (`LoginScene`):**
   - Campos de texto editáveis para Usuário e Senha.
   - Navegação por teclado (`Tab`, `Enter`, caracteres alfanuméricos) e controle físico ToS (D-Pad/Analógico para focar campos e Botão A para confirmar).
   - Renderização da interface no `SoftwareFramebuffer`.

---

## 2. Contrato da Camada de Assets (VFS)

```rust
pub trait AssetSource: Send + Sync {
    /// Obtém os bytes descompactados de um asset pelo seu caminho relativo.
    fn load_file(&self, path: &str) -> Option<Vec<u8>>;
    
    /// Verifica se o arquivo existe no arquivo/diretório.
    fn has_file(&self, path: &str) -> bool;
}
```

### 2.1 Leitor de Arquivo `.grf` (0x200)
- Lê o cabeçalho de 46 bytes (`Master of Magic\0`).
- Localiza e descompacta a tabela de entradas (`ft_offset + 46`) usando `flate2::read::ZlibDecoder`.
- Indexa nomes em um `HashMap<String, GrfEntry>` com normalização de barras (`\` para `/`) e minúsculas (*case-insensitive*).
- Extrai arquivos sob demanda com busca $O(1)$.

---

## 3. Máquina de Estados da Cena de Login (`LoginScene`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginState {
    InputCredentials,
    Authenticating,
    Success { token: u64 },
    Failed(LoginErrorCode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginField {
    Username,
    Password,
    ConnectButton,
}
```

### 3.1 Transições de Entrada:
- `Tab` ou D-Pad Vertical: Alterna foco entre `Username` $\rightarrow$ `Password` $\rightarrow$ `ConnectButton`.
- `Backspace`: Apaga o último caractere do campo ativo.
- Caracteres ASCII: Acrescenta texto ao campo ativo (com máscara `*` no campo de senha).
- `Enter` ou Botão A no controle: Submete a autenticação para o servidor Hades.

---

## 4. Invariantes de Design

- [x] **Zero Lock-in:** O cliente não depende estruturalmente do `.grf`. Qualquer pasta com assets normais (`assets/`) funciona de forma idêntica via `DirectoryArchive`.
- [x] **Zero Memory Leak:** A leitura de arquivos do pacote é feita sob demanda sem carregar o `.grf` inteiro na RAM (o arquivo de 3.4 GB permanece em disco e apenas a tabela e os assets solicitados são lidos).
- [x] **Segurança de Autenticação:** A troca de credenciais ocorre exclusivamente em streams multiplexadas protegidas por TLS 1.3 nativo do QUIC.
