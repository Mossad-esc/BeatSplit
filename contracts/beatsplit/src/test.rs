#![cfg(test)]

extern crate std;

use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, Vec};

use crate::{BeatSplitContract, BeatSplitContractClient, Error, Recipient, SplitStatus};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn setup() -> (Env, BeatSplitContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(BeatSplitContract, ());
    let client = BeatSplitContractClient::new(&env, &contract_id);
    (env, client)
}

/// Build a `BytesN<32>` hash from a constant seed byte.
fn fake_hash(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

/// Build a recipient list with the given (address, bps) pairs.
fn recipients(env: &Env, pairs: &[(Address, u32)]) -> Vec<Recipient> {
    let mut v = Vec::new(env);
    for (addr, bps) in pairs {
        v.push_back(Recipient {
            addr: addr.clone(),
            bps: *bps,
        });
    }
    v
}

// ── create_split: happy path ──────────────────────────────────────────────────

#[test]
fn create_split_happy_path() {
    let (env, client) = setup();
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    let a = Address::generate(&env);
    let b = Address::generate(&env);

    // In soroban-sdk 28, the generated client returns T directly (not Result<T>);
    // use try_* for the error-returning variant.
    let id = client.create_split(
        &creator,
        &token,
        &recipients(&env, &[(a.clone(), 7000), (b.clone(), 3000)]),
        &fake_hash(&env, 1),
    );

    assert_eq!(id, 1u64);

    let split = client.get_split(&id).unwrap();
    assert_eq!(split.id, 1);
    assert_eq!(split.creator, creator);
    assert_eq!(split.token, token);
    assert_eq!(split.total_received, 0);
    assert_eq!(split.version, 1);
    assert!(matches!(split.status, SplitStatus::Pending));
}

// ── create_split: sequential id assignment ────────────────────────────────────

#[test]
fn create_split_sequential_ids() {
    let (env, client) = setup();
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    let a = Address::generate(&env);
    let b = Address::generate(&env);
    let c = Address::generate(&env);

    let id1 = client.create_split(
        &creator,
        &token,
        &recipients(&env, &[(a.clone(), 5000), (b.clone(), 5000)]),
        &fake_hash(&env, 1),
    );

    let id2 = client.create_split(
        &creator,
        &token,
        &recipients(&env, &[(a.clone(), 3000), (b.clone(), 3000), (c.clone(), 4000)]),
        &fake_hash(&env, 2),
    );

    assert_eq!(id1, 1u64);
    assert_eq!(id2, 2u64);
}

// ── create_split: recipient count validation ──────────────────────────────────

#[test]
fn create_split_single_recipient_rejected() {
    let (env, client) = setup();
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    let a = Address::generate(&env);

    let err = client
        .try_create_split(
            &creator,
            &token,
            &recipients(&env, &[(a, 10000)]),
            &fake_hash(&env, 0),
        )
        .unwrap_err()
        .unwrap();

    assert_eq!(err, Error::InvalidRecipientCount);
}

#[test]
fn create_split_zero_recipients_rejected() {
    let (env, client) = setup();
    let creator = Address::generate(&env);
    let token = Address::generate(&env);

    let err = client
        .try_create_split(
            &creator,
            &token,
            &Vec::new(&env),
            &fake_hash(&env, 0),
        )
        .unwrap_err()
        .unwrap();

    assert_eq!(err, Error::InvalidRecipientCount);
}

#[test]
fn create_split_21_recipients_rejected() {
    let (env, client) = setup();
    let creator = Address::generate(&env);
    let token = Address::generate(&env);

    let mut pairs: std::vec::Vec<(Address, u32)> = std::vec::Vec::new();
    for _ in 0..21 {
        pairs.push((Address::generate(&env), 476));
    }

    let err = client
        .try_create_split(
            &creator,
            &token,
            &recipients(&env, &pairs),
            &fake_hash(&env, 0),
        )
        .unwrap_err()
        .unwrap();

    assert_eq!(err, Error::InvalidRecipientCount);
}

// ── create_split: zero bps ────────────────────────────────────────────────────

#[test]
fn create_split_zero_bps_rejected() {
    let (env, client) = setup();
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    let a = Address::generate(&env);
    let b = Address::generate(&env);

    let err = client
        .try_create_split(
            &creator,
            &token,
            &recipients(&env, &[(a, 10000), (b, 0)]),
            &fake_hash(&env, 0),
        )
        .unwrap_err()
        .unwrap();

    assert_eq!(err, Error::ZeroShare);
}

// ── create_split: duplicate addresses ─────────────────────────────────────────

#[test]
fn create_split_duplicate_recipient_rejected() {
    let (env, client) = setup();
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    let a = Address::generate(&env);

    let err = client
        .try_create_split(
            &creator,
            &token,
            &recipients(&env, &[(a.clone(), 5000), (a.clone(), 5000)]),
            &fake_hash(&env, 0),
        )
        .unwrap_err()
        .unwrap();

    assert_eq!(err, Error::DuplicateRecipient);
}

// ── create_split: bps total validation ───────────────────────────────────────

#[test]
fn create_split_bps_sum_9999_rejected() {
    let (env, client) = setup();
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    let a = Address::generate(&env);
    let b = Address::generate(&env);

    let err = client
        .try_create_split(
            &creator,
            &token,
            &recipients(&env, &[(a, 5000), (b, 4999)]),
            &fake_hash(&env, 0),
        )
        .unwrap_err()
        .unwrap();

    assert_eq!(err, Error::BpsTotalInvalid);
}

#[test]
fn create_split_bps_sum_10001_rejected() {
    let (env, client) = setup();
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    let a = Address::generate(&env);
    let b = Address::generate(&env);

    let err = client
        .try_create_split(
            &creator,
            &token,
            &recipients(&env, &[(a, 5000), (b, 5001)]),
            &fake_hash(&env, 0),
        )
        .unwrap_err()
        .unwrap();

    assert_eq!(err, Error::BpsTotalInvalid);
}

// ── create_split: auth ────────────────────────────────────────────────────────

#[test]
fn create_split_records_creator_auth() {
    let (env, client) = setup();
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    let a = Address::generate(&env);
    let b = Address::generate(&env);
    let hash = fake_hash(&env, 42);

    client.create_split(
        &creator,
        &token,
        &recipients(&env, &[(a.clone(), 6000), (b.clone(), 4000)]),
        &hash,
    );

    // Verify the SDK recorded that `creator` authorised `create_split`
    let auths = env.auths();
    assert!(!auths.is_empty());
    let (auth_addr, _invocation) = &auths[0];
    assert_eq!(*auth_addr, creator);
}

// ── get_split: not found ──────────────────────────────────────────────────────

#[test]
fn get_split_not_found_returns_none() {
    let (_env, client) = setup();
    assert!(client.get_split(&999u64).is_none());
}

// ── accept: partial acceptance stays Pending ─────────────────────────────────

#[test]
fn accept_partial_stays_pending() {
    let (env, client) = setup();
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    let a = Address::generate(&env);
    let b = Address::generate(&env);

    let id = client.create_split(
        &creator,
        &token,
        &recipients(&env, &[(a.clone(), 5000), (b.clone(), 5000)]),
        &fake_hash(&env, 1),
    );

    client.accept(&id, &a);

    let split = client.get_split(&id).unwrap();
    assert!(matches!(split.status, SplitStatus::Pending));
}

// ── accept: full acceptance activates ────────────────────────────────────────

#[test]
fn accept_all_activates() {
    let (env, client) = setup();
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    let a = Address::generate(&env);
    let b = Address::generate(&env);

    let id = client.create_split(
        &creator,
        &token,
        &recipients(&env, &[(a.clone(), 5000), (b.clone(), 5000)]),
        &fake_hash(&env, 1),
    );

    client.accept(&id, &a);
    client.accept(&id, &b);

    let split = client.get_split(&id).unwrap();
    assert!(matches!(split.status, SplitStatus::Active));
}

// ── accept: non-recipient rejected ───────────────────────────────────────────

#[test]
fn accept_non_recipient_rejected() {
    let (env, client) = setup();
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    let a = Address::generate(&env);
    let b = Address::generate(&env);
    let outsider = Address::generate(&env);

    let id = client.create_split(
        &creator,
        &token,
        &recipients(&env, &[(a.clone(), 5000), (b.clone(), 5000)]),
        &fake_hash(&env, 1),
    );

    let err = client
        .try_accept(&id, &outsider)
        .unwrap_err()
        .unwrap();

    assert_eq!(err, Error::NotARecipient);
}

// ── accept: double-accept rejected ───────────────────────────────────────────

#[test]
fn accept_double_accept_rejected() {
    let (env, client) = setup();
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    let a = Address::generate(&env);
    let b = Address::generate(&env);

    let id = client.create_split(
        &creator,
        &token,
        &recipients(&env, &[(a.clone(), 5000), (b.clone(), 5000)]),
        &fake_hash(&env, 1),
    );

    client.accept(&id, &a);

    let err = client.try_accept(&id, &a).unwrap_err().unwrap();
    assert_eq!(err, Error::AlreadyAccepted);
}

// ── accept: already-active split rejected ────────────────────────────────────

#[test]
fn accept_on_active_split_rejected() {
    let (env, client) = setup();
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    let a = Address::generate(&env);
    let b = Address::generate(&env);

    let id = client.create_split(
        &creator,
        &token,
        &recipients(&env, &[(a.clone(), 5000), (b.clone(), 5000)]),
        &fake_hash(&env, 1),
    );

    // Activate the split
    client.accept(&id, &a);
    client.accept(&id, &b);

    // Any recipient tries to accept again — should get NotPending
    let err = client.try_accept(&id, &a).unwrap_err().unwrap();
    assert_eq!(err, Error::NotPending);
}

// ── accept: split not found ───────────────────────────────────────────────────

#[test]
fn accept_split_not_found() {
    let (env, client) = setup();
    let a = Address::generate(&env);

    let err = client.try_accept(&999u64, &a).unwrap_err().unwrap();
    assert_eq!(err, Error::SplitNotFound);
}

// ── accept: 3-recipient activation requires all 3 ────────────────────────────

#[test]
fn accept_three_recipients_all_must_accept() {
    let (env, client) = setup();
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    let a = Address::generate(&env);
    let b = Address::generate(&env);
    let c = Address::generate(&env);

    let id = client.create_split(
        &creator,
        &token,
        &recipients(&env, &[(a.clone(), 4000), (b.clone(), 3000), (c.clone(), 3000)]),
        &fake_hash(&env, 1),
    );

    client.accept(&id, &a);
    assert!(matches!(
        client.get_split(&id).unwrap().status,
        SplitStatus::Pending
    ));

    client.accept(&id, &b);
    assert!(matches!(
        client.get_split(&id).unwrap().status,
        SplitStatus::Pending
    ));

    client.accept(&id, &c);
    assert!(matches!(
        client.get_split(&id).unwrap().status,
        SplitStatus::Active
    ));
}
