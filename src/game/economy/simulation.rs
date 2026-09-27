//! Explicit deterministic month stages and the convenience all-in-one monthly tick.

use super::{EconomyEvent, EconomyWorld, MonthlyInputs, MonthlyReport, ProvinceMonth};

impl EconomyWorld {
    /// Production and recurring trade stage. Apply recurring diplomatic payments between
    /// this and `finish_month` so they compete for the same authoritative wallets.
    pub fn begin_month(&mut self, inputs: &MonthlyInputs) -> MonthlyReport {
        self.one_time_relation_awarded.clear();
        let mut report = MonthlyReport {
            month: self.month.saturating_add(1),
            player_delta: self
                .players
                .iter()
                .map(|player| player.balances().map(|value| -value))
                .collect(),
            food_supply_ratio: vec![1.0; self.players.len()],
            province_reports: vec![ProvinceMonth::default(); self.provinces.len()],
            trade_effects: Vec::new(),
            events: Vec::new(),
        };
        for province in &mut self.provinces {
            province.validate_slave_assignment();
        }
        self.refresh_npc_markets(inputs);
        for (province, summary) in self.provinces.iter().zip(&mut report.province_reports) {
            summary.population_delta = -province.total_population();
            summary.capacity = province.capacity(&self.config);
            (summary.labor, summary.production) = province.production(&self.config);
            summary.food_requested = province.food_request(&self.config);
            if let Some(owner) = province.owner.and_then(|id| self.players.get_mut(id)) {
                for (stock, production) in owner.resources.iter_mut().zip(summary.production) {
                    *stock += production;
                }
            }
        }
        // Stocks may temporarily exceed storage until this month's trade/food consumption finishes.
        report.trade_effects = self.advance_trade(inputs, &mut report.events);
        report
    }

    /// Food allocation, demographics, migration, construction, tax, and storage stage.
    /// Political rank/vassal rewards stay with the political system and are added separately.
    pub fn finish_month(
        &mut self,
        inputs: &MonthlyInputs,
        mut report: MonthlyReport,
    ) -> MonthlyReport {
        let mut food_requested: Vec<f64> = (0..self.players.len())
            .map(|player| inputs.army_food.get(player).copied().unwrap_or(0.0).max(0.0))
            .collect();
        for (province, summary) in self.provinces.iter().zip(&report.province_reports) {
            if let Some(requested) = province.owner.and_then(|owner| food_requested.get_mut(owner))
            {
                *requested += summary.food_requested;
            }
        }
        for (player, (wallet, requested)) in self.players.iter_mut().zip(food_requested).enumerate()
        {
            let available = wallet.resources[0].max(0.0);
            let ratio = if requested > 0.0 {
                (available / requested).clamp(0.0, 1.0)
            } else {
                1.0
            };
            wallet.resources[0] = (available - requested * ratio).max(0.0);
            report.food_supply_ratio[player] = ratio;
            if ratio < 0.9 {
                report.events.push(EconomyEvent::FoodShortage {
                    player,
                    supplied: ratio,
                });
            }
        }
        for (province, summary) in self.provinces.iter_mut().zip(&mut report.province_reports) {
            summary.food_supply_ratio = if let Some(owner) = province.owner {
                report.food_supply_ratio.get(owner).copied().unwrap_or(0.0)
            } else {
                // User clarification: NPC residents are abstractly provisioned.
                // Their geographic deficits remain real trade demand/pricing,
                // without inventing hidden NPC stockpiles or starving every
                // mountain province before a player can reach it.
                1.0
            };
            province.advance_demographics(summary, &self.config);
        }
        self.migrate(inputs, &mut report.province_reports);
        // Completing a wonder cannot retroactively restore this month's diverted slave production.
        self.advance_construction(&mut report.events);
        for (province, summary) in self.provinces.iter_mut().zip(&mut report.province_reports) {
            summary.population_delta += province.total_population();
            if let Some(owner) = province.owner.and_then(|id| self.players.get_mut(id)) {
                summary.tax_income = province.tax_income(&self.config);
                let effects = province.building_effects(&self.config);
                let wonder_income = province
                    .completed_wonder
                    .and_then(|wonder| {
                        self.config.wonders.iter().find(|definition| definition.wonder_id == wonder)
                    })
                    .map(|definition| definition.monthly_influence)
                    .unwrap_or(0.0);
                summary.influence_income = province.population[0] * self.config.influence_per_noble
                    + effects.influence
                    + wonder_income;
                owner.coin += summary.tax_income;
                owner.influence += summary.influence_income;
            } else {
                // Coin is deliberately generated internally; physical NPC stocks are never invented.
                province.market.coin_treasury = (province.market.coin_treasury
                    + province.market.monthly_coin_income
                    - province.market.monthly_coin_expenses)
                    .max(0.0);
            }
            for modifier in &mut province.temporary_happiness {
                *modifier *= self.config.temporary_happiness_decay;
            }
        }
        self.recalculate_storage();
        for (wallet, delta) in self.players.iter_mut().zip(&mut report.player_delta) {
            wallet.clamp_storage();
            for (change, current) in delta.iter_mut().zip(wallet.balances()) {
                *change += current;
            }
        }
        self.month = report.month;
        self.last_report = report.clone();
        report
    }

    /// Run a complete economic month when no external diplomacy stage is needed.
    pub fn advance_month(&mut self, inputs: &MonthlyInputs) -> MonthlyReport {
        let report = self.begin_month(inputs);
        self.finish_month(inputs, report)
    }

    /// Side-effect-free forecast for HUD changes, keeping tooltips consistent with actual rules.
    pub fn forecast_month(&self, inputs: &MonthlyInputs) -> MonthlyReport {
        self.clone().advance_month(inputs)
    }

    /// Rebuild global capacity from directly owned storage buildings; vassals do not count.
    pub fn recalculate_storage(&mut self) {
        for account in &mut self.players {
            account.storage = self.config.base_storage;
        }
        for province in &self.provinces {
            if let Some(account) = province.owner.and_then(|owner| self.players.get_mut(owner)) {
                for (storage, bonus) in
                    account.storage.iter_mut().zip(province.building_effects(&self.config).storage)
                {
                    *storage += bonus;
                }
            }
        }
    }
}
