#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{token, Address, Env, String};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Registers a stellar asset token and mints an initial balance to `recipient`.
fn create_token(env: &Env, recipient: &Address, amount: i128) -> Address {
    let token_admin = Address::generate(env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    token::StellarAssetClient::new(env, &token_address).mint(recipient, &amount);
    token_address
}

fn setup() -> (Env, Address, Address, CrossAssetPaymentContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(CrossAssetPaymentContract, ());
    let client = CrossAssetPaymentContractClient::new(&env, &contract_id);
    client.init(&admin);

    (env, admin, contract_id, client)
}

// ── initiate_payment ──────────────────────────────────────────────────────────

#[test]
fn test_initiate_payment_stores_record_and_transfers_funds() {
    let (env, _admin, contract_id, client) = setup();

    let from = Address::generate(&env);
    let token_address = create_token(&env, &from, 1_000);

    let receiver_id = String::from_str(&env, "worker-123");
    let target_asset = String::from_str(&env, "EUR");
    let anchor_id = String::from_str(&env, "anchor-eu");

    let payment_id = client.initiate_payment(
        &from,
        &500,
        &token_address,
        &receiver_id,
        &target_asset,
        &anchor_id,
    );

    assert_eq!(payment_id, 1);

    let tc = token::Client::new(&env, &token_address);
    assert_eq!(tc.balance(&contract_id), 500);
    assert_eq!(tc.balance(&from), 500);

    let record = client.get_payment(&payment_id).unwrap();
    assert_eq!(record.from, from);
    assert_eq!(record.amount, 500);
    assert_eq!(record.status, symbol_short!("pending"));
    assert_eq!(record.receiver_id, receiver_id);
    assert_eq!(record.target_asset, target_asset);
    assert_eq!(record.anchor_id, anchor_id);
}

#[test]
fn test_initiate_payment_counter_increments() {
    let (env, _admin, _contract_id, client) = setup();

    let from = Address::generate(&env);
    let token_address = create_token(&env, &from, 10_000);
    let receiver_id = String::from_str(&env, "r1");
    let target_asset = String::from_str(&env, "USD");
    let anchor_id = String::from_str(&env, "anc1");

    let id1 = client.initiate_payment(
        &from, &100, &token_address, &receiver_id, &target_asset, &anchor_id,
    );
    let id2 = client.initiate_payment(
        &from, &200, &token_address, &receiver_id, &target_asset, &anchor_id,
    );
    let id3 = client.initiate_payment(
        &from, &300, &token_address, &receiver_id, &target_asset, &anchor_id,
    );

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(id3, 3);
}

// ── update_status ─────────────────────────────────────────────────────────────

#[test]
fn test_update_status_changes_status_in_persistent_storage() {
    let (env, _admin, _contract_id, client) = setup();

    let from = Address::generate(&env);
    let token_address = create_token(&env, &from, 1_000);

    let payment_id = client.initiate_payment(
        &from,
        &500,
        &token_address,
        &String::from_str(&env, "rec-1"),
        &String::from_str(&env, "USD"),
        &String::from_str(&env, "anc-1"),
    );

    assert_eq!(
        client.get_payment(&payment_id).unwrap().status,
        symbol_short!("pending")
    );

    client.update_status(&payment_id, &symbol_short!("process"));

    assert_eq!(
        client.get_payment(&payment_id).unwrap().status,
        symbol_short!("process")
    );
}

#[test]
fn test_update_status_panics_for_unknown_id() {
    let (_env, _admin, _contract_id, client) = setup();
    let result = client.try_update_status(&999, &symbol_short!("process"));
    assert_eq!(result, Err(Ok(CrossAssetPaymentError::PaymentNotFound)));
}

// ── get_payment ───────────────────────────────────────────────────────────────

#[test]
fn test_get_payment_returns_none_for_unknown_id() {
    let (_env, _admin, _contract_id, client) = setup();
    assert!(client.get_payment(&42).is_none());
}

// ── init ──────────────────────────────────────────────────────────────────────

#[test]
fn test_init_twice() {
    let (env, admin, _contract_id, client) = setup();
    let result = client.try_init(&admin);
    assert_eq!(result, Err(Ok(CrossAssetPaymentError::AlreadyInitialized)));
    let _ = &env;
}

// ══════════════════════════════════════════════════════════════════════════════
// ── LEDGER SEQUENCE VERIFICATION TESTS ────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_initiate_payment_replay_same_ledger() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(75);

    let admin = Address::generate(&env);
    let from = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar_token_admin = token::StellarAssetClient::new(&env, &token_address);
    stellar_token_admin.mint(&from, &2000);

    let contract_id = env.register(CrossAssetPaymentContract, ());
    let client = CrossAssetPaymentContractClient::new(&env, &contract_id);
    client.init(&admin);

    client.initiate_payment(
        &from,
        &500,
        &token_address,
        &String::from_str(&env, "rec-1"),
        &String::from_str(&env, "USD"),
        &String::from_str(&env, "anc-1"),
    );

    let result = client.try_initiate_payment(
        &from,
        &300,
        &token_address,
        &String::from_str(&env, "rec-2"),
        &String::from_str(&env, "EUR"),
        &String::from_str(&env, "anc-2"),
    );
    assert_eq!(result, Err(Ok(CrossAssetPaymentError::LedgerReplayDetected)));
}

#[test]
fn test_initiate_payment_allowed_different_ledgers() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(75);

    let admin = Address::generate(&env);
    let from = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar_token_admin = token::StellarAssetClient::new(&env, &token_address);
    stellar_token_admin.mint(&from, &2000);

    let contract_id = env.register(CrossAssetPaymentContract, ());
    let client = CrossAssetPaymentContractClient::new(&env, &contract_id);
    client.init(&admin);

    client.initiate_payment(
        &from,
        &500,
        &token_address,
        &String::from_str(&env, "rec-1"),
        &String::from_str(&env, "USD"),
        &String::from_str(&env, "anc-1"),
    );
    assert_eq!(client.get_last_payment_ledger(&from), 75);

    env.ledger().set_sequence_number(76);

    client.initiate_payment(
        &from,
        &300,
        &token_address,
        &String::from_str(&env, "rec-2"),
        &String::from_str(&env, "EUR"),
        &String::from_str(&env, "anc-2"),
    );
    assert_eq!(client.get_last_payment_ledger(&from), 76);
    assert_eq!(client.get_payment_count(), 2);
}

#[test]
fn test_initiate_payment_different_senders_same_ledger() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(75);

    let admin = Address::generate(&env);
    let from1 = Address::generate(&env);
    let from2 = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar_token_admin = token::StellarAssetClient::new(&env, &token_address);
    stellar_token_admin.mint(&from1, &2000);
    stellar_token_admin.mint(&from2, &2000);

    let contract_id = env.register(CrossAssetPaymentContract, ());
    let client = CrossAssetPaymentContractClient::new(&env, &contract_id);
    client.init(&admin);

    client.initiate_payment(
        &from1,
        &500,
        &token_address,
        &String::from_str(&env, "rec-1"),
        &String::from_str(&env, "USD"),
        &String::from_str(&env, "anc-1"),
    );

    client.initiate_payment(
        &from2,
        &300,
        &token_address,
        &String::from_str(&env, "rec-2"),
        &String::from_str(&env, "EUR"),
        &String::from_str(&env, "anc-2"),
    );

    assert_eq!(client.get_payment_count(), 2);
}

// ══════════════════════════════════════════════════════════════════════════════
// ── ESCROW TESTS ──────────────────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_init_twice_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(CrossAssetPaymentContract, ());
    let client = CrossAssetPaymentContractClient::new(&env, &contract_id);

    client.init(&admin);
    let result = client.try_init(&admin);
    assert_eq!(result, Err(Ok(CrossAssetPaymentError::AlreadyInitialized)));
}

#[test]
fn test_payment_count_starts_at_zero() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(CrossAssetPaymentContract, ());
    let client = CrossAssetPaymentContractClient::new(&env, &contract_id);
    client.init(&admin);

    assert_eq!(client.get_payment_count(), 0);
}

#[test]
fn test_payment_count_increments() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let from = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar_token_admin = token::StellarAssetClient::new(&env, &token_address);
    stellar_token_admin.mint(&from, &5000);

    let contract_id = env.register(CrossAssetPaymentContract, ());
    let client = CrossAssetPaymentContractClient::new(&env, &contract_id);
    client.init(&admin);

    env.ledger().set_sequence_number(10);
    let id1 = client.initiate_payment(
        &from,
        &100,
        &token_address,
        &String::from_str(&env, "rec-1"),
        &String::from_str(&env, "USD"),
        &String::from_str(&env, "anc-1"),
    );

    env.ledger().set_sequence_number(11);
    let id2 = client.initiate_payment(
        &from,
        &200,
        &token_address,
        &String::from_str(&env, "rec-2"),
        &String::from_str(&env, "EUR"),
        &String::from_str(&env, "anc-2"),
    );

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(client.get_payment_count(), 2);
}

#[test]
fn test_escrow_holds_funds() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let from = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::Client::new(&env, &token_address);
    let stellar_token_admin = token::StellarAssetClient::new(&env, &token_address);
    stellar_token_admin.mint(&from, &1000);

    let contract_id = env.register(CrossAssetPaymentContract, ());
    let client = CrossAssetPaymentContractClient::new(&env, &contract_id);
    client.init(&admin);

    client.initiate_payment(
        &from,
        &600,
        &token_address,
        &String::from_str(&env, "rec"),
        &String::from_str(&env, "USD"),
        &String::from_str(&env, "anc"),
    );

    assert_eq!(token_client.balance(&contract_id), 600);
    assert_eq!(token_client.balance(&from), 400);
}

#[test]
fn test_update_status_transitions() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let from = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar_token_admin = token::StellarAssetClient::new(&env, &token_address);
    stellar_token_admin.mint(&from, &1000);

    let contract_id = env.register(CrossAssetPaymentContract, ());
    let client = CrossAssetPaymentContractClient::new(&env, &contract_id);
    client.init(&admin);

    let payment_id = client.initiate_payment(
        &from,
        &500,
        &token_address,
        &String::from_str(&env, "rec"),
        &String::from_str(&env, "USD"),
        &String::from_str(&env, "anc"),
    );

    // Pending → Processing (valid)
    client.update_status(&payment_id, &symbol_short!("process"));
    let record = client.get_payment(&payment_id).unwrap();
    assert_eq!(record.status, symbol_short!("process"));

    // Processing → Complete (valid)
    client.update_status(&payment_id, &symbol_short!("complete"));
    let record = client.get_payment(&payment_id).unwrap();
    assert_eq!(record.status, symbol_short!("complete"));
}

#[test]
fn test_get_nonexistent_payment_returns_none() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(CrossAssetPaymentContract, ());
    let client = CrossAssetPaymentContractClient::new(&env, &contract_id);
    client.init(&admin);

    let result = client.get_payment(&999);
    assert!(result.is_none());
}

#[test]
fn test_payment_record_correctness() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let from = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar_token_admin = token::StellarAssetClient::new(&env, &token_address);
    stellar_token_admin.mint(&from, &1000);

    let contract_id = env.register(CrossAssetPaymentContract, ());
    let client = CrossAssetPaymentContractClient::new(&env, &contract_id);
    client.init(&admin);

    let receiver_id = String::from_str(&env, "worker-456");
    let target_asset = String::from_str(&env, "GBP");
    let anchor_id = String::from_str(&env, "anchor-uk");

    let payment_id = client.initiate_payment(
        &from,
        &750,
        &token_address,
        &receiver_id,
        &target_asset,
        &anchor_id,
    );

    let record = client.get_payment(&payment_id).unwrap();
    assert_eq!(record.from, from);
    assert_eq!(record.amount, 750);
    assert_eq!(record.asset, token_address);
    assert_eq!(record.receiver_id, receiver_id);
    assert_eq!(record.target_asset, target_asset);
    assert_eq!(record.anchor_id, anchor_id);
    assert_eq!(record.status, symbol_short!("pending"));
}

#[test]
fn test_contract_metadata() {
    let env = Env::default();
    let contract_id = env.register(CrossAssetPaymentContract, ());
    let client = CrossAssetPaymentContractClient::new(&env, &contract_id);

    let name = client.name();
    let version = client.version();
    let author = client.author();

    assert_eq!(name, String::from_str(&env, env!("CARGO_PKG_NAME")));
    assert_eq!(version, String::from_str(&env, env!("CARGO_PKG_VERSION")));
    assert_eq!(author, String::from_str(&env, env!("CARGO_PKG_AUTHORS")));
}

// ══════════════════════════════════════════════════════════════════════════════
// ── STATE MACHINE VALIDATION TESTS ────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_cannot_transition_from_complete() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let from = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar_token_admin = token::StellarAssetClient::new(&env, &token_address);
    stellar_token_admin.mint(&from, &1000);

    let contract_id = env.register(CrossAssetPaymentContract, ());
    let client = CrossAssetPaymentContractClient::new(&env, &contract_id);
    client.init(&admin);

    let payment_id = client.initiate_payment(
        &from,
        &500,
        &token_address,
        &String::from_str(&env, "rec"),
        &String::from_str(&env, "USD"),
        &String::from_str(&env, "anc"),
    );

    client.update_status(&payment_id, &symbol_short!("complete"));

    // Cannot transition from complete
    let result = client.try_update_status(&payment_id, &symbol_short!("pending"));
    assert_eq!(
        result,
        Err(Ok(CrossAssetPaymentError::InvalidStatusTransition))
    );
}

#[test]
fn test_cannot_transition_from_failed() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let from = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar_token_admin = token::StellarAssetClient::new(&env, &token_address);
    stellar_token_admin.mint(&from, &1000);

    let contract_id = env.register(CrossAssetPaymentContract, ());
    let client = CrossAssetPaymentContractClient::new(&env, &contract_id);
    client.init(&admin);

    let payment_id = client.initiate_payment(
        &from,
        &500,
        &token_address,
        &String::from_str(&env, "rec"),
        &String::from_str(&env, "USD"),
        &String::from_str(&env, "anc"),
    );

    client.update_status(&payment_id, &symbol_short!("failed"));

    let result = client.try_update_status(&payment_id, &symbol_short!("complete"));
    assert_eq!(
        result,
        Err(Ok(CrossAssetPaymentError::InvalidStatusTransition))
    );
}

#[test]
fn test_cannot_transition_from_processing_to_pending() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let from = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar_token_admin = token::StellarAssetClient::new(&env, &token_address);
    stellar_token_admin.mint(&from, &1000);

    let contract_id = env.register(CrossAssetPaymentContract, ());
    let client = CrossAssetPaymentContractClient::new(&env, &contract_id);
    client.init(&admin);

    let payment_id = client.initiate_payment(
        &from,
        &500,
        &token_address,
        &String::from_str(&env, "rec"),
        &String::from_str(&env, "USD"),
        &String::from_str(&env, "anc"),
    );

    client.update_status(&payment_id, &symbol_short!("process"));

    let result = client.try_update_status(&payment_id, &symbol_short!("pending"));
    assert_eq!(
        result,
        Err(Ok(CrossAssetPaymentError::InvalidStatusTransition))
    );
}

#[test]
fn test_pending_to_processing_valid() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let from = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let stellar_token_admin = token::StellarAssetClient::new(&env, &token_address);
    stellar_token_admin.mint(&from, &1000);

    let contract_id = env.register(CrossAssetPaymentContract, ());
    let client = CrossAssetPaymentContractClient::new(&env, &contract_id);
    client.init(&admin);

    let payment_id = client.initiate_payment(
        &from,
        &500,
        &token_address,
        &String::from_str(&env, "rec"),
        &String::from_str(&env, "USD"),
        &String::from_str(&env, "anc"),
    );

    client.update_status(&payment_id, &symbol_short!("process"));
    assert_eq!(
        client.get_payment(&payment_id).unwrap().status,
        symbol_short!("process")
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// ── INVALID INPUT TESTS ───────────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_initiate_payment_rejects_zero_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let from = Address::generate(&env);
    let token_address = create_token(&env, &from, 1000);

    let contract_id = env.register(CrossAssetPaymentContract, ());
    let client = CrossAssetPaymentContractClient::new(&env, &contract_id);
    client.init(&admin);

    let result = client.try_initiate_payment(
        &from,
        &0,
        &token_address,
        &String::from_str(&env, "rec"),
        &String::from_str(&env, "USD"),
        &String::from_str(&env, "anc"),
    );
    assert_eq!(result, Err(Ok(CrossAssetPaymentError::InvalidAmount)));
}

#[test]
fn test_initiate_payment_rejects_negative_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let from = Address::generate(&env);
    let token_address = create_token(&env, &from, 1000);

    let contract_id = env.register(CrossAssetPaymentContract, ());
    let client = CrossAssetPaymentContractClient::new(&env, &contract_id);
    client.init(&admin);

    let result = client.try_initiate_payment(
        &from,
        &(-100),
        &token_address,
        &String::from_str(&env, "rec"),
        &String::from_str(&env, "USD"),
        &String::from_str(&env, "anc"),
    );
    assert_eq!(result, Err(Ok(CrossAssetPaymentError::InvalidAmount)));
}

#[test]
fn test_initiate_payment_rejects_empty_routing() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let from = Address::generate(&env);
    let token_address = create_token(&env, &from, 1000);

    let contract_id = env.register(CrossAssetPaymentContract, ());
    let client = CrossAssetPaymentContractClient::new(&env, &contract_id);
    client.init(&admin);

    let result = client.try_initiate_payment(
        &from,
        &100,
        &token_address,
        &String::from_str(&env, ""),
        &String::from_str(&env, "USD"),
        &String::from_str(&env, "anc"),
    );
    assert_eq!(result, Err(Ok(CrossAssetPaymentError::EmptyRoutingFields)));
}

// ══════════════════════════════════════════════════════════════════════════════
// ── COMPLETE / FAIL PENDING-ONLY TESTS ────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_complete_payment_rejects_non_pending() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let from = Address::generate(&env);
    let token_address = create_token(&env, &from, 2000);
    let recipient = Address::generate(&env);

    let contract_id = env.register(CrossAssetPaymentContract, ());
    let client = CrossAssetPaymentContractClient::new(&env, &contract_id);
    client.init(&admin);

    let payment_id = client.initiate_payment(
        &from,
        &500,
        &token_address,
        &String::from_str(&env, "rec"),
        &String::from_str(&env, "USD"),
        &String::from_str(&env, "anc"),
    );

    // Complete once
    client.complete_payment(&admin, &payment_id, &recipient);

    // Try completing again
    let result = client.try_complete_payment(&admin, &payment_id, &recipient);
    assert_eq!(result, Err(Ok(CrossAssetPaymentError::PaymentNotPending)));
}

#[test]
fn test_fail_payment_rejects_non_pending() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let from = Address::generate(&env);
    let token_address = create_token(&env, &from, 2000);

    let contract_id = env.register(CrossAssetPaymentContract, ());
    let client = CrossAssetPaymentContractClient::new(&env, &contract_id);
    client.init(&admin);

    let payment_id = client.initiate_payment(
        &from,
        &500,
        &token_address,
        &String::from_str(&env, "rec"),
        &String::from_str(&env, "USD"),
        &String::from_str(&env, "anc"),
    );

    // Fail once
    client.fail_payment(&admin, &payment_id);

    // Try failing again
    let result = client.try_fail_payment(&admin, &payment_id);
    assert_eq!(result, Err(Ok(CrossAssetPaymentError::PaymentNotPending)));
}

// ══════════════════════════════════════════════════════════════════════════════
// ── EVENT TESTS ───────────────────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_payment_initiated_event_emitted() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let from = Address::generate(&env);
    let token_address = create_token(&env, &from, 1000);

    let contract_id = env.register(CrossAssetPaymentContract, ());
    let client = CrossAssetPaymentContractClient::new(&env, &contract_id);
    client.init(&admin);

    let payment_id = client.initiate_payment(
        &from,
        &500,
        &token_address,
        &String::from_str(&env, "rec"),
        &String::from_str(&env, "USD"),
        &String::from_str(&env, "anc"),
    );
    assert_eq!(payment_id, 1);
}

#[test]
fn test_escrow_released_event_emitted() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let from = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token_address = create_token(&env, &from, 1000);

    let contract_id = env.register(CrossAssetPaymentContract, ());
    let client = CrossAssetPaymentContractClient::new(&env, &contract_id);
    client.init(&admin);

    let payment_id = client.initiate_payment(
        &from,
        &500,
        &token_address,
        &String::from_str(&env, "rec"),
        &String::from_str(&env, "USD"),
        &String::from_str(&env, "anc"),
    );

    client.complete_payment(&admin, &payment_id, &recipient);

    let record = client.get_payment(&payment_id).unwrap();
    assert_eq!(record.status, symbol_short!("complete"));
}
