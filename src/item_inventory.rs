use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

use crate::blocks::MineralKind;
use crate::{
    AIR, BLOCK_COUNT, BRICKS, DIRT, LAVA, LEAVES, PLANKS, SAND, SLAB, STAIRS, STONE, TORCH, WATER,
    WOOD, WOODEN_DOOR,
};

pub const HOTBAR_SIZE: usize = 9;
pub const INVENTORY_SIZE: usize = 36;
pub const STACK_LIMIT: u32 = 99;

#[derive(Clone, Copy, Debug)]
pub struct ItemDefinition {
    pub name: &'static str,
    pub max_stack: u32,
    pub placeable_block: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolKind {
    Pickaxe,
    Axe,
    Shovel,
    Sword,
}

impl ToolKind {
    pub fn max_durability(self) -> u32 {
        if self == Self::Sword {
            32
        } else {
            64
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Item {
    Block(u8),
    Mineral(MineralKind),
    Tool { kind: ToolKind, durability: u32 },
    BerryRation,
    Arrow,
}

impl Item {
    pub fn definition(self) -> ItemDefinition {
        match self {
            Self::Block(block) => ItemDefinition {
                name: crate::blocks::block_definition(block).name_es,
                max_stack: STACK_LIMIT,
                placeable_block: Some(block),
            },
            Self::Mineral(kind) => ItemDefinition {
                name: match kind {
                    MineralKind::Coal => "Carbón",
                    MineralKind::RawCopper => "Cobre en bruto",
                    MineralKind::RawIron => "Hierro en bruto",
                },
                max_stack: STACK_LIMIT,
                placeable_block: None,
            },
            Self::Tool { kind, .. } => ItemDefinition {
                name: match kind {
                    ToolKind::Pickaxe => "Pico",
                    ToolKind::Axe => "Hacha",
                    ToolKind::Shovel => "Pala",
                    ToolKind::Sword => "Espada",
                },
                max_stack: 1,
                placeable_block: None,
            },
            Self::BerryRation => ItemDefinition {
                name: "Ración",
                max_stack: STACK_LIMIT,
                placeable_block: None,
            },
            Self::Arrow => ItemDefinition {
                name: "Flecha",
                max_stack: STACK_LIMIT,
                placeable_block: None,
            },
        }
    }

    pub fn stack_limit(self) -> u32 {
        self.definition().max_stack
    }

    pub fn is_valid(self) -> bool {
        match self {
            Self::Block(block) => {
                block > AIR && block < BLOCK_COUNT as u8 && block != WATER && block != LAVA
            }
            Self::Tool { kind, durability } => {
                durability > 0 && durability <= kind.max_durability()
            }
            Self::Mineral(_) | Self::BerryRation | Self::Arrow => true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStack {
    pub item: Item,
    pub count: u32,
}

impl ItemStack {
    pub fn is_valid(self) -> bool {
        self.item.is_valid() && self.count > 0 && self.count <= self.item.stack_limit()
    }
}

#[derive(Clone, Debug, Resource, Serialize, Deserialize)]
pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>,
    pub cursor: Option<ItemStack>,
    pub small_grid: [Option<ItemStack>; 4],
    pub large_grid: [Option<ItemStack>; 9],
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            slots: vec![None; INVENTORY_SIZE],
            cursor: None,
            small_grid: [None; 4],
            large_grid: [None; 9],
        }
    }
}

impl Inventory {
    pub fn is_valid(&self) -> bool {
        self.slots.len() == INVENTORY_SIZE
            && self
                .slots
                .iter()
                .chain(self.small_grid.iter())
                .chain(self.large_grid.iter())
                .chain(std::iter::once(&self.cursor))
                .all(|slot| slot.is_none_or(ItemStack::is_valid))
    }

    pub fn count(&self, block: u8) -> u32 {
        self.slots
            .iter()
            .flatten()
            .filter(|stack| stack.item == Item::Block(block))
            .map(|stack| stack.count)
            .sum()
    }

    pub fn try_add(&mut self, item: Item, count: u32) -> Result<(), &'static str> {
        if !item.is_valid() || count == 0 {
            return Err("Objeto o cantidad no válidos.");
        }
        let capacity: u64 = self
            .slots
            .iter()
            .map(|slot| match slot {
                None => u64::from(item.stack_limit()),
                Some(stack) if stack.item == item => u64::from(item.stack_limit() - stack.count),
                _ => 0,
            })
            .sum();
        if capacity < u64::from(count) {
            return Err("El inventario está lleno.");
        }
        let mut remaining = count;
        for stack in self
            .slots
            .iter_mut()
            .flatten()
            .filter(|stack| stack.item == item)
        {
            let added = remaining.min(item.stack_limit() - stack.count);
            stack.count += added;
            remaining -= added;
        }
        for slot in &mut self.slots {
            if remaining == 0 {
                break;
            }
            if slot.is_none() {
                let added = remaining.min(item.stack_limit());
                *slot = Some(ItemStack { item, count: added });
                remaining -= added;
            }
        }
        Ok(())
    }

    pub fn add(&mut self, block: u8) -> Result<(), u8> {
        self.try_add(Item::Block(block), 1).map_err(|_| block)
    }

    pub fn remove(&mut self, block: u8) -> bool {
        if let Some(slot) = self
            .slots
            .iter_mut()
            .find(|slot| slot.is_some_and(|s| s.item == Item::Block(block)))
        {
            take_one(slot);
            true
        } else {
            false
        }
    }
}

pub fn take_one(slot: &mut Option<ItemStack>) {
    if let Some(stack) = slot {
        stack.count -= 1;
        if stack.count == 0 {
            *slot = None;
        }
    }
}

pub fn click_slot(slot: &mut Option<ItemStack>, cursor: &mut Option<ItemStack>, right: bool) {
    match (slot.as_mut(), cursor.as_mut(), right) {
        (Some(stack), None, true) => {
            let taken = stack.count.div_ceil(2);
            *cursor = Some(ItemStack {
                item: stack.item,
                count: taken,
            });
            stack.count -= taken;
            if stack.count == 0 {
                *slot = None;
            }
        }
        (None, Some(held), true) => {
            *slot = Some(ItemStack {
                item: held.item,
                count: 1,
            });
            held.count -= 1;
            if held.count == 0 {
                *cursor = None;
            }
        }
        (Some(stack), Some(held), true)
            if stack.item == held.item && stack.count < stack.item.stack_limit() =>
        {
            stack.count += 1;
            held.count -= 1;
            if held.count == 0 {
                *cursor = None;
            }
        }
        (Some(stack), Some(held), false) if stack.item == held.item => {
            let moved = held.count.min(stack.item.stack_limit() - stack.count);
            stack.count += moved;
            held.count -= moved;
            if held.count == 0 {
                *cursor = None;
            }
        }
        (_, _, false) => std::mem::swap(slot, cursor),
        _ => {}
    }
}

pub fn recipe(grid: &[Option<ItemStack>], width: usize) -> Option<ItemStack> {
    if width == 0 || grid.len() != width * width {
        return None;
    }
    let occupied: Vec<_> = grid
        .iter()
        .enumerate()
        .filter(|(_, slot)| slot.is_some())
        .collect();
    let min_x = occupied.iter().map(|(i, _)| i % width).min()?;
    let max_x = occupied.iter().map(|(i, _)| i % width).max()?;
    let min_y = occupied.iter().map(|(i, _)| i / width).min()?;
    let max_y = occupied.iter().map(|(i, _)| i / width).max()?;
    let shape_width = max_x - min_x + 1;
    let shape_height = max_y - min_y + 1;
    let mut shape = Vec::new();
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            shape.push(match grid[y * width + x] {
                None => AIR,
                Some(ItemStack {
                    item: Item::Block(block),
                    ..
                }) => block,
                _ => return None,
            });
        }
    }
    let tool = |kind: ToolKind| Item::Tool {
        kind,
        durability: kind.max_durability(),
    };
    let (item, count) = match (shape_width, shape_height, shape.as_slice()) {
        (3, 3, [STONE, STONE, STONE, AIR, WOOD, AIR, AIR, WOOD, AIR]) => {
            (tool(ToolKind::Pickaxe), 1)
        }
        (2, 3, [STONE, STONE, STONE, WOOD, AIR, WOOD]) => (tool(ToolKind::Axe), 1),
        (1, 3, [STONE, WOOD, WOOD]) => (tool(ToolKind::Shovel), 1),
        (1, 3, [STONE, STONE, WOOD]) => (tool(ToolKind::Sword), 1),
        (2, 2, [LEAVES, LEAVES, LEAVES, WOOD]) => (Item::BerryRation, 1),
        (3, 1, [LEAVES, SAND, LEAVES]) => (Item::Arrow, 5),
        (2, 3, [WOOD, WOOD, WOOD, WOOD, WOOD, WOOD]) => (Item::Block(WOODEN_DOOR), 3),
        (3, 3, [WOOD, AIR, AIR, WOOD, WOOD, AIR, WOOD, WOOD, WOOD]) => (Item::Block(STAIRS), 4),
        (3, 1, [WOOD, WOOD, WOOD]) => (Item::Block(SLAB), 6),
        (2, 2, [STONE, STONE, STONE, STONE]) => (Item::Block(BRICKS), 4),
        (1, 1, [WOOD]) => (Item::Block(PLANKS), 4),
        (1, 2, [STONE, WOOD]) => (Item::Block(TORCH), 8),
        _ => return None,
    };
    Some(ItemStack { item, count })
}

pub fn craft(grid: &mut [Option<ItemStack>], width: usize, cursor: &mut Option<ItemStack>) -> bool {
    let Some(result) = recipe(grid, width) else {
        return false;
    };
    match cursor {
        None => *cursor = Some(result),
        Some(held)
            if held.item == result.item && held.count + result.count <= held.item.stack_limit() =>
        {
            held.count += result.count;
        }
        _ => return false,
    }
    for slot in grid.iter_mut().filter(|slot| slot.is_some()) {
        take_one(slot);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DIRT, STONE};

    #[test]
    fn slots_split_merge_swap_and_respect_capacity() {
        let mut inventory = Inventory::default();
        inventory.try_add(Item::Block(DIRT), 100).unwrap();
        assert_eq!(inventory.slots[0].unwrap().count, 99);
        assert_eq!(inventory.slots[1].unwrap().count, 1);
        click_slot(&mut inventory.slots[0], &mut inventory.cursor, true);
        assert_eq!(inventory.cursor.unwrap().count, 50);
        assert_eq!(inventory.slots[0].unwrap().count, 49);
        click_slot(&mut inventory.slots[1], &mut inventory.cursor, true);
        assert_eq!(inventory.slots[1].unwrap().count, 2);
        click_slot(&mut inventory.slots[0], &mut inventory.cursor, false);
        assert!(inventory.cursor.is_none());
        inventory.try_add(Item::Block(STONE), 1).unwrap();
        click_slot(&mut inventory.slots[2], &mut inventory.cursor, false);
        click_slot(&mut inventory.slots[0], &mut inventory.cursor, false);
        assert_eq!(inventory.slots[0].unwrap().item, Item::Block(STONE));
        assert_eq!(inventory.cursor.unwrap().item, Item::Block(DIRT));
        assert!(inventory.is_valid());
    }

    #[test]
    fn full_inventory_rejects_insert_without_losing_items() {
        let mut inventory = Inventory::default();
        inventory
            .try_add(Item::Block(DIRT), (INVENTORY_SIZE as u32) * STACK_LIMIT)
            .unwrap();
        assert!(inventory.try_add(Item::Block(STONE), 1).is_err());
        assert_eq!(inventory.count(DIRT), INVENTORY_SIZE as u32 * STACK_LIMIT);
        assert_eq!(inventory.count(STONE), 0);
    }

    fn grid(pattern: &[u8], width: usize) -> Vec<Option<ItemStack>> {
        assert_eq!(pattern.len(), width * width);
        pattern
            .iter()
            .map(|&block| {
                (block != AIR).then_some(ItemStack {
                    item: Item::Block(block),
                    count: 2,
                })
            })
            .collect()
    }

    #[test]
    fn all_recipes_match_and_consume_one_from_each_ingredient() {
        let cases: &[(usize, &[u8], Item, u32)] = &[
            (
                3,
                &[STONE, STONE, STONE, AIR, WOOD, AIR, AIR, WOOD, AIR],
                Item::Tool {
                    kind: ToolKind::Pickaxe,
                    durability: 64,
                },
                1,
            ),
            (
                3,
                &[STONE, STONE, AIR, STONE, WOOD, AIR, AIR, WOOD, AIR],
                Item::Tool {
                    kind: ToolKind::Axe,
                    durability: 64,
                },
                1,
            ),
            (
                3,
                &[STONE, AIR, AIR, WOOD, AIR, AIR, WOOD, AIR, AIR],
                Item::Tool {
                    kind: ToolKind::Shovel,
                    durability: 64,
                },
                1,
            ),
            (
                3,
                &[STONE, AIR, AIR, STONE, AIR, AIR, WOOD, AIR, AIR],
                Item::Tool {
                    kind: ToolKind::Sword,
                    durability: 32,
                },
                1,
            ),
            (2, &[LEAVES, LEAVES, LEAVES, WOOD], Item::BerryRation, 1),
            (
                3,
                &[LEAVES, SAND, LEAVES, AIR, AIR, AIR, AIR, AIR, AIR],
                Item::Arrow,
                5,
            ),
            (
                3,
                &[WOOD, WOOD, AIR, WOOD, WOOD, AIR, WOOD, WOOD, AIR],
                Item::Block(WOODEN_DOOR),
                3,
            ),
            (
                3,
                &[WOOD, AIR, AIR, WOOD, WOOD, AIR, WOOD, WOOD, WOOD],
                Item::Block(STAIRS),
                4,
            ),
            (
                3,
                &[WOOD, WOOD, WOOD, AIR, AIR, AIR, AIR, AIR, AIR],
                Item::Block(SLAB),
                6,
            ),
            (2, &[STONE, STONE, STONE, STONE], Item::Block(BRICKS), 4),
            (2, &[AIR, AIR, AIR, WOOD], Item::Block(PLANKS), 4),
            (2, &[STONE, AIR, WOOD, AIR], Item::Block(TORCH), 8),
        ];
        for &(width, pattern, item, count) in cases {
            let mut cells = grid(pattern, width);
            assert_eq!(recipe(&cells, width), Some(ItemStack { item, count }));
            let mut cursor = None;
            assert!(craft(&mut cells, width, &mut cursor));
            assert_eq!(cursor, Some(ItemStack { item, count }));
            assert!(cells.iter().flatten().all(|stack| stack.count == 1));
        }
    }

    #[test]
    fn incompatible_or_full_cursor_does_not_consume_ingredients() {
        let mut cells = grid(&[WOOD, AIR, AIR, AIR], 2);
        let original = cells.clone();
        let mut cursor = Some(ItemStack {
            item: Item::Block(STONE),
            count: 1,
        });
        assert!(!craft(&mut cells, 2, &mut cursor));
        assert_eq!(cells, original);
        cursor = Some(ItemStack {
            item: Item::Block(PLANKS),
            count: 62,
        });
        assert!(!craft(&mut cells, 2, &mut cursor));
        assert_eq!(cells, original);
    }

    #[test]
    fn inventory_save_round_trip_preserves_every_container() {
        let mut inventory = Inventory::default();
        inventory
            .try_add(
                Item::Tool {
                    kind: ToolKind::Pickaxe,
                    durability: 13,
                },
                1,
            )
            .unwrap();
        inventory.cursor = Some(ItemStack {
            item: Item::Arrow,
            count: 5,
        });
        inventory.small_grid[3] = Some(ItemStack {
            item: Item::Block(WOOD),
            count: 2,
        });
        inventory.large_grid[8] = Some(ItemStack {
            item: Item::BerryRation,
            count: 1,
        });
        let encoded = ron::to_string(&inventory).unwrap();
        let decoded: Inventory = ron::from_str(&encoded).unwrap();
        assert!(decoded.is_valid());
        assert_eq!(decoded.slots, inventory.slots);
        assert_eq!(decoded.cursor, inventory.cursor);
        assert_eq!(decoded.small_grid, inventory.small_grid);
        assert_eq!(decoded.large_grid, inventory.large_grid);
    }
}
