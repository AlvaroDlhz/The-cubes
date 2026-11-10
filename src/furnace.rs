//! Furnace smelting: a pure simulation so it can run for furnaces in
//! unloaded chunks and be tested without the engine.

use serde::{Deserialize, Serialize};

use crate::blocks::{MineralKind, PLANKS, WOOD};
use crate::item_inventory::{take_one, Item, ItemStack};

/// Seconds a lit furnace needs to turn one input item into its result.
pub const SMELT_SECONDS: f32 = 8.0;

/// What an item becomes when smelted, if anything.
pub fn smelting_result(item: Item) -> Option<Item> {
    match item {
        Item::Mineral(MineralKind::RawCopper) => Some(Item::Mineral(MineralKind::CopperIngot)),
        Item::Mineral(MineralKind::RawIron) => Some(Item::Mineral(MineralKind::IronIngot)),
        _ => None,
    }
}

/// Seconds one unit of an item keeps a furnace lit, if it burns at all.
pub fn fuel_seconds(item: Item) -> Option<f32> {
    match item {
        Item::Mineral(MineralKind::Coal) => Some(SMELT_SECONDS * 8.0),
        Item::Block(WOOD) => Some(SMELT_SECONDS * 1.5),
        Item::Block(PLANKS) => Some(SMELT_SECONDS * 0.75),
        _ => None,
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Furnace {
    pub input: Option<ItemStack>,
    pub fuel: Option<ItemStack>,
    pub output: Option<ItemStack>,
    /// Seconds of fire left from the fuel unit being burned.
    pub burn_remaining: f32,
    /// Total seconds the fuel unit being burned lasts.
    pub burn_total: f32,
    /// Seconds already spent smelting the current input item.
    pub progress: f32,
}

impl Furnace {
    pub fn is_lit(&self) -> bool {
        self.burn_remaining > 0.0
    }

    pub fn is_empty(&self) -> bool {
        self.input.is_none() && self.fuel.is_none() && self.output.is_none()
    }

    /// The result of the current input, if the output slot has room for it.
    fn pending_result(&self) -> Option<Item> {
        let result = smelting_result(self.input?.item)?;
        match self.output {
            None => Some(result),
            Some(stack) if stack.item == result && stack.count < result.stack_limit() => {
                Some(result)
            }
            Some(_) => None,
        }
    }

    /// Advances the furnace by `seconds`. Fuel is only lit when there is
    /// something to smelt, but a unit already burning is never refunded.
    pub fn tick(&mut self, mut seconds: f32) {
        while seconds > 0.0 {
            let result = self.pending_result();
            if result.is_none() {
                self.progress = 0.0;
            }
            if !self.is_lit() {
                let fuel = self.fuel.and_then(|stack| fuel_seconds(stack.item));
                match (result, fuel) {
                    (Some(_), Some(duration)) => {
                        take_one(&mut self.fuel);
                        self.burn_remaining = duration;
                        self.burn_total = duration;
                    }
                    _ => {
                        self.progress = 0.0;
                        return;
                    }
                }
            }
            let mut step = seconds.min(self.burn_remaining);
            if result.is_some() {
                step = step.min(SMELT_SECONDS - self.progress);
            }
            self.burn_remaining -= step;
            seconds -= step;
            if let Some(result) = result {
                self.progress += step;
                if self.progress >= SMELT_SECONDS {
                    self.progress = 0.0;
                    take_one(&mut self.input);
                    match self.output.as_mut() {
                        Some(stack) => stack.count += 1,
                        None => {
                            self.output = Some(ItemStack {
                                item: result,
                                count: 1,
                            })
                        }
                    }
                }
            }
        }
    }

    pub fn is_valid(&self) -> bool {
        [self.input, self.fuel, self.output]
            .into_iter()
            .flatten()
            .all(ItemStack::is_valid)
            && [self.burn_remaining, self.burn_total, self.progress]
                .into_iter()
                .all(|value| value.is_finite() && value >= 0.0)
            && self.burn_remaining <= self.burn_total
            && self.progress <= SMELT_SECONDS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::STONE;

    fn stack(item: Item, count: u32) -> Option<ItemStack> {
        Some(ItemStack { item, count })
    }

    const RAW_COPPER: Item = Item::Mineral(MineralKind::RawCopper);
    const COPPER: Item = Item::Mineral(MineralKind::CopperIngot);
    const COAL: Item = Item::Mineral(MineralKind::Coal);

    #[test]
    fn smelts_one_item_per_interval_and_burns_fuel_only_when_needed() {
        let mut furnace = Furnace {
            input: stack(RAW_COPPER, 2),
            fuel: stack(Item::Block(PLANKS), 4),
            ..Default::default()
        };
        furnace.tick(SMELT_SECONDS - 0.5);
        assert!(furnace.is_lit());
        assert_eq!(furnace.output, None);
        // One plank lasts 6 s, so a second one was lit to keep going.
        assert_eq!(furnace.fuel, stack(Item::Block(PLANKS), 2));
        furnace.tick(0.5);
        assert_eq!(furnace.output, stack(COPPER, 1));
        assert_eq!(furnace.input, stack(RAW_COPPER, 1));
        furnace.tick(SMELT_SECONDS);
        assert_eq!(furnace.output, stack(COPPER, 2));
        assert_eq!(furnace.input, None);
        // 16 s of smelting need three 6 s planks; the fourth is untouched.
        assert_eq!(furnace.fuel, stack(Item::Block(PLANKS), 1));
        furnace.tick(100.0);
        assert!(!furnace.is_lit());
        assert_eq!(furnace.fuel, stack(Item::Block(PLANKS), 1));
        assert!(furnace.is_valid());
    }

    #[test]
    fn many_small_ticks_match_one_large_tick() {
        let start = Furnace {
            input: stack(RAW_COPPER, 5),
            fuel: stack(COAL, 1),
            ..Default::default()
        };
        let mut coarse = start.clone();
        coarse.tick(30.0);
        let mut fine = start;
        for _ in 0..3000 {
            fine.tick(0.01);
        }
        assert_eq!(coarse.output, stack(COPPER, 3));
        assert_eq!(fine.output, coarse.output);
        assert_eq!(fine.input, coarse.input);
        assert!((fine.progress - coarse.progress).abs() < 0.05);
    }

    #[test]
    fn idles_without_fuel_smeltable_input_or_output_room() {
        let mut no_fuel = Furnace {
            input: stack(RAW_COPPER, 1),
            ..Default::default()
        };
        no_fuel.tick(60.0);
        assert_eq!(no_fuel.input, stack(RAW_COPPER, 1));
        assert_eq!(no_fuel.output, None);

        let mut not_smeltable = Furnace {
            input: stack(Item::Block(STONE), 1),
            fuel: stack(COAL, 1),
            ..Default::default()
        };
        not_smeltable.tick(60.0);
        assert_eq!(not_smeltable.fuel, stack(COAL, 1));

        let mut blocked = Furnace {
            input: stack(RAW_COPPER, 1),
            fuel: stack(COAL, 1),
            output: stack(Item::Mineral(MineralKind::IronIngot), 1),
            ..Default::default()
        };
        blocked.tick(60.0);
        assert_eq!(blocked.fuel, stack(COAL, 1));
        assert_eq!(blocked.input, stack(RAW_COPPER, 1));

        let mut full = Furnace {
            input: stack(RAW_COPPER, 1),
            fuel: stack(COAL, 1),
            output: stack(COPPER, COPPER.stack_limit()),
            ..Default::default()
        };
        full.tick(60.0);
        assert_eq!(full.input, stack(RAW_COPPER, 1));
    }

    #[test]
    fn progress_resets_when_the_fire_goes_out_or_input_is_removed() {
        let mut furnace = Furnace {
            input: stack(RAW_COPPER, 1),
            fuel: stack(Item::Block(PLANKS), 1),
            ..Default::default()
        };
        furnace.tick(20.0);
        assert_eq!(furnace.output, None);
        assert_eq!(furnace.progress, 0.0);
        assert!(!furnace.is_lit());

        let mut furnace = Furnace {
            input: stack(RAW_COPPER, 1),
            fuel: stack(COAL, 1),
            ..Default::default()
        };
        furnace.tick(4.0);
        assert_eq!(furnace.progress, 4.0);
        furnace.input = None;
        furnace.tick(1.0);
        assert_eq!(furnace.progress, 0.0);
        assert!(furnace.is_lit());
    }
}
