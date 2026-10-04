# SPEC-0008: Transação Atômica de Itens do Chão (Floor Items), UUIDs Universais e Proteção contra Rollback/DoS

- **Autor:** Hades Core Team
- **Data:** 2026-10-02
- **Status:** Approved
- **Módulo Afetado:** `crates/hades-core` (`items`)

---

## 1. Contexto e Motivação

Em servidores de arquitetura clássica dos anos 2000, um dos exploits mais devastadores para a economia é a **Duplicação de Itens por Desincronia / DoS / Crash do Servidor de Mapas (Rollback Dupe / Ghost Floor Items)**:
1. **Falta de Atomicidade:** O descarte no chão e a adição/remoção de inventário são tratados em etapas separadas ou em processos distintos (`map-server` volátil vs `char-server` com auto-save desfasado).
2. **Ghost Floor Items:** Sob picos de DoS, a inserção no inventário ocorre, mas o descarte da struct do chão falha ou sofre atraso, permitindo que cliques repetidos recolham o mesmo item múltiplas vezes.
3. **Falta de Identificador Único Universal (UUID):** Itens não possuem identificadores únicos globais; dois itens de mesma ID base (`nameid`) são indistinguíveis no banco de dados.

O **Hades Engine** resolve essa vulnerabilidade na raiz arquitetural através de:
* **Identificadores Únicos Universais (`ItemInstanceId` de 64 bits):** Toda instância de item no mundo possui um número serial imutável.
* **Transação Atômica Pura de Chão $\leftrightarrow$ Mochila:** A remoção do chão e a inserção no inventário são indivisíveis.
* **Idempotência Estrita sob Spam/DoS:** Mesmo que 10.000 pacotes de coleta cheguem para o mesmo item no mesmo tick, exatamente **uma única** solicitação é aceita; todas as 9.999 restantes falham imediatamente em $O(1)$ com zero mutação de estado e zero alocações na heap.

---

## 2. Contrato de Dados

### 2.1 Identificador Universal de Instância (`ItemInstanceId`)
Inteiro de 64 bits (`u64`), gerado monotonicamente:
* Suporta $1.8 \times 10^{19}$ itens únicos ao longo da vida do servidor sem colisões.
* Rastreia a linhagem do item (quem dropou, quando e em qual mapa).

### 2.2 Dados do Item (`ItemInstance`)
```text
[ ItemInstance - Memória Contígua ]
- instance_id: ItemInstanceId (8 bytes - UUID único)
- item_id: u32                (4 bytes - ID do protótipo na db, ex: 501 = Poção Vermelha)
- amount: u32                 (4 bytes - quantidade)
- refine: u8                  (1 byte  - refinamento +0 a +10)
- cards: [u16; 4]             (8 bytes - slots de cartas)
```

### 2.3 Item no Chão (`FloorItem`)
Entidade residente no chão do mapa:
* `floor_id`: Inteiro `u32` sequencial local do mapa.
* `item`: Estrutura `ItemInstance` com seu UUID preservado.
* `position`: `Position` (coordenada $X, Y$).
* `owner_id`: `Option<EntityId>` (jogador com prioridade de saque pelo MVP/monstro).
* `priority_until_tick`: `Tick` (após este tick, qualquer jogador pode saquear).
* `despawn_tick`: `Tick` (após este tick, o item some do chão se ninguém pegar).

---

## 3. Operações Unitárias Atômicas

1. `drop_item_to_floor(inventory: &mut Inventory, slot_idx: usize, amount: u32, pos: Position, floor_manager: &mut FloorItemManager, current_tick: Tick) -> Result<u32, ItemError>`
   * Remove atômica e irreversivelmente o item do inventário e o transfere para o chão com seu UUID original.
2. `pickup_floor_item(inventory: &mut Inventory, picker_id: EntityId, floor_id: u32, picker_pos: Position, floor_manager: &mut FloorItemManager, current_tick: Tick) -> Result<ItemInstance, ItemError>`
   * **Atomicidade Garantida:**
     1. Valida distância ($\le 2$ células via Chebyshev distance).
     2. Valida prioridade de loot (se não expirou e não for o dono, rejeita com `LootPriorityActive`).
     3. Remove o item do chão imediatamente via `floor_manager.take(floor_id)`. Se outro pacote já retirou, retorna `ItemNotFound`.
     4. Insere no inventário do jogador validando capacidade e sobrepeso.
     5. Se a inserção no inventário falhar (mochila cheia/sobrepeso), devolve o item ao chão sem perder o UUID.
3. `Inventory::contains_instance_id(&self, instance_id: ItemInstanceId) -> bool`
   * Checagem em tempo $O(1)$ contra injeção de itens duplicados.

---

## 4. Invariantes e Regras de Segurança

- [x] **Invariante de Exclusão Mútua (No Ghost Items):** Um item só pode existir em exatamente **um** lugar ao mesmo tempo (ou no inventário de A, ou no inventário de B, ou no chão do mapa).
- [x] **Idempotência contra DoS:** Múltiplas requisições de coleta simultâneas no mesmo tick processam apenas a primeira; todas as seguintes recebem `Err(ItemNotFound)` com custo $O(1)$ de CPU e zero alocações.
- [x] **Detecção de UUID Duplicado:** É impossível inserir no inventário um item cujo `instance_id` já esteja ativo no servidor.

---

## 5. Suíte de Testes Unitários Obrigatórios

1. `test_floor_item_atomic_pickup_happy_path`: Valida descarte no chão e coleta atômica com preservação idêntica de UUID e propriedades.
2. `test_prevent_dos_spam_pickup_duplication`: Simula 100 requisições simultâneas de coleta no mesmo tick para o mesmo `floor_id`; exatamente 1 tem sucesso e 99 falham.
3. `test_reject_pickup_beyond_range`: Rejeita coleta de itens a mais de 2 células de distância.
4. `test_loot_priority_protection`: Garante que outros jogadores não conseguem recolher o loot de quem matou o monstro até expirar o timer.
5. `test_anti_dupe_rejects_duplicate_instance_id`: Rejeita inserção de um item com UUID já existente.
