//! Nationwide civic celebrations with immediate rewards and independent cooldowns.

use super::campaign::Campaign;

pub(crate) const EVENT_COOLDOWN_MONTHS: u32 = 24;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CivicEvent {
    Theater,
    Feast,
    Games,
    GrainDole,
    Patronage,
}

impl CivicEvent {
    pub(crate) const ALL: [Self; 5] =
        [Self::Theater, Self::Feast, Self::Games, Self::GrainDole, Self::Patronage];

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Theater => "Organize Theater",
            Self::Feast => "Organize a Feast",
            Self::Games => "Organize Games",
            Self::GrainDole => "Hold a Grain Dole",
            Self::Patronage => "Call for Patronage",
        }
    }

    pub(crate) const fn description(self) -> &'static str {
        match self {
            Self::Theater => "Stage a performance for the citizens.",
            Self::Feast => "Welcome the nobility to a public banquet.",
            Self::Games => "Fill the circus with races and spectacle.",
            Self::GrainDole => "Share food with the common people.",
            Self::Patronage => "Gather gifts from wealthy supporters.",
        }
    }

    pub(crate) const fn held_label(self) -> &'static str {
        match self {
            Self::Theater => "Theater performance held",
            Self::Feast => "Feast held",
            Self::Games => "Games held",
            Self::GrainDole => "Grain dole held",
            Self::Patronage => "Patronage gathered",
        }
    }

    /// Costs at a representative single-province population. Actual quotes scale
    /// with the owned population receiving the event's effect.
    const fn base_cost(self) -> EventCost {
        match self {
            Self::Theater => EventCost {
                coin: 100.0,
                food: 0.0,
                influence: 0.0,
            },
            Self::Feast => EventCost {
                coin: 120.0,
                food: 30.0,
                influence: 0.0,
            },
            Self::Games => EventCost {
                coin: 160.0,
                food: 0.0,
                influence: 0.0,
            },
            Self::GrainDole => EventCost {
                coin: 70.0,
                food: 70.0,
                influence: 0.0,
            },
            Self::Patronage => EventCost {
                coin: 0.0,
                food: 0.0,
                influence: 15.0,
            },
        }
    }

    const fn reference_population(self) -> f64 {
        match self {
            Self::Theater => 140.0,                // Citizens
            Self::Feast | Self::Patronage => 70.0, // Nobles
            Self::Games => 230.0,                  // Plebeians
            Self::GrainDole => 400.0,              // Plebeians and Slaves
        }
    }

    fn beneficiaries(self, population: [f64; 4]) -> f64 {
        match self {
            Self::Theater => population[1],
            Self::Feast | Self::Patronage => population[0],
            Self::Games => population[2],
            Self::GrainDole => population[2] + population[3],
        }
    }

    /// Noble, Citizen, Plebeian, Slave happiness gained immediately in every owned province.
    const fn happiness(self) -> [f64; 4] {
        match self {
            Self::Theater => [0.0, 10.0, 0.0, 0.0],
            Self::Feast => [10.0, 0.0, 0.0, 0.0],
            Self::Games => [0.0, 0.0, 10.0, 0.0],
            Self::GrainDole => [0.0, 0.0, 8.0, 8.0],
            Self::Patronage => [0.0; 4],
        }
    }

    const fn base_coin_reward(self) -> f64 {
        match self {
            Self::Patronage => 90.0,
            _ => 0.0,
        }
    }

    pub(crate) fn reward_label(self, coin_reward: f64) -> String {
        match self {
            Self::Theater => "+10 happiness to citizens".to_owned(),
            Self::Feast => "+10 happiness to nobles".to_owned(),
            Self::Games => "+10 happiness to plebeians".to_owned(),
            Self::GrainDole => "+8 happiness to plebeians and slaves".to_owned(),
            Self::Patronage => format!("+{coin_reward:.0} sestertius"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct EventCost {
    pub coin: f64,
    pub food: f64,
    pub influence: f64,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct EventQuote {
    pub cost: EventCost,
    pub coin_reward: f64,
    pub beneficiaries: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EventError {
    Unavailable,
    Cooldown {
        ready_month: u32,
    },
    CannotAfford,
}

impl Campaign {
    /// Scale every cost by its direct beneficiaries, using current owned populations.
    pub(crate) fn event_quote(&self, player: usize, event: CivicEvent) -> EventQuote {
        let beneficiaries: f64 = self
            .economy
            .provinces
            .iter()
            .filter(|p| p.owner == Some(player))
            .map(|p| event.beneficiaries(p.population))
            .sum::<f64>()
            .max(0.0);
        let scale = beneficiaries / event.reference_population();
        let base = event.base_cost();
        EventQuote {
            cost: EventCost {
                coin: (base.coin * scale).ceil(),
                food: (base.food * scale).ceil(),
                influence: (base.influence * scale).ceil(),
            },
            coin_reward: (event.base_coin_reward() * scale).floor(),
            beneficiaries,
        }
    }

    pub(crate) fn event_availability(
        &self,
        player: usize,
        event: CivicEvent,
    ) -> Result<(), EventError> {
        if !self.active
            || self.defeated.get(player).copied().unwrap_or(false)
            || self.actors.get(player).is_none()
            || self.economy.players.get(player).is_none()
            || !self.economy.provinces.iter().any(|p| p.owner == Some(player))
        {
            return Err(EventError::Unavailable);
        }
        if let Some(&last) = self.event_used.get(&(player, event)) {
            let ready_month = last.saturating_add(EVENT_COOLDOWN_MONTHS);
            if self.economy.month < ready_month {
                return Err(EventError::Cooldown {
                    ready_month,
                });
            }
        }
        let quote = self.event_quote(player, event);
        if quote.beneficiaries <= 0.0 {
            return Err(EventError::Unavailable);
        }
        let wallet = &self.economy.players[player];
        let cost = quote.cost;
        if wallet.coin < cost.coin
            || wallet.resources[0] < cost.food
            || wallet.influence < cost.influence
        {
            return Err(EventError::CannotAfford);
        }
        Ok(())
    }

    pub(crate) fn hold_event(
        &mut self,
        player: usize,
        event: CivicEvent,
    ) -> Result<(), EventError> {
        self.event_availability(player, event)?;
        let quote = self.event_quote(player, event);
        let cost = quote.cost;
        let wallet = &mut self.economy.players[player];
        wallet.coin = wallet.coin - cost.coin + quote.coin_reward;
        wallet.resources[0] -= cost.food;
        wallet.influence -= cost.influence;
        self.actors[player].coin = wallet.coin;
        self.actors[player].influence = wallet.influence;

        for province in self.economy.provinces.iter_mut().filter(|p| p.owner == Some(player)) {
            for (class, requested) in event.happiness().into_iter().enumerate() {
                let before = province.happiness[class];
                province.happiness[class] = (before + requested).clamp(0.0, 100.0);
                // The monthly happiness calculation is rebuilt from current effects. Carry
                // this one-time gain through that calculation; it then decays naturally.
                province.temporary_happiness[class] += province.happiness[class] - before;
            }
        }
        self.event_used.insert((player, event), self.economy.month);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::economy::{EconomicProvince, EconomyWorld, Terrain};
    use crate::game::politics::PoliticalPlayer;

    #[test]
    fn nationwide_event_is_instant_and_each_type_has_its_own_24_month_cooldown() {
        let mut campaign = Campaign::default();
        campaign.active = true;
        let mut provinces: Vec<_> = (0..3)
            .map(|_| {
                EconomicProvince::new(
                    "Province",
                    20.0,
                    Terrain::Plains,
                    false,
                    [1.0; 3],
                    [10.0, 70.0, 115.0, 85.0],
                    2,
                )
            })
            .collect();
        provinces[0].owner = Some(0);
        provinces[1].owner = Some(0);
        provinces[2].owner = Some(1);
        campaign.economy = EconomyWorld::new(2, provinces, vec![vec![]; 3]);
        campaign.actors = vec![PoliticalPlayer::default(); 2];
        campaign.economy.players[0].coin = 500.0;
        campaign.hold_event(0, CivicEvent::Theater).unwrap();
        assert_eq!(campaign.economy.players[0].coin, 400.0);
        assert_eq!(campaign.actors[0].coin, 400.0);
        assert_eq!(campaign.economy.provinces[0].happiness[1], 60.0);
        assert_eq!(campaign.economy.provinces[1].happiness[1], 60.0);
        assert_eq!(campaign.economy.provinces[2].happiness[1], 50.0);
        assert_eq!(
            campaign.hold_event(0, CivicEvent::Theater),
            Err(EventError::Cooldown {
                ready_month: 24
            })
        );
        assert!(campaign.hold_event(0, CivicEvent::Games).is_ok());
        campaign.economy.month = 23;
        assert!(matches!(
            campaign.event_availability(0, CivicEvent::Theater),
            Err(EventError::Cooldown { .. })
        ));
        campaign.economy.month = 24;
        assert!(campaign.hold_event(0, CivicEvent::Theater).is_ok());
    }

    #[test]
    fn unaffordable_event_does_not_charge_or_start_cooldown() {
        let mut campaign = Campaign::default();
        campaign.active = true;
        let mut province =
            EconomicProvince::new("Home", 20.0, Terrain::Plains, false, [1.0; 3], [10.0; 4], 1);
        province.owner = Some(0);
        campaign.economy = EconomyWorld::new(1, vec![province], vec![vec![]]);
        campaign.actors = vec![PoliticalPlayer::default()];
        campaign.economy.players[0].coin = 0.0;
        assert_eq!(campaign.hold_event(0, CivicEvent::Theater), Err(EventError::CannotAfford));
        assert_eq!(campaign.economy.players[0].coin, 0.0);
        assert!(campaign.event_used.is_empty());
    }

    #[test]
    fn patronage_converts_influence_to_money_without_a_monthly_modifier() {
        let mut campaign = Campaign::default();
        campaign.active = true;
        let mut province = EconomicProvince::new(
            "Home",
            20.0,
            Terrain::Plains,
            false,
            [1.0; 3],
            [70.0, 140.0, 230.0, 170.0],
            1,
        );
        province.owner = Some(0);
        campaign.economy = EconomyWorld::new(1, vec![province], vec![vec![]]);
        campaign.actors = vec![PoliticalPlayer::default()];
        campaign.economy.players[0].coin = 0.0;
        campaign.economy.players[0].influence = 15.0;

        campaign.hold_event(0, CivicEvent::Patronage).unwrap();

        assert_eq!(campaign.economy.players[0].coin, 90.0);
        assert_eq!(campaign.economy.players[0].influence, 0.0);
        assert_eq!(campaign.actors[0].coin, 90.0);
        assert_eq!(campaign.economy.provinces[0].happiness, [50.0; 4]);

        campaign.economy.provinces[0].population[0] = 140.0;
        campaign.economy.players[0].influence = 30.0;
        campaign.economy.month = 24;
        let quote = campaign.event_quote(0, CivicEvent::Patronage);
        assert_eq!(quote.cost.influence, 30.0);
        assert_eq!(quote.coin_reward, 180.0);
        campaign.hold_event(0, CivicEvent::Patronage).unwrap();
        assert_eq!(campaign.economy.players[0].coin, 270.0);
        assert_eq!(campaign.economy.players[0].influence, 0.0);
    }

    #[test]
    fn every_event_cost_scales_with_its_owned_beneficiaries() {
        let mut campaign = Campaign::default();
        let mut home = EconomicProvince::new(
            "Home",
            20.0,
            Terrain::Plains,
            false,
            [1.0; 3],
            [70.0, 140.0, 230.0, 170.0],
            2,
        );
        home.owner = Some(0);
        let mut foreign = home.clone();
        foreign.owner = Some(1);
        foreign.population = [1_000.0; 4];
        campaign.economy = EconomyWorld::new(2, vec![home, foreign], vec![vec![]; 2]);
        for (event, coin, food, influence) in [
            (CivicEvent::Theater, 100.0, 0.0, 0.0),
            (CivicEvent::Feast, 120.0, 30.0, 0.0),
            (CivicEvent::Games, 160.0, 0.0, 0.0),
            (CivicEvent::GrainDole, 70.0, 70.0, 0.0),
            (CivicEvent::Patronage, 0.0, 0.0, 15.0),
        ] {
            let quote = campaign.event_quote(0, event);
            assert_eq!(
                (quote.cost.coin, quote.cost.food, quote.cost.influence),
                (coin, food, influence)
            );
        }
        campaign.economy.provinces[0].population = [140.0, 280.0, 460.0, 340.0];
        for (event, coin, food, influence) in [
            (CivicEvent::Theater, 200.0, 0.0, 0.0),
            (CivicEvent::Feast, 240.0, 60.0, 0.0),
            (CivicEvent::Games, 320.0, 0.0, 0.0),
            (CivicEvent::GrainDole, 140.0, 140.0, 0.0),
            (CivicEvent::Patronage, 0.0, 0.0, 30.0),
        ] {
            let quote = campaign.event_quote(0, event);
            assert_eq!(
                (quote.cost.coin, quote.cost.food, quote.cost.influence),
                (coin, food, influence)
            );
        }
    }
}
