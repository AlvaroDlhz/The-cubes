use crate::world::terrain::TerrainBlockPalette;

pub(super) fn surface_block(_depth: i32, blocks: TerrainBlockPalette) -> u8 {
    blocks.stone
}
