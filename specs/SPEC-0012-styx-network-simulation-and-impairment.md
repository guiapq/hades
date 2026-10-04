# SPEC-0012: Styx - Simulador de Degradação de Rede e Perfis de Conexão

- **Autor:** Hades Core Team
- **Data:** 2026-10-02
- **Status:** Approved
- **Módulo Afetado:** `crates/styx` (ou `crates/hades-styx`)

---

## 1. Contexto e Motivação

O Projeto Hades adota o transporte moderno **QUIC / WebTransport** sobre TLS 1.3 com pacotes ultracompactos de 6 bytes (`MovementDelta`). No entanto, na internet real, os jogadores acessam o jogo sob condições adversas de infraestrutura: redes móveis com instabilidade de sinal, conexões de fibra de alta performance, conexões rurais com alta taxa de perda e enlaces de satélite (órbita baixa ou geoestacionária).

O subsistema **Styx** (em alusão ao rio mitológico que separa a terra dos vivos do submundo) é o motor de testes e simulação de rede do Hades. Sua responsabilidade é interceptar o tráfego UDP/QUIC entre o cliente e o servidor, introduzindo intencionalmente:
1. **Latência Base (Ping RTT):** Atraso de propagação física da conexão.
2. **Jitter (Variação de Latência):** Desvio padrão e flutuação temporal por pacote.
3. **Perda de Pacotes (Packet Loss):** Descarte estocástico e perdas em rajada (*burst drop*).
4. **Reordenação e Duplicação:** Inversão na ordem de chegada de datagramas UDP.
5. **Perfis de Conexão Pré-definidos:** Simulações pré-configuradas baseadas em cenários do mundo real.

---

## 2. Topologia Geográfica e Perfis de Rede Homologados (`StyxProfile`)

A maioria dos serviços em nuvem (Google Cloud `southamerica-east1`, AWS `sa-east-1`, data centers da Hostinger/Equinix) concentra-se na região metropolitana de São Paulo e Campinas.

Para um jogador situado no **Norte ou Nordeste do Brasil** (ex: Amapá, Pará, Maranhão, Ceará, etc.), os pacotes percorrem anéis terrestres (via Belém/Fortaleza/Brasília) ou cabos submarinos ópticos ao longo da costa atlântica, totalizando entre 3.000 km e 5.000 km de percurso físico de fibra até os datacenters de São Paulo:
* **Física Óptica:** A velocidade da luz na fibra de sílica ($c/n \approx 200.000\text{ km/s}$) impõe um limite físico estrito: cada 1.000 km de fibra adiciona no mínimo 10 ms de RTT puro, somado a múltiplos saltos BGP, anéis metropolitanos GPON e pontos de troca de tráfego (IX.br).
* **Consequência:** Mesmo em conexões residenciais de fibra óptica de alta qualidade no Norte e Nordeste do país, o RTT real para São Paulo nunca fica abaixo de **40 a 45 ms** (e no caso do Amapá ou cidades do interior, tipicamente 48 a 60 ms).

### 2.1 Tabela de Perfis de Conexão Realistas

| Perfil (`StyxProfile`) | RTT Médio (ms) | RTT Mínimo (ms) | Jitter (±ms) | Perda de Pacotes (%) | Descrição e Topologia Física |
| :--- | :---: | :---: | :---: | :---: | :--- |
| **Fibra (`Fiber`)** | **48 ms** | 42 ms | ±3 ms | 0.05% | FTTH banda larga em capitais do Norte/Nordeste com trânsito de fibra direto até SP. |
| **Cidade (`City`)** | **65 ms** | 55 ms | ±8 ms | 0.2% | Provedor local urbano (GPON/Coaxial) com enlaces secundários de peering até os datacenters. |
| **Móvel (`Mobile`)** | **95 ms** | 78 ms | ±28 ms | 1.5% | Celular 4G/5G com handover de rádio em capitais do Norte/Nordeste. |
| **Rural Móvel (`RuralMobile`)** | **220 ms** | 160 ms | ±85 ms | 7.0% | 3G/4G distante em rodovias (ex: BR-156) e interiores/ribeirinhos, via rádio de micro-ondas intermediário. |
| **Órbita Baixa (`LowOrbit`)** | **68 ms** | 50 ms | ±20 ms | 1.8% | Satélite LEO na Amazônia/Norte (enlace espacial com descida em teleporto gateway em Brasília/SP). |
| **Estacionário (`Geostationary`)** | **680 ms** | 590 ms | ±65 ms | 4.0% | Satélite GEO tradicional Banda Ka/Ku (altitude orbital de ~35.786 km de distância física). |
| **Apocalipse (`Apocalypse`)** | **1750 ms** | 1500 ms | ±250 ms | 20.0% | Condições extremas de rede degradada/colapsada (1500~2000 ms ping, alto jitter e 20% de perda). |

### 2.2 Modificador de Destino Internacional (`StyxTargetRegion`)
Caso a instância do servidor Hades esteja hospedada no exterior (ex: nós baratos da Hostinger / OVH / DigitalOcean em Miami ou Virgínia - US East):
* **América do Sul (São Paulo - Default):** $1.0\times$ (valores base da tabela acima).
* **América do Norte (Miami / Virgínia):** $+45\text{ ms}$ de RTT adicional em todos os perfis (ex: Fibra AP $\rightarrow$ Miami salta para $\approx 90\text{ ms}$).

---

## 3. Política de Ativação por Ambiente

Para garantir máxima performance em servidores reais sem sacrificar o rigor dos testes locais:

1. **Produção (`Production` / Release Builds):**
   * **Padrão: DESLIGADO (`StyxMode::Disabled` / Passthrough Puro).**
   * Zero overhead de temporizadores, zero buffers extras e zero alocação de memória. O tráfego UDP/QUIC transita diretamente entre as interfaces de rede do sistema operacional.
2. **Ambiente de Testes e CI (`cfg(test)` / Test Suites / Playtests Locais):**
   * **Padrão: FIBRA (`StyxProfile::Fiber` - RTT 48 ms, Jitter ±3 ms).**
   * **Motivo:** Evita a ilusão de latência zero de loopback (`127.0.0.1` a 0.05 ms). Força toda a suíte de testes de integração, predição de movimento e sincronização AoI a validar sob a latência física real de fibra óptica do Norte/Nordeste para os datacenters.
3. **Controle Dinâmico via Ambiente e Código:**
   * **Variável de Ambiente:** `STYX_PROFILE=disabled|fiber|city|mobile|rural|low_orbit|geo|apocalipse`
   * **Construtor Programático:** `StyxConfig::default()` adota `Disabled` em builds `release` e `Fiber` em testes automatizados, permitindo seleção explícita de perfil quando desejado.

---

## 4. Modelo Matemático do Styx

Para cada pacote de saída com timestamp de envio $t_{\text{envio}}$:

1. **Decisão de Descarte (Loss):**
   $$R_{\text{loss}} \sim U(0, 1)$$
   Se $R_{\text{loss}} < P_{\text{loss}}$, o pacote é descartado sumariamente sem transmissão.

2. **Cálculo da Latência com Jitter:**
   $$D_{\text{one\_way}} = \frac{\text{RTT}_{\text{base}}}{2} + J$$
   Onde $J$ é gerado por distribuição gaussiana ou uniforme no intervalo $[-\text{jitter}, +\text{jitter}]$, limitado a $D_{\text{one\_way}} \ge 0$.

3. **Fila de Entrega Ordenada por Timestamp:**
   O pacote atrasado é enfileirado com timestamp de entrega programado:
   $$t_{\text{entrega}} = t_{\text{envio}} + D_{\text{one\_way}}$$
   Se múltiplos pacotes forem agendados para entrega fora de ordem devido a jitter severo, eles chegam ao destinatário desordenados, validando a robustez do bitpacking de 6 bytes do Hades.

---

## 5. Arquitetura de Software do Styx

O Styx pode ser acionado de duas formas:

1. **Proxy UDP Assíncrono Transparente (Modo Standalone):**
   - Escuta em uma porta intermediária (ex: `:45000`) e encaminha para o `HadesServer` (ex: `:45796`), aplicando o perfil ativo em ambas as direções (cliente $\leftrightarrow$ servidor).
2. **Camada de Testes em Código (Modo Programático / CI):**
   - Em testes de integração do `hades-net` e `berenice`, o Styx é instanciado diretamente para verificar a estabilidade do tick de 20 Hz e da interpolação de movimento sob qualquer um dos 6 perfis.

---

## 6. Invariantes de Projeto

- **Determinismo Opcional com Seed:** Possibilidade de fornecer uma semente pseudoaleatória para reproduzir exatamente a mesma sequência de atrasos e perdas de pacote em testes de regressão.
- **Zero Impacto no Código de Produção:** O motor Hades principal e o cliente Berenice operam normalmente via sockets QUIC padrão; o Styx atua na borda como emulador de enlace.
