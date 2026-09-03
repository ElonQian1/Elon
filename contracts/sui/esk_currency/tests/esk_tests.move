// Copyright (c) 2026 Yilong Project.
// SPDX-License-Identifier: Apache-2.0

#[test_only]
module esk_currency::esk_tests;

use esk_currency::esk::{Self, ESK};
use sui::coin::{Coin, TreasuryCap};
use sui::coin_registry::MetadataCap;
use sui::test_scenario;

const PUBLISHER: address = @0xA11CE;

/// The manifest and Move package must use exactly the same precision.
#[test]
fun decimals_match_genesis_manifest() {
    assert!(esk::decimals() == 6);
}

/// One billion display units at six decimals must stay inside u64 and match the
/// machine-readable manifest exactly.
#[test]
fun supply_matches_genesis_manifest() {
    assert!(esk::total_supply_base_units() == 1_000_000_000_000_000);
}

/// Exercise the actual module initializer. The publisher receives the fixed
/// supply and metadata capability, while no TreasuryCap survives initialization.
#[test]
fun initializer_mints_once_and_consumes_treasury_cap() {
    let mut scenario = test_scenario::begin(PUBLISHER);
    esk::init_for_testing(scenario.ctx());
    test_scenario::next_tx(&mut scenario, PUBLISHER);

    assert!(test_scenario::has_most_recent_for_sender<Coin<ESK>>(&scenario));
    assert!(test_scenario::has_most_recent_for_sender<MetadataCap<ESK>>(&scenario));
    assert!(!test_scenario::has_most_recent_for_sender<TreasuryCap<ESK>>(&scenario));

    let supply = test_scenario::take_from_sender<Coin<ESK>>(&scenario);
    assert!(supply.value() == esk::total_supply_base_units());
    let metadata_cap = test_scenario::take_from_sender<MetadataCap<ESK>>(&scenario);

    test_scenario::return_to_sender(&scenario, supply);
    test_scenario::return_to_sender(&scenario, metadata_cap);
    test_scenario::end(scenario);
}
