# tt-map

Terminal TTRPG map tool. Roguelike aesthetic, local-only, built with Ratatui.

---

## Fase 1 — Skeleton + Grid + Cursor

El objetivo de esta fase es tener una app que corre, muestra un grid, y responde al teclado.
Nada de pintar todavía. Solo moverse.

### Checklist

- [ ] El terminal se inicializa correctamente (raw mode, pantalla alternativa)
- [ ] El terminal se restaura al salir, incluso si hay un panic
- [ ] Hay un loop principal: renderizar → leer input → repetir
- [ ] Se renderiza un grid de tamaño fijo (ej. 40×20) hecho de celdas vacías (`.`)
- [ ] Hay un cursor visible sobre el grid (ej. la celda del cursor se resalta o usa `@`)
- [ ] El cursor se mueve con las flechas del teclado
- [ ] El cursor se mueve con `h j k l` (estilo vim)
- [ ] El cursor no puede salirse de los límites del grid
- [ ] Presionar `q` cierra la app limpiamente

### Criterio de éxito

Puedo correr `cargo run`, ver un grid en la terminal, moverme por él, y salir con `q`.
El terminal queda en el mismo estado que antes de correr el programa.
