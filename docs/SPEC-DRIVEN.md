# Desenvolvimento Orientado a Especificações (Spec-Driven Development) 📐🧪

> **Regra de Ouro do Hades:** Nenhuma linha de lógica de simulação, protocolo de rede ou algoritmo espacial é implementada sem uma especificação prévia aprovada e sua respectiva suíte de testes unitários.

---

## 1. Por que Spec-Driven no Hades?

Servidores de jogos e sistemas concorrentes de rede são conhecidos pela extrema facilidade de introduzir *bugs silenciosos*, *race conditions* e *comportamento não-determinístico*. 

No rAthena e em motores legados, regras de combate e movimentação foram empilhadas ao longo de 20 anos sem especificação formal, resultando em:
* Efeitos colaterais imprevisíveis entre sistemas;
* Falta de cobertura de testes unitários;
* Impossibilidade de validar se um tick é 100% determinístico.

No Hades, **a especificação é o contrato da verdade**. O código em Rust é apenas a materialização do contrato.

---

## 2. O Fluxo de Desenvolvimento (The Hades Workflow)

Cada nova funcionalidade (feature), protocolo de pacote ou sistema de jogo deve obrigatoriamente seguir este ciclo de 4 etapas:

```
+-------------------+      +---------------------+      +---------------------+      +---------------------+
| 1. Especificação  | ---> | 2. Testes Unitários | ---> | 3. Implementação    | ---> | 4. Validação &      |
|    (RFC / Spec)   |      |    (TDD / Invariants)      |    (Rust Core)      |      |    Potato Benchmark |
+-------------------+      +---------------------+      +---------------------+      +---------------------+
```

### Etapa 1: Elaboração da Especificação (`specs/`)
Antes de abrir o editor de código para programar a funcionalidade, cria-se um arquivo de especificação detalhando:
1. **Contrato de Dados:** Estrutura em bits/bytes, alinhamento de memória e empacotamento de rede.
2. **Operações Unitárias:** Quais transições de estado a feature realiza e quais entradas são aceitas.
3. **Invariantes do Sistema:** O que **nunca** pode acontecer (ex: "HP nunca fica negativo", "Entidade nunca ocupa célula bloqueada", "Tick nunca aloca na heap").
4. **Limites de Orçamento:** Impacto esperado na CPU (< microsegundos) e RAM.

### Etapa 2: Escrita dos Testes Unitários Primeiro (TDD)
O desenvolvedor implementa os testes unitários que validam a especificação:
* Casos felizes (*happy paths*).
* Casos de borda (*edge cases*): limites de coordenadas do mapa, desconexão abrupta durante troca, ataques simultâneos no mesmo tick.
* **Testes de Determinismo:** Dada uma semente (*seed*) e a mesma fila de inputs, o resultado do tick precisa ser exatamente o mesmo bit a bit.

### Etapa 3: Implementação Pura em Rust
Com a especificação e os testes desenhados:
* Implementa-se a lógica como uma **função pura** ou sistema ECS sem efeitos colaterais externos.
* Todos os testes unitários devem passar (`cargo test`).

### Etapa 4: Validação de Performance (Potato Budget Check)
* A feature deve ser submetida a benchmarks de micro-performance (`cargo bench` com Criterion).
* Se a nova lógica violar o limite de tempo do tick (< 3 ms totais para 500 jogadores), a implementação é refatorada antes de ser mesclada na branch principal.

---

## 3. Template Oficial de Especificação (RFC / Spec Template)

Toda nova feature proposta deve usar o modelo abaixo (salvo no diretório `specs/SPEC-XXXX-nome.md`):

```markdown
# SPEC-XXXX: [Título da Feature / Sistema]

- **Autor:** [Nome / Usuário]
- **Data:** [YYYY-MM-DD]
- **Status:** [Draft | Under Review | Approved | Implemented]
- **Módulo Afetado:** [Network | AoI | Movement | Combat | Persistence]

## 1. Contexto e Motivação
[Descrição clara do problema a ser resolvido ou da funcionalidade adicionada.]

## 2. Contrato de Dados (Layout & Bitpacking)
[Como os dados são representados em memória e nos pacotes de rede.]
- Tamanho exato em bytes:
- Campos e tipos:
- Representação binária / bitmask:

## 3. Operação Unitária e Transição de Estado
- **Input:** [Dados que entram na esteira do tick]
- **Regras de Negócio:** [Lógica pura aplicada]
- **Output:** [Novo estado da entidade e deltas de rede gerados]

## 4. Invariantes e Regras de Segurança
- [ ] Regra 1 (ex: Nenhuma alocação na heap durante a execução)
- [ ] Regra 2 (ex: Coordenadas X e Y devem estar dentro dos limites do mapa)
- [ ] Regra 3 (ex: Ação rejeitada silenciosamente se a entidade estiver atordoada)

## 5. Casos de Teste Obrigatórios (Test Suite)
- [ ] `test_fluxo_normal`: Cenário básico funcionando.
- [ ] `test_limite_borda`: Parâmetros máximos e mínimos.
- [ ] `test_rejeicao_invalida`: Inputs forjados ou impossíveis são descartados.
- [ ] `test_determinismo`: Repetição do teste com os mesmos inputs gera o mesmo estado.

## 6. Avaliação de Impacto no Potato Budget
- Consumo adicional de RAM por entidade: [ex: 0 bytes adicionais / reaproveita slot]
- Tempo estimado de processamento por entidade: [ex: < 15 nanossegundos]
```

---

## 4. Definição de Pronto (Definition of Done - DoD)

Uma alteração ou feature só é considerada concluída quando:

1. [ ] A especificação em `specs/` está completa e aprovada.
2. [ ] Todos os testes unitários cobrindo as regras e casos de borda foram escritos e passam com 100% de sucesso (`cargo test`).
3. [ ] O código segue o guia de estilo e passa na análise estática (`cargo clippy -- -D warnings`).
4. [ ] Nenhuma alocação de heap dinâmica foi adicionada dentro do loop de tick.
5. [ ] A documentação da arquitetura foi atualizada se houver mudanças estruturais.
