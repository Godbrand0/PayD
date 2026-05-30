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
    Address,                            // clawback_admin
    Address,                            // admin
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
    let admin = Address::generate(&e);
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
        admin,
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
    admin: &Address,
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
        admin,
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// ── INITIALIZATION TESTS ──────────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_initialize_success() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, token_client, _, client) =
        setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
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

    assert_eq!(token_client.balance(&funder), 90_000);
    assert_eq!(client.get_admin(), admin);
}

#[test]
fn test_initialize_twice_fails() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );
    let result = client.try_initialize(
        &funder,
        &beneficiary,
        &token_contract,
        &e.ledger().timestamp(),
        &100,
        &1000,
        &10000,
        &clawback_admin,
        &admin,
    );
    assert_eq!(result, Err(Ok(ContractError::AlreadyInitialized)));
}

#[test]
fn test_initialize_cliff_greater_than_duration_fails() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    let start_time = e.ledger().timestamp();
    let result = client.try_initialize(
        &funder,
        &beneficiary,
        &token_contract,
        &start_time,
        &500, // cliff > duration
        &100, // duration
        &1000,
        &clawback_admin,
        &admin,
    );
    assert_eq!(result, Err(Ok(ContractError::InvalidDuration)));
}

#[test]
fn test_initialize_zero_amount_fails() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    let start_time = e.ledger().timestamp();
    let result = client.try_initialize(
        &funder,
        &beneficiary,
        &token_contract,
        &start_time,
        &100,
        &1000,
        &0,
        &clawback_admin,
        &admin,
    );
    assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
}

#[test]
fn test_initialize_negative_amount_fails() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    let start_time = e.ledger().timestamp();
    let result = client.try_initialize(
        &funder,
        &beneficiary,
        &token_contract,
        &start_time,
        &100,
        &1000,
        &-100,
        &clawback_admin,
        &admin,
    );
    assert_eq!(result, Err(Ok(ContractError::InvalidAmount)));
}

#[test]
fn test_initialize_cliff_equals_duration() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    let start_time = e.ledger().timestamp();
    client.initialize(
        &funder,
        &beneficiary,
        &token_contract,
        &start_time,
        &1000,
        &1000,
        &5000,
        &clawback_admin,
        &admin,
    );
    let config = client.get_config();
    assert_eq!(config.cliff_seconds, 1000);
    assert_eq!(config.duration_seconds, 1000);
}

#[test]
fn test_initialize_zero_cliff() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    let start_time = e.ledger().timestamp();
    client.initialize(
        &funder,
        &beneficiary,
        &token_contract,
        &start_time,
        &0,
        &1000,
        &5000,
        &clawback_admin,
        &admin,
    );
    let config = client.get_config();
    assert_eq!(config.cliff_seconds, 0);
}

// ══════════════════════════════════════════════════════════════════════════════
// ── VESTING CALCULATION TESTS ─────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_vested_amount_before_cliff() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 50);
    assert_eq!(client.get_vested_amount(), 0);
    assert_eq!(client.get_claimable_amount(), 0);
}

#[test]
fn test_vested_amount_at_cliff_boundary() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 100);
    assert_eq!(client.get_vested_amount(), 1000);
}

#[test]
fn test_vested_amount_linear_progression() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let start = e.ledger().timestamp();

    e.ledger().set_timestamp(start + 250);
    assert_eq!(client.get_vested_amount(), 2500);

    e.ledger().set_timestamp(start + 500);
    assert_eq!(client.get_vested_amount(), 5000);

    e.ledger().set_timestamp(start + 750);
    assert_eq!(client.get_vested_amount(), 7500);
}

#[test]
fn test_vested_amount_at_duration_end() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 1000);
    assert_eq!(client.get_vested_amount(), 10000);
}

#[test]
fn test_vested_amount_after_duration_end() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 5000);
    assert_eq!(client.get_vested_amount(), 10000);
}

// ══════════════════════════════════════════════════════════════════════════════
// ── CLAIM TESTS ───────────────────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_claim_before_cliff_does_nothing() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, token_client, _, client) =
        setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 50);

    client.claim();
    assert_eq!(token_client.balance(&beneficiary), 0);
    let config = client.get_config();
    assert_eq!(config.claimed_amount, 0);
}

#[test]
fn test_claim_partial_vesting() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, token_client, _, client) =
        setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 200);

    client.claim();
    assert_eq!(token_client.balance(&beneficiary), 2000);
    let config = client.get_config();
    assert_eq!(config.claimed_amount, 2000);
}

#[test]
fn test_claim_multiple_partial_claims() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, token_client, _, client) =
        setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let start = e.ledger().timestamp();

    e.ledger().set_timestamp(start + 200);
    e.ledger().set_sequence_number(10);
    client.claim();
    assert_eq!(token_client.balance(&beneficiary), 2000);

    e.ledger().set_timestamp(start + 500);
    e.ledger().set_sequence_number(20);
    client.claim();
    assert_eq!(token_client.balance(&beneficiary), 5000);

    e.ledger().set_timestamp(start + 1000);
    e.ledger().set_sequence_number(30);
    client.claim();
    assert_eq!(token_client.balance(&beneficiary), 10000);
}

#[test]
fn test_claim_full_after_duration() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, token_client, _, client) =
        setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
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
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, token_client, _, client) =
        setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 200);
    e.ledger().set_sequence_number(10);
    client.claim();
    assert_eq!(token_client.balance(&beneficiary), 2000);

    e.ledger().set_sequence_number(11);
    client.claim();
    assert_eq!(token_client.balance(&beneficiary), 2000);
}

// ══════════════════════════════════════════════════════════════════════════════
// ── CLAWBACK TESTS ────────────────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_clawback_before_any_vesting() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, token_client, _, client) =
        setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    client.clawback();

    assert_eq!(token_client.balance(&clawback_admin), 10000);
    let config = client.get_config();
    assert!(!config.is_active);
    assert_eq!(config.total_amount, 0);
}

#[test]
fn test_clawback_partial_vesting() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, token_client, _, client) =
        setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 300);

    client.clawback();

    assert_eq!(token_client.balance(&clawback_admin), 7000);
    let config = client.get_config();
    assert!(!config.is_active);
    assert_eq!(config.total_amount, 3000);
}

#[test]
fn test_clawback_then_beneficiary_can_claim_vested() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, token_client, _, client) =
        setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 400);
    e.ledger().set_sequence_number(10);

    client.clawback();
    assert_eq!(token_client.balance(&clawback_admin), 6000);

    e.ledger().set_timestamp(start + 2000);
    e.ledger().set_sequence_number(20);
    client.claim();
    assert_eq!(token_client.balance(&beneficiary), 4000);
}

#[test]
fn test_clawback_twice_fails() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    e.ledger().set_sequence_number(10);
    client.clawback();
    e.ledger().set_sequence_number(11);
    let result = client.try_clawback();
    assert_eq!(result, Err(Ok(ContractError::AlreadyRevoked)));
}

#[test]
fn test_clawback_after_full_vesting() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, token_client, _, client) =
        setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 2000);

    client.clawback();
    assert_eq!(token_client.balance(&clawback_admin), 0);

    let config = client.get_config();
    assert!(!config.is_active);
    assert_eq!(config.total_amount, 10000);
}

#[test]
fn test_clawback_after_partial_claim() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, token_client, _, client) =
        setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let start = e.ledger().timestamp();

    e.ledger().set_timestamp(start + 200);
    e.ledger().set_sequence_number(10);
    client.claim();
    assert_eq!(token_client.balance(&beneficiary), 2000);

    e.ledger().set_timestamp(start + 500);
    e.ledger().set_sequence_number(20);
    client.clawback();

    assert_eq!(token_client.balance(&clawback_admin), 5000);

    e.ledger().set_timestamp(start + 2000);
    e.ledger().set_sequence_number(30);
    client.claim();
    assert_eq!(token_client.balance(&beneficiary), 5000);
}

// ══════════════════════════════════════════════════════════════════════════════
// ── TOKEN BALANCE INVARIANT TESTS ─────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_contract_balance_after_full_claim() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, token_client, _, client) =
        setup();
    let contract_id = client.address.clone();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 2000);

    client.claim();

    assert_eq!(token_client.balance(&contract_id), 0);
    assert_eq!(token_client.balance(&beneficiary), 10000);
}

#[test]
fn test_contract_balance_after_clawback_and_claim() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, token_client, _, client) =
        setup();
    let contract_id = client.address.clone();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let start = e.ledger().timestamp();

    e.ledger().set_timestamp(start + 600);
    e.ledger().set_sequence_number(10);
    client.clawback();

    assert_eq!(token_client.balance(&contract_id), 6000);
    assert_eq!(token_client.balance(&clawback_admin), 4000);

    e.ledger().set_timestamp(start + 2000);
    e.ledger().set_sequence_number(20);
    client.claim();

    assert_eq!(token_client.balance(&contract_id), 0);
    assert_eq!(token_client.balance(&beneficiary), 6000);
}

// ══════════════════════════════════════════════════════════════════════════════
// ── LEDGER SEQUENCE VERIFICATION TESTS (Issue #173) ───────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_claim_replay_same_ledger_fails() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 500);
    e.ledger().set_sequence_number(10);

    client.claim();
    let result = client.try_claim();
    assert_eq!(result, Err(Ok(ContractError::LedgerReplayDetected)));
}

#[test]
fn test_claim_allowed_different_ledgers() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, token_client, _, client) =
        setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
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
    let admin = Address::generate(&e);
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
        &0,
        &1000,
        &10000,
        &clawback_admin,
        &admin,
    );

    e.ledger().set_timestamp(start_time + 1);
    assert_eq!(client.get_vested_amount(), 10);
}

#[test]
fn test_original_vesting_flow() {
    let e = Env::default();
    e.mock_all_auths();

    let funder = Address::generate(&e);
    let beneficiary = Address::generate(&e);
    let clawback_admin = Address::generate(&e);
    let admin = Address::generate(&e);
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
        &admin,
    );

    let config = client.get_config();
    assert_eq!(config.total_amount, amount);
    assert!(config.is_active);
    assert_eq!(token_client.balance(&contract_id), 10000);
    assert_eq!(token_client.balance(&funder), 0);

    assert_eq!(client.get_vested_amount(), 0);
    assert_eq!(client.get_claimable_amount(), 0);

    e.ledger().set_timestamp(start_time + 200);
    e.ledger().set_sequence_number(10);

    let vested = client.get_vested_amount();
    let expected = 10000 * 200 / 1000;
    assert_eq!(vested, expected);
    assert_eq!(client.get_claimable_amount(), expected);

    client.claim();
    assert_eq!(token_client.balance(&beneficiary), expected);
    assert_eq!(client.get_claimable_amount(), 0);

    e.ledger().set_timestamp(start_time + 500);
    e.ledger().set_sequence_number(20);
    assert_eq!(client.get_vested_amount(), 5000);
    assert_eq!(client.get_claimable_amount(), 3000);

    client.clawback();
    assert_eq!(token_client.balance(&clawback_admin), 5000);
    assert_eq!(token_client.balance(&contract_id), 3000);

    let config_revoked = client.get_config();
    assert!(!config_revoked.is_active);
    assert_eq!(config_revoked.total_amount, 5000);

    e.ledger().set_timestamp(start_time + 2000);
    e.ledger().set_sequence_number(30);
    assert_eq!(client.get_vested_amount(), 5000);

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
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let new_beneficiary = Address::generate(&e);
    client.transfer_beneficiary(&new_beneficiary);

    let config = client.get_config();
    assert_eq!(config.beneficiary, new_beneficiary);
}

#[test]
fn test_transferred_beneficiary_can_claim() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, token_client, _, client) =
        setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let new_beneficiary = Address::generate(&e);
    client.transfer_beneficiary(&new_beneficiary);

    e.ledger().with_mut(|l| l.timestamp += 500);

    client.claim();

    assert!(token_client.balance(&new_beneficiary) > 0);
    assert_eq!(token_client.balance(&beneficiary), 0);
}

#[test]
fn test_transfer_beneficiary_fails_when_inactive() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    client.clawback();

    let new_beneficiary = Address::generate(&e);
    let result = client.try_transfer_beneficiary(&new_beneficiary);
    assert_eq!(result, Err(Ok(ContractError::GrantInactive)));
}

// ══════════════════════════════════════════════════════════════════════════════
// ── ADMIN GOVERNANCE TESTS ───────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_set_admin_success() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let new_admin = Address::generate(&e);
    client.set_admin(&new_admin);

    assert_eq!(client.get_admin(), new_admin);
}

#[test]
fn test_set_admin_same_admin_fails() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let result = client.try_set_admin(&admin);
    assert_eq!(result, Err(Ok(ContractError::SameAdmin)));
}

// ══════════════════════════════════════════════════════════════════════════════
// ── EMERGENCY PAUSE TESTS ─────────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_pause_and_unpause() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    assert!(!client.is_paused());

    client.set_paused(&true);
    assert!(client.is_paused());

    client.set_paused(&false);
    assert!(!client.is_paused());
}

#[test]
fn test_claim_blocked_when_paused() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    client.set_paused(&true);

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 500);

    let result = client.try_claim();
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

#[test]
fn test_clawback_blocked_when_paused() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    client.set_paused(&true);

    let result = client.try_clawback();
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

#[test]
fn test_claim_resumes_after_unpause() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, token_client, _, client) =
        setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    client.set_paused(&true);

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 500);

    assert!(client.try_claim().is_err());

    client.set_paused(&false);
    client.claim();
    assert!(token_client.balance(&beneficiary) > 0);
}

// ══════════════════════════════════════════════════════════════════════════════
// ── PARTIAL CLAWBACK TESTS ────────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_partial_clawback_reduces_grant() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, token_client, _, client) =
        setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 300);

    client.partial_clawback(&2000);

    let config = client.get_config();
    assert!(config.is_active);
    assert_eq!(config.total_amount, 8000);
    assert_eq!(token_client.balance(&clawback_admin), 2000);
}

#[test]
fn test_partial_clawback_preserves_vesting() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, token_client, _, client) =
        setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 300);

    client.partial_clawback(&4000);
    assert_eq!(client.get_config().total_amount, 6000);

    e.ledger().set_timestamp(start + 1000);
    e.ledger().set_sequence_number(20);
    client.claim();

    assert_eq!(token_client.balance(&beneficiary), 6000);
}

#[test]
fn test_partial_clawback_below_claimed_fails() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 500);
    e.ledger().set_sequence_number(10);
    client.claim();

    let result = client.try_partial_clawback(&8000);
    assert_eq!(result, Err(Ok(ContractError::ClawbackBelowClaimed)));
}

#[test]
fn test_partial_clawback_inactive_grant_fails() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    client.clawback();

    let result = client.try_partial_clawback(&1000);
    assert_eq!(result, Err(Ok(ContractError::AlreadyRevoked)));
}

#[test]
fn test_partial_clawback_paused_contract_fails() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    client.set_paused(&true);

    let result = client.try_partial_clawback(&1000);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

#[test]
fn test_partial_clawback_zero_amount_fails() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let result = client.try_partial_clawback(&0);
    assert_eq!(result, Err(Ok(ContractError::InvalidClawbackAmount)));
}

// ══════════════════════════════════════════════════════════════════════════════
// ── VESTING SCHEDULE EXTENSION TESTS ──────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_extend_vesting_increases_duration() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    client.extend_vesting(&500);

    let config = client.get_config();
    assert_eq!(config.duration_seconds, 1500);
}

#[test]
fn test_extend_vesting_slows_vesting_rate() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 300);

    let vested_before = client.get_vested_amount();

    client.extend_vesting(&1000);

    e.ledger().set_timestamp(start + 900);
    let vested_after = client.get_vested_amount();

    assert!(vested_after > vested_before);
    assert!(vested_after < 9000);
}

#[test]
fn test_extend_vesting_inactive_grant_fails() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    client.clawback();

    let result = client.try_extend_vesting(&500);
    assert_eq!(result, Err(Ok(ContractError::AlreadyRevoked)));
}

#[test]
fn test_extend_vesting_zero_seconds_fails() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let result = client.try_extend_vesting(&0);
    assert_eq!(result, Err(Ok(ContractError::InvalidExtension)));
}

#[test]
fn test_extend_vesting_multiple_extensions() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, _, _, client) = setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    client.extend_vesting(&500);
    assert_eq!(client.get_config().duration_seconds, 1500);

    client.extend_vesting(&500);
    assert_eq!(client.get_config().duration_seconds, 2000);
}

#[test]
fn test_extend_vesting_then_clawback_still_works() {
    let (e, funder, beneficiary, clawback_admin, admin, token_contract, token_client, _, client) =
        setup();
    init_default(
        &client,
        &e,
        &funder,
        &beneficiary,
        &token_contract,
        &clawback_admin,
        &admin,
    );

    let start = e.ledger().timestamp();
    e.ledger().set_timestamp(start + 300);

    client.extend_vesting(&1000);

    e.ledger().set_timestamp(start + 500);
    client.clawback();

    let config = client.get_config();
    assert!(!config.is_active);
    assert!(token_client.balance(&clawback_admin) > 0);
}
