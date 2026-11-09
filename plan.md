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

### 4. 🍖 Sistema de hambre refinado con variedad de comida
**Estado**: ⏳ PLANIFICADO
**Complejidad**: Media | **Estimado**: 1-2 horas

**Descripción**:
- **Carne**: drops de mobs, saturation media
- **Bayas**: de hojas, saturation baja
- **Manzanas**: de árboles especiales, saturation media
- **Trigo/Pan**: crafteable, saturation alta
- **Setas**: encontradas en bosques, algunas venenosas
- Cocinar en fogata → mejor valor nutricional
- Sistema de efectos negativos (envenenamiento)

**Requisitos previos**:
- Biomas (✅ ya existen)
- Mobs y drops (✅ ya existe)

**Cambios de código necesarios**:
- `Survival` → añadir `last_food_type` para diferenciar saturations
- Nuevos bloques: `APPLE`, `WHEAT`, `MUSHROOM`
- Generador de terreno → incluir nuevos bloques
- `eat_food()` → parámetro de tipo comida con diferente saturación
- Efectos de poción: Duration + tipo

**Pruebas a crear**:
- Diferentes comidas tienen saturación correcta
- Alimentos cocinados mejoran valor
- Veneno reduce vida (integración con `apply_damage`)

---

### 5. 🏠 Construcción de estructuras y refugios
**Estado**: ⏳ PLANIFICADO
**Complejidad**: Alta | **Estimado**: 3-4 horas

**Descripción**:
- **Puertas**: bloques que se abren/cierran con interact
- **Escaleras**: bloques de media altura, permite movimiento vertical diagonal
- **Slabs**: mitad de altura, dos pueden formar un bloque normal
- **Ladrillo**: crafteable desde arcilla
- **Tablones**: procesamiento de madera

**Requisitos previos**:
- Sistema de bloques (✅ ya existe)
- Modelo de voxel (✅ ya existe)

**Cambios de código necesarios**:
- Enum `Block` → aumentar de 7 a ~20 tipos
- Generador de malla → soporte para bloques parciales
- Sistema de estado de puerta (abierta/cerrada)
- Colisiones → soporte para bloques parciales
- Crafteo → nuevas recetas

**Pruebas a crear**:
- Escaleras permiten subida correcta
- Slabs se pueden apilar
- Puertas bloquenan a jugador en estado cerrado
- Colisiones funcionan en media altura

---

### 6. 💡 Sistema de luz con antorchas y linterna
**Estado**: ⏳ PLANIFICADO
**Complejidad**: Alta | **Estimado**: 2-3 horas

**Descripción**:
- **Antorcha**: bloque luminoso crafteable
- **Linterna portátil**: item que lleva el jugador (tipo gafas)
- **Niebla de distancia**: mayor en la noche, menor con luz
- **Mobs hostiles spawean en zonas oscuras**: <8 luz
- Sistema simple de propagación de luz por chunk

**Requisitos previos**:
- Ciclo día/noche (✅ ya existe)
- Mobs (✅ ya existe)

**Cambios de código necesarios**:
- `Chunk` → array de luz (0-15 por voxel)
- Función `light_level_at()` para raycast de luz
- Material de malla → shader con luz local
- Spawn de mobs → verificar `light_level_at() < 8`
- HUD → mostrar portador de linterna

**Pruebas a crear**:
- Antorchas iluminan radio correcto
- Mobs no spawean cerca de antorchas
- Niebla aumenta en zonas oscuras

---

### 6. 🌍 Biomas adicionales y variados
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
