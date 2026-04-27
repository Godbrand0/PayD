#![no_std]
#![allow(clippy::too_many_arguments)]
use soroban_sdk::{
    Address, Env, String, contract, contractevent, contractimpl, contracttype, token,
};

#[contracttype]
#[derive(Clone)]
pub struct VestingConfig {
    pub beneficiary: Address,
    pub token: Address,
    pub start_time: u64,
    pub cliff_seconds: u64,
    pub duration_seconds: u64,
    pub total_amount: i128,
    pub claimed_amount: i128,
    pub clawback_admin: Address,
    pub is_active: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct VestingSnapshot {
    pub timestamp: u64,
    pub vested_amount: i128,
    pub claimable_amount: i128,
    pub locked_amount: i128,
    pub claimed_amount: i128,
    pub total_amount: i128,
    pub progress_bps: u32,
    pub is_active: bool,
}

#[contracttype]
pub enum DataKey {
    Config,
    /// Tracks the last ledger sequence in which a claim was processed.
    LastClaimLedger,
    /// Tracks the last ledger sequence in which a clawback was processed.
    LastClawbackLedger,
}

// ── Events ────────────────────────────────────────────────────────────────────

/// Emitted when the vesting escrow is successfully funded and configured.
#[contractevent]
pub struct VestingInitializedEvent {
    pub beneficiary: Address,
    pub token: Address,
    pub total_amount: i128,
    pub cliff_seconds: u64,
    pub duration_seconds: u64,
    pub start_time: u64,
}

/// Emitted when the beneficiary successfully claims vested tokens.
#[contractevent]
pub struct TokensClaimedEvent {
    pub beneficiary: Address,
    pub amount: i128,
    pub total_claimed: i128,
}

/// Emitted when the clawback admin terminates the grant early.
#[contractevent]
pub struct ClawbackExecutedEvent {
    pub clawback_admin: Address,
    pub unvested_returned: i128,
    pub vested_remaining: i128,
}

/// Emitted when the beneficiary address is transferred to a new account.
#[contractevent]
pub struct BeneficiaryTransferredEvent {
    pub old_beneficiary: Address,
    pub new_beneficiary: Address,
}

const PERSISTENT_TTL_THRESHOLD: u32 = 20_000;
const PERSISTENT_TTL_EXTEND_TO: u32 = 120_000;
const BASIS_POINTS_DENOMINATOR: u32 = 10_000;

#[contract]
pub struct VestingContract;

#[contractimpl]
impl VestingContract {
    // ── SEP-0034 Contract Metadata (Issue #263) ───────────────────────────

    /// Returns the human-readable contract name (SEP-0034).
    pub fn name(env: Env) -> String {
        String::from_str(&env, env!("CARGO_PKG_NAME"))
    }

    /// Returns the contract version string (SEP-0034).
    pub fn version(env: Env) -> String {
        String::from_str(&env, env!("CARGO_PKG_VERSION"))
    }

    /// Returns the contract author / organization (SEP-0034).
    pub fn author(env: Env) -> String {
        String::from_str(&env, env!("CARGO_PKG_AUTHORS"))
    }

    /// Funds and initializes the vesting escrow.
    ///
    /// This function can only be called once. The `funder` authorizes the
    /// transfer of `amount` tokens into the contract, after which the grant
    /// becomes claimable according to the configured cliff and duration.
    pub fn initialize(
        e: Env,
        funder: Address,
        beneficiary: Address,
        token: Address,
        start_time: u64,
        cliff_seconds: u64,
        duration_seconds: u64,
        amount: i128,
        clawback_admin: Address,
    ) {
        if e.storage().persistent().has(&DataKey::Config) {
            panic!("Already initialized");
        }

        funder.require_auth();

        if duration_seconds < cliff_seconds {
            panic!("Duration must be greater than or equal to cliff");
        }

        if amount <= 0 {
            panic!("Amount must be positive");
        }

        let config = VestingConfig {
            beneficiary: beneficiary.clone(),
            token: token.clone(),
            start_time,
            cliff_seconds,
            duration_seconds,
            total_amount: amount,
            claimed_amount: 0,
            clawback_admin,
            is_active: true,
        };

        e.storage().persistent().set(&DataKey::Config, &config);
        Self::bump_config_ttl(&e);

        // Transfer tokens from funder to contract
        let client = token::Client::new(&e, &token);
        client.transfer(&funder, e.current_contract_address(), &amount);

        VestingInitializedEvent {
            beneficiary,
            token,
            total_amount: amount,
            cliff_seconds,
            duration_seconds,
            start_time,
        }
        .publish(&e);
    }

    /// Claims all currently vested and unclaimed tokens for the beneficiary.
    ///
    /// The beneficiary must authorize the call. If no new tokens have vested,
    /// the function is a no-op.
    pub fn claim(e: Env) {
        let mut config: VestingConfig = e
            .storage()
            .persistent()
            .get(&DataKey::Config)
            .expect("Config entry unavailable; restore and retry");

        config.beneficiary.require_auth();

        // Ledger sequence verification: prevent duplicate claims in the same ledger
        Self::require_unique_ledger(&e, &DataKey::LastClaimLedger);

        let vested = Self::calc_vested(&e, &config);
        let claimable = Self::calc_claimable(vested, config.claimed_amount);

        if claimable <= 0 {
            // Nothing to claim, just return
            return;
        }

        // Update state
        config.claimed_amount += claimable;
        e.storage().persistent().set(&DataKey::Config, &config);
        Self::bump_config_ttl(&e);

        // Transfer tokens
        let client = token::Client::new(&e, &config.token);
        client.transfer(
            &e.current_contract_address(),
            &config.beneficiary,
            &claimable,
        );

        TokensClaimedEvent {
            beneficiary: config.beneficiary,
            amount: claimable,
            total_claimed: config.claimed_amount,
        }
        .publish(&e);
    }

    /// Terminates future vesting and returns unvested tokens to the admin.
    ///
    /// Already vested but unclaimed tokens remain in escrow for the beneficiary.
    /// The clawback admin must authorize the call.
    pub fn clawback(e: Env) {
        let mut config: VestingConfig = e
            .storage()
            .persistent()
            .get(&DataKey::Config)
            .expect("Config entry unavailable; restore and retry");

        config.clawback_admin.require_auth();

        // Ledger sequence verification: prevent duplicate clawback in the same ledger
        Self::require_unique_ledger(&e, &DataKey::LastClawbackLedger);

        if !config.is_active {
            panic!("Already revoked/inactive");
        }

        // Calculate what has vested so far
        let vested = Self::calc_vested(&e, &config);
        let vested_floor = Self::max_i128(vested, config.claimed_amount);

        // The unvested amount is the total scheduled minus what has vested.
        // Never reduce the grant below what was already claimed; this makes
        // the operation conservative even under unusual local-test timestamp
        // manipulation.
        let unvested = config.total_amount - vested_floor;

        // Update config to stop future vesting
        // We set total_amount to vested, so effectively the grant is capped at what was vested at this moment
        config.total_amount = vested_floor;
        config.is_active = false;
        e.storage().persistent().set(&DataKey::Config, &config);
        Self::bump_config_ttl(&e);

        if unvested > 0 {
            // Return unvested tokens to admin
            let client = token::Client::new(&e, &config.token);
            client.transfer(
                &e.current_contract_address(),
                &config.clawback_admin,
                &unvested,
            );
        }

        // vested_remaining = vested - already_claimed (still held in contract for beneficiary)
        let vested_remaining = Self::calc_claimable(vested_floor, config.claimed_amount);
        ClawbackExecutedEvent {
            clawback_admin: config.clawback_admin,
            unvested_returned: unvested,
            vested_remaining,
        }
        .publish(&e);
    }

    /// Returns the amount that has vested at the current ledger timestamp.
    pub fn get_vested_amount(e: Env) -> i128 {
        let config: VestingConfig = e
            .storage()
            .persistent()
            .get(&DataKey::Config)
            .expect("Config entry unavailable; restore and retry");
        // Reading state should not modify TTL; extend only on write
        Self::calc_vested(&e, &config)
    }

    /// Returns the amount that is vested and not yet claimed.
    pub fn get_claimable_amount(e: Env) -> i128 {
        let config: VestingConfig = e
            .storage()
            .persistent()
            .get(&DataKey::Config)
            .expect("Config entry unavailable; restore and retry");
        // Reading state should not modify TTL; extend only on write
        let vested = Self::calc_vested(&e, &config);
        Self::calc_claimable(vested, config.claimed_amount)
    }

    /// Returns the current escrow configuration.
    pub fn get_config(e: Env) -> VestingConfig {
        let config: VestingConfig = e
            .storage()
            .persistent()
            .get(&DataKey::Config)
            .expect("Config entry unavailable; restore and retry");
        // Reading state should not modify TTL; extend only on write
        config
    }

    /// Returns the amount still held by the escrow contract for this grant.
    ///
    /// This includes vested-but-unclaimed tokens and, while the grant is active,
    /// future unvested tokens. It never returns a negative value.
    pub fn get_locked_amount(e: Env) -> i128 {
        let config: VestingConfig = e
            .storage()
            .persistent()
            .get(&DataKey::Config)
            .expect("Config entry unavailable; restore and retry");
        Self::calc_locked(config.total_amount, config.claimed_amount)
    }

    /// Returns the vested amount at an arbitrary timestamp without changing state.
    pub fn preview_vested_amount(e: Env, timestamp: u64) -> i128 {
        let config: VestingConfig = e
            .storage()
            .persistent()
            .get(&DataKey::Config)
            .expect("Config entry unavailable; restore and retry");
        Self::calc_vested_at(timestamp, &config)
    }

    /// Returns current vesting progress in basis points where 10_000 is 100%.
    pub fn get_vesting_progress_bps(e: Env) -> u32 {
        let config: VestingConfig = e
            .storage()
            .persistent()
            .get(&DataKey::Config)
            .expect("Config entry unavailable; restore and retry");
        let vested = Self::calc_vested(&e, &config);
        Self::calc_progress_bps(vested, config.total_amount)
    }

    /// Returns a compact read-only snapshot of the current escrow state.
    pub fn get_vesting_snapshot(e: Env) -> VestingSnapshot {
        let config: VestingConfig = e
            .storage()
            .persistent()
            .get(&DataKey::Config)
            .expect("Config entry unavailable; restore and retry");
        let timestamp = e.ledger().timestamp();
        let vested_amount = Self::calc_vested_at(timestamp, &config);
        let claimable_amount = Self::calc_claimable(vested_amount, config.claimed_amount);
        let locked_amount = Self::calc_locked(config.total_amount, config.claimed_amount);

        VestingSnapshot {
            timestamp,
            vested_amount,
            claimable_amount,
            locked_amount,
            claimed_amount: config.claimed_amount,
            total_amount: config.total_amount,
            progress_bps: Self::calc_progress_bps(vested_amount, config.total_amount),
            is_active: config.is_active,
        }
    }

    /// Transfers the vesting grant to a new beneficiary address. Only the
    /// `clawback_admin` may call this (e.g. to handle account migration).
    /// The new beneficiary inherits all unclaimed vested and future tokens.
    pub fn transfer_beneficiary(e: Env, new_beneficiary: Address) {
        let mut config: VestingConfig = e
            .storage()
            .persistent()
            .get(&DataKey::Config)
            .expect("Config entry unavailable; restore and retry");

        config.clawback_admin.require_auth();

        if !config.is_active {
            panic!("Vesting grant is no longer active");
        }

        let old_beneficiary = config.beneficiary.clone();
        config.beneficiary = new_beneficiary.clone();
        e.storage().persistent().set(&DataKey::Config, &config);
        Self::bump_config_ttl(&e);

        BeneficiaryTransferredEvent {
            old_beneficiary,
            new_beneficiary,
        }
        .publish(&e);
    }

    /// Extends TTL for the vesting configuration entry.
    pub fn bump_ttl(e: Env) {
        let config: VestingConfig = e
            .storage()
            .persistent()
            .get(&DataKey::Config)
            .expect("Config entry unavailable; restore and retry");
        config.clawback_admin.require_auth();
        Self::bump_config_ttl(&e);
    }

    fn calc_vested(e: &Env, config: &VestingConfig) -> i128 {
        Self::calc_vested_at(e.ledger().timestamp(), config)
    }

    fn calc_vested_at(now: u64, config: &VestingConfig) -> i128 {
        let cliff_at = config.start_time.saturating_add(config.cliff_seconds);
        let end_at = config.start_time.saturating_add(config.duration_seconds);

        if now < cliff_at {
            return 0;
        }

        if now >= end_at || !config.is_active {
            return config.total_amount;
        }

        // Linear vesting — overflow-safe: divide total by duration first,
        // then scale by elapsed to avoid intermediate multiplication overflow.
        let time_elapsed = now.saturating_sub(config.start_time);
        let total = config.total_amount;
        let elapsed = time_elapsed as u128;
        let duration = config.duration_seconds as u128;

        // vested = (total / duration) * elapsed + (total % duration) * elapsed / duration
        let duration_i128 = config.duration_seconds as i128;
        let per_unit = total / duration_i128;
        let remainder = (total % duration_i128) as u128;
        let remainder_component = ((remainder * elapsed) / duration) as i128;
        per_unit * time_elapsed as i128 + remainder_component
    }

    fn calc_claimable(vested: i128, claimed: i128) -> i128 {
        if vested <= claimed {
            0
        } else {
            vested - claimed
        }
    }

    fn calc_locked(total: i128, claimed: i128) -> i128 {
        if total <= claimed { 0 } else { total - claimed }
    }

    fn calc_progress_bps(vested: i128, total: i128) -> u32 {
        if total <= 0 || vested <= 0 {
            return 0;
        }
        if vested >= total {
            return BASIS_POINTS_DENOMINATOR;
        }

        let multiplier = BASIS_POINTS_DENOMINATOR as i128;
        match vested.checked_mul(multiplier) {
            Some(scaled) => (scaled / total) as u32,
            None => {
                let units = total / multiplier;
                if units <= 0 {
                    return 0;
                }
                let coarse = vested / units;
                if coarse >= multiplier {
                    BASIS_POINTS_DENOMINATOR
                } else {
                    coarse as u32
                }
            }
        }
    }

    fn max_i128(lhs: i128, rhs: i128) -> i128 {
        if lhs >= rhs { lhs } else { rhs }
    }

    /// Returns the ledger sequence of the last successful claim.
    pub fn get_last_claim_ledger(e: Env) -> u32 {
        e.storage()
            .persistent()
            .get(&DataKey::LastClaimLedger)
            .unwrap_or(0)
    }

    /// Returns the ledger sequence of the last successful clawback.
    pub fn get_last_clawback_ledger(e: Env) -> u32 {
        e.storage()
            .persistent()
            .get(&DataKey::LastClawbackLedger)
            .unwrap_or(0)
    }

    /// Ensures the operation has not already been executed in the current ledger
    /// sequence, preventing replay attacks. Records the current ledger on success.
    fn require_unique_ledger(e: &Env, key: &DataKey) {
        let current_ledger = e.ledger().sequence();
        let last_ledger: u32 = e.storage().persistent().get(key).unwrap_or(0);
        if last_ledger == current_ledger && current_ledger != 0 {
            panic!("Operation already processed in this ledger sequence");
        }
        e.storage().persistent().set(key, &current_ledger);
        e.storage().persistent().extend_ttl(
            key,
            PERSISTENT_TTL_THRESHOLD,
            PERSISTENT_TTL_EXTEND_TO,
        );
    }

    fn bump_config_ttl(e: &Env) {
        if e.storage().persistent().has(&DataKey::Config) {
            e.storage().persistent().extend_ttl(
                &DataKey::Config,
                PERSISTENT_TTL_THRESHOLD,
                PERSISTENT_TTL_EXTEND_TO,
            );
        }
    }
}

#[cfg(test)]
mod test;

#[cfg(test)]
mod test_escrow_logic;
