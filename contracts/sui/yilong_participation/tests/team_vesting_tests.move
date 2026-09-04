// Copyright (c) 2026 Yilong Project.
// SPDX-License-Identifier: Apache-2.0

#[test_only]
module yilong_participation::team_vesting_tests;

use esk_currency::esk::ESK;
use sui::clock::{Self, Clock};
use sui::coin::{Self, Coin};
use sui::test_scenario::{Self, Scenario};
use yilong_participation::genesis_allocation;
use yilong_participation::team_vesting::{Self, TeamVesting};

const PUBLISHER: address = @0xA11CE;
const DISTRIBUTION: address = @0xD157;
const BENEFICIARY: address = @0x7EA0;
const TREASURY: address = @0x7EA5;
const LIQUIDITY: address = @0x1110;
const ATTACKER: address = @0xBAD;

const START_MS: u64 = 1_000;
const CLIFF_MS: u64 = 1_200;
const END_MS: u64 = 2_001;
const TEAM_UNITS: u64 = 101;
const OTHER_BUCKET_UNITS: u64 = 1;
const TOTAL_UNITS: u64 = 1_000_000_000_000_000;
const USER_UNITS: u64 = TOTAL_UNITS - TEAM_UNITS - 4;

fun digest(): vector<u8> {
    let mut result = vector[];
    let mut index = 0u64;
    while (index < 32) {
        result.push_back(0x5A);
        index = index + 1;
    };
    result
}

fun create_schedule(scenario: &mut Scenario): Clock {
    let mut clock = clock::create_for_testing(scenario.ctx());
    clock::set_for_testing(&mut clock, START_MS - 1);
    genesis_allocation::init_for_testing(scenario.ctx());
    test_scenario::next_tx(scenario, PUBLISHER);
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
        OTHER_BUCKET_UNITS,
        OTHER_BUCKET_UNITS,
        OTHER_BUCKET_UNITS,
        OTHER_BUCKET_UNITS,
        START_MS,
        CLIFF_MS,
        END_MS,
        digest(),
        &clock,
        scenario.ctx(),
    );
    test_scenario::next_tx(scenario, BENEFICIARY);
    clock
}

fun take_schedule(scenario: &Scenario): TeamVesting {
    test_scenario::take_from_sender(scenario)
}

fun assert_conserved(vesting: &TeamVesting) {
    assert!(
        team_vesting::claimed_base_units(vesting)
            + team_vesting::remaining_base_units(vesting)
            == team_vesting::total_base_units(vesting),
    );
    assert!(team_vesting::total_base_units(vesting) == TEAM_UNITS);
    assert!(team_vesting::beneficiary(vesting) == BENEFICIARY);
    assert!(team_vesting::start_ms(vesting) == START_MS);
    assert!(team_vesting::cliff_ms(vesting) == CLIFF_MS);
    assert!(team_vesting::end_ms(vesting) == END_MS);
}

#[test]
fun cliff_midpoint_and_end_release_every_unit_without_dust() {
    let mut scenario = test_scenario::begin(PUBLISHER);
    let mut clock = create_schedule(&mut scenario);
    let mut vesting = take_schedule(&scenario);
    assert_conserved(&vesting);
    assert!(team_vesting::vested_base_units(&vesting, &clock) == 0);
    assert!(team_vesting::claimable(&vesting, &clock) == 0);

    clock::set_for_testing(&mut clock, CLIFF_MS);
    let first_claimable = team_vesting::claimable(&vesting, &clock);
    assert!(first_claimable == 20);
    assert!(team_vesting::vested_base_units(&vesting, &clock) == 20);
    team_vesting::claim(&mut vesting, &clock, scenario.ctx());
    assert!(team_vesting::claimed_base_units(&vesting) == first_claimable);
    assert_conserved(&vesting);
    test_scenario::return_to_sender(&scenario, vesting);

    test_scenario::next_tx(&mut scenario, BENEFICIARY);
    let first_payment = test_scenario::take_from_sender<Coin<ESK>>(&scenario);
    assert!(first_payment.value() == first_claimable);
    let mut vesting = take_schedule(&scenario);
    test_scenario::return_to_sender(&scenario, first_payment);

    clock::set_for_testing(&mut clock, 1_500);
    let midpoint_claimable = team_vesting::claimable(&vesting, &clock);
    assert!(midpoint_claimable == 30);
    team_vesting::claim(&mut vesting, &clock, scenario.ctx());
    assert!(team_vesting::claimed_base_units(&vesting) == 50);
    assert_conserved(&vesting);
    test_scenario::return_to_sender(&scenario, vesting);

    test_scenario::next_tx(&mut scenario, BENEFICIARY);
    let midpoint_payment = test_scenario::take_from_sender<Coin<ESK>>(&scenario);
    assert!(midpoint_payment.value() == midpoint_claimable);
    let mut vesting = take_schedule(&scenario);
    test_scenario::return_to_sender(&scenario, midpoint_payment);

    clock::set_for_testing(&mut clock, END_MS);
    let final_claimable = team_vesting::claimable(&vesting, &clock);
    assert!(final_claimable == 51);
    assert!(team_vesting::vested_base_units(&vesting, &clock) == TEAM_UNITS);
    team_vesting::claim(&mut vesting, &clock, scenario.ctx());
    assert!(team_vesting::claimed_base_units(&vesting) == TEAM_UNITS);
    assert!(team_vesting::remaining_base_units(&vesting) == 0);
    assert_conserved(&vesting);
    test_scenario::return_to_sender(&scenario, vesting);

    test_scenario::next_tx(&mut scenario, BENEFICIARY);
    let final_payment = test_scenario::take_from_sender<Coin<ESK>>(&scenario);
    assert!(final_payment.value() == final_claimable);
    test_scenario::return_to_sender(&scenario, final_payment);
    let vesting = take_schedule(&scenario);
    assert_conserved(&vesting);
    test_scenario::return_to_sender(&scenario, vesting);
    clock::destroy_for_testing(clock);
    test_scenario::end(scenario);
}

#[test, expected_failure(abort_code = 5, location = yilong_participation::team_vesting)]
fun claim_before_cliff_fails_instead_of_minting_zero_coin() {
    let mut scenario = test_scenario::begin(PUBLISHER);
    let clock = create_schedule(&mut scenario);
    let mut vesting = take_schedule(&scenario);
    assert!(team_vesting::claimable(&vesting, &clock) == 0);
    team_vesting::claim(&mut vesting, &clock, scenario.ctx());
    test_scenario::return_to_sender(&scenario, vesting);
    clock::destroy_for_testing(clock);
    test_scenario::end(scenario);
}

#[test, expected_failure(abort_code = 4, location = yilong_participation::team_vesting)]
fun non_beneficiary_cannot_claim_even_when_amount_is_available() {
    let mut scenario = test_scenario::begin(PUBLISHER);
    let mut clock = create_schedule(&mut scenario);
    clock::set_for_testing(&mut clock, END_MS);
    test_scenario::next_tx(&mut scenario, ATTACKER);
    let mut vesting = test_scenario::take_from_address<TeamVesting>(&scenario, BENEFICIARY);
    assert!(team_vesting::claimable(&vesting, &clock) == TEAM_UNITS);
    team_vesting::claim(&mut vesting, &clock, scenario.ctx());
    test_scenario::return_to_sender(&scenario, vesting);
    clock::destroy_for_testing(clock);
    test_scenario::end(scenario);
}

#[test, expected_failure(abort_code = 5, location = yilong_participation::team_vesting)]
fun same_millisecond_second_claim_rejects_zero_amount() {
    let mut scenario = test_scenario::begin(PUBLISHER);
    let mut clock = create_schedule(&mut scenario);
    clock::set_for_testing(&mut clock, 1_500);
    let mut vesting = take_schedule(&scenario);
    team_vesting::claim(&mut vesting, &clock, scenario.ctx());
    assert!(team_vesting::claimable(&vesting, &clock) == 0);
    team_vesting::claim(&mut vesting, &clock, scenario.ctx());
    test_scenario::return_to_sender(&scenario, vesting);
    clock::destroy_for_testing(clock);
    test_scenario::end(scenario);
}
