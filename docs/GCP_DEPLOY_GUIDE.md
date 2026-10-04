# Guia de Implantação Gratuita no Google Cloud Platform (GCP Always Free) ☁️🚀

Este guia detalha o passo a passo para hospedar os servidores **`hades-login`** e **`hades-world`** em uma máquina virtual Linux 100% gratuita no **Google Cloud Compute Engine** (`e2-micro`), aproveitando o suporte nativo a **QUIC / UDP**.

---

## 1. O Que a Máquina Gratuita do GCP Oferece?
- **Instância:** `e2-micro` (2 vCPUs compartilhadas, 1.0 GB RAM).
- **Custo:** **$0.00 / mês** (permanente no programa *Always Free*).
- **Regiões Válidas (Always Free):**
  - `us-central1` (Iowa)
  - `us-west1` (Oregon)
  - `us-east1` (South Carolina)
- **Disco:** 30 GB Standard Persistent Disk gratuito.
- **Rede:** 1 IP IPv4 externo incluído sem custos.

---

## 2. Passo 1: Criando a Instância no GCP

Você pode criar via **Console Web do Google Cloud** ou via terminal com **`gcloud`**:

### Opção A: Via Terminal (`gcloud CLI`)
```bash
gcloud compute instances create hades-server \
    --project="SEU_PROJECT_ID" \
    --zone="us-central1-a" \
    --machine-type="e2-micro" \
    --image-family="ubuntu-2204-lts" \
    --image-project="ubuntu-os-cloud" \
    --boot-disk-size="30GB" \
    --boot-disk-type="pd-standard" \
    --tags="hades-game-server"
```

### Opção B: Via Console Web (Interface Gráfica)
1. Acesse o [Google Cloud Console](https://console.cloud.google.com/).
2. Vá em **Compute Engine** $\rightarrow$ **Instâncias de VM** $\rightarrow$ **Criar Instância**.
3. **Nome:** `hades-server`.
4. **Região:** `us-central1 (Iowa)` ou `us-east1 (South Carolina)`.
5. **Tipo de máquina:** `e2-micro` (1 GB de memória).
6. **Disco de inicialização:** Clique em *Alterar* $\rightarrow$ Selecione **Ubuntu 22.04 LTS** ou **Ubuntu 24.04 LTS** (Tamanho: 30 GB).
7. Em **Rede** $\rightarrow$ **Tags de rede**, adicione a tag: `hades-game-server`.
8. Clique em **Criar**.

---

## 3. Passo 2: Liberando as Portas UDP no Firewall do GCP

O protocolo QUIC opera sobre **UDP**. Precisamos abrir as portas `4433` (Login) e `4434` (World) na VPC:

### Opção A: Via Terminal (`gcloud CLI`)
```bash
gcloud compute firewall-rules create allow-hades-udp \
    --direction=INGRESS \
    --priority=1000 \
    --network=default \
    --action=ALLOW \
    --rules=udp:4433,udp:4434 \
    --source-ranges=0.0.0.0/0 \
    --target-tags=hades-game-server \
    --description="Permitir trafego QUIC/UDP do Hades"
```

### Opção B: Via Console Web
1. No menu lateral, acesse **Rede VPC** $\rightarrow$ **Regras de firewall**.
2. Clique em **Criar regra de firewall**:
   - **Nome:** `allow-hades-quic`
   - **Destino:** `Tags de destino especificadas`
   - **Tags de destino:** `hades-game-server`
   - **Intervalos de IPv4 de origem:** `0.0.0.0/0`
   - **Protocolos e portas:** Marque **UDP** e digite `4433, 4434`.
3. Clique em **Criar**.

---

## 4. Passo 3: Inicializando o Servidor na VM

1. Conecte-se à VM via SSH:
   ```bash
   gcloud compute ssh hades-server --zone=us-central1-a
   # Ou clique no botão "SSH" diretamente pelo Console Web
   ```

2. Clone o repositório ou envie os arquivos do projeto:
   ```bash
   git clone https://github.com/SEU_USUARIO/hades.git
   cd hades
   ```

3. Execute o script de configuração automatizado:
   ```bash
   ./deploy/gcp_setup.sh
   ```

4. Inicie os servidores em background:
   ```bash
   # Inicia Login Server (:4433 UDP)
   nohup ./target/release/hades-login > login.log 2>&1 &

   # Inicia World Server (:4434 UDP)
   nohup ./target/release/hades-world > world.log 2>&1 &
   ```

5. Verifique se os serviços estão escutando:
   ```bash
   ss -ulnp | grep -E "4433|4434"
   ```

---

## 5. Passo 4: Conectando o Cliente Berenice da Sua Máquina Local

No seu computador local, basta apontar a variável de ambiente para o **IP público** da sua VM no GCP:

```bash
# Substitua pelo IP externo exibido no console do GCP
export HADES_WORLD_SERVER="34.123.45.67:4434"

# Execute o cliente Berenice a 60 FPS
cargo run -p berenice --bin berenice
```

Você estará jogando com o cliente conectando em tempo real através da infraestrutura de fibra ótica global do Google Cloud via **QUIC sobre TLS 1.3**!
