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
está definida, en `~/.local/share/the-cubes/worlds`. Los datos generales se
guardan en un archivo RON y cada chunk modificado en un archivo independiente
dentro del directorio asociado al mundo. Los chunks se escriben progresivamente
y el estado del jugador se guarda automáticamente cada minuto, además de al
usar **Guardar y volver al menú**. El HUD y la pausa muestran el estado del
guardado; si hay un error se informa y se reintenta.

## Controles en el mundo

- `W A S D`: caminar.
- `Espacio`: saltar; subir en modo vuelo.
- `Mayús izquierdo`: correr.
- `Ctrl izquierdo`: bajar en modo vuelo.
- `F`: alternar entre caminar y volar.
- Ratón: mirar.
- `Esc`: pausar; en el menú de pausa se puede continuar o guardar y volver.
- Clic izquierdo: quitar y recoger un bloque.
- Clic izquierdo sobre una criatura: golpearla si está bajo la mira y delante
  del bloque.
- Clic derecho: colocar una unidad del bloque seleccionado.
- `1`–`6` o rueda del ratón: seleccionar uno de los seis espacios de la barra.
- `E`: consumir comida y recuperar hambre.
- `C`: abrir/cerrar la mesa de crafteo; `Esc` también la cierra.

La interfaz muestra una mira, el modo de movimiento, el bloque activo y una
barra rápida de seis espacios con las cantidades del inventario: césped, tierra,
piedra, arena, madera y hojas. El mundo mezcla praderas, bosques, desiertos y
montañas según la semilla, con transiciones suaves, superficies y vegetación
propias de cada región. El HUD indica el bioma actual. No se pueden colocar
bloques dentro del jugador.

El modo supervivencia añade 20 puntos de vida y hambre. El hambre disminuye con
el tiempo y más rápido al correr; romper hojas puede dar bayas y derrotar animales
proporciona comida. Con hambre llena la vida se regenera, y llegar a cero causa
daño. Las caídas fuertes también hacen daño. Al morir, reapareces en el inicio
con media vida y hambre, y conservas inventario y experiencia. Romper bloques
concede experiencia y sube de nivel; las barras de vida, hambre y experiencia
aparecen en el HUD.

Praderas y bosques pueden albergar animales pasivos que deambulan y huyen; al
derrotarlos obtienes comida y experiencia. De noche aparecen acechadores que
persiguen y atacan al jugador, y conceden experiencia al ser derrotados. La
población está limitada y las criaturas lejanas desaparecen para mantener acotado
el coste de simulación.

La mesa de crafteo ofrece seis recetas: pico de madera (3 madera y 2 piedra),
hacha de madera (2 madera y 1 piedra), pala de madera (2 madera y 1 piedra),
espada de madera (2 madera y 2 piedra), ración de bayas (3 hojas y 1 madera)
y flechas (2 hojas y 1 arena; produce 5 flechas). Las herramientas de minería
tienen 64 usos y dan materiales y experiencia adicionales al romper sus bloques
correspondientes. La pala es 2x más eficiente en arena y tierra, y la espada
golpea con 2 puntos de daño en lugar de 1 sin romper bloques. Las recetas y la
durabilidad se conservan al guardar el mundo.

El ciclo día/noche dura 24 minutos reales: cada minuto real equivale a una hora
del juego. La iluminación cambia suavemente durante el amanecer, el día, el
atardecer y la noche. El cielo tiene un gradiente procedural, estrellas que se
atenúan con la luz diurna y un sol y una luna cúbicos. El reloj aparece en el
HUD y se detiene al pausar.

## Base técnica

- Chunks `16×16×16` almacenados como arreglos densos de IDs de bloque, no como
  una entidad por bloque.
- Streaming infinito horizontal que mantiene hasta 49 chunks alrededor del
  jugador y descarga los lejanos para acotar la memoria y las mallas activas.
- Greedy meshing por chunk: omite caras ocultas y fusiona caras coplanares del
  mismo bloque.
- Las ediciones reconstruyen los assets de malla existentes y solo marcan el
  chunk afectado y los vecinos que comparten un borde.
- El terreno es determinista por semilla; la generación, restauración de chunks
  guardados y creación de mallas avanzan progresivamente durante la pantalla de
  carga. Solo se guardan chunks modificados; el terreno no editado se regenera
  desde la semilla al explorar o cargar el mundo.
