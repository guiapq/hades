# SPEC-0004: Parser de Mapas Binários (.gat) para CollisionGrid do Hades

- **Autor:** Hades Core Team
- **Data:** 2026-10-02
- **Status:** Approved
- **Módulo Afetado:** `crates/hades-ro-prere` (Map Data / Spatial Engine)

---

## 1. Contexto e Motivação

Em ecossistemas legados de MMORPGs 2.5D, a informação de colisão geográfica de cada mapa reside em arquivos binários de extensão `.gat` (Ground Altitude Table).
Para que o **Hades** execute com total compatibilidade espacial com mapas clássicos:
1. Deve ser capaz de parsear buffers binários de `.gat` sem dependências externas de C/C++.
2. Mapear os tipos de células do formato `.gat` diretamente para o nosso bitset ultracompacto [`CollisionGrid`](../crates/hades-core/src/collision.rs) (1 bit por tile, 128 KB para $1024 \times 1024$).
3. Garantir parsing zero-copy / rápido, com validação de assinatura (*magic header*) e dimensões de mapa.

---

## 2. Estrutura Binária do Arquivo .gat Clássico

O cabeçalho e corpo do `.gat` seguem a especificação binária canônica:

```text
[ Offset 0x00..0x03 ] Magic Header: "GRAT" (4 bytes ASCII)
[ Offset 0x04..0x05 ] Major Version (u16 Little Endian, ex: 1 ou 2)
[ Offset 0x06..0x07 ] Minor Version (u16 Little Endian, ex: 1 ou 2)
[ Offset 0x08..0x0B ] Width (u32 Little Endian - largura em tiles)
[ Offset 0x0C..0x0F ] Height (u32 Little Endian - altura em tiles)
[ Offset 0x10..End  ] Array de Células: (Width * Height) * 20 bytes cada
```

### Estrutura de Cada Célula (20 bytes):
* `upper_left_height`: `f32` (4 bytes)
* `upper_right_height`: `f32` (4 bytes)
* `bottom_left_height`: `f32` (4 bytes)
* `bottom_right_height`: `f32` (4 bytes)
* `cell_type`: `u32` Little Endian (4 bytes)

### Tipos de Células (`cell_type`):
| Tipo Numérico | Significado Original | Tradução no Hades `CollisionGrid` |
| :--- | :--- | :--- |
| `0` | **Walkable & Snipable** (Terreno livre normal) | `walkable = true` |
| `1` | **Non-Walkable & Non-Snipable** (Parede/Obstáculo sólido) | `walkable = false` |
| `2` | **Non-Walkable & Snipable** (Água rasa / despenhadeiro) | `walkable = false` (para movimento) |
| `3` | **Walkable & Snipable** (Água rasa andável) | `walkable = true` |
| `4` | **Non-Walkable & Snipable** (Obstáculo baixo) | `walkable = false` (para movimento) |
| `5` | **Non-Walkable & Snipable** (Abismo / atirável) | `walkable = false` (para movimento) |
| `6` | **Walkable & Snipable** (Gelo / terreno especial) | `walkable = true` |

---

## 3. Operações Unitárias

1. `parse_gat_to_collision_grid(bytes: &[u8]) -> Result<CollisionGrid, GatParseError>`
   * Valida o magic header `GRAT`.
   * Lê dimensões `width` e `height`.
   * Preenche o bitset contíguo do `CollisionGrid` mapeando as células andáveis (`0, 3, 6`).
2. `GatCellType::is_walkable(type_id: u32) -> bool`
   * Classificação discreta do tipo de célula.

---

## 4. Invariantes e Regras de Segurança

- [x] Arquivos com tamanho inferior a 16 bytes ou sem o magic `GRAT` são rejeitados imediatamente.
- [x] Se o payload não tiver bytes suficientes para `width * height * 20`, retorna erro de integridade de dados (*TruncatedBuffer*).
- [x] O `CollisionGrid` gerado é estritamente $O(1)$ para consultas durante o tick do servidor.
