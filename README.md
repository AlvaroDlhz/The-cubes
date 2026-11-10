# The Cubes
## Arquitectura del proyecto

El juego es una biblioteca (`src/lib.rs`) con un binario mínimo
(`src/main.rs`) que llama a `the_cubes::run()`.

```
src/
├── main.rs            Punto de entrada: llama a the_cubes::run()
├── lib.rs             Registro de plugins y sistemas, estados y escena de juego
├── blocks/            IDs y definiciones de bloques
├── item_inventory.rs  Objetos, pilas, inventario y tabla de recetas
├── furnace.rs         Simulación del horno: fundición y combustible
├── texture_layout.rs  Atlas de texturas generado por build.rs
├── rendering.rs       Mallas de chunk y coordenadas UV
├── world/
│   ├── mod.rs         Chunk
│   ├── data.rs        WorldData, acceso a bloques y punto de aparición
│   ├── terrain.rs     Generación procedural actual
│   ├── legacy.rs      Generadores antiguos para mundos ya guardados
│   └── biomes/        Reglas de cada bioma
├── streaming.rs       Carga inicial, streaming de chunks y reconstrucción de mallas
├── save.rs            Formato de guardado, migración y persistencia de chunks
├── player.rs          Movimiento, cámara y colisiones
├── mining.rs          Romper y colocar bloques, drops y partículas
├── survival.rs        Vida, hambre, comida y experiencia
├── mobs.rs            Criaturas
├── sky.rs             Ciclo día/noche, cielo, sol y luna
├── hud.rs             HUD, barra rápida y chat
├── inventory_ui.rs    Pantallas de inventario y crafteo
├── menus.rs           Menús, creación de mundo, carga y pausa
└── tests.rs           Pruebas de integración de los sistemas
```

Los módulos del juego comparten nombres mediante `use crate::*`; es un paso
intermedio que mantiene cada área en su archivo sin fijar aún las fronteras
entre ellas.


Prototipo voxel en Rust + Bevy para explorar un sandbox de bloques original.
No utiliza assets ni contenido de Minecraft.

## Ejecutar

Requiere Rust estable y las dependencias de sistema de Bevy para ventanas y
renderizado. Audio y gamepad están desactivados mientras el juego no los usa,
evitando dependencias nativas de ALSA y libudev en Linux.

```sh
cargo run
```

## Texturas de bloques

Las imágenes editables están en `assets/blocks/`, una por bloque, con dos
archivos para el césped (`grass_side.png` y `grass_top.png`). Cada PNG debe
medir exactamente 32 × 32 píxeles. Las imágenes iniciales amplían las texturas
anteriores sin añadir detalle; se pueden editar para aprovechar la resolución.

Al compilar, `build.rs` valida los archivos y genera automáticamente un atlas
de 1024 × 1024 integrado en el ejecutable; cada textura lleva alrededor un borde
que repite sus píxeles de orilla para que no se mezcle con las vecinas. Modificar un PNG requiere recompilar
el juego; no hay recarga en caliente. El manifiesto
`assets/blocks/textures.json` asocia alias con archivos y el atlas asigna las
posiciones automáticamente. Para añadir un bloque, agrega su textura al
manifiesto y referencia el alias en su definición.
El antiguo `assets/texture_atlas.png` se conserva como referencia y ya no se
utiliza para renderizar.

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

El formato de guardado actual es la versión 3 e incluye los nuevos materiales.
Para probar la generación y el inventario de minerales, crea un mundo nuevo;
los guardados anteriores pueden no ser compatibles.

## Controles en el mundo

- `W A S D`: caminar.
- `Espacio`: saltar; subir en modo vuelo.
- `Mayús izquierdo`: correr.
- `Ctrl izquierdo`: bajar en modo vuelo.
- `F`: alternar entre caminar y volar.
- Ratón: mirar.
- `Esc`: pausar; en el menú de pausa se puede continuar o guardar y volver.
- Mantener clic izquierdo: romper y recoger un bloque. Las grietas muestran
  el progreso; soltar el botón o cambiar de bloque lo reinicia.
- Clic izquierdo sobre una criatura: golpearla si está bajo la mira y delante
  del bloque.
- Clic derecho: colocar el bloque seleccionado, consumir una ración seleccionada
  o abrir el horno al que apuntas.
- `1`–`9` o rueda del ratón: seleccionar un espacio de la barra.
- `E`: abrir/cerrar el inventario y su cuadrícula 2×2.
- `R`: consumir una unidad de la comida obtenida al explorar.
- `Esc` en el inventario: cerrar el inventario.
- `C`: abrir/cerrar la mesa de crafteo; `Esc` también la cierra.

La interfaz muestra una mira, el modo de movimiento, el bloque activo y una
barra rápida de nueve espacios configurables. Los mundos nuevos combinan
continentes, colinas, cordilleras, playas, ríos, océanos y biomas con vegetación
propia; también generan cuevas, acuíferos y vetas subterráneas de carbón, cobre
y hierro. El agua se puede
atravesar y nadar con `Espacio`. El HUD indica el bioma actual. No se pueden
colocar bloques dentro del jugador.

El modo supervivencia añade 20 puntos de vida y hambre. El hambre disminuye con
el tiempo y más rápido al correr; romper hojas puede dar bayas y derrotar animales
proporciona comida. Con hambre llena la vida se regenera, y llegar a cero causa
daño. Las caídas fuertes también hacen daño. Al morir, reapareces en el inicio
con media vida y hambre, y conservas inventario y experiencia. Romper bloques
concede experiencia y sube de nivel; las barras de vida, hambre y experiencia
aparecen en el HUD.

El sistema de alimentos es variado y genera naturalmente en el terreno:
- **Ración de Bayas** (saturación 4): crafteable desde hojas y madera.
- **Manzanas** (saturación 6): generadas en hojas de árboles (15% probabilidad).
- **Trigo** (saturación 8): encontrado en pasto de llanuras (8% probabilidad).
- **Setas** (saturación 2): encontradas en pasto de bosques (5% probabilidad);
  las setas venenosas causales 2 puntos de daño además de restaurar hambre.
- **Carne Cruda** (saturación 3): drops de animales derrotados.
- **Carne Cocinada** (saturación 8): versión mejorada de la carne cruda.

Praderas y bosques pueden albergar animales pasivos que deambulan y huyen; al
derrotarlos obtienes comida y experiencia. De noche aparecen acechadores que
persiguen y atacan al jugador, y conceden experiencia al ser derrotados. La
población está limitada y las criaturas lejanas desaparecen para mantener acotado
el coste de simulación.

El inventario tiene 36 espacios, incluidos los nueve de la barra, con pilas de
hasta 99 unidades; las herramientas ocupan un espacio cada una. En la interfaz,
el clic izquierdo recoge, combina o intercambia pilas; el derecho recoge la
mitad o coloca una unidad. Pulsa el resultado para fabricar. Los objetos que
queden en el cursor o las cuadrículas se conservan al cerrar y guardar.

### Recetas

Las filas se separan con `/`; `M` es madera, `P` piedra, `H` hojas, `A` arena,
`C` lingote de cobre, `F` lingote de hierro y `.` un espacio vacío. Las recetas que caben en 2×2 se pueden hacer en el
inventario (`E`); las demás necesitan la cuadrícula 3×3 (`C`).

| Resultado | Patrón | Cantidad |
|---|---|---:|
| Pico | `PPP/.M./.M.` | 1 |
| Hacha | `PP/PM/.M` | 1 |
| Pala | `P/M/M` | 1 |
| Espada | `P/P/M` | 1 |
| Ración de bayas | `HH/HM` | 1 |
| Flechas | `HAH` | 5 |
| Puerta | `MM/MM/MM` | 3 |
| Escaleras | `M../MM./MMM` | 4 |
| Losas | `MMM` | 6 |
| Ladrillos | `PP/PP` | 4 |
| Tablones | `M` | 4 |
| Antorchas | `P/M` | 8 |
| Horno | `PPP/P.P/PPP` | 1 |

Las cuatro herramientas se fabrican también en cobre e hierro con el mismo
patrón, cambiando la piedra por lingotes (por ejemplo, pico de cobre:
`CCC/.M./.M.`). Todas las recetas están en la tabla `RECIPES` de
`src/item_inventory.rs`; añadir una entrada basta para que funcione en ambas
cuadrículas.

### Horno y metales

Coloca un horno y ábrelo con clic derecho. Tiene un espacio para el material,
otro para el combustible y uno de salida del que solo se puede retirar. Funde
cobre y hierro en bruto en lingotes; cada unidad tarda 8 segundos. El carbón
alimenta 8 fundiciones por unidad, la madera 1,5 y los tablones 0,75. El
combustible solo se enciende cuando hay algo que fundir, y el horno sigue
trabajando aunque cierres su pantalla o te alejes. Al romperlo suelta lo que
contenga. Su contenido se guarda con el mundo.

| Material | Usos (espada) | Tiempo de minería | Daño de espada |
|---|---:|---:|---:|
| Piedra | 64 (32) | dureza / 2 | 2 |
| Cobre | 128 (64) | dureza / 3 | 3 |
| Hierro | 256 (128) | dureza / 4 | 4 |

El carbón y el cobre se extraen con cualquier pico; el mineral de hierro exige
un pico de cobre o mejor.

Las flechas son objetos de inventario; todavía no hay arco ni disparo.

Las puertas cerradas bloquean el paso del jugador; abiertas permiten movimiento libre.
Escaleras y losas no son bloqueantes para colisión pero se renderizan en el mundo
(implementación parcial: renderización completa, colisión pasable). Las herramientas
de minería dan materiales y experiencia adicionales.
Selecciona la herramienta en la barra: el pico sirve para piedra, minerales y
hornos, el hacha para madera y hojas, y la pala para arena y tierra. Gastan un
uso solo al romper el bloque.
Las pilas, la selección y la durabilidad se conservan al guardar el mundo.

El ciclo día/noche dura 24 minutos reales: cada minuto real equivale a una hora
del juego. La iluminación cambia suavemente durante el amanecer, el día, el
atardecer y la noche. El cielo tiene un gradiente procedural, estrellas que se
atenúan con la luz diurna y un sol y una luna cúbicos. El reloj aparece en el
HUD y se detiene al pausar.

## Base técnica

- Chunks `16×16×16` almacenados como arreglos densos de IDs de bloque, no como
  una entidad por bloque.
- Mundo de 128 bloques de altura con chunks verticales; el streaming mantiene
  hasta tres capas verticales alrededor del jugador y descarga las lejanas.
- Mallas por chunk: omiten caras ocultas y conservan la textura y variación
  UV de cada bloque por separado.
- Los índices de malla usan 16 bits cuando caben y pasan a 32 bits solo para
  chunks excepcionalmente densos.
- Las ediciones reconstruyen los assets de malla existentes y solo marcan el
  chunk afectado y los vecinos que comparten un borde.
- El terreno es determinista por semilla; la generación, restauración de chunks
  guardados y creación de mallas avanzan progresivamente durante la pantalla de
  carga. Solo se guardan chunks modificados; el terreno no editado se regenera
  desde la semilla al explorar o cargar el mundo. Los mundos previos conservan
  su generador original; los nuevos registran la versión de terreno en sus
  metadatos.

## Rendimiento y escalabilidad

El rendimiento es un criterio de diseño del juego, también en equipos con GPU
integrada. Los sistemas que trabajan con chunks deben limitar el trabajo por
frame, evitar asignaciones y ordenamientos repetidos, y reconstruir solo los
datos que cambiaron. La generación del terreno debe seguir siendo determinista.
Antes de ampliar distancias, entidades o efectos visuales, hay que medir su
impacto en tiempo de frame, memoria y tamaño de mallas.

Para medir generación y mallas en el mismo equipo sin abrir la ventana:

```sh
cargo run --release -- --perf-bench
```

El informe muestra media, mediana, p95 y máximo de 128 iteraciones, más el
tamaño medio de los buffers de las mallas muestreadas y una estimación del
payload de vértices e índices para el límite de chunks residentes. Esa
estimación no incluye copias CPU/GPU ni sobrecarga interna del motor; los
tiempos son microbenchmarks de CPU y no incluyen renderizado, streaming ni
guardado.
