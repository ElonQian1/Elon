// Copyright (c) 2026 Yilong Project.
// SPDX-License-Identifier: Apache-2.0

#[test_only]
module yilong_participation::genesis_allocation_tests;

use esk_currency::esk::ESK;
use sui::clock::{Self, Clock};
use sui::coin::{Self, Coin, TreasuryCap};
use sui::object;
use sui::test_scenario::{Self, Scenario};
use yilong_participation::genesis_allocation::{Self, GenesisAllocationReceipt};
use yilong_participation::team_vesting::{Self, TeamVesting};

const PUBLISHER: address = @0xA11CE;
const DISTRIBUTION: address = @0xD157;
const BENEFICIARY: address = @0x7EA0;
const TREASURY: address = @0x7EA5;
const LIQUIDITY: address = @0x1110;
const NOW_MS: u64 = 1_000;

const USER_UNITS: u64 = 250_000_000_000_000;
const TEAM_UNITS: u64 = 200_000_000_000_000;
const TREASURY_UNITS: u64 = 250_000_000_000_000;
const LIQUIDITY_UNITS: u64 = 150_000_000_000_000;
const COMMUNITY_UNITS: u64 = 100_000_000_000_000;
const SECURITY_UNITS: u64 = 50_000_000_000_000;
const TOTAL_UNITS: u64 = 1_000_000_000_000_000;

fun manifest_digest(): vector<u8> {
    let mut result = vector[];
    let mut index = 0u64;
    while (index < 32) {
        result.push_back(0xA5);
        index = index + 1;
    };
    result
}

fun assert_digest(actual: &vector<u8>, expected: &vector<u8>) {
    assert!(actual.length() == expected.length());
    let mut index = 0u64;
    while (index < actual.length()) {
        assert!(*actual.borrow(index) == *expected.borrow(index));
        index = index + 1;
    }
}

fun start_allocation(scenario: &mut Scenario): Clock {
    let mut clock = clock::create_for_testing(scenario.ctx());
    clock::set_for_testing(&mut clock, NOW_MS);
    genesis_allocation::init_for_testing(scenario.ctx());
    test_scenario::next_tx(scenario, PUBLISHER);
    clock
}

fun allocate_valid(scenario: &mut Scenario, clock: &Clock) {
    let cap = test_scenario::take_from_sender(scenario);
    let supply = coin::mint_for_testing<ESK>(TOTAL_UNITS, scenario.ctx());
    genesis_allocation::allocate(
        cap,
        supply,
        DISTRIBUTION,
        BENEFICIARY,
        TREASURY,
        LIQUIDITY,
        USER_UNITS,
        TEAM_UNITS,
        TREASURY_UNITS,
        LIQUIDITY_UNITS,
        COMMUNITY_UNITS,
        SECURITY_UNITS,
        NOW_MS,
        NOW_MS + 200,
        NOW_MS + 1_100,
        manifest_digest(),
        clock,
        scenario.ctx(),
    );
}

#[test]
fun exactly_six_buckets_are_delivered_and_receipted_once() {
    let mut scenario = test_scenario::begin(PUBLISHER);
    let clock = start_allocation(&mut scenario);
    allocate_valid(&mut scenario, &clock);

    assert!(!test_scenario::has_most_recent_for_sender<genesis_allocation::GenesisAllocationCap>(&scenario));
    assert!(!test_scenario::has_most_recent_for_sender<TreasuryCap<ESK>>(&scenario));

    test_scenario::next_tx(&mut scenario, DISTRIBUTION);
    let distribution_one = test_scenario::take_from_sender<Coin<ESK>>(&scenario);
    let distribution_two = test_scenario::take_from_sender<Coin<ESK>>(&scenario);
    let first_distribution = distribution_one.value();
    let second_distribution = distribution_two.value();
    let first_distribution_id = object::id(&distribution_one);
    let second_distribution_id = object::id(&distribution_two);
    assert!(
        (first_distribution == USER_UNITS && second_distribution == COMMUNITY_UNITS)
            || (first_distribution == COMMUNITY_UNITS && second_distribution == USER_UNITS),
    );
    test_scenario::return_to_sender(&scenario, distribution_one);
    test_scenario::return_to_sender(&scenario, distribution_two);

    test_scenario::next_tx(&mut scenario, TREASURY);
    let treasury_one = test_scenario::take_from_sender<Coin<ESK>>(&scenario);
    let treasury_two = test_scenario::take_from_sender<Coin<ESK>>(&scenario);
    let first_treasury = treasury_one.value();
    let second_treasury = treasury_two.value();
    let first_treasury_id = object::id(&treasury_one);
    let second_treasury_id = object::id(&treasury_two);
    assert!(
        (first_treasury == TREASURY_UNITS && second_treasury == SECURITY_UNITS)
            || (first_treasury == SECURITY_UNITS && second_treasury == TREASURY_UNITS),
    );
    test_scenario::return_to_sender(&scenario, treasury_one);
    test_scenario::return_to_sender(&scenario, treasury_two);

    test_scenario::next_tx(&mut scenario, LIQUIDITY);
    let liquidity = test_scenario::take_from_sender<Coin<ESK>>(&scenario);
    assert!(liquidity.value() == LIQUIDITY_UNITS);
    let liquidity_id = object::id(&liquidity);
    test_scenario::return_to_sender(&scenario, liquidity);

    test_scenario::next_tx(&mut scenario, BENEFICIARY);
    let vesting = test_scenario::take_from_sender<TeamVesting>(&scenario);
    assert!(team_vesting::beneficiary(&vesting) == BENEFICIARY);
    assert!(team_vesting::total_base_units(&vesting) == TEAM_UNITS);
    assert!(team_vesting::claimed_base_units(&vesting) == 0);
    assert!(team_vesting::remaining_base_units(&vesting) == TEAM_UNITS);

    let receipt = test_scenario::take_immutable<GenesisAllocationReceipt>(&scenario);
    assert!(genesis_allocation::user_migration_and_ecosystem_units(&receipt) == USER_UNITS);
    assert!(genesis_allocation::team_vesting_units(&receipt) == TEAM_UNITS);
    assert!(genesis_allocation::project_treasury_units(&receipt) == TREASURY_UNITS);
    assert!(genesis_allocation::liquidity_units(&receipt) == LIQUIDITY_UNITS);
    assert!(genesis_allocation::community_contributors_units(&receipt) == COMMUNITY_UNITS);
    assert!(genesis_allocation::security_operations_reserve_units(&receipt) == SECURITY_UNITS);
    assert!(genesis_allocation::total_base_units(&receipt) == TOTAL_UNITS);
    assert!(genesis_allocation::distribution(&receipt) == DISTRIBUTION);
    assert!(genesis_allocation::team_beneficiary(&receipt) == BENEFICIARY);
    assert!(genesis_allocation::treasury(&receipt) == TREASURY);
    assert!(genesis_allocation::liquidity_recipient(&receipt) == LIQUIDITY);
    assert!(genesis_allocation::start_ms(&receipt) == NOW_MS);
    assert!(genesis_allocation::cliff_ms(&receipt) == NOW_MS + 200);
    assert!(genesis_allocation::end_ms(&receipt) == NOW_MS + 1_100);
    assert!(genesis_allocation::executed_at_ms(&receipt) == NOW_MS);
    assert!(genesis_allocation::team_vesting_id(&receipt) == object::id(&vesting));
    assert!(genesis_allocation::team_vesting_id(&receipt) == team_vesting::vesting_id(&vesting));
    if (first_distribution == USER_UNITS) {
        assert!(genesis_allocation::user_migration_and_ecosystem_coin_id(&receipt) == first_distribution_id);
        assert!(genesis_allocation::community_contributors_coin_id(&receipt) == second_distribution_id);
    } else {
        assert!(genesis_allocation::user_migration_and_ecosystem_coin_id(&receipt) == second_distribution_id);
        assert!(genesis_allocation::community_contributors_coin_id(&receipt) == first_distribution_id);
    };
    if (first_treasury == TREASURY_UNITS) {
        assert!(genesis_allocation::project_treasury_coin_id(&receipt) == first_treasury_id);
        assert!(genesis_allocation::security_operations_reserve_coin_id(&receipt) == second_treasury_id);
    } else {
        assert!(genesis_allocation::project_treasury_coin_id(&receipt) == second_treasury_id);
        assert!(genesis_allocation::security_operations_reserve_coin_id(&receipt) == first_treasury_id);
    };
    assert!(genesis_allocation::liquidity_coin_id(&receipt) == liquidity_id);
    let expected_digest = manifest_digest();
    assert_digest(genesis_allocation::manifest_digest(&receipt), &expected_digest);
    assert!(
        first_distribution + second_distribution + first_treasury + second_treasury
            + LIQUIDITY_UNITS + team_vesting::remaining_base_units(&vesting)
            == TOTAL_UNITS,
    );

    test_scenario::return_to_sender(&scenario, vesting);
    test_scenario::return_immutable(receipt);
    clock::destroy_for_testing(clock);
    test_scenario::end(scenario);
}

fun invalid_allocation(
    distribution: address,
    beneficiary: address,
    treasury: address,
    liquidity: address,
    user_units: u64,
    team_units: u64,
    treasury_units: u64,
    liquidity_units: u64,
    community_units: u64,
    security_units: u64,
    start_ms: u64,
    cliff_ms: u64,
    end_ms: u64,
    digest: vector<u8>,
) {
    let mut scenario = test_scenario::begin(PUBLISHER);
    let clock = start_allocation(&mut scenario);
    let cap = test_scenario::take_from_sender(&scenario);
    let supply = coin::mint_for_testing<ESK>(TOTAL_UNITS, scenario.ctx());
    genesis_allocation::allocate(
        cap, supply, distribution, beneficiary, treasury, liquidity,
        user_units, team_units, treasury_units, liquidity_units, community_units, security_units,
        start_ms, cliff_ms, end_ms, digest, &clock, scenario.ctx(),
    );
    clock::destroy_for_testing(clock);
    test_scenario::end(scenario);
}

#[test, expected_failure(abort_code = 4, location = yilong_participation::genesis_allocation)]
fun reject_bucket_sum_mismatch() {
    invalid_allocation(DISTRIBUTION, BENEFICIARY, TREASURY, LIQUIDITY,
        USER_UNITS + 1, TEAM_UNITS, TREASURY_UNITS, LIQUIDITY_UNITS,
        COMMUNITY_UNITS, SECURITY_UNITS, NOW_MS + 100, NOW_MS + 200, NOW_MS + 1_100,
        manifest_digest());
}

#[test, expected_failure(abort_code = 3, location = yilong_participation::genesis_allocation)]
fun reject_coin_that_is_not_the_complete_fixed_supply() {
    let mut scenario = test_scenario::begin(PUBLISHER);
    let clock = start_allocation(&mut scenario);
    let cap = test_scenario::take_from_sender(&scenario);
    let supply = coin::mint_for_testing<ESK>(TOTAL_UNITS - 1, scenario.ctx());
    genesis_allocation::allocate(
        cap, supply, DISTRIBUTION, BENEFICIARY, TREASURY, LIQUIDITY,
        USER_UNITS, TEAM_UNITS, TREASURY_UNITS, LIQUIDITY_UNITS,
        COMMUNITY_UNITS, SECURITY_UNITS, NOW_MS, NOW_MS + 200, NOW_MS + 1_100,
        manifest_digest(), &clock, scenario.ctx(),
    );
    clock::destroy_for_testing(clock);
    test_scenario::end(scenario);
}

#[test, expected_failure(abort_code = 2, location = yilong_participation::genesis_allocation)]
fun reject_any_zero_bucket() {
    invalid_allocation(DISTRIBUTION, BENEFICIARY, TREASURY, LIQUIDITY,
        USER_UNITS + TEAM_UNITS, 0, TREASURY_UNITS, LIQUIDITY_UNITS,
        COMMUNITY_UNITS, SECURITY_UNITS, NOW_MS + 100, NOW_MS + 200, NOW_MS + 1_100,
        manifest_digest());
}

#[test, expected_failure(abort_code = 1, location = yilong_participation::genesis_allocation)]
fun reject_zero_or_repeated_roles() {
    invalid_allocation(DISTRIBUTION, BENEFICIARY, TREASURY, DISTRIBUTION,
        USER_UNITS, TEAM_UNITS, TREASURY_UNITS, LIQUIDITY_UNITS,
        COMMUNITY_UNITS, SECURITY_UNITS, NOW_MS + 100, NOW_MS + 200, NOW_MS + 1_100,
        manifest_digest());
}

#[test, expected_failure(abort_code = 0, location = yilong_participation::genesis_allocation)]
fun reject_zero_role_address() {
    invalid_allocation(@0x0, BENEFICIARY, TREASURY, LIQUIDITY,
        USER_UNITS, TEAM_UNITS, TREASURY_UNITS, LIQUIDITY_UNITS,
        COMMUNITY_UNITS, SECURITY_UNITS, NOW_MS + 100, NOW_MS + 200, NOW_MS + 1_100,
        manifest_digest());
}

#[test, expected_failure(abort_code = 5, location = yilong_participation::genesis_allocation)]
fun reject_wrong_manifest_digest_length() {
    invalid_allocation(DISTRIBUTION, BENEFICIARY, TREASURY, LIQUIDITY,
        USER_UNITS, TEAM_UNITS, TREASURY_UNITS, LIQUIDITY_UNITS,
        COMMUNITY_UNITS, SECURITY_UNITS, NOW_MS + 100, NOW_MS + 200, NOW_MS + 1_100,
        vector[0xA5]);
}

#[test, expected_failure(abort_code = 2, location = yilong_participation::team_vesting)]
fun reject_past_start_or_invalid_schedule() {
    invalid_allocation(DISTRIBUTION, BENEFICIARY, TREASURY, LIQUIDITY,
        USER_UNITS, TEAM_UNITS, TREASURY_UNITS, LIQUIDITY_UNITS,
        COMMUNITY_UNITS, SECURITY_UNITS, NOW_MS - 1, NOW_MS + 200, NOW_MS + 1_100,
        manifest_digest());
}

#[test, expected_failure(abort_code = 3, location = yilong_participation::team_vesting)]
fun reject_non_increasing_schedule() {
    invalid_allocation(DISTRIBUTION, BENEFICIARY, TREASURY, LIQUIDITY,
        USER_UNITS, TEAM_UNITS, TREASURY_UNITS, LIQUIDITY_UNITS,
        COMMUNITY_UNITS, SECURITY_UNITS, NOW_MS + 100, NOW_MS + 100, NOW_MS + 1_100,
        manifest_digest());
}
