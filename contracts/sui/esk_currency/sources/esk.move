// Copyright (c) 2026 Yilong Project.
// SPDX-License-Identifier: Apache-2.0

/// Fixed-supply Yilong ESK currency core.
///
/// This module deliberately contains no trading, NAV, redemption, service-order,
/// governance, or profit-distribution behavior. Those facts belong to versioned
/// platform ledgers or a separate participation package.
module esk_currency::esk;

use sui::coin_registry;

const DECIMALS: u8 = 6;
const TOTAL_SUPPLY_BASE_UNITS: u64 = 1_000_000_000_000_000;

/// One-time witness and canonical type for ESK.
public struct ESK has drop {}

/// Creates exactly one billion ESK at six decimals and permanently gives up the
/// TreasuryCap. The publishing account is an ephemeral handoff account: the
/// deployment ceremony must transfer the initial Coin, MetadataCap, and package
/// UpgradeCap to the distinct roles declared by the signed genesis manifest.
fun init(witness: ESK, ctx: &mut TxContext) {
    let (mut currency, mut treasury_cap) = coin_registry::new_currency_with_otw(
        witness,
        DECIMALS,
        b"ESK".to_string(),
        b"Yilong ESK".to_string(),
        b"Yilong ecosystem token; market priced, no fixed yield or legal equity".to_string(),
        b"".to_string(),
        ctx,
    );

    let total_supply = treasury_cap.mint(TOTAL_SUPPLY_BASE_UNITS, ctx);
    currency.make_supply_fixed(treasury_cap);
    let metadata_cap = currency.finalize(ctx);

    transfer::public_transfer(metadata_cap, ctx.sender());
    transfer::public_transfer(total_supply, ctx.sender());
}

/// Stable constant exposed for manifests, indexers, and cross-repository checks.
public fun decimals(): u8 { DECIMALS }

/// Stable constant exposed without relying on floating-point display units.
public fun total_supply_base_units(): u64 { TOTAL_SUPPLY_BASE_UNITS }

/// Runs the real initializer in a Sui transaction scenario. This function is
/// removed from production bytecode and exists only to verify capability flow.
#[test_only]
public fun init_for_testing(ctx: &mut TxContext) { init(ESK {}, ctx) }
