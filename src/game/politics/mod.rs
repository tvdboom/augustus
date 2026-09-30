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

/// The preserved visible political ladder, plus the status of an expired Consul.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
    /// One of the two time-limited highest republican offices.
    Consul,
    /// Former Consul; must be reappointed before seeking Augustus.
    Proconsul,
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
            Self::Proconsul => "Proconsul",
            Self::Augustus => "Augustus",
        }
    }

    /// Slot in the original six-rank artwork; Proconsul shares the Consul slot.
    pub fn ladder_index(self) -> usize {
        match self {
            Self::Quaestor => 0,
            Self::Aedile => 1,
            Self::Praetor => 2,
            Self::Censor => 3,
            Self::Consul | Self::Proconsul => 4,
            Self::Augustus => 5,
        }
    }
}

/// Mutable political actor; coin and Influence mirror the authoritative economy.
#[derive(Debug, Clone, Default)]
pub struct PoliticalPlayer {
    /// Current office; bonuses do not stack with earlier offices.
    pub rank: PoliticalRank,
    /// Available coin.
    pub coin: f64,
    /// Available Influence.
    pub influence: f64,
    /// Exclusive expiry month of a 24-month Consul term.
    pub consul_until: Option<u32>,
    /// Earliest month a former Consul may regain a seat.
    pub consul_again_at: u32,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Currency {
    /// Economic wealth.
    Coin,
    /// Political capital.
    Influence,
}

/// User-facing reasons why a requested political action cannot proceed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// Both Consul seats are occupied at the time of appointment.
    NoConsulSeat,
    /// This action requires unexpired evidence against the target.
    ScandalRequired,
    /// Too few senators currently support this player.
    InsufficientSupport,
    /// A former Consul must wait at least a year.
    ConsulCooldown,
}

impl std::fmt::Display for PoliticalError {
    /// Render an actionable explanation suitable for disabled-button tooltips.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidAmount => "Choose a finite, nonnegative amount.",
            Self::InsufficientFunds => "Insufficient funds for this action.",
            Self::MissingTarget => "The selected target no longer exists.",
            Self::Ineligible => "The current office or province state does not permit this action.",
            Self::AlreadyUsed => "This action is still on cooldown.",
            Self::NoConsulSeat => "Both Consul seats are occupied.",
            Self::ScandalRequired => "An unexpired scandal against this player is required.",
            Self::InsufficientSupport => {
                "Attract the required number of senators before seeking this office."
            },
            Self::ConsulCooldown => {
                "A former Consul must wait 12 months before returning to office."
            },
        })
    }
}

/// Reproducible authoritative random stream; clients render the recorded outcomes.
#[derive(Debug, Clone)]
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
