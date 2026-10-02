# Guia de Configuração do Ambiente de Desenvolvimento (DEV-SETUP) 🛠️🐧

> Este guia cobre a instalação de dependências e configuração do ambiente de desenvolvimento do **Hades Engine** nas principais distribuições Linux: **Ubuntu, Debian, Fedora, Rocky Linux e Omarchy (Arch Linux)**.

---

## 1. Dependências do Sistema por Distribuição

O Hades utiliza Rust puro para a sua lógica de simulação, mas requer ferramentas nativas de compilação (`gcc`/`clang`, `pkg-config`, `openssl` e `cmake`) para dependências de rede (QUIC/WebTransport) e criptografia.

### 🐧 Ubuntu (22.04 LTS, 24.04 LTS ou superior)
```bash
sudo apt update
sudo apt install -y \
    build-essential \
    curl \
    git \
    pkg-config \
    libssl-dev \
    cmake \
    clang \
    lld
```
*(Opcional - Headers Vulkan para shaders de aceleração por GPU):*
```bash
sudo apt install -y libvulkan-dev vulkan-tools
```

---

### 🍥 Debian (11 Bullseye, 12 Bookworm ou Testing)
```bash
sudo apt update
sudo apt install -y \
    build-essential \
    curl \
    git \
    pkg-config \
    libssl-dev \
    cmake \
    clang \
    lld
```

---

### 🎩 Fedora (39, 40 ou superior)
```bash
sudo dnf check-update
sudo dnf groupinstall -y "Development Tools"
sudo dnf install -y \
    curl \
    git \
    pkgconf-pkg-config \
    openssl-devel \
    cmake \
    clang \
    lld
```
*(Opcional - Headers Vulkan):*
```bash
sudo dnf install -y vulkan-loader-devel vulkan-tools
```

---

### ⛰️ Rocky Linux / AlmaLinux / RHEL (9.x)
```bash
sudo dnf check-update
sudo dnf --enablerepo=crb groupinstall -y "Development Tools"
sudo dnf install -y \
    curl \
    git \
    pkgconfig \
    openssl-devel \
    cmake \
    clang \
    lld
```

---

### 🏹 Omarchy / Arch Linux / Manjaro
```bash
sudo pacman -Syu --needed \
    base-devel \
    curl \
    git \
    pkgconf \
    openssl \
    cmake \
    clang \
    lld
```
*(Opcional - Headers Vulkan):*
```bash
sudo pacman -S --needed vulkan-devel vulkan-tools
```

---

## 2. Instalação da Toolchain Rust

Recomenda-se utilizar o compilador Rust em versão **Stable 1.80+**.

### Método Recomendado: `rustup` oficial
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

Adicione os componentes essenciais de qualidade de código:
```bash
rustup component add clippy rustfmt
```

### Método Alternativo: `mise-en-place`
Se você utiliza o `mise` para gerenciar toolchains:
```bash
mise use -g rust@stable
```

Verifique se a instalação está correta:
```bash
rustc --version
cargo --version
```

---

## 3. Clonando o Repositório e Compilando

```bash
# 1. Clone o repositório
git clone https://github.com/guiapq/hades.git
cd hades

# 2. Compile o workspace
cargo build

# 3. Execute a suíte de testes unitários
cargo test
```

A saída esperada dos testes deve ser:
```text
test result: ok. 12 passed; 0 failed; 0 ignored; finished in 0.00s
```

---

## 4. Padrões de Qualidade e CI Local

Antes de submeter commits ou Pull Requests, certifique-se de validar:

1. **Formatação de Código:**
   ```bash
   cargo fmt --all --check
   ```
   *(Para aplicar as correções automaticamente: `cargo fmt --all`)*

2. **Linter Estrito (Clippy):**
   ```bash
   cargo clippy --workspace --all-targets -- -D warnings
   ```

3. **Testes Unitários e Determinismo:**
   ```bash
   cargo test --workspace
   ```

---

## 5. Configuração Opcional: Compilação Cruzada para ARM64

Para testar binários destinados aos servidores de baixo custo ARM64 (AWS Graviton, Oracle Ampere, etc.):

```bash
# 1. Adicione o target aarch64
rustup target add aarch64-unknown-linux-gnu

# 2. Instale o compilador cruzado no Debian/Ubuntu:
sudo apt install -y gcc-aarch64-linux-gnu

# 3. Compile para ARM64:
cargo build --target aarch64-unknown-linux-gnu --release
```
