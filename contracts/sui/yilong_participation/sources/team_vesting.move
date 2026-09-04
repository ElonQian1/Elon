// Copyright (c) 2026 Yilong Project.
// SPDX-License-Identifier: Apache-2.0

/// Immutable-beneficiary linear vesting for the genesis ESK team bucket.
///
/// The object is address-owned and deliberately lacks `store`: there is no
/// public transfer, revocation, recovery, schedule update, or beneficiary
/// replacement path. Time is read only from Sui's immutable Clock input.
module yilong_participation::team_vesting;

use esk_currency::esk::ESK;
use sui::balance::{Self, Balance};
use sui::clock::{Self, Clock};
use sui::coin::{Self, Coin};
use sui::object::{Self, ID, UID};

const E_ZERO_BENEFICIARY: u64 = 0;
const E_ZERO_BALANCE: u64 = 1;
const E_START_IN_PAST: u64 = 2;
const E_INVALID_SCHEDULE: u64 = 3;
const E_NOT_BENEFICIARY: u64 = 4;
const E_NOTHING_CLAIMABLE: u64 = 5;
const E_INVARIANT: u64 = 6;

/// The single address-owned lock for the genesis team allocation.
///
/// `key` without `store` prevents generic public transfer or wrapping. Only
/// this module can initially transfer the object, and it exposes no later
/// transfer function.
public struct TeamVesting has key {
    id: UID,
    beneficiary: address,
    total_base_units: u64,
    claimed_base_units: u64,
    start_ms: u64,
    cliff_ms: u64,
    end_ms: u64,
    remaining: Balance<ESK>,
}

/// Creates the canonical team lock and transfers it directly to its permanent
/// beneficiary. Package visibility prevents unrelated callers from creating
/// look-alike genesis locks through this API.
public(package) fun create_and_transfer(
    locked_coin: Coin<ESK>,
    beneficiary: address,
    start_ms: u64,
    cliff_ms: u64,
    end_ms: u64,
    clock: &Clock,
    ctx: &mut TxContext,
): ID {
    assert!(beneficiary != @0x0, E_ZERO_BENEFICIARY);
    let total_base_units = coin::value(&locked_coin);
    assert!(total_base_units > 0, E_ZERO_BALANCE);

    let now_ms = clock::timestamp_ms(clock);
    assert!(start_ms >= now_ms, E_START_IN_PAST);
    assert!(start_ms < cliff_ms && cliff_ms < end_ms, E_INVALID_SCHEDULE);

    let vesting = TeamVesting {
        id: object::new(ctx),
        beneficiary,
        total_base_units,
        claimed_base_units: 0,
        start_ms,
        cliff_ms,
        end_ms,
        remaining: coin::into_balance(locked_coin),
    };
    let vesting_id = object::id(&vesting);
    transfer::transfer(vesting, beneficiary);
    vesting_id
}

/// Claims every newly vested base unit and transfers the resulting Coin only
/// to the beneficiary recorded at genesis.
public fun claim(
    vesting: &mut TeamVesting,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert!(ctx.sender() == vesting.beneficiary, E_NOT_BENEFICIARY);
    assert_invariant(vesting);

    let vested = vested_at(vesting, clock::timestamp_ms(clock));
    assert!(vested >= vesting.claimed_base_units, E_INVARIANT);
    let amount = vested - vesting.claimed_base_units;
    assert!(amount > 0, E_NOTHING_CLAIMABLE);

    let claimed_balance = balance::split(&mut vesting.remaining, amount);
    vesting.claimed_base_units = vesting.claimed_base_units + amount;
    assert_invariant(vesting);

    let claimed_coin = coin::from_balance(claimed_balance, ctx);
    transfer::public_transfer(claimed_coin, vesting.beneficiary);
}

/// Amount currently vested under the cliff-plus-linear schedule.
public fun vested_base_units(vesting: &TeamVesting, clock: &Clock): u64 {
    assert_invariant(vesting);
    vested_at(vesting, clock::timestamp_ms(clock))
}

/// Amount the beneficiary can claim now without creating a zero-value Coin.
public fun claimable(vesting: &TeamVesting, clock: &Clock): u64 {
    assert_invariant(vesting);
    let vested = vested_at(vesting, clock::timestamp_ms(clock));
    assert!(vested >= vesting.claimed_base_units, E_INVARIANT);
    vested - vesting.claimed_base_units
}

public fun vesting_id(vesting: &TeamVesting): ID { object::id(vesting) }

public fun beneficiary(vesting: &TeamVesting): address { vesting.beneficiary }

public fun total_base_units(vesting: &TeamVesting): u64 { vesting.total_base_units }

public fun claimed_base_units(vesting: &TeamVesting): u64 { vesting.claimed_base_units }

public fun remaining_base_units(vesting: &TeamVesting): u64 {
    assert_invariant(vesting);
    balance::value(&vesting.remaining)
}

public fun start_ms(vesting: &TeamVesting): u64 { vesting.start_ms }

public fun cliff_ms(vesting: &TeamVesting): u64 { vesting.cliff_ms }

public fun end_ms(vesting: &TeamVesting): u64 { vesting.end_ms }

/// Cliff blocks all release. At and after the cliff, vesting catches up to the
/// linear start-to-end curve. The end branch releases every rounding remainder.
fun vested_at(vesting: &TeamVesting, now_ms: u64): u64 {
    if (now_ms < vesting.cliff_ms) {
        0
    } else if (now_ms >= vesting.end_ms) {
        vesting.total_base_units
    } else {
        let elapsed = (now_ms - vesting.start_ms) as u128;
        let duration = (vesting.end_ms - vesting.start_ms) as u128;
        (((vesting.total_base_units as u128) * elapsed) / duration) as u64
    }
}

fun assert_invariant(vesting: &TeamVesting) {
    assert!(vesting.claimed_base_units <= vesting.total_base_units, E_INVARIANT);
    assert!(
        balance::value(&vesting.remaining) ==
            vesting.total_base_units - vesting.claimed_base_units,
        E_INVARIANT,
    );
}
