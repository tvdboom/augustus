//! Monthly NPC border aggression, using the ordinary movement and combat records.

use super::campaign::Campaign;
use super::campaign_notifications::{NoticeKind, NoticeSeverity};
use crate::game::economy::BuildingType;
use crate::game::military::*;
use crate::game::politics::diplomacy::{garrison_bonus, PoliticalState};

impl Campaign {
    /// Unopposed NPC occupation erodes the owner's Control using the player occupation curve.
    /// A new occupation first applies pressure at the following monthly boundary.
    pub(super) fn resolve_npc_occupation_control(&mut self, previous: &[Option<ForceOwner>]) {
        for (province, &previous_occupation) in previous.iter().enumerate() {
            let Some(ForceOwner::Local(home)) = self.military.provinces[province].occupation else {
                continue;
            };
            let occupier = ForceOwner::Local(home);
            let PoliticalState::Owned {
                owner,
            } = self.politics[province].state
            else {
                continue;
            };
            if home == province
                || previous_occupation != Some(occupier)
                || !self.npc_wars[owner][home]
                || self.military.province_in_battle(province)
                || self.military.provinces[province]
                    .forces
                    .get(&ForceOwner::Player(owner))
                    .is_some_and(|units| units.iter().any(|unit| unit.current_manpower > 0.))
            {
                continue;
            }
            let strength: f64 = self.military.provinces[province]
                .forces
                .get(&occupier)
                .into_iter()
                .flatten()
                .filter(|unit| unit.current_manpower > 0. && unit.morale > 0.)
                .map(|unit| unit.effective_strength(&self.military.config))
                .sum();
            let power =
                strength * self.military.config.rank_control[self.military.rank(occupier) as usize];
            let loss = garrison_bonus(power, &self.diplomacy_config);
            if loss > 0. {
                let _ = self.politics[province].apply_local_control_loss(loss);
            }
        }
    }

    /// Peaceful guests use their host's food; invading NPC armies supply themselves.
    pub(super) fn army_provisioner(
        &self,
        province: Option<ProvinceId>,
        owner: ForceOwner,
    ) -> ForceOwner {
        let Some(location) = province else {
            return owner;
        };
        let Some(host) = self.economy.provinces.get(location).and_then(|p| p.owner) else {
            return owner;
        };
        if matches!(owner, ForceOwner::Local(home) if home != location && self.npc_wars[host][home])
        {
            owner
        } else {
            ForceOwner::Player(host)
        }
    }

    /// Local armies can return home or invade a hostile player's adjacent province.
    pub(super) fn npc_access_snapshot(&self) -> Vec<Vec<MilitaryAccess>> {
        (0..self.politics.len())
            .map(|home| {
                self.economy
                    .provinces
                    .iter()
                    .enumerate()
                    .map(|(id, province)| {
                        if home == id {
                            MilitaryAccess::Peaceful
                        } else if self.graph[home].neighbors.contains(&id)
                            && province.owner.is_some_and(|player| self.npc_wars[player][home])
                        {
                            MilitaryAccess::Invasion
                        } else {
                            MilitaryAccess::Blocked
                        }
                    })
                    .collect()
            })
            .collect()
    }

    /// Roll once per eligible province–player pair at the completed monthly boundary.
    /// Each province commits at most one army, prioritizing hostile visitors at home.
    pub(super) fn advance_npc_aggression(&mut self) {
        if !self.active || self.senate.winner.is_some() {
            return;
        }
        let chance = self.military.config.npc_invasion_monthly_chance.clamp(0., 1.);
        if !chance.is_finite() || chance <= 0. {
            return;
        }
        for home in 0..self.politics.len() {
            let local = ForceOwner::Local(home);
            let state = &self.military.provinces[home];
            if !matches!(
                self.politics[home].state,
                PoliticalState::Independent { .. } | PoliticalState::Vassal { .. }
            ) || state.slave_rebellion
                || state.occupation.is_some_and(|occupier| occupier != local)
                || self.military.province_in_battle(home)
                || self.military.movements.iter().any(|m| m.owner == local)
                || self.military.battles.iter().any(|battle| {
                    battle.attackers.plans.contains_key(&local)
                        || battle.defenders.plans.contains_key(&local)
                })
                || self.military.provinces.iter().enumerate().any(|(id, state)| {
                    id != home
                        && state.forces.get(&local).is_some_and(|units| {
                            units.iter().any(|u| u.current_manpower > 0. && u.morale > 0.)
                        })
                })
            {
                continue;
            }
            let units: Vec<_> = state
                .forces
                .get(&local)
                .into_iter()
                .flatten()
                .filter(|unit| unit.current_manpower > 0. && unit.morale > 0.)
                .map(|unit| unit.id)
                .collect();
            if units.is_empty() {
                continue;
            }
            for player in 0..self.actors.len() {
                if self.defeated.get(player).copied().unwrap_or(false)
                    || self.politics[home].relation(player)
                        >= self.military.config.npc_invasion_relation
                    || state
                        .forces
                        .get(&ForceOwner::Player(player))
                        .is_some_and(|units| units.iter().any(|unit| unit.current_manpower > 0.))
                {
                    continue;
                }
                let target = self.graph[home]
                    .neighbors
                    .iter()
                    .copied()
                    .filter(|&id| {
                        matches!(self.politics[id].state, PoliticalState::Owned { owner } if owner == player)
                            && !self.military.province_in_battle(id)
                            && self.military.provinces[id].occupation.is_none()
                    })
                    .min_by(|&a, &b| {
                        self.military
                            .stationed_strength(a, ForceOwner::Player(player))
                            .total_cmp(&self.military.stationed_strength(b, ForceOwner::Player(player)))
                            .then(a.cmp(&b))
                    });
                let Some(target) = target else {
                    continue;
                };
                if self.diplomacy_rng.unit() >= chance {
                    continue;
                }
                let ordered = self.military.order_movement_route(
                    home,
                    local,
                    &units,
                    None,
                    &[target],
                    &self.graph,
                    |owner, id| {
                        if owner == local && id == target {
                            MilitaryAccess::Invasion
                        } else {
                            MilitaryAccess::Blocked
                        }
                    },
                );
                if let Ok(order) = ordered {
                    self.military
                        .movements
                        .iter_mut()
                        .find(|m| m.id == order)
                        .unwrap()
                        .attack_target = Some(ForceOwner::Player(player));
                    self.npc_wars[player][home] = true;
                    self.military_pressure.remove(&(player, home));
                    self.notifications.province_notice(
                        player,
                        target,
                        self.economy.month,
                        NoticeSeverity::Warning,
                        NoticeKind::InvasionBegins,
                        format!("{} is attacking", self.economy.provinces[home].name),
                        format!(
                            "Very hostile relations prompted an army from {} to march on {}.",
                            self.economy.provinces[home].name, self.economy.provinces[target].name,
                        ),
                    );
                }
                break;
            }
        }
    }

    /// Foreign NPC forces fight hostile troops and occupy an unguarded enemy province.
    pub(super) fn begin_npc_encounter(
        &mut self,
        province: ProvinceId,
        home: ProvinceId,
        origin: Option<ProvinceId>,
    ) {
        let attacker = ForceOwner::Local(home);
        if home == province
            || self.military.provinces[province].forces.get(&attacker).is_none_or(|units| {
                !units.iter().any(|unit| unit.current_manpower > 0. && unit.morale > 0.)
            })
        {
            return;
        }
        if let Some(battle) = self.military.battles.iter().find(|b| b.province == province) {
            let against_attackers =
                battle.attackers.plans.keys().any(|&owner| self.forces_hostile(attacker, owner));
            let against_defenders =
                battle.defenders.plans.keys().any(|&owner| self.forces_hostile(attacker, owner));
            let coalition = if battle.attackers.plans.contains_key(&attacker) {
                Some(true)
            } else if battle.defenders.plans.contains_key(&attacker) {
                Some(false)
            } else if against_defenders && !against_attackers {
                Some(true)
            } else if against_attackers && !against_defenders {
                Some(false)
            } else {
                None
            };
            if let Some(attacking) = coalition {
                let _ = self.military.join_battle(province, attacker, attacking);
            }
            return;
        }
        let territorial_owner = self.economy.provinces[province].owner;
        let mut candidates: Vec<_> = self.military.provinces[province]
            .forces
            .iter()
            .filter_map(|(&owner, units)| {
                (self.forces_hostile(attacker, owner)
                    && units.iter().any(|unit| unit.current_manpower > 0.))
                .then_some(owner)
            })
            .collect();
        candidates.sort_by_key(|&owner| {
            (Some(owner) != territorial_owner.map(ForceOwner::Player), owner)
        });
        let mut defenders = Vec::new();
        for candidate in candidates {
            if defenders.iter().all(|&other| !self.forces_hostile(candidate, other)) {
                defenders.push(candidate);
            }
        }
        if defenders.is_empty() {
            if territorial_owner.is_some_and(|player| self.npc_wars[player][home])
                && self.military.provinces[province].occupation != Some(attacker)
            {
                self.military.provinces[province].occupation = Some(attacker);
                self.record_military_event(&MilitaryEvent::OccupationEstablished {
                    province,
                    owner: attacker,
                });
            }
        } else if self
            .military
            .start_battle(
                province,
                &[attacker],
                &defenders,
                territorial_owner,
                origin.or(Some(home)),
                self.graph[province].terrain,
                self.economy.provinces[province].level(BuildingType::CityWalls),
                u64::from(self.economy.month) * 179 + province as u64 + 1,
            )
            .is_ok()
        {
            self.notify_battle_started(province);
        }
    }
}
