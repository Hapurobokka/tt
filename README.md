# tt-map

Terminal TTRPG map tool. Roguelike aesthetic, local-only, built with Ratatui.

---

## Completado

- **Fase 1** — Skeleton: terminal init/restore, game loop, grid, cursor con `hjkl`/flechas, salir con `q`
- **Fase 2** — Pintar: struct `Cell`, grid 2D, paleta de 8 colores (Tab/BackTab), modo Drawing y Deleting, color en status bar
- **Fase 3** — Tokens: struct `Token` con Display, colocar (`t`), mover (`m`), eliminar (`Ctrl+X`), tokens múltiples por celda permitidos (resolución pendiente para fase futura)
- **Fase 3.1** — Refinamiento de dibujado y arquitectura:
  - `State::Active(Mode)` como contrato de ciclo de vida: misma tecla o `Space` para commit, `Esc` para cancelar
  - Overlay (`HashMap<(usize,usize), Cell>`) para preview de cambios antes de confirmar
  - Movimiento diagonal con `y/u/b/n` (convención nethack)
  - Rectángulos: `D` para pintar, `X` para borrar — preview en vivo mientras se mueve el cursor
  - `Position` struct; `paint` acepta posición explícita en vez de leer del cursor
  - Keybinds: `d` dibujar, `D` rectángulo, `x` borrar terreno, `X` borrar rect, `Ctrl+X` eliminar token, `m` mover token
  - Lints estrictos de Clippy (`pedantic`, `nursery`, `unsafe_code = deny`) en `Cargo.toml`

---

## Fase 3.2 — Colocación de tokens

### Flujo de `PlacingToken`

1. `t` → entra en `Active(PlacingToken { character: None })`, cursor sigue siendo `@`
2. Cualquier tecla imprimible → `PlacingToken { character: Some(c) }`, cursor cambia a `c` con el `fg_color` actual
3. Mover el cursor a la posición deseada
4. `t` o `Space` → coloca el token con `character: c` y `fg_color: PALETTE[fg_color_i]`
5. `Esc` en cualquier momento cancela

El color del token se hereda de `fg_color_i` en el momento del commit — el usuario lo selecciona antes de presionar `t`, igual que con el brush.

### Checklist

- [x] `Mode::PlacingToken { character: Option<char> }` reemplaza el `PlacingToken` actual
- [x] Primera tecla imprimible tras `t` asigna `character: Some(c)` y cambia `cursor.character` y `cursor.fg_color`
- [x] El token se coloca al hacer commit con `fg_color: PALETTE[fg_color_i]`
- [x] `commit()` y `revert()` manejan `PlacingToken` (quitar los `todo!()`)
- [x] El render muestra el título/borde correcto para `PlacingToken`

---

## Fase 4 — Save/Load + Layout + Comandos

### Layout objetivo

```
┌───────────────────────┬─────────────┐
│                       │ [ Paleta ]  │
│     MAPA (grid)       │ ■ ■ ■ ■     │
│                       │             │
│                       │ [ Tokens ]  │
│                       │ 't' (3,4)   │
│                       │ '@' (7,2)   │
├───────────────────────┴─────────────┤
│ EXPLORING | COLOR: White            │
└─────────────────────────────────────┘
```

En modo comando, la barra inferior se convierte en input:

```
│ :w mapa.json_                       │
```

### Refactor de arquitectura (prerequisito)

Antes del layout, el código se reorganiza:

- [x] `Cell` pasa a tener tres campos: `bg_color`, `fg_color`, `terrain: char`
- [ ] El estado se divide en `MapState` (cells, tokens) y `UiState` (state, color_i, message)
- [ ] El render se divide en funciones puras: `render_map`, `render_sidebar`, `render_statusbar`, `render_commandline`
- [ ] `impl Widget for &App` solo coordina el layout y llama a esas funciones

### Checklist

**Terreno**

- [x] Existe una paleta de caracteres de terreno (ej. `.` suelo, `#` pared, `~` agua, `%` árbol...)
- [x] El modo Drawing pinta `bg_color` de la celda (color de zona)
- [x] Existe un modo Terrain que pinta el carácter y `fg_color` de la celda
- [x] Ambas paletas (color y terreno) son seleccionables independientemente

**Layout**

- [ ] El área se divide en: mapa (izquierda), panel lateral (derecha), barra inferior
- [ ] El panel lateral muestra la paleta de colores con el color actual resaltado
- [ ] El panel lateral muestra la lista de tokens activos (carácter + posición)
- [ ] La barra inferior muestra el estado actual y el color seleccionado en modo normal

**Comandos**

- [ ] Existe `State::Command(String)` para almacenar el input del usuario
- [ ] Presionar `:` entra en modo comando (la barra inferior se convierte en input)
- [ ] Los caracteres escritos se acumulan en el `String` del estado
- [ ] `Backspace` borra el último carácter del comando
- [ ] `Esc` cancela el comando y vuelve a `Normal`
- [ ] `Enter` ejecuta el comando

**Save/Load**

- [ ] `Cell` y `Token` implementan `Serialize` y `Deserialize`
- [ ] `MapState` también es serializable (es lo único que se guarda)
- [ ] `:w <nombre>` guarda el mapa actual en `<nombre>.json`
- [ ] `:e <nombre>` carga un mapa desde `<nombre>.json` y reemplaza el estado actual
- [ ] Si el archivo no existe o es inválido, se muestra un mensaje de error en la barra inferior

### Criterio de éxito

Puedo dibujar un mapa con colores de zona y caracteres de terreno, colocar tokens,
guardar con `:w sesion`, cerrar, volver a abrir, cargar con `:e sesion`, y ver exactamente el mismo estado.

## Notas de diseño (para fases futuras)

- **Undo stack**: cada acción confirmada empuja un `Action` (enum con variantes por tipo de cambio)
  a un `Vec`. `u` hace pop y revierte. Coherente con el modelo commit ya implementado.
- **Tokens apilados**: múltiples tokens pueden coexistir en una celda. Pendiente: UI para elegir cuál mover cuando hay más de uno.
- Unicode (kanji, emoji) es técnicamente soportado por `char` pero caracteres anchos (2 celdas) rompen el grid — pendiente para el futuro.
