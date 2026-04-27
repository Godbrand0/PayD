use super::*;
use soroban_sdk::{
    Address, Env,
    testutils::{Address as _, Ledger},
    token,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn setup() -> (
    Env,
    Address,                            // funder
    Address,                            // beneficiary
    Address,                            // clawback admin
    Address,                            // token contract address
    token::Client<'static>,             // token client
    token::StellarAssetClient<'static>, // token admin client
    VestingContractClient<'static>,     // vesting contract client
) {
    let e = Env::default();
    e.mock_all_auths();

    let funder = Address::generate(&e);
    let beneficiary = Address::generate(&e);
    let clawback_admin = Address::generate(&e);
    let contract_id = e.register(VestingContract, ());
    let client = VestingContractClient::new(&e, &contract_id);

    let token_admin = Address::generate(&e);
    let token_contract = e
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::Client::new(&e, &token_contract);
    let token_admin_client = token::StellarAssetClient::new(&e, &token_contract);

    token_admin_client.mint(&funder, &100_000);

    (
        e,
        funder,
        beneficiary,
        clawback_admin,
        token_contract,
        token_client,
        token_admin_client,
        client,
    )
}

fn init_default(
    client: &VestingContractClient,
    e: &Env,
    funder: &Address,
    beneficiary: &Address,
    token: &Address,
    clawback_admin: &Address,
) {
    let start_time = e.ledger().timestamp();
    client.initialize(
        funder,
        beneficiary,
        token,
        &start_time,
        &100,   // cliff_seconds
        &1000,  // duration_seconds
        &10000, // amount
        clawback_admin,
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// ── INITIALIZATION TESTS ──────────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_initialize_success() {
    let (e, funder, beneficiary, clawback_admin, token_contract, token_client, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );

    let config = client.get_config();
    assert_eq!(config.beneficiary, beneficiary);
    assert_eq!(config.token, token_contract);
    assert_eq!(config.total_amount, 10000);
    assert_eq!(config.claimed_amount, 0);
    assert!(config.is_active);
    assert_eq!(config.cliff_seconds, 100);
    assert_eq!(config.duration_seconds, 1000);
    assert_eq!(config.clawback_admin, clawback_admin);

    // Tokens transferred from funder to contract
    assert_eq!(token_client.balance(&funder), 90_000);
}

#[test]
#[should_panic(expected = "Already initialized")]
fn test_initialize_twice_panics() {
    let (e, funder, beneficiary, clawback_admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );
    // Second initialization should fail
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );
}

#[test]
#[should_panic(expected = "Duration must be greater than or equal to cliff")]
fn test_initialize_cliff_greater_than_duration_panics() {
    let (e, funder, beneficiary, clawback_admin, token_contract, _, _, client) = setup();
    let start_time = e.ledger().timestamp();
    client.initialize(
        &funder,
        &beneficiary,
        &token_contract,
        &start_time,
        &500, // cliff > duration
        &100, // duration
        &1000,
        &clawback_admin,
    );
}

#[test]
#[should_panic(expected = "Amount must be positive")]
fn test_initialize_zero_amount_panics() {
    let (e, funder, beneficiary, clawback_admin, token_contract, _, _, client) = setup();
    let start_time = e.ledger().timestamp();
    client.initialize(
        &funder,
        &beneficiary,
        &token_contract,
        &start_time,
        &100,
        &1000,
        &0, // zero amount
        &clawback_admin,
    );
}

#[test]
#[should_panic(expected = "Amount must be positive")]
fn test_initialize_negative_amount_panics() {
    let (e, funder, beneficiary, clawback_admin, token_contract, _, _, client) = setup();
    let start_time = e.ledger().timestamp();
    client.initialize(
        &funder,
        &beneficiary,
        &token_contract,
        &start_time,
        &100,
        &1000,
        &-100, // negative amount
        &clawback_admin,
    );
}

#[test]
fn test_initialize_cliff_equals_duration() {
    let (e, funder, beneficiary, clawback_admin, token_contract, _, _, client) = setup();
    let start_time = e.ledger().timestamp();
    // cliff == duration should succeed (all tokens vest at once after cliff)
    client.initialize(
        &funder,
        &beneficiary,
        &token_contract,
        &start_time,
        &1000,
        &1000,
        &5000,
        &clawback_admin,
    );
    let config = client.get_config();
    assert_eq!(config.cliff_seconds, 1000);
    assert_eq!(config.duration_seconds, 1000);
}

#[test]
fn test_initialize_zero_cliff() {
    let (e, funder, beneficiary, clawback_admin, token_contract, _, _, client) = setup();
    let start_time = e.ledger().timestamp();
    // Zero cliff should succeed — tokens begin vesting immediately
    client.initialize(
        &funder,
        &beneficiary,
        &token_contract,
        &start_time,
        &0,
        &1000,
        &5000,
        &clawback_admin,
    );
    let config = client.get_config();
    assert_eq!(config.cliff_seconds, 0);
}

// ══════════════════════════════════════════════════════════════════════════════
// ── VESTING CALCULATION TESTS ─────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_vested_amount_before_cliff() {
    let (e, funder, beneficiary, clawback_admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );

    // Before cliff (< start + 100), nothing is vested
    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 50);
    assert_eq!(client.get_vested_amount(), 0);
    assert_eq!(client.get_claimable_amount(), 0);
}

#[test]
fn test_vested_amount_at_cliff_boundary() {
    let (e, funder, beneficiary, clawback_admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );

    let start = e.ledger().timestamp();
    // Exactly at cliff (start + 100): 100/1000 = 10% = 1000 tokens
    e.ledger().set_timestamp(start + 100);
    assert_eq!(client.get_vested_amount(), 1000);
}

#[test]
fn test_vested_amount_linear_progression() {
    let (e, funder, beneficiary, clawback_admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );

    let start = e.ledger().timestamp();

    // 25%
    e.ledger().set_timestamp(start + 250);
    assert_eq!(client.get_vested_amount(), 2500);

    // 50%
    e.ledger().set_timestamp(start + 500);
    assert_eq!(client.get_vested_amount(), 5000);

    // 75%
    e.ledger().set_timestamp(start + 750);
    assert_eq!(client.get_vested_amount(), 7500);
}

#[test]
fn test_vested_amount_at_duration_end() {
    let (e, funder, beneficiary, clawback_admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 1000);
    assert_eq!(client.get_vested_amount(), 10000);
}

#[test]
fn test_vested_amount_after_duration_end() {
    let (e, funder, beneficiary, clawback_admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );

    let start = e.ledger().timestamp();
    // Well past duration — should cap at total
    e.ledger().set_timestamp(start + 5000);
    assert_eq!(client.get_vested_amount(), 10000);
}

// ══════════════════════════════════════════════════════════════════════════════
// ── CLAIM TESTS ───────────────────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_claim_before_cliff_does_nothing() {
    let (e, funder, beneficiary, clawback_admin, token_contract, token_client, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 50);

    // Claim should be a no-op before cliff
    client.claim();
    assert_eq!(token_client.balance(&beneficiary), 0);
    let config = client.get_config();
    assert_eq!(config.claimed_amount, 0);
}

#[test]
fn test_claim_partial_vesting() {
    let (e, funder, beneficiary, clawback_admin, token_contract, token_client, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 200);

    // 200/1000 = 20% = 2000 tokens
    client.claim();
    assert_eq!(token_client.balance(&beneficiary), 2000);
    let config = client.get_config();
    assert_eq!(config.claimed_amount, 2000);
}

#[test]
fn test_claim_multiple_partial_claims() {
    let (e, funder, beneficiary, clawback_admin, token_contract, token_client, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );

    let start = e.ledger().timestamp();

    // First claim at 20%
    e.ledger().set_timestamp(start + 200);
    e.ledger().set_sequence_number(10);
    client.claim();
    assert_eq!(token_client.balance(&beneficiary), 2000);

    // Second claim at 50% — should only get the delta (3000)
    e.ledger().set_timestamp(start + 500);
    e.ledger().set_sequence_number(20);
    client.claim();
    assert_eq!(token_client.balance(&beneficiary), 5000);

    // Third claim at 100%
    e.ledger().set_timestamp(start + 1000);
    e.ledger().set_sequence_number(30);
    client.claim();
    assert_eq!(token_client.balance(&beneficiary), 10000);
}

#[test]
fn test_claim_full_after_duration() {
    let (e, funder, beneficiary, clawback_admin, token_contract, token_client, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 2000);

    client.claim();
    assert_eq!(token_client.balance(&beneficiary), 10000);
    let config = client.get_config();
    assert_eq!(config.claimed_amount, 10000);
}

#[test]
fn test_claim_idempotent_when_nothing_new() {
    let (e, funder, beneficiary, clawback_admin, token_contract, token_client, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 200);
    e.ledger().set_sequence_number(10);
    client.claim();
    assert_eq!(token_client.balance(&beneficiary), 2000);

    // Same timestamp, different ledger — no new tokens vested
    e.ledger().set_sequence_number(11);
    client.claim();
    assert_eq!(token_client.balance(&beneficiary), 2000);
}

// ══════════════════════════════════════════════════════════════════════════════
// ── CLAWBACK TESTS ────────────────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_clawback_before_any_vesting() {
    let (e, funder, beneficiary, clawback_admin, token_contract, token_client, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );

    // Clawback immediately (before cliff) — all tokens go back to admin
    client.clawback();

    assert_eq!(token_client.balance(&clawback_admin), 10000);
    let config = client.get_config();
    assert!(!config.is_active);
    assert_eq!(config.total_amount, 0);
}

#[test]
fn test_clawback_partial_vesting() {
    let (e, funder, beneficiary, clawback_admin, token_contract, token_client, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );

    let start = e.ledger().timestamp();
    // 30% vested = 3000 tokens
    e.ledger().set_timestamp(start + 300);

    client.clawback();

    // Admin gets unvested (7000)
    assert_eq!(token_client.balance(&clawback_admin), 7000);
    let config = client.get_config();
    assert!(!config.is_active);
    assert_eq!(config.total_amount, 3000); // Capped at vested
}

#[test]
fn test_clawback_then_beneficiary_can_claim_vested() {
    let (e, funder, beneficiary, clawback_admin, token_contract, token_client, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 400);
    e.ledger().set_sequence_number(10);

    // Clawback at 40% — admin gets 6000 back
    client.clawback();
    assert_eq!(token_client.balance(&clawback_admin), 6000);

    // Beneficiary can still claim the 4000 vested tokens
    e.ledger().set_timestamp(start + 2000); // well past duration
    e.ledger().set_sequence_number(20);
    client.claim();
    assert_eq!(token_client.balance(&beneficiary), 4000);
}

#[test]
#[should_panic(expected = "Already revoked/inactive")]
fn test_clawback_twice_panics() {
    let (e, funder, beneficiary, clawback_admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );

    e.ledger().set_sequence_number(10);
    client.clawback();
    e.ledger().set_sequence_number(11);
    client.clawback(); // Should panic
}

#[test]
fn test_clawback_after_full_vesting() {
    let (e, funder, beneficiary, clawback_admin, token_contract, token_client, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 2000);

    // All tokens are vested — clawback returns nothing to admin
    client.clawback();
    assert_eq!(token_client.balance(&clawback_admin), 0);

    let config = client.get_config();
    assert!(!config.is_active);
    assert_eq!(config.total_amount, 10000); // Fully vested
}

#[test]
fn test_clawback_after_partial_claim() {
    let (e, funder, beneficiary, clawback_admin, token_contract, token_client, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );

    let start = e.ledger().timestamp();

    // Claim 20%
    e.ledger().set_timestamp(start + 200);
    e.ledger().set_sequence_number(10);
    client.claim();
    assert_eq!(token_client.balance(&beneficiary), 2000);

    // Advance to 50% then clawback
    e.ledger().set_timestamp(start + 500);
    e.ledger().set_sequence_number(20);
    client.clawback();

    // Admin gets 5000 unvested, contract holds 3000 (5000 vested - 2000 claimed)
    assert_eq!(token_client.balance(&clawback_admin), 5000);

    // Beneficiary can claim remaining 3000
    e.ledger().set_timestamp(start + 2000);
    e.ledger().set_sequence_number(30);
    client.claim();
    assert_eq!(token_client.balance(&beneficiary), 5000); // 2000 + 3000
}

// ══════════════════════════════════════════════════════════════════════════════
// ── TOKEN BALANCE INVARIANT TESTS ─────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_contract_balance_after_full_claim() {
    let (e, funder, beneficiary, clawback_admin, token_contract, token_client, _, client) = setup();
    let contract_id = client.address.clone();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 2000);

    client.claim();

    // Contract should hold zero tokens
    assert_eq!(token_client.balance(&contract_id), 0);
    assert_eq!(token_client.balance(&beneficiary), 10000);
}

#[test]
fn test_contract_balance_after_clawback_and_claim() {
    let (e, funder, beneficiary, clawback_admin, token_contract, token_client, _, client) = setup();
    let contract_id = client.address.clone();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );

    let start = e.ledger().timestamp();

    // Clawback at 60%
    e.ledger().set_timestamp(start + 600);
    e.ledger().set_sequence_number(10);
    client.clawback();

    // Contract: 10000 - 4000 (unvested returned) = 6000 remaining
    assert_eq!(token_client.balance(&contract_id), 6000);
    assert_eq!(token_client.balance(&clawback_admin), 4000);

    // Beneficiary claims all 6000 vested
    e.ledger().set_timestamp(start + 2000);
    e.ledger().set_sequence_number(20);
    client.claim();

    // Contract should be empty
    assert_eq!(token_client.balance(&contract_id), 0);
    assert_eq!(token_client.balance(&beneficiary), 6000);

    // Invariant: funder_initial = beneficiary_final + admin_final + funder_remaining
    // 100000 = 6000 + 4000 + 90000 ✓
}

// ══════════════════════════════════════════════════════════════════════════════
// ── LEDGER SEQUENCE VERIFICATION TESTS (Issue #173) ───────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
#[should_panic(expected = "Operation already processed in this ledger sequence")]
fn test_claim_replay_same_ledger() {
    let (e, funder, beneficiary, clawback_admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 500);
    e.ledger().set_sequence_number(10);

    client.claim();
    // Second claim in the same ledger should panic
    client.claim();
}

#[test]
fn test_claim_allowed_different_ledgers() {
    let (e, funder, beneficiary, clawback_admin, token_contract, token_client, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );

    let start = e.ledger().timestamp();

    e.ledger().set_timestamp(start + 200);
    e.ledger().set_sequence_number(10);
    client.claim();
    assert_eq!(client.get_last_claim_ledger(), 10);

    e.ledger().set_timestamp(start + 500);
    e.ledger().set_sequence_number(20);
    client.claim();
    assert_eq!(client.get_last_claim_ledger(), 20);
    assert_eq!(token_client.balance(&beneficiary), 5000);
}

// ══════════════════════════════════════════════════════════════════════════════
// ── EDGE CASE TESTS ───────────────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_zero_cliff_immediate_vesting() {
    let e = Env::default();
    e.mock_all_auths();

    let funder = Address::generate(&e);
    let beneficiary = Address::generate(&e);
    let clawback_admin = Address::generate(&e);
    let contract_id = e.register(VestingContract, ());
    let client = VestingContractClient::new(&e, &contract_id);

    let token_admin = Address::generate(&e);
    let token_contract = e
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_admin_client = token::StellarAssetClient::new(&e, &token_contract);
    token_admin_client.mint(&funder, &10000);

    let start_time = e.ledger().timestamp();
    client.initialize(
        &funder,
        &beneficiary,
        &token_contract,
        &start_time,
        &0, // zero cliff
        &1000,
        &10000,
        &clawback_admin,
    );

    // At time = start + 1, tokens should already be vesting
    e.ledger().set_timestamp(start_time + 1);
    assert_eq!(client.get_vested_amount(), 10); // 10000 * 1 / 1000
}

#[test]
fn test_original_vesting_flow() {
    // Preserving the original comprehensive test from before
    let e = Env::default();
    e.mock_all_auths();

    let funder = Address::generate(&e);
    let beneficiary = Address::generate(&e);
    let clawback_admin = Address::generate(&e);
    let contract_id = e.register(VestingContract, ());
    let client = VestingContractClient::new(&e, &contract_id);

    let token_admin = Address::generate(&e);
    let token_contract = e
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::Client::new(&e, &token_contract);
    let token_admin_client = token::StellarAssetClient::new(&e, &token_contract);
    token_admin_client.mint(&funder, &10000);

    let start_time = e.ledger().timestamp();
    let cliff_seconds = 100;
    let duration_seconds = 1000;
    let amount = 10000;

    client.initialize(
        &funder,
        &beneficiary,
        &token_contract,
        &start_time,
        &cliff_seconds,
        &duration_seconds,
        &amount,
        &clawback_admin,
    );

    let config = client.get_config();
    assert_eq!(config.total_amount, amount);
    assert!(config.is_active);
    assert_eq!(token_client.balance(&contract_id), 10000);
    assert_eq!(token_client.balance(&funder), 0);

    // 1. Check before cliff
    assert_eq!(client.get_vested_amount(), 0);
    assert_eq!(client.get_claimable_amount(), 0);

    // 2. Advance past cliff (20%)
    e.ledger().set_timestamp(start_time + 200);
    e.ledger().set_sequence_number(10);

    let vested = client.get_vested_amount();
    let expected = 10000 * 200 / 1000;
    assert_eq!(vested, expected);
    assert_eq!(client.get_claimable_amount(), expected);

    // 3. Claim
    client.claim();
    assert_eq!(token_client.balance(&beneficiary), expected);
    assert_eq!(client.get_claimable_amount(), 0);

    // 4. Advance to 50%
    e.ledger().set_timestamp(start_time + 500);
    e.ledger().set_sequence_number(20);
    assert_eq!(client.get_vested_amount(), 5000);
    assert_eq!(client.get_claimable_amount(), 3000);

    // 5. Clawback
    client.clawback();
    assert_eq!(token_client.balance(&clawback_admin), 5000);
    assert_eq!(token_client.balance(&contract_id), 3000);

    let config_revoked = client.get_config();
    assert!(!config_revoked.is_active);
    assert_eq!(config_revoked.total_amount, 5000);

    // 6. Advance past end — vested still capped at 5000
    e.ledger().set_timestamp(start_time + 2000);
    e.ledger().set_sequence_number(30);
    assert_eq!(client.get_vested_amount(), 5000);

    // 7. Beneficiary claims the rest
    client.claim();
    assert_eq!(token_client.balance(&beneficiary), 2000 + 3000);
    assert_eq!(token_client.balance(&contract_id), 0);
}

// ══════════════════════════════════════════════════════════════════════════════
// ── SEP-0034 METADATA TESTS ──────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_contract_metadata() {
    let e = Env::default();
    let contract_id = e.register(VestingContract, ());
    let client = VestingContractClient::new(&e, &contract_id);

    let name = client.name();
    let version = client.version();
    let author = client.author();

    assert_eq!(
        name,
        soroban_sdk::String::from_str(&e, env!("CARGO_PKG_NAME"))
    );
    assert_eq!(
        version,
        soroban_sdk::String::from_str(&e, env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(
        author,
        soroban_sdk::String::from_str(&e, env!("CARGO_PKG_AUTHORS"))
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// ── BENEFICIARY TRANSFER TESTS ───────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_transfer_beneficiary_updates_config() {
    let (e, funder, beneficiary, clawback_admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );

    let new_beneficiary = Address::generate(&e);
    client.transfer_beneficiary(&new_beneficiary);

    let config = client.get_config();
    assert_eq!(config.beneficiary, new_beneficiary);
}

#[test]
fn test_transferred_beneficiary_can_claim() {
    let (e, funder, beneficiary, clawback_admin, token_contract, token_client, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );

    let new_beneficiary = Address::generate(&e);
    client.transfer_beneficiary(&new_beneficiary);

    // Advance past cliff and partial duration so tokens are vested
    e.ledger().with_mut(|l| l.timestamp += 500);

    client.claim();

    // New beneficiary received tokens; old beneficiary received nothing
    assert!(token_client.balance(&new_beneficiary) > 0);
    assert_eq!(token_client.balance(&beneficiary), 0);
}

#[test]
#[should_panic(expected = "Vesting grant is no longer active")]
fn test_transfer_beneficiary_panics_when_inactive() {
    let (e, funder, beneficiary, clawback_admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
    );

    // Clawback deactivates the grant
    client.clawback();

    let new_beneficiary = Address::generate(&e);
    client.transfer_beneficiary(&new_beneficiary);
}
