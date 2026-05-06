# tt-map

Terminal TTRPG map tool. Roguelike aesthetic, local-only, built with Ratatui.

---

## Completado

- **Fase 1** — Skeleton: terminal init/restore, game loop, grid, cursor con `hjkl`/flechas, salir con `q`
- **Fase 2** — Pintar: `Cell`, grid 2D, paleta de colores, modo Drawing/Deleting, status bar
- **Fase 3** — Tokens: colocar (`t` + char), mover (`m`), eliminar (`Ctrl+X`); rectángulos (`D`/`X`); overlay para preview; `State::Active(Mode)` como contrato de ciclo de vida; movimiento diagonal `y/u/b/n`; colores y caracteres de terreno independientes; lints estrictos de Clippy

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

**Layout**

- [ ] El área se divide en: mapa (izquierda), panel lateral (derecha), barra inferior
- [ ] El panel lateral muestra la paleta de colores con el color actual resaltado
- [ ] El panel lateral muestra la lista de tokens activos (carácter + posición)
- [x] La barra inferior muestra el estado actual y el color seleccionado en modo normal

**Comandos**

- [ ] Existe `State::Command(String)` para almacenar el input del usuario
- [x] Presionar `:` entra en modo comando (la barra inferior se convierte en input)
- [x] Los caracteres escritos se acumulan en el `String` del estado
- [x] `Backspace` borra el último carácter del comando
- [x] `Esc` cancela el comando y vuelve a `Normal`
- [ ] `Enter` ejecuta el comando

**Save/Load**

- [x] `Cell` y `Token` implementan `Serialize` y `Deserialize`
- [x] `MapState` también es serializable (es lo único que se guarda)
- [ ] `:w <nombre>` guarda el mapa actual en `<nombre>.json`
- [ ] `:e <nombre>` carga un mapa desde `<nombre>.json` y reemplaza el estado actual
- [ ] Si el archivo no existe o es inválido, se muestra un mensaje de error en la barra inferior
- [ ] Cargar un mapa directamente desde la línea de comandos (ej: `tt mapa.json`)

### Criterio de éxito

Puedo dibujar un mapa con colores de zona y caracteres de terreno, colocar tokens,
guardar con `:w sesion`, cerrar, volver a abrir, cargar con `:e sesion`, y ver exactamente el mismo estado.

## Notas de diseño (para fases futuras)

- **Undo stack**: cada acción confirmada empuja un `Action` (enum con variantes por tipo de cambio)
  a un `Vec`. `u` hace pop y revierte. Coherente con el modelo commit ya implementado.
- **Exportar como texto plano**: `:export <nombre>` (o similar) vuelca el grid actual como un archivo `.txt` — cada celda es su `character`, tokens incluidos. Tiene todo el sentido: la herramienta trabaja con texto, el output natural ES texto.
- Unicode (kanji, emoji) es técnicamente soportado por `char` pero caracteres anchos (2 celdas) rompen el grid — pendiente para el futuro.
- **Tokens multi-celda**: enemigos de 2×2 o 3×3 son comunes en TTRPG. El modelo actual almacena cada `Token` en su posición exacta (`HashMap<Position, Vec<Token>>`); para multi-celda, la mejor opción es añadir `size: (u8, u8)` al struct y guardar el token solo en la celda ancla. Las celdas ocupadas se calculan dinámicamente. `token_at()` deja de ser un lookup O(1) y pasa a ser un escaneo de bounding boxes O(n tokens) — aceptable dado el número esperado de tokens.
