// Copyright (c) 2026 Yilong Project.
// SPDX-License-Identifier: Apache-2.0

/// One-shot six-bucket allocation of the fixed ESK genesis supply.
///
/// This module cannot mint or burn ESK. It consumes the one fixed-supply Coin
/// plus a unique capability, transfers every base unit, creates the team lock,
/// and freezes a receipt containing enough facts for independent reconstruction.
module yilong_participation::genesis_allocation;

use esk_currency::esk::{Self, ESK};
use sui::clock::{Self, Clock};
use sui::coin::{Self, Coin};
use sui::object::{Self, ID, UID};
use yilong_participation::team_vesting;

const MANIFEST_DIGEST_LENGTH: u64 = 32;

const E_ZERO_ROLE: u64 = 0;
const E_ROLE_COLLISION: u64 = 1;
const E_ZERO_BUCKET: u64 = 2;
const E_NOT_TOTAL_SUPPLY: u64 = 3;
const E_ALLOCATION_SUM: u64 = 4;
const E_MANIFEST_DIGEST: u64 = 5;
const E_REMAINDER: u64 = 6;

/// Unique bearer authority created once by this module's initializer.
///
/// `store` permits an explicit handoff from the publishing account to the
/// approved distribution role. The value has neither `copy` nor `drop` and is
/// destroyed by the sole allocation entry point.
public struct GenesisAllocationCap has key, store {
    id: UID,
}

/// Immutable evidence for the only canonical allocation performed by this
/// package instance.
public struct GenesisAllocationReceipt has key {
    id: UID,
    manifest_digest: vector<u8>,
    total_base_units: u64,
    distribution: address,
    team_beneficiary: address,
    treasury: address,
    liquidity_recipient: address,
    user_migration_and_ecosystem_units: u64,
    team_vesting_units: u64,
    project_treasury_units: u64,
    liquidity_units: u64,
    community_contributors_units: u64,
    security_operations_reserve_units: u64,
    start_ms: u64,
    cliff_ms: u64,
    end_ms: u64,
    executed_at_ms: u64,
    user_migration_and_ecosystem_coin_id: ID,
    team_vesting_id: ID,
    project_treasury_coin_id: ID,
    liquidity_coin_id: ID,
    community_contributors_coin_id: ID,
    security_operations_reserve_coin_id: ID,
}

fun init(ctx: &mut TxContext) {
    transfer::public_transfer(
        GenesisAllocationCap { id: object::new(ctx) },
        ctx.sender(),
    );
}

/// Consumes the unique allocation authority and the complete fixed ESK supply.
/// All outputs are transferred inside this transaction; nothing is returned to
/// the caller and there is no residual balance or administrative withdrawal.
public fun allocate(
    cap: GenesisAllocationCap,
    mut supply: Coin<ESK>,
    distribution: address,
    team_beneficiary: address,
    treasury: address,
    liquidity_recipient: address,
    user_migration_and_ecosystem_units: u64,
    team_vesting_units: u64,
    project_treasury_units: u64,
    liquidity_units: u64,
    community_contributors_units: u64,
    security_operations_reserve_units: u64,
    start_ms: u64,
    cliff_ms: u64,
    end_ms: u64,
    manifest_digest: vector<u8>,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert_roles(distribution, team_beneficiary, treasury, liquidity_recipient);
    assert!(vector::length(&manifest_digest) == MANIFEST_DIGEST_LENGTH, E_MANIFEST_DIGEST);
    assert_positive_buckets(
        user_migration_and_ecosystem_units,
        team_vesting_units,
        project_treasury_units,
        liquidity_units,
        community_contributors_units,
        security_operations_reserve_units,
    );

    let total_base_units = coin::value(&supply);
    assert!(total_base_units == esk::total_supply_base_units(), E_NOT_TOTAL_SUPPLY);
    let allocation_sum = (user_migration_and_ecosystem_units as u128) +
        (team_vesting_units as u128) +
        (project_treasury_units as u128) +
        (liquidity_units as u128) +
        (community_contributors_units as u128) +
        (security_operations_reserve_units as u128);
    assert!(allocation_sum == (total_base_units as u128), E_ALLOCATION_SUM);

    let user_migration_coin = coin::split(
        &mut supply,
        user_migration_and_ecosystem_units,
        ctx,
    );
    let team_coin = coin::split(&mut supply, team_vesting_units, ctx);
    let project_treasury_coin = coin::split(&mut supply, project_treasury_units, ctx);
    let liquidity_coin = coin::split(&mut supply, liquidity_units, ctx);
    let community_coin = coin::split(&mut supply, community_contributors_units, ctx);
    assert!(coin::value(&supply) == security_operations_reserve_units, E_REMAINDER);

    let user_migration_coin_id = object::id(&user_migration_coin);
    let project_treasury_coin_id = object::id(&project_treasury_coin);
    let liquidity_coin_id = object::id(&liquidity_coin);
    let community_coin_id = object::id(&community_coin);
    let security_reserve_coin_id = object::id(&supply);
    let executed_at_ms = clock::timestamp_ms(clock);
    let team_vesting_id = team_vesting::create_and_transfer(
        team_coin,
        team_beneficiary,
        start_ms,
        cliff_ms,
        end_ms,
        clock,
        ctx,
    );

    let receipt = GenesisAllocationReceipt {
        id: object::new(ctx),
        manifest_digest,
        total_base_units,
        distribution,
        team_beneficiary,
        treasury,
        liquidity_recipient,
        user_migration_and_ecosystem_units,
        team_vesting_units,
        project_treasury_units,
        liquidity_units,
        community_contributors_units,
        security_operations_reserve_units,
        start_ms,
        cliff_ms,
        end_ms,
        executed_at_ms,
        user_migration_and_ecosystem_coin_id: user_migration_coin_id,
        team_vesting_id,
        project_treasury_coin_id,
        liquidity_coin_id,
        community_contributors_coin_id: community_coin_id,
        security_operations_reserve_coin_id: security_reserve_coin_id,
    };

    let GenesisAllocationCap { id } = cap;
    id.delete();
    transfer::freeze_object(receipt);
    transfer::public_transfer(user_migration_coin, distribution);
    transfer::public_transfer(community_coin, distribution);
    transfer::public_transfer(project_treasury_coin, treasury);
    transfer::public_transfer(supply, treasury);
    transfer::public_transfer(liquidity_coin, liquidity_recipient);
}

fun assert_positive_buckets(
    user_migration_and_ecosystem_units: u64,
    team_vesting_units: u64,
    project_treasury_units: u64,
    liquidity_units: u64,
    community_contributors_units: u64,
    security_operations_reserve_units: u64,
) {
    assert!(user_migration_and_ecosystem_units > 0, E_ZERO_BUCKET);
    assert!(team_vesting_units > 0, E_ZERO_BUCKET);
    assert!(project_treasury_units > 0, E_ZERO_BUCKET);
    assert!(liquidity_units > 0, E_ZERO_BUCKET);
    assert!(community_contributors_units > 0, E_ZERO_BUCKET);
    assert!(security_operations_reserve_units > 0, E_ZERO_BUCKET);
}

fun assert_roles(
    distribution: address,
    team_beneficiary: address,
    treasury: address,
    liquidity_recipient: address,
) {
    assert!(
        distribution != @0x0 && team_beneficiary != @0x0 &&
            treasury != @0x0 && liquidity_recipient != @0x0,
        E_ZERO_ROLE,
    );
    assert!(distribution != team_beneficiary, E_ROLE_COLLISION);
    assert!(distribution != treasury, E_ROLE_COLLISION);
    assert!(distribution != liquidity_recipient, E_ROLE_COLLISION);
    assert!(team_beneficiary != treasury, E_ROLE_COLLISION);
    assert!(team_beneficiary != liquidity_recipient, E_ROLE_COLLISION);
    assert!(treasury != liquidity_recipient, E_ROLE_COLLISION);
}

#[test_only]
public fun init_for_testing(ctx: &mut TxContext) { init(ctx) }

public fun allocation_cap_id(cap: &GenesisAllocationCap): ID { object::id(cap) }

public fun receipt_id(receipt: &GenesisAllocationReceipt): ID { object::id(receipt) }

public fun manifest_digest(receipt: &GenesisAllocationReceipt): &vector<u8> {
    &receipt.manifest_digest
}

public fun total_base_units(receipt: &GenesisAllocationReceipt): u64 {
    receipt.total_base_units
}

public fun distribution(receipt: &GenesisAllocationReceipt): address { receipt.distribution }

public fun team_beneficiary(receipt: &GenesisAllocationReceipt): address {
    receipt.team_beneficiary
}

public fun treasury(receipt: &GenesisAllocationReceipt): address { receipt.treasury }

public fun liquidity_recipient(receipt: &GenesisAllocationReceipt): address {
    receipt.liquidity_recipient
}

public fun user_migration_and_ecosystem_units(receipt: &GenesisAllocationReceipt): u64 {
    receipt.user_migration_and_ecosystem_units
}

public fun team_vesting_units(receipt: &GenesisAllocationReceipt): u64 {
    receipt.team_vesting_units
}

public fun project_treasury_units(receipt: &GenesisAllocationReceipt): u64 {
    receipt.project_treasury_units
}

public fun liquidity_units(receipt: &GenesisAllocationReceipt): u64 { receipt.liquidity_units }

public fun community_contributors_units(receipt: &GenesisAllocationReceipt): u64 {
    receipt.community_contributors_units
}

public fun security_operations_reserve_units(receipt: &GenesisAllocationReceipt): u64 {
    receipt.security_operations_reserve_units
}

public fun start_ms(receipt: &GenesisAllocationReceipt): u64 { receipt.start_ms }

public fun cliff_ms(receipt: &GenesisAllocationReceipt): u64 { receipt.cliff_ms }

public fun end_ms(receipt: &GenesisAllocationReceipt): u64 { receipt.end_ms }

public fun executed_at_ms(receipt: &GenesisAllocationReceipt): u64 { receipt.executed_at_ms }

public fun user_migration_and_ecosystem_coin_id(receipt: &GenesisAllocationReceipt): ID {
    receipt.user_migration_and_ecosystem_coin_id
}

public fun team_vesting_id(receipt: &GenesisAllocationReceipt): ID { receipt.team_vesting_id }

public fun project_treasury_coin_id(receipt: &GenesisAllocationReceipt): ID {
    receipt.project_treasury_coin_id
}

public fun liquidity_coin_id(receipt: &GenesisAllocationReceipt): ID {
    receipt.liquidity_coin_id
}

public fun community_contributors_coin_id(receipt: &GenesisAllocationReceipt): ID {
    receipt.community_contributors_coin_id
}

public fun security_operations_reserve_coin_id(receipt: &GenesisAllocationReceipt): ID {
    receipt.security_operations_reserve_coin_id
}
