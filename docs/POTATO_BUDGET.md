# O "Potato Budget": 500 Jogadores em 1 vCPU e 512MB RAM 🥔⚡

> Guia prático de dimensionamento e limites computacionais para rodar o motor Hades em hardware ultra-econômico.

---

## 1. O Alvo de Hardware

* **CPU:** 1 vCPU compartilhada (ex: DigitalOcean Basic Droplet $4/mês, AWS t4g.nano/micro, GCP e2-micro, Hetzner CAX11).
* **Memória RAM:** 512MB a 1GB.
* **Rede:** Interface de 1 Gbps com limites moderados de transferência mensal.
* **Meta de Carga:** **200 a 500 Jogadores Concorrentes (CCU)** simultâneos no mesmo mapa ou mundo sem atraso de tick (*tick stall*).

---

## 2. Orçamento de Tempo de CPU (Tick Budget)

Para um servidor de MMORPG 2D/2.5D com interpolação no cliente, **20 Hz** (50 ms por tick) é a taxa ideal.

```
+--------------------------------------------------------------------+
| 50 ms (Janela Total do Tick)                                       |
|                                                                    |
| [ Simulação ] [ Rede I/O ] [            Ocioso / Folga do SO      ]|
|    < 3 ms        < 2 ms                       > 45 ms              |
+--------------------------------------------------------------------+
```

* **Simulação de Gameplay:** Máximo de **3 ms** de tempo de CPU por tick.
* **Rede e Montagem de Buffers:** Máximo de **2 ms** por tick.
* **Margem Ociosa:** **45 ms** (90% do tempo) livre para o kernel do Linux processar interrupts, background tasks e absorver concorrência de vizinhos barulhentos (*noisy neighbors* de cloud compartilhada).

---

## 3. Orçamento de Memória RAM (< 50 MB total)

O objetivo é manter a pegada de memória do processo tão baixa que o sistema operacional nunca acione swap ou o OOM Killer.

| Componente | Quantidade | Tamanho Unitário | Memória Estimada |
| :--- | :--- | :--- | :--- |
| **Entidades Jogadores** | 1.000 slots pré-alocados | ~512 bytes | **~0,5 MB** |
| **Entidades Mobs/NPCs** | 10.000 slots pré-alocados | ~256 bytes | **~2,5 MB** |
| **Grids de Colisão (Bitsets)** | 20 mapas de 1024x1024 tiles | 128 KB por mapa | **~2,5 MB** |
| **Buffers de Rede (Ring Buffers)** | Fila de envio e recebimento | Buffers contíguos | **~4,0 MB** |
| **Runtime do Binário Rust** | Código compilado e heaps | Estático | **~10,0 MB** |
| **Total Estimado em Execução** | | | **~20 a 30 MB** |

> **Regra de Ouro:** Alocação zero durante o loop de tick. Toda a memória das entidades é reservada durante a inicialização em pools contíguas (`SlotMap` / arenas indexadas por inteiros).

---

## 4. Otimizações de Rede: Evitando o Colapso de Syscalls

Em 1 vCPU, fazer 500 chamadas `send()` a cada 50ms gera **10.000 context switches por segundo**, saturando a CPU com trabalho do kernel em vez do jogo.

### A Estratégia de Envio do Hades:
1. **Agrupamento Linear:** Todos os datagramas a serem disparados em um tick são montados em uma única estrutura `mmsghdr`.
2. **`sendmmsg`:** Uma única chamada de sistema despacha todos os pacotes em lote diretamente para o socket UDP.
3. **Bitpacking Compacto:** O pacote de sincronização de posição consome apenas **6 bytes**:
   * ID da Entidade: 16 bits
   * Posição X: 12 bits
   * Posição Y: 12 bits
   * Ação/Direção: 8 bits
4. **Largura de Banda por Jogador:**
   * 20 monstros visíveis $\times$ 6 bytes $\times$ 20 ticks = **2,4 KB/s por jogador**.
   * 500 jogadores ativos consom apenas **~1,2 MB/s (menos de 10 Mbps)** de banda de saída do servidor!

---

## 5. As 4 Regras Invioláveis do Modo Batata

1. **Nunca aloque memória no loop de tick:** Não use `Box::new`, não crie novos `Vec` nem instancie objetos dinâmicos dentro do ciclo de 50ms. Reutilize buffers limpos.
2. **Nunca consulte o disco ou banco no tick:** Operações de banco de dados são proibidas na thread de simulação. Persistência é 100% assíncrona.
3. **Monstro sem jogador perto dorme:** Zonas sem jogadores não executam IA, não procuram alvos e não recalculam rotas.
4. **Use aproximações de Manhattan antes do A\*:** Monstro se movendo em direção ao jogador só roda pathfinding se colidir com uma parede; em terreno livre, ele anda diretamente pelo vetor de distância.
