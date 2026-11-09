# The Cubes

Prototipo voxel en Rust + Bevy para explorar un sandbox de bloques original.
No utiliza assets ni contenido de Minecraft.

## Ejecutar

Requiere Rust estable y las dependencias de sistema de Bevy para ventanas y
renderizado. Audio y gamepad están desactivados mientras el juego no los usa,
evitando dependencias nativas de ALSA y libudev en Linux.

```sh
cargo run
```

## Recorrido del jugador

1. El menú principal permite jugar, crear un mundo o salir.
2. **Jugar** lista los mundos guardados.
3. **Crear mundo** permite elegir nombre y semilla.
4. La pantalla de carga genera o restaura por etapas el área inicial y prepara
   sus mallas progresivamente; se puede cancelar.
5. `Esc` abre la pausa. **Guardar y volver al menú** persiste mundo, jugador,
   orientación, hora del día, bloque seleccionado e inventario.
6. Al desplazarte, el terreno se genera proceduralmente alrededor del jugador;
   las zonas lejanas se descargan y pueden regenerarse al volver.

Los mundos se guardan en `$XDG_DATA_HOME/the-cubes/worlds` o, si esa variable no
está definida, en `~/.local/share/the-cubes/worlds`. El archivo usa RON e incluye
una versión de formato para validar futuras migraciones. El guardado es manual
desde el menú de pausa.

## Controles en el mundo

- `W A S D`: caminar.
- `Espacio`: saltar; subir en modo vuelo.
- `Mayús izquierdo`: correr.
- `Ctrl izquierdo`: bajar en modo vuelo.
- `F`: alternar entre caminar y volar.
- Ratón: mirar.
- `Esc`: pausar; en el menú de pausa se puede continuar o guardar y volver.
- Clic izquierdo: quitar y recoger un bloque.
- Clic derecho: colocar una unidad del bloque seleccionado.
- `1`–`6` o rueda del ratón: seleccionar uno de los seis espacios de la barra.

La interfaz muestra una mira, el modo de movimiento, el bloque activo y una
barra rápida de seis espacios con las cantidades del inventario. Actualmente hay
tres tipos de bloque; los otros tres espacios quedan disponibles para futuras
adiciones. No se pueden colocar bloques dentro del jugador.

El ciclo día/noche dura 24 minutos reales: cada minuto real equivale a una hora
del juego. La iluminación cambia suavemente durante el amanecer, el día, el
atardecer y la noche. El cielo tiene un gradiente procedural, estrellas que se
atenúan con la luz diurna y un sol y una luna cúbicos. El reloj aparece en el
HUD y se detiene al pausar.

## Base técnica

- Chunks `16×16×16` almacenados como arreglos densos de IDs de bloque, no como
  una entidad por bloque.
- Streaming infinito horizontal con radio de carga y descarga para mantener
  acotados la memoria y el número de mallas activas.
- Greedy meshing por chunk: omite caras ocultas y fusiona caras coplanares del
  mismo bloque.
- Las ediciones reconstruyen los assets de malla existentes y solo marcan el
  chunk afectado y los vecinos que comparten un borde.
- El terreno es determinista por semilla; la generación, restauración de chunks
  guardados y creación de mallas avanzan progresivamente durante la pantalla de
  carga. Solo se guardan chunks modificados; el terreno no editado se regenera
  desde la semilla al explorar o cargar el mundo.
