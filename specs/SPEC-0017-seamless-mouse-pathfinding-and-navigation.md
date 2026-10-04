# SPEC-0017: Sistema de Pathfinding A* e Navegação Contínua por Mouse (Seamless Mouse Navigation)

## 1. Visão Geral e Contexto
Em MMORPGs isométricos clássicos em 2.5D, a interação primária do jogador é orientada por mouse (*click-to-move*), coexistindo harmoniosamente com esquemas modernos híbridos (teclado WASD e Gamepad estilo *Tree of Savior*).

Esta especificação define:
1. O algoritmo **A\* Pathfinding** sobre a grade de colisão de 1 bit ([`CollisionGrid`](../crates/hades-core/src/collision.rs)) em `hades-core`, reutilizável tanto pelo servidor quanto pelo cliente Berenice.
2. A heurística **Octile Distance** para 8 direções (custos 10 cardinal e 14 diagonal) com prevenção de corte de quinas (*corner cutting protection*).
3. O resolvedor de célula caminhável mais próxima (*nearest walkable fallback*), garantindo que cliques próximos a obstáculos (paredes, monumentos, construções) guiem o jogador até a margem caminhável mais próxima sem falhas silenciosas.
4. O módulo de **Navegação Contínua por Mouse** (*Seamless Mouse Tracking*) no cliente Berenice, suportando tanto cliques isolados quanto manter o botão esquerdo pressionado para condução fluida do personagem pela cena 2.5D.
5. Renderização de indicador visual de destino (*ground click target marker*) no chão isométrico.

---

## 2. Requisitos Técnicos e Potato Budget

1. **Zero Alocações Dinâmicas Recorrentes:** O pathfinder deve operar com buffer pré-alocado reutilizável ou pilha limitada (máximo de nós abertos/fechados limitados a 1024 nós), impedindo pressão sobre o garbage collector ou fragmentação do heap.
2. **Custo Computacional Delimitado:** O número máximo de expansões de nós no A\* é limitado a `MAX_EXPANSIONS = 512` passos por cálculo. Se o destino estiver fora de alcance ou inalcançável dentro desse teto, retorna o caminho mais próximo alcançado.
3. **Resolução de Coordenadas em Tempo Real:** Conversão imediata de coordenadas de pixels da tela para coordenadas de células do mapa através da [`IsometricProjection`](../crates/berenice/src/render/isometric.rs).
4. **Coexistência Híbrida Sem Costuras:** Se o mouse for detectado e utilizado, o personagem navega pela rota calculada; se o jogador pressionar WASD ou mover o analógico do Gamepad, a rota do mouse é interrompida instantaneamente em favor da entrada direta.

---

## 3. Modelo Matemático do A\* (8 Direções)

### 3.1 Custos e Heurística Octile
- Custo Cardinal ($N, S, L, O$): $10$
- Custo Diagonal ($NE, SE, SO, NO$): $14$ ($\approx 10 \times \sqrt{2}$)

Dados $\Delta x = |x_1 - x_2|$ e $\Delta y = |y_1 - y_2|$:
$$h(x, y) = 10 \cdot \max(\Delta x, \Delta y) + 4 \cdot \min(\Delta x, \Delta y)$$

### 3.2 Regra de Corte de Quina (Corner Cutting)
Para mover de $(x, y)$ para $(x + dx, y + dy)$ na diagonal, ambas as células ortogonais adjacentes $(x + dx, y)$ e $(x, y + dy)$ devem ser caminháveis, evitando que o jogador atravesse quinas de paredes colidentes.

---

## 4. Estruturas de Dados e Wire Messages

### 4.1 `PathResult`
```rust
pub struct PathResult {
    pub waypoints: Vec<Position>,
    pub reachable: bool,
}
```

### 4.2 Estado de Navegação do Mouse no Cliente
```rust
pub struct MouseNavigation {
    pub is_active: bool,
    pub is_dragging: bool,
    pub target_pos: Option<Position>,
    pub path: Vec<Position>,
    pub click_marker_timer: f32, // tempo restante de exibição do clique visual
    pub click_marker_pos: Option<Position>,
}
```

---

## 5. Casos de Teste Obrigatórios (TDD)

1. **Linha Reta Livre:** Caminho ortogonal e diagonal sem obstáculos.
2. **Desvio de Obstáculo em U:** O algoritmo contorna um muro em formato de U ou L.
3. **Corte de Quina Bloqueado:** Não permite diagonal cortando quina sólida.
4. **Destino Não Caminhável:** Clique em parede encontra a célula caminhável vizinha mais próxima.
5. **Alcance Máximo:** Busca truncada gracefully sem travar a CPU ao exceder `MAX_EXPANSIONS`.
6. **Integração no Cliente Berenice:** Clique na tela gera rota, atualiza destino e despacha `MoveRequest` ao World Server.
