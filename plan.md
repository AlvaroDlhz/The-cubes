# Plan de desarrollo: The Cubes

## Visión general
Completar The Cubes con 10 features principales para transformarlo de un sandbox minimalista a un juego de supervivencia/construcción completo con progresión, variedad de contenido y dirección de juego.

---

## Features principales

### 1. ✅ Mobs y combate básico
**Estado**: ✅ COMPLETADO (Sprint actual)
- [x] Animal pasivo (Grazer) que deambula y huye
- [x] Criatura hostil nocturna (NightStalker)
- [x] Sistema de daño a mobs con raycast
- [x] Drops de comida y experiencia

**Próximas mejoras**: Más variedad de mobs (Oveja, Vaca, Pollo, Araña)

---

### 2. ✅ Más tipos de herramientas especializadas
**Estado**: ✅ COMPLETADO
**Complejidad**: Media | **Estimado**: 1-2 horas (Actual: 1 hora)

**Implementación completada**:
- ✅ Enums: `CraftRecipe::WoodenShovel`, `CraftRecipe::WoodenSword`, `CraftRecipe::Arrows`
- ✅ `CraftedTools` estructura: `shovel_durability`, `sword_durability`
- ✅ `tool_durability_for_block()`: lógica especial para pala (2x eficiencia)
- ✅ Combate: espada hace 2 puntos de daño y consume durabilidad
- ✅ Crafteo: recetas implementadas en `craft_recipe()`
- ✅ UI de crafteo: 5 botones nuevos en mesa de crafteo
- ✅ Persistencia: guardado/carga de durabilidades nuevas
- ✅ Pruebas: 2 tests nuevos + validación de 37 tests totales

**Requisitos previos**:
- Sistema de crafteo (✅ ya existe)
- Inventario (✅ ya existe)

**Cambios de código realizados**:
- `CraftRecipe` enum → 4 nuevos tipos
- `CraftedTools` struct → 2 nuevos campos
- `tool_durability_for_block()` → lógica de 2x para pala
- `edit_blocks()` → daño variable según espada
- `WorldSave` struct → 2 nuevos campos de durabilidad
- `snapshot_world()` → serialización de nuevas herramientas
- `craft_recipe()` → 4 nuevos casos
- UI crafteo → 5 botones + descripción actualizada
- Tests → validación de efficiencia y durabilidad

**Pruebas creadas**:
- ✅ `shovel_is_twice_as_efficient_on_sand_and_dirt`
- ✅ `sword_consumes_durability_on_mob_hits`

---

### 3. 🎮 Interfaz de usuario tipo Minecraft (Inventario mejorado)
**Estado**: ✅ COMPLETADO
**Complejidad**: Media | **Estimado**: 2-3 horas (Actual: ~1 hora)

**Implementación completada**:
- ✅ Panel de inventario completo estilo Minecraft (880×706px)
- ✅ Área de crafteo 2×2 con visualización de resultado
- ✅ Grid de inventario mostrando todos los bloques con cantidades
- ✅ Abre/cierra con tecla E (compatible con comer)
- ✅ Se cierra con ESC
- ✅ Cursor visible cuando inventario está abierto
- ✅ Estilo visual oscuro similar a Minecraft
- ✅ Toda la functionality anterior se mantiene (retrocompatibilidad 100%)

**Requisitos previos**:
- Sistema de UI Bevy (✅ ya existe)
- Inventario (✅ ya existe)
- Sistema de crafteo (✅ ya existe)

**Cambios de código realizados**:
- Componentes UI nuevos: `InventoryPanel`, `InventoryCraftingGrid`, `InventoryGrid`, `CraftingGridSlot`, `InventoryCraftResult`
- `CraftingUi` resource → agregados `inventory_open`, `inventory_just_opened` flags
- `consume_food()` → modificado para alternar inventario con E
- Nueva función `toggle_inventory_visibility()` → sistema que maneja visibilidad
- Nueva función `block_type_name()` → helper para convertir ID de bloque a nombre
- `setup_inventory_ui()` → constructor completo del panel
- `setup_game_scene()` → integración de setup_inventory_ui

**Pruebas validadas**:
- ✅ Todos 37 tests existentes pasan
- ✅ Compilación limpia sin errores
- ✅ E key abre/cierra inventario correctamente
- ✅ Inventario muestra bloques y cantidades
- ✅ UI renderiza correctamente en pantalla
- ✅ Cursor grab/release funciona

**Próximas mejoras** (cuando se implemente):
- Drag-and-drop entre slots
- Click derecho para comer/usar items
- Sistema de recipes visuales
- Animación de apertura/cierre
- Barra de búsqueda/filtros

---

### 4. ✅ Sistema de hambre refinado con variedad de comida
**Estado**: ✅ COMPLETADO
**Complejidad**: Media | **Estimado**: 1-2 horas (Actual: ~2 horas)

**Implementación completada**:
- ✅ Nuevos bloques: `APPLE` (7), `WHEAT` (8), `MUSHROOM` (9)
- ✅ `FoodType` enum con 6 variantes:
  - `BerryRation`: saturation 4 (comida base)
  - `Apple`: saturation 6 (de árboles)
  - `Wheat`: saturation 8 (de campos)
  - `Mushroom`: saturation 2 (de bosques, puede ser venenoso)
  - `RawMeat`: saturation 3 (de mobs)
  - `CookedMeat`: saturation 8 (versión cocinada)
- ✅ Método `saturation()` en `FoodType` → devuelve valor diferente por tipo
- ✅ Método `is_poisonous()` en `FoodType` → genera efecto de daño para setas
- ✅ Función `eat_food_of_type()` → aplica saturación variable y efectos
- ✅ Generación de terreno:
  - Manzanas: 15% probabilidad en hojas de árbol
  - Trigo: 8% probabilidad en pasto de llanuras
  - Setas: 5% probabilidad en pasto de bosques
- ✅ Sistema de envenenamiento → setas venenosas reducen vida en 2 puntos
- ✅ Retrocompatibilidad: `LEGACY_BLOCK_COUNT = 4`, `BLOCK_COUNT = 10`

**Requisitos previos**:
- Biomas (✅ ya existen)
- Mobs y drops (✅ ya existe)

**Cambios de código realizados**:
- `FoodType` enum (6 variantes) con métodos `saturation()` e `is_poisonous()`
- `block_type_name()` → soporte para Manzana, Trigo, Hongo
- `eat_food()` → refactorizado a usar `eat_food_of_type(FoodType)`
- `eat_food_of_type()` → nueva función con saturación variable y efectos
- `generate_chunk()` → 3 nuevos bucles de generación de alimentos
- Constantes de bloque: `APPLE = 7`, `WHEAT = 8`, `MUSHROOM = 9`

**Pruebas creadas** (4 nuevos tests, todos pasando):
- ✅ `different_food_types_have_correct_saturation` → valida FoodType.saturation()
- ✅ `poisonous_mushrooms_cause_damage` → confirma daño de setas venenosas
- ✅ `cooked_meat_provides_more_saturation_than_raw_meat` → valida cooking multiplier
- ✅ `eating_different_foods_increases_hunger_differently` → prueba saturación diferencial

**Tests validados**:
- ✅ Compilación limpia sin errores
- ✅ Todos 41 tests pasando (37 existentes + 4 nuevos)
- ✅ Test `eating_restores_hunger_and_consumes_food` actualizado para BerryRation (saturation 4)

---

### 5. ✅ Construcción de estructuras y refugios
**Estado**: ✅ COMPLETADO
**Complejidad**: Alta | **Estimado**: 3-4 horas (Actual: ~2 horas)

**Implementación completada**:
- ✅ Nuevos bloques: `WOODEN_DOOR` (10), `STAIRS` (11), `SLAB` (12), `BRICKS` (13), `PLANKS` (14)
- ✅ `DoorState` enum con estados Open/Closed
- ✅ Sistema de interacción de puertas (click derecho alterna abierto/cerrado)
- ✅ Función `is_block_solid()` que considera:
  - Puertas abiertas = no bloqueantes (paso libre)
  - Puertas cerradas = bloqueantes (colisión)
  - Escaleras y losas = no bloqueantes (renderización parcial)
- ✅ Actualización de colisiones: `collides_with_world()` usa `is_block_solid()` en lugar de comparación directa
- ✅ Estado persistente: HashMap `door_states` en `WorldData` para guardar estado de puertas
- ✅ Recetas de crafteo para 5 nuevos bloques:
  - Puertas: 6 madera → 3 puertas
  - Escaleras: 6 madera → 4 escaleras
  - Losas: 3 madera → 6 losas
  - Ladrillos: 4 piedra → 4 ladrillos
  - Tablones: 1 madera → 4 tablones (2x eficiencia material)
- ✅ UI: 5 botones nuevos en mesa de crafteo con descripción de features

**Requisitos previos**:
- Sistema de bloques (✅ ya existe)
- Modelo de voxel (✅ ya existe)

**Cambios de código realizados**:
- Constantes de bloque: `WOODEN_DOOR = 10`, `STAIRS = 11`, `SLAB = 12`, `BRICKS = 13`, `PLANKS = 14`
- Enums: `DoorState`, `StairRotation`, `StairHalf` (para futuras mejoras)
- `WorldData` struct → nuevo campo `door_states: HashMap<IVec3, DoorState>`
- `block_type_name()` → soporte para 5 nuevos tipos de bloque
- `craft_recipe()` → 5 nuevos casos de receta con yields correctos
- `Inventory.add()` → acepta todos los nuevos bloques
- Función `is_block_solid()` → lógica de colisión con estados de puerta
- `collides_with_world()` → usa `is_block_solid()` en lugar de `AIR` comparison
- `edit_blocks()` → interacción con puertas (toggle estado en right-click)
- UI de crafteo → 5 botones nuevos (Puertas, Escaleras, Losas, Ladrillos, Tablones)

**Pruebas creadas** (3 nuevos tests, todos pasando):
- ✅ `doors_block_player_collision_when_closed_but_allow_passage_when_open` → valida colisión y paso
- ✅ `stairs_and_slabs_do_not_block_collision_but_render` → confirma bloques parciales
- ✅ `crafting_structures_creates_expected_quantities` → valida yields de todas las recetas

**Tests validados**:
- ✅ Compilación limpia sin errores (9 warnings de dead code)
- ✅ Todos 44 tests pasando (41 previos + 3 nuevos)

**Próximas mejoras** (cuando se implementen):
- Rotación de escaleras (4 direcciones diferentes)
- Media altura correcta para escaleras y slabs (renderización parcial, ya funciona colisión)
- Puertas dobles
- Trapdoors (compuertas)
- Fences (vallas)
- Más tipos de madera (roble, abeto, abedul)

### 6. ✅ Sistema de luz con antorchas
**Estado**: ✅ COMPLETADO
**Complejidad**: Media | **Estimado**: 2-3 horas (Actual: ~1.5 horas)

**Implementación completada**:
- ✅ Nuevo bloque: `TORCH` (15)
- ✅ Receta de crafteo: 1 madera + 1 piedra → 8 antorchas
- ✅ Función `light_level_at()` que calcula:
  - Luz solar base: 15.0 encima de la superficie, decrece 0.3 por bloque de profundidad, mínimo 0.5
  - Luz de antorchas: radio 8 bloques, falloff basado en distancia, máximo 8.0 por antorcha
- ✅ Sistema de iluminación que permite ajustes futuros para fog y rendering
- ✅ Interfaz de crafteo: botón para antorchas en mesa de crafteo

**Requisitos previos**:
- Sistema de crafteo (✅ ya existe)
- Bloques y terreno (✅ ya existe)

**Cambios de código realizados**:
- Constante de bloque: `TORCH = 15`
- Enum `CraftRecipe` → nuevo caso `Torch`
- Función `light_level_at()` → cálculo de nivel de luz en posición
- `craft_recipe()` → caso Torch con receta 1 madera + 1 piedra → 8 antorchas
- `block_type_name()` → soporte para TORCH
- `Inventory.add()` → acepta TORCH
- UI de crafteo → botón para antorchas con descripción

**Pruebas validadas**:
- ✅ Compilación limpia sin errores (10 warnings de dead code)
- ✅ Todos 44 tests pasando

**Próximas mejoras** (cuando se implementen):
- Linterna portátil: item que lleva el jugador
- Niebla de distancia: mayor en la noche, menor con luz
- Shaders: iluminación dinámica en mallas de voxel
- Mobs hostiles: limitar spawn en zonas con luz (< 8)
- Propagación de luz mejorada: iluminación en tiempo real

---

### 7. 🌍 Biomas adicionales y variados
**Estado**: ⏳ PLANIFICADO
**Complejidad**: Alta | **Estimado**: 4-6 horas

**Descripción**:
- **Cueva interior**: Dentro de montañas, oscuridad, minerales
- **Playa rocosa**: Junto a costa, arena + rocas
- **Tundra/Nieve**: Clima frío, bloques de hielo, árboles escasos
- **Volcán**: Bioma peligroso, lava, obsidiana, fuego
- **Pantano**: Agua estancada, lodo, plantas venenosas
- Cada bioma con flora/fauna única

**Requisitos previos**:
- Generador de terreno (✅ ya existe)
- Biomas base (✅ ya existe)
- Mobs (para fauna unique)

**Cambios de código necesarios**:
- `Biome` enum → agregar nuevos tipos
- `biome_at()` → logic de ruido para nuevos biomas
- `generate_chunk()` → soporte para nuevos bloques
- `generate_trees()` → variaciones por bioma
- Spawn de mobs → por bioma compatible

**Pruebas a crear**:
- Nuevos biomas generan en semillas correctas
- Transiciones suaves entre biomas
- Flora y fauna son correctas por bioma

---

### 7. 👹 Más variedad de mobs y comportamientos
**Estado**: ⏳ PLANIFICADO
**Complejidad**: Media-Alta | **Estimado**: 3-4 horas

**Descripción**:
- **Oveja**: Pasiva, da lana al cortarla
- **Vaca**: Pasiva, da leche (item), da carne
- **Pollo**: Pasiva/vuela, pone huevos, da carne
- **Zombi**: Hostil lento, spawea de noche, débil
- **Araña**: Hostil rápida, trepa paredes, solo de noche
- **Pasivos agresivos**: Abejas (atacan si dañadas)

**Requisitos previos**:
- Sistema de mobs (✅ ya existe)
- Sistema de comida (feature 3)
- Biomas (✅ ya existe)

**Cambios de código necesarios**:
- `MobKind` enum → agregar nuevos tipos
- `Mob` component → diferenciar lógica por tipo
- `update_mobs()` → AI específica por mob
- Texturas/materiales → nuevos modelos
- Drops → comida específica por mob

**Pruebas a crear**:
- Cada mob tiene movimiento correcto
- Drops son correctos
- Spawning respeta bioma

---

### 8. 📦 Sistema de almacenamiento y cajas
**Estado**: ⏳ PLANIFICADO
**Complejidad**: Alta | **Estimado**: 2-3 horas

**Descripción**:
- **Cajas crafteable**: 16 slots de almacenamiento
- **Interfaz visual**: Grid 4x4 mostrado en HUD
- **Interfaz de transferencia rápida**: arrastrar entre inventarios
- **Persistencia**: cajas guardan contenido al guardar mundo
- **Dobles cajas**: dos cajas juntas = 32 slots

**Requisitos previos**:
- Sistema de inventario (✅ ya existe)
- Guardado de mundo (✅ ya existe)

**Cambios de código necesarios**:
- `BlockEntity` component → cajas tienen inventario
- `WorldData.block_inventories` → HashMap<IVec3, Inventory>
- UI → nueva interfaz de cajas
- `snapshot_world()` → serializar cajas
- `load_or_generate_chunk()` → deserializar cajas

**Pruebas a crear**:
- Items se pueden mover entre inventarios
- Cajas persisten tras guardar/cargar
- Límite de stack respetado

---

### 9. 🎯 Objetivos y progresión con hitos
**Estado**: ⏳ PLANIFICADO
**Complejidad**: Media | **Estimado**: 1-2 horas

**Descripción**:
- **Libro de objetivos**: 15-20 objetivos
  - "Obtén 10 bloques de piedra"
  - "Derrota 5 mobs hostiles"
  - "Crea una herramienta"
  - "Alcanza nivel 5"
- **Objetivos progresivos**: desbloquean nuevas recetas
- **Estadísticas**: bloques rotos, mobs derrotados, distancia, tiempo jugado
- **Progresión de herramientas**: Madera → Piedra → Hierro → Diamante

**Requisitos previos**:
- Sistema de experiencia (✅ ya existe)
- Mobs (✅ ya existe)

**Cambios de código necesarios**:
- `Quest` struct + `QuestProgress`
- `Survival` → tracking de estadísticas
- HUD → mostrar objetivos activos
- `snapshot_world()` → guardar progreso de quests

**Pruebas a crear**:
- Objetivos se completan correctamente
- Nuevas recetas se desbloquean
- Estadísticas se registran

---

### 10. 🗺️ Mapamundi y sistema de waypoints
**Estado**: ⏳ PLANIFICADO
**Complejidad**: Media | **Estimado**: 2-3 horas

**Descripción**:
- **Mini-mapa en HUD**: muestra chunks cargados + jugador
- **Waypoints/Marcadores**: coloca banderas en el mapa
- **Brújula**: item que apunta al punto de inicio
- **Exploración visual**: mapa se revela al explorar
- **Nombres de biomas**: mostrados en mini-mapa

**Requisitos previos**:
- HUD system (✅ ya existe)
- Biomas (✅ ya existe)

**Cambios de código necesarios**:
- `MiniMap` component → UI para mapa
- `Waypoint` component → entities en el mundo
- Shader → renderizado del mini-mapa
- `snapshot_world()` → guardar waypoints

**Pruebas a crear**:
- Mini-mapa muestra posición correcta
- Waypoints aparecen en el mapa
- Brújula apunta al spawn

---

## Roadmap recomendado

### Phase 1: Enriquecimiento de combate (2-3 sprints)
1. **Sprint 1**: Herramientas especializadas (Pala, Espada, Arco)
2. **Sprint 2**: Más mobs (Oveja, Vaca, Pollo, Zombi, Araña)
3. **Sprint 3**: Comida variada + efectos

### Phase 2: Exploración y atmósfera (2-3 sprints)
4. **Sprint 4**: Sistema de luz + antorchas
5. **Sprint 5**: Nuevos biomas (Cueva, Tundra, Volcán, Pantano)
6. **Sprint 6**: Biomas especiales (Playa, estructuras naturales)

### Phase 3: Construcción y almacenamiento (1-2 sprints)
7. **Sprint 7**: Bloques especiales (Puertas, Escaleras, Slabs)
8. **Sprint 8**: Sistema de cajas y almacenamiento

### Phase 4: Dirección de juego (1 sprint)
9. **Sprint 9**: Objetivos + Progresión + Estadísticas
10. **Sprint 10**: Mapamundi + Waypoints

---

## Estadísticas del proyecto

| Métrica | Valor |
|---------|-------|
| **Líneas de código** | ~5,200 |
| **Pruebas unitarias** | 35 |
| **Bloques únicos** | 7 (GRASS, DIRT, STONE, SAND, WOOD, LEAVES, AIR) |
| **Biomas** | 4 (Plains, Forest, Desert, Mountain) |
| **Mobs** | 2 (Grazer, NightStalker) |
| **Herramientas** | 2 (Pickaxe, Axe) |
| **Recetas de crafteo** | 3 |

---

## Notas y consideraciones

### Rendimiento
- **Chunks activos**: 49 horizontal + streaming
- **Máximo de mobs**: 8 (5 pasivos, 3 hostiles)
- **Límite de almacenamiento**: 4,096 chunks modificados
- Considerar LOD (Level of Detail) cuando tengamos muchos biomas

### Compatibilidad de guardados
- **SAVE_VERSION = 1**: Todos los guardados nuevos deben ser compatibles hacia atrás
- Considerar migración de datos si cambiamos bloques/items
- Los mobs NO se persisten (regeneran al cargar)

### Arquitectura ECS
- **Sin conflictos de consultas**: mobs + edit_blocks ya son compatibles
- **Patrón de sistema**: (capture, logic, apply) en cadena
- Evitar recursos mutables en paralelo

### Próximas herramientas de desarrollo
- **Editor de bloques**: visualizador de chunks para debug
- **Exportador de mapas**: PNG del mundo generado
- **Benchmarker**: FPS en diferentes distancias de carga

---

## Cómo contribuir

1. Elige una feature del roadmap
2. Copia la plantilla de sección
3. Marca como `🔄 EN PROGRESO` al comenzar
4. Abre un PR con cambios aislados
5. Ejecuta tests: `cargo test`
6. Marca como ✅ al completar

---

**Última actualización**: 2026-11-09
**Autor**: Alvaro + Copilot
