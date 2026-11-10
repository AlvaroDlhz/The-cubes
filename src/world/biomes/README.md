# Biomas

Cada bioma tiene su propia carpeta y `mod.rs`. Ahí se mantienen sus reglas de
clasificación por clima, los bloques de superficie, las plantas y sus
características propias.

## Añadir un bioma

1. Crea `src/world/biomes/<bioma>/mod.rs`.
2. Implementa `matches` usando `Climate` para definir dónde aparece.
3. Implementa `surface_block` y, cuando aplique, `plant_block` o reglas de
   generación de características.
4. Declara el módulo y conéctalo en `classify` y en las funciones de despacho
   de este directorio.
5. Añade la variante a `Biome` y actualiza sus funciones públicas en
   `world/terrain.rs`.

La generación compartida (ruido, altura, agua, cuevas y ensamblado de chunks)
permanece en `world/terrain.rs`. Las reglas que distinguen a cada bioma van en
su carpeta. Usa `TerrainBlockPalette` para que el generador también funcione
con los bloques configurados por el juego.

## Pradera

Las reglas de vegetación, claros, grupos de árboles y frecuencia de cabañas
están en `plains/mod.rs` y `world/terrain.rs`. Para comparar vegetación entre
semillas ejecuta `cargo run -- --plains-report`. La ambientación de aves
está en `assets/audio/plains_ambience.wav`; se activa con
`cargo run --features plains-audio` en sistemas con las bibliotecas de
desarrollo de ALSA instaladas.
