# Hoja de ruta de bloques, objetos y progresión

## Alcance

Minecraft Java 26.3 registra más de mil bloques y objetos; ese conteo incluye
variantes y registros técnicos. No conviene copiarlos todos de una vez: The
Cubes debe crecer por sistemas jugables y mantener una identidad propia. Cada
etapa debe añadir contenido que funcione de extremo a extremo: generación u
obtención, inventario, uso/crafteo, guardado y presentación.

## Inventario actual de The Cubes

- **28 tipos de bloque distintos de aire:** hierba, tierra, piedra, arena,
  madera, hojas, manzana, trigo, hongo, puerta, escaleras, losa, ladrillos,
  tablones, antorcha, agua, lava, hielo, nieve, obsidiana, barro, grava, flor
  roja, flor amarilla, hierba alta, mineral de carbón, mineral de cobre y
  mineral de hierro.
- **35 identidades de objeto aproximadas:** 26 bloques que pueden ir al
  inventario (no incluye agua ni lava), cuatro herramientas, ración de bayas,
  flecha y tres materiales minerales. Las durabilidades no cuentan como tipos
  distintos.
- Ya existen generación por biomas, cuevas, supervivencia, crafteo básico,
  guardado de mundos y dos tipos de criaturas.

## Etapas propuestas

### 0. Base de contenido — en curso

- Definiciones comunes para bloques y objetos.
- Texturas por alias en `assets/blocks/textures.json`, con atlas generado al
  compilar.
- IDs estables y migraciones de guardado al cambiar estructuras de datos.
- Recetas y drops expresados como datos, no dispersos en varios `match`
  (hecho para recetas y drops de bloques).

### 1. Minería y progresión de materiales — en curso

- Minerales subterráneos: carbón, cobre, hierro, oro, lapislázuli, redstone,
  diamante y esmeralda.
- Recursos diferenciados: mineral en bruto, lingote y gemas/polvo donde tengan
  una función real.
- Horno con combustible, cocción, fundición y progreso visible.
- Herramientas de madera, piedra, cobre, hierro, oro y diamante; niveles de
  herramienta requeridos y velocidad por material.
- Distribución por profundidad y bioma, rareza y drops coherentes.

**Ya implementado:** vetas deterministas de carbón, cobre y hierro bajo tierra;
se recogen como objetos apilables, y los tres minerales exigen un pico. Las
texturas tienen alias y PNG propios. La versión del terreno aumenta para que
los mundos nuevos reciban esta generación.

También implementado: horno con combustible (carbón, madera, tablones) que
funde cobre y hierro en lingotes, con contenido persistente; herramientas de
cobre e hierro con más durabilidad, velocidad y daño; el mineral de hierro
exige un pico de cobre. Las recetas son una tabla de datos (`RECIPES`).

**Siguiente entrega:** cocinar comida en el horno (requiere que la carne sea un
objeto de inventario) y añadir más minerales cuando tengan un uso dentro de
esta progresión.

### 2. Cultivo y cocina

- Semillas, tierra labrada, hidratación, etapas de crecimiento y cosecha.
- Cultivos adicionales y alimentos con diferencias útiles, no solo cosméticas.
- Horno y fogata para cocinar; recetas y conservación de comida.

### 3. Construcción y almacenamiento

- Cofres y barriles con inventario persistente.
- Vallas, puertas de valla, trampillas, escaleras y losas con colisiones y
  orientación correctas.
- Familias visuales de madera, piedra y ladrillo; iluminación decorativa y
  bloques funcionales.

### 4. Exploración y biomas

- Estructuras con botín y motivos para visitar cada bioma.
- Cuevas más variadas, vetas, lagos subterráneos y puntos de interés.
- Flora, bloques y recursos exclusivos de zonas concretas.
- Mapas o herramientas de orientación para expediciones largas.

### 5. Criaturas, equipo y combate

- Más animales con drops y comportamiento propios.
- Enemigos con roles distintos, ataques legibles y botín equilibrado.
- Armadura, escudo, arco y proyectiles; mejorar combate, animaciones y sonido.

### 6. Automatización

- Interruptores, placas, cables/señales y mecanismos sencillos.
- Máquinas o bloques que resuelvan tareas repetitivas.
- Implementarlo solo después de que construcción y estados de bloque sean
  sólidos.

### 7. Objetivo de largo plazo

- Una meta de supervivencia propia: región final, portal o expedición especial,
  jefe y recompensas que cambien cómo se juega.
- No replicar dimensiones de Minecraft sin una función clara para el juego.

### 8. Calidad y publicación

- Opciones gráficas y de controles, accesibilidad, audio y tutorial inicial.
- Compatibilidad de guardados, rendimiento, pruebas de regresión y manejo de
  errores.
- Multijugador como proyecto separado: sincronización, autoridad del servidor,
  persistencia y seguridad antes de abrirlo a partidas en red.
- Documentar el formato de contenido para que añadir bloques no requiera tocar
  muchos sistemas a la vez.

## Regla de avance

Avanzar una etapa pequeña cada vez. Una familia de contenido se considera lista
cuando se puede encontrar, recoger, almacenar, usar/craftear, guardar y volver a
cargar sin perder datos; además debe tener una función clara en la progresión.
