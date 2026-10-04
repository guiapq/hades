# SPEC-0006: Particionamento Espacial em Buckets O(1) (AoI) e Motor de Ticks Determinístico (20 Hz)

- **Autor:** Hades Core Team
- **Data:** 2026-10-02
- **Status:** Approved
- **Módulo Afetado:** `crates/hades-core` (`aoi` e `tick`)

---

## 1. Contexto e Motivação

Conforme o **Potato Budget** e o documento de [Arquitetura](../../docs/ARCHITECTURE.md), um servidor de alta densidade computacional (capaz de rodar 500 CCU em uma máquina de $4/mês) não pode:
1. Fazer verificações de proximidade ingênuas de ordem $O(N^2)$ (comparar cada entidade com todas as outras entidades ativas do mundo).
2. Simular IA de monstros em áreas do mapa onde não há nenhum jogador humano presente (*Spatial Economics*).
3. Enviar pacotes para clientes de entidades distantes fora do campo de visão da tela.
4. Ter variações ou pausas não-determinísticas no tempo do ciclo de simulação.

A solução do Hades é o **Particionamento Espacial em Buckets $O(1)$** aliado a um **Loop de Ticks Fixo a 20 Hz** (50 ms por ciclo).

---

## 2. Contrato de Dados (Grid de Buckets & AoI)

### 2.1 Dimensões dos Buckets
* O mundo é particionado em células quadradas de **$32 \times 32$ tiles**:
  $$\text{bucket\_x} = \left\lfloor \frac{x}{32} \right\rfloor, \quad \text{bucket\_y} = \left\lfloor \frac{y}{32} \right\rfloor$$
* Um mapa de $1024 \times 1024$ tiles possui exatamente $32 \times 32 = 1024$ buckets.
* Cada bucket mantém:
  * Um conjunto compacto de `EntityId`s residentes.
  * Contagem de jogadores humanos presentes (para determinação de estado ativo vs. hibernação).

### 2.2 Área de Interesse (AoI — Raio $3 \times 3$)
* O campo de visão de um jogador corresponde ao seu bucket central e aos **8 buckets vizinhos** (um bloco de $3 \times 3$ buckets = $96 \times 96$ tiles, suficiente para cobrir qualquer resolução de tela com folga):
  $$\text{AoI}(bx, by) = \{ (bx + dx, by + dy) \mid dx, dy \in \{-1, 0, 1\} \}$$
* Consultar todas as entidades no campo de visão de um jogador exige apenas inspecionar **9 buckets contíguos**, em tempo estritamente $O(1)$.

### 2.3 Economia Espacial & Hibernação (*Spatial Economics*)
* Um bucket é classificado como:
  * **Ativo (Tier 0 - 20 Hz):** Há pelo menos 1 jogador humano em seu raio $3 \times 3$. Monstros e projéteis são simulados a cada 50ms.
  * **Hibernando (Tier 2 - Dormindo):** Zero jogadores humanos no raio $3 \times 3$. **Nenhum ciclo de CPU é gasto com IA de monstros**.

### 2.4 Transições de Setor (Eventos de Entrada e Saída)
Quando uma entidade cruza a fronteira de um bucket $(bx_1, by_1) \rightarrow (bx_2, by_2)$:
* **Novos Observadores (Spawn):** Jogadores presentes nos buckets pertencentes a $\text{AoI}_2 \setminus \text{AoI}_1$ recebem notificação de aparição da entidade.
* **Observadores Antigos (Despawn):** Jogadores presentes em $\text{AoI}_1 \setminus \text{AoI}_2$ recebem notificação de desaparecimento da entidade.
* **Observadores Constantes (Delta):** Jogadores em $\text{AoI}_1 \cap \text{AoI}_2$ recebem apenas o `MovementDelta` de 6 bytes.

---

## 3. O Loop de Ticks Fixo (`TickEngine`)

* Frequência fixa: **20 Hz (50.000 microssegundos / 50 ms)**.
* Pipeline estrito de 4 fases lineares a cada tick $T$:
  1. **Fase de Ingress:** Esvazia a fila atômica de inputs recebidos da rede.
  2. **Fase de Simulação:** Executa as operações unitárias puras apenas para entidades em buckets ativos.
  3. **Fase Espacial:** Atualiza os buckets de entidades que cruzaram fronteiras $32 \times 32$.
  4. **Fase de Egress:** Agrupa os deltas gerados para os jogadores de acordo com seu raio de AoI.

---

## 4. Invariantes e Regras de Segurança

- [x] **Zero $O(N^2)$:** Nunca é feita varredura global de entidades no mapa.
- [x] **Zero Heap Allocation no Tick:** A migração de entidades entre buckets ocorre em buffers fixos pré-alocados.
- [x] **Spatial Economics:** Zero chamadas de IA para monstros em setores hibernados.
- [x] **Determinismo Estrito:** A mesma sequência de ticks com os mesmos inputs reproduz o mesmo estado final de posições e visibilidade.

---

## 5. Suíte de Testes Unitários Obrigatórios

1. `test_spatial_grid_bucket_coordinates`: Valida a conversão correta de coordenadas de tiles $(X, Y)$ para índices de buckets.
2. `test_entity_bucket_insertion_and_removal`: Adiciona, move e remove entidades do grid espacial em tempo $O(1)$.
3. `test_aoi_3x3_query`: Valida se a consulta de vizinhança retorna exatamente as entidades dos 9 buckets adjacentes.
4. `test_spatial_economics_hibernation`: Garante que um bucket sem jogadores em seu raio $3 \times 3$ entra em hibernação.
5. `test_bucket_migration_diff_spawn_despawn`: Valida as listas de novos observadores e observadores perdidos ao cruzar a fronteira de um bucket.
