# Diretório de Especificações (RFCs / Specs)

Aqui ficam armazenadas todas as especificações formais de features e protocolos do Hades, seguindo o modelo definido em [docs/SPEC-DRIVEN.md](../docs/SPEC-DRIVEN.md).

## Índice de Especificações

| SPEC | Título | Status | Módulo |
| :--- | :--- | :--- | :--- |
| [SPEC-0001](SPEC-0001-core-types-and-bitpacking.md) | Tipos Fundamentais, Bitpacking de Movimento (6B) e Grid de Colisão Compacto | Approved | `hades-core` |
| [SPEC-0002](SPEC-0002-prere-stats-and-attributes.md) | Atributos Primários e Derivados Pré-Renovação (Classic 2.5D) | Approved | `hades-ro-prere` |
| [SPEC-0003](SPEC-0003-prere-combat-resolution.md) | Resolução Atômica de Combate Físico e Mágico Pré-Renovação | Approved | `hades-ro-prere` |
| [SPEC-0004](SPEC-0004-gat-map-parser.md) | Parser de Mapas Binários (.gat) para CollisionGrid do Hades | Approved | `hades-ro-prere` |
| [SPEC-0005](SPEC-0005-combat-modifiers-elements-and-skills.md) | Modificadores de Tamanho de Arma, Tabela Elemental e Habilidades Clássicas Pré-Renovação | Approved | `hades-ro-prere` |
| [SPEC-0006](SPEC-0006-aoi-and-tick-engine.md) | Particionamento Espacial em Buckets O(1) (AoI) e Motor de Ticks Determinístico (20 Hz) | Approved | `hades-core` |
| [SPEC-0007](SPEC-0007-quic-webtransport-network-layer.md) | Camada de Transporte de Rede QUIC / WebTransport e Multiplexação de Canais | Approved | `hades-net` |
| [SPEC-0008](SPEC-0008-atomic-floor-items-and-dupe-prevention.md) | Transação Atômica de Itens do Chão, UUIDs Universais e Proteção contra Rollback/DoS | Approved | `hades-core` |
| [SPEC-0009](SPEC-0009-berenice-client-and-input-engine.md) | Cliente Berenice — Entrada Multiplataforma (Gamepads, Teclado/Mouse ToS) e Motor Leve | Approved | `berenice` |
| [SPEC-0010](SPEC-0010-berenice-login-auth-and-vfs-assets.md) | Subsistema de Autenticação, Interface de Login e Camada VFS (Assets & GRF) | Approved | `berenice` |
| [SPEC-0011](SPEC-0011-multiplayer-world-server-and-aoi-sync.md) | Servidor Mundial Multi-Sessão e Sincronização AoI em Tempo Real (QUIC TLS 1.3) | Approved | `hades-net` |
| [SPEC-0012](SPEC-0012-styx-network-simulation-and-impairment.md) | Motor de Simulação de Rede e Degradação Sintética Styx | Approved | `hades-styx` |
| [SPEC-0013](SPEC-0013-asset-rendering-spr-and-rsm-3d-buildings.md) | Renderização de Sprites 2D e Modelos 3D de Prédios (.rsm) | Approved | `berenice` |
| [SPEC-0014](SPEC-0014-rsw-world-scene-parser-and-3d-placement.md) | Parser de Cenas de Mundo 3D (.rsw) e Posicionamento no Chão | Approved | `hades-ro-prere` |
| [SPEC-0015](SPEC-0015-login-auth-webtransport-protocol.md) | Protocolo de Autenticação Hades (WebTransport) | Approved | `hades-login` |
| [SPEC-0016](SPEC-0016-world-server-wire-protocol-and-movement.md) | Protocolo do World Server e Movimento em Tempo Real | Approved | `hades-world` |
| [SPEC-0017](SPEC-0017-seamless-mouse-pathfinding-and-navigation.md) | Sistema de Pathfinding A* e Navegação Contínua por Mouse | Approved | `hades-core` / `berenice` |
| [SPEC-0018](SPEC-0018-omarchy-and-tiling-wm-responsive-viewport.md) | Camada de Compatibilidade Omarchy e Redimensionamento Responsivo para Tiling Window Managers | Approved | `berenice` |
| [SPEC-0019](SPEC-0019-act-animation-parser-and-cycles.md) | Parser de Ações/Animações (.act) e Ciclos de Movimento/Repouso | Approved | `hades-ro-prere` / `berenice` |
| [SPEC-0020](SPEC-0020-equipment-visual-layers-and-attachment-points.md) | Camadas Visuais de Equipamentos e Pontos de Ancoragem (Attach Points) | Approved | `berenice` / `hades-ro-prere` |
| [SPEC-0021](SPEC-0021-combo-action-cadence-and-rhythmic-combat.md) | Cadência Fixa, Encadeamento de Combos Rítmicos e Feedback Visual de Combate | Approved | `berenice` |
| [SPEC-0022](SPEC-0022-training-dummy-entity-and-combat-hit-feedback.md) | Entidade de Treino Gelatinosa, Feedback de Impacto e Combate Físico em Tempo Real | Approved | `berenice` |
| [SPEC-0023](SPEC-0023-evasive-maneuver-hud-bars-and-status-inventory-windows.md) | Movimento Evasivo (L2), Barras de Status (HP/SP) e Janelas de Atributos e Inventário | Approved | `berenice` / `hades-ro-prere` |
| [SPEC-0024](SPEC-0024-realtime-particle-system-and-combat-impacts.md) | Sistema de Partículas em Tempo Real e Impactos de Combate | Approved | `berenice` |
| [SPEC-0025](SPEC-0025-mouse-only-combat-and-shounen-battle-shouts.md) | Camada de Compatibilidade Mouse-Only/Touchpad e Gritos de Batalha Anime Shounen | Approved | `berenice` |
| [SPEC-0026](SPEC-0026-nordic-battle-cries-and-auto-idle-sitting.md) | Gritos de Batalha Nórdicos e Sistema de Descanso Automático (Auto-Idle Sit) | Approved | `berenice` |
| [SPEC-0027](SPEC-0027-gcp-cloud-deployment-and-wan-networking.md) | Implantação em Nuvem Gratuita (GCP Free Tier) e Conectividade WAN via QUIC | Approved | `hades-net` / `hades-world` / `hades-login` / `berenice` |
| [SPEC-0028](SPEC-0028-gnd-ground-texturing-and-expanded-hud.md) | Texturização de Solo (.gnd + .bmp), Interface Despoluída e Mundo Vivo (docs/VALVE.md) | Approved | `hades-ro-prere` / `berenice` |



