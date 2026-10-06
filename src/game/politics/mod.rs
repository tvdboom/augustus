//! Provincial diplomacy, espionage and the shared Roman Senate.
//!
//! These deterministic, renderer-independent rules consume explicit input snapshots.
//! The economy remains authoritative for wallets; callers copy balances into the
//! political actors before actions and reconcile them afterwards. No passive income
//! is silently added here: the monthly income functions return amounts to the economy.

pub mod diplomacy;
pub mod espionage;
pub mod senate;

/// Stable player index shared with the local match and economy.
pub type PlayerId = usize;
/// Stable province index shared with the historical map.
pub type ProvinceId = usize;

/// The six-rank political ladder from Quaestor to Augustus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum PoliticalRank {
    /// Initial office, retained from the existing game.
    #[default]
    Quaestor,
    /// First promotion, requiring Influence and loyal senators.
    Aedile,
    /// Support-gated office after Aedile.
    Praetor,
    /// Support-gated office between Praetor and Consul.
    Censor,
    /// Highest republican rank before Augustus.
    Consul,
    /// Winning office, awarded by Senate support or the conquest of Rome.
    Augustus,
}

impl PoliticalRank {
    /// Name used in explanatory UI.
    pub fn label(self) -> &'static str {
        match self {
            Self::Quaestor => "Quaestor",
            Self::Aedile => "Aedile",
            Self::Praetor => "Praetor",
            Self::Censor => "Censor",
            Self::Consul => "Consul",
            Self::Augustus => "Augustus",
        }
    }

    /// Slot in the original six-rank artwork.
    pub fn ladder_index(self) -> usize {
        match self {
            Self::Quaestor => 0,
            Self::Aedile => 1,
            Self::Praetor => 2,
            Self::Censor => 3,
            Self::Consul => 4,
            Self::Augustus => 5,
        }
    }
}

/// Mutable political actor; coin and Influence mirror the authoritative economy.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PoliticalPlayer {
    /// Current office; bonuses do not stack with earlier offices.
    pub rank: PoliticalRank,
    /// Available coin.
    pub coin: f64,
    /// Available Influence.
    pub influence: f64,
    /// Last promotion month; only one rank may be gained each month.
    pub promoted_at: Option<u32>,
}

impl PoliticalPlayer {
    /// Atomically spend a finite, nonnegative amount; never create debt.
    pub fn spend(&mut self, currency: Currency, amount: f64) -> Result<(), PoliticalError> {
        if !amount.is_finite() || amount < 0.0 {
            return Err(PoliticalError::InvalidAmount);
        }
        let balance = match currency {
            Currency::Coin => &mut self.coin,
            Currency::Influence => &mut self.influence,
        };
        if *balance + 1e-9 < amount {
            return Err(PoliticalError::InsufficientFunds);
        }
        *balance = (*balance - amount).max(0.0);
        Ok(())
    }
}

/// The two spendable political currencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Currency {
    /// Economic wealth.
    Coin,
    /// Political capital.
    Influence,
}

/// User-facing reasons why a requested political action cannot proceed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PoliticalError {
    /// Amount was negative, infinite, or not a number.
    InvalidAmount,
    /// Wallet cannot cover this action.
    InsufficientFunds,
    /// Player, province, scandal or campaign was not present.
    MissingTarget,
    /// The province state or rank does not permit this action.
    Ineligible,
    /// An action limited by its cooldown has already been performed.
    AlreadyUsed,
    /// The campaign has already been won.
    CampaignFinished,
    /// Discrediting needs an existing rival patron; neutral senators have none.
    SenatorIsNeutral,
    /// Discrediting cannot target a senator's allegiance to the acting player.
    RivalPatronRequired,
    /// A player can maintain only one ongoing action on each senator.
    SenatorArrangementActive {
        /// Existing relationship blocking the requested action.
        action: senate::SenatorAction,
        /// Number of months left; ignored for ongoing lobbying.
        months_remaining: u32,
    },
    /// This action requires unexpired evidence against the target.
    ScandalRequired,
    /// Too few senators currently support this player.
    InsufficientSupport,
}

impl std::fmt::Display for PoliticalError {
    /// Render an actionable explanation suitable for disabled-button tooltips.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Self::SenatorArrangementActive {
            action,
            months_remaining,
        } = self
        {
            return if *action == senate::SenatorAction::Lobby {
                f.write_str(
                    "You are already lobbying this senator. Cancel it using the card's cross.",
                )
            } else if *action == senate::SenatorAction::Bribe {
                f.write_str(
                    "You are already bribing this senator. Cancel it using the card's cross.",
                )
            } else {
                let months = if *months_remaining == 1 {
                    "month"
                } else {
                    "months"
                };
                write!(f, "Active {}: {months_remaining} {months} remaining. Cancel it using the card's cross.", match action {
                    senate::SenatorAction::Patronage => "patronage",
                    _ => "coercion",
                })
            };
        }
        f.write_str(match self {
            Self::InvalidAmount => "Choose a finite, nonnegative amount.",
            Self::InsufficientFunds => "Insufficient funds for this action.",
            Self::MissingTarget => "The selected target no longer exists.",
            Self::Ineligible => "Your rank or this province blocks the action.",
            Self::AlreadyUsed => "This action is still on cooldown.",
            Self::CampaignFinished => "The campaign has ended.",
            Self::SenatorIsNeutral => "This senator is neutral and has no patron. Select a senator who supports another player.",
            Self::RivalPatronRequired => "This senator supports you. Select a senator who supports another player.",
            Self::SenatorArrangementActive { .. } => unreachable!(),
            Self::ScandalRequired => "An unexpired scandal against this player is required.",
            Self::InsufficientSupport => {
                "Attract the required number of senators before seeking this office."
            },
        })
    }
}

/// Reproducible authoritative random stream; clients render the recorded outcomes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PoliticalRng(u64);

impl PoliticalRng {
    /// Construct from the match seed, not wall-clock time or connection order.
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }
    /// Draw a uniform number in `[0,1)` using SplitMix64.
    pub fn unit(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^= z >> 31;
        (z >> 11) as f64 / ((1_u64 << 53) as f64)
    }
}
