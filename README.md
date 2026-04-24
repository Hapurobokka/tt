# tt-map

Terminal TTRPG map tool. Roguelike aesthetic, local-only, built with Ratatui.

---

## Fase 1 — Skeleton + Grid + Cursor

El objetivo de esta fase es tener una app que corre, muestra un grid, y responde al teclado.
Nada de pintar todavía. Solo moverse.

### Checklist

- [x] El terminal se inicializa correctamente (raw mode, pantalla alternativa)
- [x] El terminal se restaura al salir, incluso si hay un panic
- [x] Hay un loop principal: renderizar → leer input → repetir
- [x] Se renderiza un grid de tamaño fijo (ej. 40×20) hecho de celdas vacías (`.`)
- [x] Hay un cursor visible sobre el grid (ej. la celda del cursor se resalta o usa `@`)
- [x] El cursor se mueve con las flechas del teclado
- [x] El cursor se mueve con `h j k l` (estilo vim)
- [x] El cursor no puede salirse de los límites del grid
- [x] Presionar `q` cierra la app limpiamente

### Criterio de éxito

Puedo correr `cargo run`, ver un grid en la terminal, moverme por él, y salir con `q`.
El terminal queda en el mismo estado que antes de correr el programa.

---

## Fase 2 — Pintar celdas + Paleta de colores

El objetivo es poder colorear celdas del grid con color de fondo.
El cursor sigue siendo `@`. Las celdas pintadas muestran su color. Las vacías siguen siendo `.`.

### Checklist

- [x] Existe un struct `Cell` con al menos un campo `bg_color: Color`
- [x] El `App` tiene un grid 2D (`Vec<Vec<Cell>>`) que representa el estado del mapa
- [x] El grid se inicializa del tamaño del área dibujable al arrancar
- [x] Las celdas se renderizan con su `bg_color` (las vacías sin color, las pintadas con su color)
- [x] Hay una paleta de colores predefinida (mínimo 8 colores)
- [x] El color actual se puede cambiar con las teclas `1`–`8` (o Tab para ciclar)
- [x] Presionar `Space` o `Enter` pinta la celda bajo el cursor con el color actual
- [x] Presionar `x` limpia la celda bajo el cursor (vuelve a vacía)
- [x] El color actualmente seleccionado se muestra en algún lugar de la UI (título, borde, o status bar)

### Criterio de éxito

Puedo moverme por el grid, seleccionar colores con número, pintar celdas, borrarlas,
y ver los colores reflejados en pantalla en tiempo real.

---

## Notas de diseño (para fases futuras)

- **Modelo commit**: las acciones no se aplican hasta confirmar con `Space`/`Enter`.
  `Escape` cancela y descarta los cambios. Implica estado "preview" o snapshot al entrar al modo.
- **Undo stack**: cada acción confirmada empuja un `Action` (enum con variantes por tipo de cambio)
  a un `Vec`. `u` hace pop y revierte. Se diseña junto con el modelo commit para que sean coherentes.

---

## Fase 3 — Tokens

Los tokens son entidades que viven *sobre* el grid. Tienen posición, un carácter visual,
y un color de foreground. El cursor sigue existiendo independientemente de los tokens.

### Checklist

- [x] Existe un struct `Token` con campos: `x: u16`, `y: u16`, `character: char`, `fg_color: Color`
- [x] El `App` tiene un `Vec<Token>` para almacenar todos los tokens
- [x] Presionar `t` coloca un nuevo token en la posición del cursor (carácter y color por defecto)
- [x] Los tokens se renderizan encima del grid (su carácter reemplaza visualmente lo que haya en esa celda)
- [ ] Si el cursor está sobre una celda con un token y se presiona `Enter`, ese token queda **seleccionado**
- [ ] Con un token seleccionado, `hjkl` y las flechas mueven el token (el cursor va con él)
- [ ] Presionar `Enter` o `Escape` suelta el token seleccionado (vuelve a modo navigate)
- [x] Presionar `d` cuando el cursor está sobre un token lo elimina
- [ ] No pueden existir dos tokens en la misma celda (al mover o colocar, verificar colisión)

### Criterio de éxito

Puedo colocar tokens en el mapa, seleccionarlos, moverlos por el grid,
y eliminarlos. Los tokens son visualmente distintos del cursor y del grid.
