# Project Hades 🏛️⚡

> **Next-Generation 2D/2.5D MMORPG Backend Engine & Indie Hub**
> 
> *Projetado para máxima densidade computacional, baixa latência e economia espacial — capaz de rodar 500 CCU numa máquina de $4/mês ou escalar para dezenas de milhares com aceleração por hardware.*

---

## 🎯 Visão do Projeto

O **Hades** nasce para romper com a arquitetura herdada dos anos 2000 (rAthena, eAthena, TFS/OpenTibia). Em vez de manter retrocompatibilidade com clientes e protocolos legados em TCP síncrono, o Hades é uma **engine de backend moderna construída do zero em Rust**, voltada para desenvolvedores de **MMORPGs indies** modernos (2D e 2.5D).

### Pilares Fundamentais:
1. **Rede Moderna (Zero Head-of-Line Blocking):** WebTransport e QUIC nativos (HTTP/3) suportando clientes Desktop, Mobile e Web (WebGPU/Canvas) no mesmo protocolo e sem proxies.
2. **Data-Oriented & Spatial Economics:** Inspirado pela filosofia do *YggEngine* — operações unitárias puras, Spatial LOD (Level of Detail), Ticks variáveis e particionamento espacial em grade $O(1)$.
3. **Potato-Ready ($4/mês = 500 CCU):** Otimizado para rodar com zero alocações na heap durante o tick e batching de syscalls (`sendmmsg`), consumindo menos de 50MB de RAM.
4. **Escala Flexível com Hardware Bleeding Edge:** Alvo nativo em **ARM64** e **x86_64**, com aceleração opcional via **Vulkan Compute / wgpu** (Flow fields massivos) e **ONNX/IA local** (anti-cheat comportamental e NPCs inteligentes).
5. **Developer Experience (DX) para Indies:** Game logic em **WASM** ou **Lua** com hot-reload em tempo de execução, e SDKs clientes prontos para **Godot, Unity e Bevy**.
6. **Spec-Driven & Test-First (SDD/TDD):** Nenhuma linha de lógica é implementada sem especificação prévia aprovada, contratos de dados explícitos e suíte completa de testes unitários.

---

## 📚 Documentação Completa

Consulte a documentação técnica detalhada em:
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — Especificação detalhada da arquitetura, rede, algoritmos espaciais e modelo de concorrência.
- [docs/POTATO_BUDGET.md](docs/POTATO_BUDGET.md) — Guia de engenharia e limites para rodar 200-500 CCU em 1 vCPU e 512MB RAM.
- [docs/SPEC-DRIVEN.md](docs/SPEC-DRIVEN.md) — Metodologia de desenvolvimento guiada por especificações, ciclo de vida de features e template de RFCs.
- [DEV-SETUP.md](DEV-SETUP.md) — Guia de configuração do ambiente de desenvolvimento (Ubuntu, Debian, Fedora, Rocky Linux e Omarchy/Arch).

---

## 🛠️ Tech Stack Planejada

| Componente | Escolha Técnica | Motivação |
| :--- | :--- | :--- |
| **Linguagem Core** | **Rust** | Zero-cost abstractions, sem pausas de GC, concorrência segura, cross-compile nativo. |
| **Transporte** | **WebTransport / QUIC (`quinn`)** | Multiplexação de streams confiáveis + datagramas não-confiáveis, compatível com Web e Desktop. |
| **Serialização** | **Bitpacking + FlatBuffers** | Zero-copy deserialization, pacotes de movimento de 4 a 6 bytes. |
| **Syscalls de Rede** | **`sendmmsg` / `io_uring`** | Envio de milhares de datagramas em uma única transição de kernel. |
| **Simulação (ECS)** | **`hecs` / DoD customizado** | Dados contíguos na memória, amigável para cache L1/L2/L3 e auto-vetorização SIMD. |
| **Scripting / Lógica** | **Wasmtime (WASM) / mlua (LuaJIT)** | Alteração de regras e quests sem recompilar o binário. |
| **Persistência** | **In-Memory + SQLite WAL (Write-Behind)** | Zero I/O de disco durante os ticks de gameplay. |
| **Computação Opcional** | **`wgpu` (Vulkan/Metal) + ONNX Runtime** | Shaders para IA de mobs em massa e modelos locais de anti-cheat. |

---

## 🗺️ Roadmap de Desenvolvimento

- [ ] **Fase 1: Protocolo de Rede & Benchmarks de I/O**
  - Implementação de servidor QUIC / WebTransport com canais confiáveis e datagramas.
  - Testes de envio em lote com `sendmmsg` simulando 500 conexões com jitter e packet drop.
- [ ] **Fase 2: Motor de Ticks & Particionamento Espacial (AoI)**
  - Loop de tick fixo determinístico (15-20 Hz).
  - Grade espacial em cubos/buckets $O(1)$ para publicação/subscrição de visão.
  - Compressão delta com bitmasks (estado compacto).
- [ ] **Fase 3: Módulos de Simulação 2D/2.5D**
  - Grid de células (estilo Ragnarok/Tibia) com bitset de colisão.
  - Movimentação, interpolação e sistema de combate atômico.
- [ ] **Fase 4: Runtime de Scripts & Persistência**
  - Integração de runtime WASM/Lua com hot-reload.
  - Camada de persistência assíncrona SQLite WAL em thread desacoplada.
- [ ] **Fase 5: SDKs Clientes & Exemplos**
  - SDK de referência para Godot 4.x.
  - Projeto demonstrativo jogável.

---

## 📜 Licença

Distribuído sob licença MIT ou Apache 2.0 (a definir).
