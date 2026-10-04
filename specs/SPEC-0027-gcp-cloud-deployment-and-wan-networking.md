# SPEC-0027: Implantação em Nuvem Gratuita (GCP Free Tier) e Conectividade WAN via QUIC ☁️🌐

## 1. Contexto e Filosofia

O **Project Hades** foi concebido desde o primeiro commit sob a regra do **"Potato Budget"**:
> *O motor deve sustentar até 500 jogadores simultâneos (CCU) com menos de 50 MB de RAM em uma máquina de $4/mês ou gratuita na nuvem.*

Com a conclusão do marco **v0.2**, a simulação, a grade espacial (AoI), a cadência de combate rítmica e a camada de renderização isométrica estão consolidadas. Para o marco **v0.3**, o objetivo é testar os servidores em um ambiente de produção real na internet pública (WAN) utilizando a instância **Always Free** do **Google Cloud Platform (GCP)**.

---

## 2. Topologia de Infraestrutura no Google Cloud (Always Free)

O GCP disponibiliza permanentemente e sem custos o seguinte perfil de computação na região dos EUA (`us-central1`, `us-west1` ou `us-east1`):

| Recurso | Especificação no GCP Free Tier | Consumo Estimado do Hades | Margem de Segurança |
| :--- | :--- | :--- | :--- |
| **Instância** | `e2-micro` (2 vCPUs compartilhadas) | 1 thread de tick (15 Hz) + thread de I/O de rede | 85%+ de CPU ociosa |
| **Memória RAM** | 1.0 GB RAM total | ~40 MB RAM (`hades-login` + `hades-world`) | **> 900 MB livres** |
| **Disco** | 30 GB Persistent Disk Standard | ~50 MB (Binários stripped + SQLite WAL) | 99% livre |
| **Endereço IP** | 1 Endereço IPv4 Externo efêmero/estático | IPv4 público com suporte a UDP direto | Nativo |
| **Egress de Rede** | 1 GB de tráfego de saída gratuito/mês | Pacotes ultra-compactos (4 a 6 bytes por delta) | Suporta centenas de horas de testes |

```
                                      INTERNET PÚBLICA (WAN)
                                                 │
                                                 │ QUIC / WebTransport sobre UDP
                                                 │
                    ┌────────────────────────────┴───────────────────────────┐
                    │                  GCP VPC FIREWALL                      │
                    │        Permite: UDP 5000 (Login) & UDP 5001 (World)   │
                    └────────────────────────────┬───────────────────────────┘
                                                 │
                    ┌────────────────────────────▼───────────────────────────┐
                    │               GCP VM (e2-micro / Ubuntu)               │
                    │                                                        │
                    │   ┌────────────────────────────────────────────────┐   │
                    │   │              hades-login (:5000 UDP)           │   │
                    │   │  - Autenticação e Seleção de Personagens       │   │
                    │   │  - Banco SQLite WAL em memória / disco         │   │
                    │   │  - Consumo: ~15 MB RAM                         │   │
                    │   └──────────────────────┬─────────────────────────┘   │
                    │                          │                             │
                    │   ┌──────────────────────▼─────────────────────────┐   │
                    │   │              hades-world (:5001 UDP)           │   │
                    │   │  - Grid Espacial AoI 3x3 em baldes O(1)        │   │
                    │   │  - Tick determinístico de 20 Hz (50ms)         │   │
                    │   │  - Bitpacking e movimentação                   │   │
                    │   │  - Consumo: ~25 MB RAM                         │   │
                    │   └────────────────────────────────────────────────┘   │
                    └────────────────────────────────────────────────────────┘
```

---

## 3. Segurança e TLS 1.3 em Conexões Remotas

O protocolo QUIC exige criptografia TLS 1.3 obrigatória em todos os pacotes.

### Estratégia de Certificados no GCP:
1. **Modo 1: Validação por Fingerprint SHA-256 (Padrão para Testes Rápidos)**
   - O servidor gera dinamicamente um certificado ECDSA autoassinado na inicialização.
   - O hash SHA-256 do certificado é impresso no log de boot:
     ```
     [INFO hades-world] QUIC TLS 1.3 Fingerprint: 4f:a2:89:12:...:e7
     ```
   - O cliente `berenice` aceita o certificado quando configurado com a flag `--cert-fingerprint <HASH>` ou modo `--insecure-cert` para testes controlados.
2. **Modo 2: Certificado Let's Encrypt / Domínio Dinâmico (Produção)**
   - Apontamento de DNS dinâmico gratuito (ex: DuckDNS `meu-hades.duckdns.org`).
   - Obtenção automatizada de certificado válido via Certbot/ACME.

---

## 4. Parametrização Dinâmica do Cliente (`berenice`)

Atualmente, o cliente `berenice` possui endereços locais `127.0.0.1:5000` e `127.0.0.1:5001` como constantes padrão. Para a v0.3, ele deve aceitar configuração externa com tolerância graciosa:

### Argumentos de Linha de Comando:
```bash
# Conectar a servidor remoto no GCP
cargo run -p berenice --bin berenice -- --host 34.123.45.67 --login-port 5000 --world-port 5001
```

### Variáveis de Ambiente (`.env` ou shell):
```env
HADES_SERVER_HOST=34.123.45.67
HADES_LOGIN_PORT=5000
HADES_WORLD_PORT=5001
HADES_INSECURE_CERT=true
```

---

## 5. Medição de Latência WAN e Qualidade de Conexão no HUD

Em conexões de longa distância pela internet, a latência de trânsito (RTT) e flutuações de jitter influenciam diretamente o *game feel*.

1. **Ping / RTT Periódico:**
   - O cliente envia um pacote `Ping(Instant)` pelo canal não-bloqueante a cada 1000 ms.
   - Ao receber o `Pong`, calcula o RTT em milissegundos.
2. **Exibição Visual Discreta no HUD:**
   - Exibir na barra superior do cliente: `PING: 42ms [WAN GCP]`.
   - Código de cores:
     - Verde (`< 80ms`): Latência ideal para RPG de ação rítmico.
     - Amarelo (`80ms - 150ms`): Aceitável com interpolação preditiva.
     - Vermelho (`> 150ms`): Alerta de instabilidade de rota.

---

## 6. Automação de Deploy no GCP (`deploy/`)

Para garantir implantação em minutos em uma VM limpa recém-criada no GCP:

1. **Script de Instalação e Inicialização (`deploy/gcp_setup.sh`):**
   - Instalação de dependências essenciais do Linux.
   - Configuração de regras locais de firewall (`ufw allow 5000/udp`, `ufw allow 5001/udp`).
   - Download dos binários compilados em release.
2. **Serviços Systemd Independentes:**
   - `deploy/hades-login.service`: Reinício automático em caso de falha.
   - `deploy/hades-world.service`: Isolado com limites estritos de memória (`MemoryMax=64M`).

---

## 7. Critérios de Aceitação e Testes

- [ ] Os servidores `hades-login` e `hades-world` compilam em modo `--release` e iniciam com menos de 45 MB de RAM combinados.
- [ ] O cliente `berenice` aceita parâmetro `--host <IP>` e se conecta com sucesso a instâncias remotas sem código hardcoded.
- [ ] O firewall do GCP aceita datagramas UDP nas portas configuradas sem bloqueio por roteadores NAT.
- [ ] A navegação e o combate com bonecos e entidades permanecem sincronizados sob latência real de internet (> 30 ms).
- [ ] O HUD exibe a métrica de RTT (Ping) em tempo real calculada via QUIC datagrams.
