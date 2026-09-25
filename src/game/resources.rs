//! Player resource balances and monthly simulation.

use super::*;

impl Default for HudResources {
    fn default() -> Self {
        // Other resources still use local preview balances until their systems exist.
        Self {
            players: vec![[
                HudResource {
                    amount: 20.0,
                    monthly_delta: 0.0,
                },
                HudResource {
                    amount: 0.0,
                    monthly_delta: 0.0,
                },
                HudResource {
                    amount: 0.0,
                    monthly_delta: 0.0,
                },
                HudResource {
                    amount: 201.0,
                    monthly_delta: 2.0,
                },
                HudResource {
                    amount: 40.0,
                    monthly_delta: 1.0,
                },
                HudResource {
                    amount: 0.0,
                    monthly_delta: 0.0,
                },
                HudResource {
                    amount: 50.0,
                    monthly_delta: 0.0,
                },
            ]],
            happiness: vec![
                [HudResource {
                    amount: 50.0,
                    monthly_delta: 0.0,
                }; 4],
            ],
            happiness_edict_delta: vec![[0.0; 4]],
            famine_months: vec![0],
        }
    }
}

impl HudResources {
    pub(super) fn start_players(&mut self, count: usize, ownership: &ProvinceOwnership) {
        let baseline = self.players[0];
        self.famine_months = vec![0; count];
        self.happiness_edict_delta = vec![[0.0; 4]; count];
        self.happiness = vec![
            [HudResource {
                amount: 50.0,
                monthly_delta: 0.0
            }; 4];
            count
        ];
        self.players = (0..count)
            .map(|player| {
                let mut resources = baseline;
                for (resource, production) in
                    resources[..3].iter_mut().zip(ownership.net_production_for(player))
                {
                    resource.monthly_delta = production;
                }
                resources[3].monthly_delta = ownership.coin_delta_for(player);
                resources[4].monthly_delta = ownership.influence_delta_for(player);
                resources[5].amount = ownership.total_population_for(player);
                resources[5].monthly_delta =
                    ownership.population_change_for(player, resources[0].amount, 0);
                resources[6] = HudResource {
                    amount: 50.0,
                    monthly_delta: 0.0,
                };
                resources
            })
            .collect();
    }

    pub(super) fn for_player(&self, player: usize) -> [HudResource; 7] {
        let mut resources = self.players.get(player).copied().unwrap_or(self.players[0]);
        resources[6] = self
            .happiness_for(player)
            .into_iter()
            .min_by(|a, b| a.amount.total_cmp(&b.amount))
            .unwrap_or(HudResource {
                amount: 50.0,
                monthly_delta: 0.0,
            });
        resources
    }

    pub(super) fn happiness_for(&self, player: usize) -> [HudResource; 4] {
        self.happiness.get(player).copied().unwrap_or(self.happiness[0])
    }

    pub(super) fn apply_governance(&mut self, player: usize, governance: Governance) {
        let next = governance.happiness_deltas();
        if let (Some(classes), Some(previous)) =
            (self.happiness.get_mut(player), self.happiness_edict_delta.get_mut(player))
        {
            for (class, (new, old)) in classes.iter_mut().zip(next.into_iter().zip(*previous)) {
                class.monthly_delta += new - old;
            }
            *previous = next;
        }
    }

    pub(super) fn refresh_player_rates(&mut self, player: usize, ownership: &ProvinceOwnership) {
        self.apply_governance(player, ownership.governance_for(player));
        let Some(resources) = self.players.get_mut(player) else {
            return;
        };
        for (resource, production) in
            resources[..3].iter_mut().zip(ownership.net_production_for(player))
        {
            resource.monthly_delta = production;
        }
        resources[3].monthly_delta = ownership.coin_delta_for(player);
        resources[4].monthly_delta = ownership.influence_delta_for(player);
        resources[5].monthly_delta = ownership.population_change_for(
            player,
            resources[0].amount,
            self.famine_months[player],
        );
    }

    pub(super) fn advance(
        &mut self,
        months: usize,
        ownership: &mut ProvinceOwnership,
        local_practice: bool,
    ) {
        for _ in 0..months {
            for player in 0..self.players.len() {
                if local_practice {
                    self.apply_governance(player, ownership.governance_for(player));
                }
                let resources = &mut self.players[player];
                let population_change = if local_practice {
                    ownership.population_change_for(
                        player,
                        resources[0].amount,
                        self.famine_months[player],
                    )
                } else {
                    0.0
                };
                if local_practice {
                    for (resource, production) in
                        resources[..3].iter_mut().zip(ownership.net_production_for(player))
                    {
                        resource.monthly_delta = production;
                    }
                    resources[3].monthly_delta = ownership.coin_delta_for(player);
                    resources[4].monthly_delta = ownership.influence_delta_for(player);
                }
                for (index, resource) in resources.iter_mut().enumerate() {
                    if (local_practice && index == 5) || index == 6 {
                        continue;
                    }
                    resource.amount = (resource.amount + resource.monthly_delta).max(0.0);
                }
                for class in &mut self.happiness[player] {
                    class.amount = (class.amount + class.monthly_delta).clamp(0.0, 100.0);
                }
                resources[6] = self.happiness[player]
                    .into_iter()
                    .min_by(|a, b| a.amount.total_cmp(&b.amount))
                    .unwrap_or(HudResource {
                        amount: 50.0,
                        monthly_delta: 0.0,
                    });
                if local_practice {
                    let famine = &mut self.famine_months[player];
                    *famine = if resources[0].amount <= 0.0 {
                        famine.saturating_add(1)
                    } else {
                        0
                    };
                    ownership.advance_population(player, population_change);
                    resources[5].amount = ownership.total_population_for(player);
                    resources[5].monthly_delta =
                        ownership.population_change_for(player, resources[0].amount, *famine);
                    for (resource, production) in
                        resources[..3].iter_mut().zip(ownership.net_production_for(player))
                    {
                        resource.monthly_delta = production;
                    }
                    resources[3].monthly_delta = ownership.coin_delta_for(player);
                    resources[4].monthly_delta = ownership.influence_delta_for(player);
                }
            }
        }
    }
}
