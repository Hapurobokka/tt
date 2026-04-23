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
