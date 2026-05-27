#![cfg(test)]

use crate::{RevenueSplitContract, RevenueSplitContractClient, RecipientShare, RevenueSplitError};
use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Env, Vec};
use soroban_sdk::token::Client as TokenClient;
use soroban_sdk::token::StellarAssetClient;

fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (Address, StellarAssetClient<'a>, TokenClient<'a>) {
    e.mock_all_auths();
    let contract_id = e.register_stellar_asset_contract_v2(admin.clone()).address();
    let stellar_asset_client = StellarAssetClient::new(e, &contract_id);
    let token_client = TokenClient::new(e, &contract_id);
    (contract_id, stellar_asset_client, token_client)
}

// ══════════════════════════════════════════════════════════════════════════════
// ── INITIALIZATION ────────────────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_initialization() {
    let env = Env::default();
    let contract_id = env.register(RevenueSplitContract, ());
    let client = RevenueSplitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);

    let shares = Vec::from_array(&env, [
        RecipientShare { destination: recipient1.clone(), basis_points: 6000 },
        RecipientShare { destination: recipient2.clone(), basis_points: 4000 },
    ]);

    let result = client.try_init(&admin, &shares);
    assert_eq!(result, Ok(Ok(())));
}

#[test]
fn test_init_invalid_shares_sum() {
    let env = Env::default();
    let contract_id = env.register(RevenueSplitContract, ());
    let client = RevenueSplitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let recipient1 = Address::generate(&env);

    let shares = Vec::from_array(&env, [
        RecipientShare { destination: recipient1.clone(), basis_points: 5000 },
    ]);

    let result = client.try_init(&admin, &shares);
    assert_eq!(result, Err(Ok(RevenueSplitError::BasisPointsSumMismatch)));
}

#[test]
fn test_init_duplicate_recipient() {
    let env = Env::default();
    let contract_id = env.register(RevenueSplitContract, ());
    let client = RevenueSplitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let recipient = Address::generate(&env);

    let shares = Vec::from_array(&env, [
        RecipientShare { destination: recipient.clone(), basis_points: 5000 },
        RecipientShare { destination: recipient, basis_points: 5000 },
    ]);

    let result = client.try_init(&admin, &shares);
    assert_eq!(result, Err(Ok(RevenueSplitError::DuplicateRecipient)));
}

#[test]
fn test_double_init() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(RevenueSplitContract, ());
    let client = RevenueSplitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let recipient = Address::generate(&env);

    let shares = Vec::from_array(&env, [
        RecipientShare { destination: recipient.clone(), basis_points: 10000 },
    ]);

    client.init(&admin, &shares);
    let result = client.try_init(&admin, &shares);
    assert_eq!(result, Err(Ok(RevenueSplitError::AlreadyInitialized)));
}

// ══════════════════════════════════════════════════════════════════════════════
// ── DISTRIBUTION ──────────────────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_distribution() {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token_id, stellar_asset_client, token_client) = create_token_contract(&env, &token_admin);

    let contract_id = env.register(RevenueSplitContract, ());
    let contract_client = RevenueSplitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);
    let recipient3 = Address::generate(&env);

    let shares = Vec::from_array(&env, [
        RecipientShare { destination: recipient1.clone(), basis_points: 5000 },
        RecipientShare { destination: recipient2.clone(), basis_points: 3000 },
        RecipientShare { destination: recipient3.clone(), basis_points: 2000 },
    ]);

    contract_client.init(&admin, &shares);

    let sender = Address::generate(&env);
    stellar_asset_client.mint(&sender, &1000);

    contract_client.distribute(&token_id, &sender, &1000);

    assert_eq!(token_client.balance(&sender), 0);
    assert_eq!(token_client.balance(&recipient1), 500);
    assert_eq!(token_client.balance(&recipient2), 300);
    assert_eq!(token_client.balance(&recipient3), 200);
}

#[test]
fn test_update_recipients() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(RevenueSplitContract, ());
    let client = RevenueSplitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let recipient1 = Address::generate(&env);

    let shares = Vec::from_array(&env, [
        RecipientShare { destination: recipient1.clone(), basis_points: 10000 },
    ]);
    client.init(&admin, &shares);

    let recipient2 = Address::generate(&env);
    let new_shares = Vec::from_array(&env, [
        RecipientShare { destination: recipient1.clone(), basis_points: 5000 },
        RecipientShare { destination: recipient2.clone(), basis_points: 5000 },
    ]);

    client.update_recipients(&new_shares);
}

#[test]
fn test_update_recipients_rejects_zero_share() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(RevenueSplitContract, ());
    let client = RevenueSplitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let recipient1 = Address::generate(&env);

    let shares = Vec::from_array(&env, [
        RecipientShare { destination: recipient1.clone(), basis_points: 10000 },
    ]);
    client.init(&admin, &shares);

    let recipient2 = Address::generate(&env);
    let new_shares = Vec::from_array(&env, [
        RecipientShare { destination: recipient1, basis_points: 10000 },
        RecipientShare { destination: recipient2, basis_points: 0 },
    ]);

    let result = client.try_update_recipients(&new_shares);
    assert_eq!(result, Err(Ok(RevenueSplitError::ZeroBasisPoints)));
}

#[test]
fn test_set_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(RevenueSplitContract, ());
    let client = RevenueSplitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let recipient = Address::generate(&env);

    let shares = Vec::from_array(&env, [
        RecipientShare { destination: recipient.clone(), basis_points: 10000 },
    ]);

    client.init(&admin, &shares);
    client.set_admin(&new_admin);
    assert_eq!(client.get_admin(), new_admin);
}

// ══════════════════════════════════════════════════════════════════════════════
// ── LEDGER SEQUENCE VERIFICATION ──────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_distribute_replay_same_ledger() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(50);

    let token_admin = Address::generate(&env);
    let (token_id, stellar_asset_client, _) = create_token_contract(&env, &token_admin);

    let contract_id = env.register(RevenueSplitContract, ());
    let client = RevenueSplitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let recipient = Address::generate(&env);

    let shares = Vec::from_array(&env, [
        RecipientShare { destination: recipient.clone(), basis_points: 10000 },
    ]);

    client.init(&admin, &shares);

    let sender = Address::generate(&env);
    stellar_asset_client.mint(&sender, &2000);

    client.distribute(&token_id, &sender, &1000);
    let result = client.try_distribute(&token_id, &sender, &500);
    assert_eq!(result, Err(Ok(RevenueSplitError::LedgerReplayDetected)));
}

#[test]
fn test_sep0034_metadata() {
    let env = Env::default();
    let contract_id = env.register(RevenueSplitContract, ());
    let client = RevenueSplitContractClient::new(&env, &contract_id);

    assert_eq!(
        client.name(),
        soroban_sdk::String::from_str(&env, env!("CARGO_PKG_NAME"))
    );
    assert_eq!(
        client.version(),
        soroban_sdk::String::from_str(&env, env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(
        client.author(),
        soroban_sdk::String::from_str(&env, env!("CARGO_PKG_AUTHORS"))
    );
}

#[test]
fn test_distribute_allowed_different_ledgers() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(50);

    let token_admin = Address::generate(&env);
    let (token_id, stellar_asset_client, token_client) =
        create_token_contract(&env, &token_admin);

    let contract_id = env.register(RevenueSplitContract, ());
    let contract_client = RevenueSplitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);

    let shares = Vec::from_array(&env, [
        RecipientShare { destination: recipient1.clone(), basis_points: 3333 },
        RecipientShare { destination: recipient2.clone(), basis_points: 6667 },
    ]);

    contract_client.init(&admin, &shares);

    let sender = Address::generate(&env);
    stellar_asset_client.mint(&sender, &1000);

    contract_client.distribute(&token_id, &sender, &1000);

    assert_eq!(token_client.balance(&sender), 0);
    let r1 = token_client.balance(&recipient1);
    let r2 = token_client.balance(&recipient2);
    assert_eq!(r1 + r2, 1000);
}

#[test]
fn test_update_recipients_invalid_sum() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(RevenueSplitContract, ());
    let client = RevenueSplitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let recipient1 = Address::generate(&env);

    let shares = Vec::from_array(&env, [
        RecipientShare { destination: recipient1.clone(), basis_points: 10000 },
    ]);
    client.init(&admin, &shares);

    let recipient2 = Address::generate(&env);
    let bad_shares = Vec::from_array(&env, [
        RecipientShare { destination: recipient1.clone(), basis_points: 4000 },
        RecipientShare { destination: recipient2.clone(), basis_points: 5000 },
    ]);
    let result = client.try_update_recipients(&bad_shares);
    assert_eq!(result, Err(Ok(RevenueSplitError::BasisPointsSumMismatch)));
}

#[test]
fn test_distribute_updates_ledger_state() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(50);

    let token_admin = Address::generate(&env);
    let (token_id, stellar_asset_client, token_client) =
        create_token_contract(&env, &token_admin);

    let contract_id = env.register(RevenueSplitContract, ());
    let client = RevenueSplitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let recipient = Address::generate(&env);
    let shares = Vec::from_array(&env, [
        RecipientShare { destination: recipient.clone(), basis_points: 10000 },
    ]);
    client.init(&admin, &shares);

    let sender = Address::generate(&env);
    stellar_asset_client.mint(&sender, &2000);

    client.distribute(&token_id, &sender, &1000);
    assert_eq!(client.get_last_distribute_ledger(), 50);

    env.ledger().set_sequence_number(51);
    client.distribute(&token_id, &sender, &500);
    assert_eq!(client.get_last_distribute_ledger(), 51);
    assert_eq!(token_client.balance(&recipient), 1500);
}

#[test]
fn test_get_recipients_returns_current_configuration() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(RevenueSplitContract, ());
    let client = RevenueSplitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);

    let shares = Vec::from_array(&env, [
        RecipientShare { destination: recipient1.clone(), basis_points: 7000 },
        RecipientShare { destination: recipient2.clone(), basis_points: 3000 },
    ]);

    client.init(&admin, &shares);

    let stored = client.get_recipients();
    assert_eq!(stored, shares);
}

#[test]
fn test_preview_distribution_preserves_remainder_on_last_recipient() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(RevenueSplitContract, ());
    let client = RevenueSplitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);

    let shares = Vec::from_array(&env, [
        RecipientShare { destination: recipient1.clone(), basis_points: 3333 },
        RecipientShare { destination: recipient2.clone(), basis_points: 6667 },
    ]);

    client.init(&admin, &shares);

    let preview = client.preview_distribution(&1000);
    let first = preview.get(0).unwrap();
    let second = preview.get(1).unwrap();

    assert_eq!(first.destination, recipient1);
    assert_eq!(first.amount, 333);
    assert_eq!(second.destination, recipient2);
    assert_eq!(second.amount, 667);
}

#[test]
fn test_total_distributed_accumulates_across_calls() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient1 = Address::generate(&env);
    let recipient2 = Address::generate(&env);

    let (token_contract, token_admin_client, token_client) =
        create_token_contract(&env, &admin);
    token_admin_client.mint(&sender, &100_000);

    let contract_id = env.register(RevenueSplitContract, ());
    let client = RevenueSplitContractClient::new(&env, &contract_id);

    let shares = Vec::from_array(&env, [
        RecipientShare { destination: recipient1.clone(), basis_points: 5000 },
        RecipientShare { destination: recipient2.clone(), basis_points: 5000 },
    ]);
    client.init(&admin, &shares);

    env.ledger().set_sequence_number(1);
    client.distribute(&token_contract, &sender, &10_000);
    assert_eq!(client.get_total_distributed(&token_contract), 10_000);

    env.ledger().set_sequence_number(2);
    client.distribute(&token_contract, &sender, &5_000);
    assert_eq!(client.get_total_distributed(&token_contract), 15_000);

    assert_eq!(token_client.balance(&recipient1), 7_500);
    assert_eq!(token_client.balance(&recipient2), 7_500);
}

#[test]
fn test_total_distributed_starts_at_zero() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (token_contract, _, _) = create_token_contract(&env, &admin);

    let contract_id = env.register(RevenueSplitContract, ());
    let client = RevenueSplitContractClient::new(&env, &contract_id);

    let shares = Vec::from_array(&env, [
        RecipientShare {
            destination: Address::generate(&env),
            basis_points: 10_000,
        },
    ]);
    client.init(&admin, &shares);

    assert_eq!(client.get_total_distributed(&token_contract), 0);
}

// ══════════════════════════════════════════════════════════════════════════════
// ── EVENT EMISSION ────────────────────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_revenue_distributed_event() {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let (token_id, stellar_asset_client, _) = create_token_contract(&env, &token_admin);

    let contract_id = env.register(RevenueSplitContract, ());
    let client = RevenueSplitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let recipient = Address::generate(&env);

    let shares = Vec::from_array(&env, [
        RecipientShare { destination: recipient.clone(), basis_points: 10000 },
    ]);
    client.init(&admin, &shares);

    let sender = Address::generate(&env);
    stellar_asset_client.mint(&sender, &1000);

    // Distribution should succeed
    client.distribute(&token_id, &sender, &500);
}

#[test]
fn test_recipients_updated_event() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(RevenueSplitContract, ());
    let client = RevenueSplitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let recipient1 = Address::generate(&env);

    let shares = Vec::from_array(&env, [
        RecipientShare { destination: recipient1.clone(), basis_points: 10000 },
    ]);
    client.init(&admin, &shares);

    let recipient2 = Address::generate(&env);
    let new_shares = Vec::from_array(&env, [
        RecipientShare { destination: recipient1.clone(), basis_points: 5000 },
        RecipientShare { destination: recipient2.clone(), basis_points: 5000 },
    ]);
    // Update should succeed
    client.update_recipients(&new_shares);

    let stored = client.get_recipients();
    assert_eq!(stored.len(), 2);
}

#[test]
fn test_admin_changed_event() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(RevenueSplitContract, ());
    let client = RevenueSplitContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let recipient = Address::generate(&env);

    let shares = Vec::from_array(&env, [
        RecipientShare { destination: recipient.clone(), basis_points: 10000 },
    ]);

    client.init(&admin, &shares);
    client.set_admin(&new_admin);
    assert_eq!(client.get_admin(), new_admin);
}
