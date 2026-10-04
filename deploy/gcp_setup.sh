#!/usr/bin/env bash
set -euo pipefail

# Script de Inicialização Rápida do Hades no Google Cloud Platform (GCP e2-micro)
echo "🏛️⚡ [HADES GCP SETUP] Configurando ambiente Linux Always Free..."

# 1. Atualiza repositórios essenciais
sudo apt-get update -y
sudo apt-get install -y build-essential pkg-config libssl-dev curl ufw git

# 1.1 Garante 2GB de Swap em máquinas pequenas (e2-micro) para evitar OOM no build
TOTAL_MEM=$(free -m | awk '/^Mem:/{print $2}')
if [ "$TOTAL_MEM" -lt 1500 ] && [ ! -f /swapfile ]; then
    echo "💾 Configurando 2GB de Swap temporário para compilação segura na e2-micro..."
    sudo fallocate -l 2G /swapfile || sudo dd if=/dev/zero of=/swapfile bs=1M count=2048
    sudo chmod 600 /swapfile
    sudo mkswap /swapfile
    sudo swapon /swapfile
fi

# 2. Configura Firewall Local (UFW) se estiver ativo
if sudo ufw status | grep -q "Status: active"; then
    echo "🛡️ Configurando regras de portas UDP no UFW..."
    sudo ufw allow 4433/udp comment 'Hades Login QUIC'
    sudo ufw allow 4434/udp comment 'Hades World QUIC'
    sudo ufw allow 22/tcp comment 'SSH'
fi

# 3. Detecta IP Público Externo da VM no GCP
PUBLIC_IP=$(curl -s -H "Metadata-Flavor: Google" http://metadata.google.internal/computeMetadata/v1/instance/network-interfaces/0/access-configs/0/external-ip || curl -s ifconfig.me || echo "127.0.0.1")
echo "🌐 IP Público Detectado: $PUBLIC_IP"

# 4. Gera arquivo de ambiente local se não existir
if [ ! -f .env ]; then
    echo "📄 Gerando .env a partir de gcp_env.example..."
    sed "s/<IP_PUBLICO_GCP>/$PUBLIC_IP/g" deploy/gcp_env.example > .env
fi

# 5. Instala Rust se não estiver presente
if ! command -v cargo &> /dev/null; then
    echo "🦀 Instalando Rust Toolchain estável..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# 6. Compila servidores em modo release otimizado
echo "🔨 Compilando hades-login e hades-world em modo Release..."
cargo build --release -p hades-login -p hades-world

# 7. Configura serviços Systemd para execução resiliente e reinício automático
if [ -d /etc/systemd/system ]; then
    echo "⚙️ Instalando serviços systemd (hades-login e hades-world)..."
    # Ajusta o usuário e diretório de execução atual no arquivo de serviço
    CURRENT_USER=$(whoami)
    CURRENT_DIR=$(pwd)
    sed -e "s|User=ubuntu|User=$CURRENT_USER|g" -e "s|/home/ubuntu/hades|$CURRENT_DIR|g" deploy/hades-login.service | sudo tee /etc/systemd/system/hades-login.service > /dev/null
    sed -e "s|User=ubuntu|User=$CURRENT_USER|g" -e "s|/home/ubuntu/hades|$CURRENT_DIR|g" deploy/hades-world.service | sudo tee /etc/systemd/system/hades-world.service > /dev/null
    sudo systemctl daemon-reload
    sudo systemctl enable hades-login hades-world
    sudo systemctl restart hades-login hades-world
    echo "🚀 Serviços iniciados com sucesso via systemd!"
    echo "   Verificar status: sudo systemctl status hades-login hades-world"
    echo "   Ver logs: journalctl -u hades-world -f"
fi

echo "✅ Configuração da VM concluída com sucesso!"
echo "🌐 Conecte seu cliente local com:"
echo "   cargo run -p berenice -- --host $PUBLIC_IP"
