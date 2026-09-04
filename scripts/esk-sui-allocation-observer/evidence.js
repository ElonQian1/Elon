function coinBucket(item, amount, owner, changeKind) {
  return { object_id: item.id, base_units: amount, owner, change_kind: changeKind,
    version: item.output.version, digest: item.output.digest,
    bcs_sha256: item.output.coin.bcs_sha256 }
}

function formatEvidence({ expected, publication, allocation, checkpoint, receipt,
  receiptCreation, cap, supply, direct, vesting }) {
  return {
    chain_identifier: expected.chain_identifier,
    currency_package_id: expected.currency_package_id,
    participation_package: {
      package_id: publication.package_id, version: publication.package_version,
      digest: publication.package_digest,
      publication_digest: publication.publication_digest,
      publication_effects_digest: publication.effects_digest,
      publication_lamport_version: publication.lamport_version,
      checkpoint_sequence: publication.checkpoint_sequence,
      checkpoint_digest: publication.checkpoint_digest,
    },
    allocation: {
      digest: expected.allocation_digest, effects_digest: allocation.effects_digest,
      lamport_version: allocation.lamport_version,
      sender: allocation.sender, timestamp: allocation.timestamp.value,
      checkpoint_sequence: expected.allocation_checkpoint_sequence,
      checkpoint_digest: expected.allocation_checkpoint_digest,
    },
    observation_checkpoint: {
      sequence: checkpoint.sequence, digest: checkpoint.digest,
      timestamp: checkpoint.timestamp.value,
    },
    manifest_digest: expected.manifest_digest,
    holders: expected.holders,
    cap: {
      object_id: expected.allocation_cap_object_id,
      publication_owner: cap.publishedOwner,
      publication_version: cap.published.version,
      publication_digest: cap.published.digest,
      consumed_version: cap.consumed.version,
      consumed_digest: cap.consumed.digest,
      bcs_sha256: cap.consumedCap.bcs_sha256,
    },
    receipt: {
      object_id: expected.allocation_receipt_object_id,
      version: receiptCreation.version, digest: receiptCreation.digest,
      bcs_sha256: receipt.bcs_sha256, executed_at_ms: receipt.executed_at_ms,
    },
    supply_input: {
      object_id: expected.initial_supply_coin_object_id,
      base_units: expected.expected_supply_base_units,
      version: supply.input.version, digest: supply.input.digest,
      bcs_sha256: supply.input.coin.bcs_sha256,
    },
    buckets: {
      user_migration_and_ecosystem: coinBucket(direct.user_migration_and_ecosystem,
        expected.buckets.user_migration_and_ecosystem, expected.holders.distribution, 'created'),
      team_vesting: {
        object_id: expected.team_vesting_object_id,
        base_units: expected.buckets.team_vesting,
        owner: expected.holders.team_beneficiary, change_kind: 'created',
        version: vesting.creation.version, digest: vesting.creation.digest,
        bcs_sha256: vesting.initial.bcs_sha256,
      },
      project_treasury: coinBucket(direct.project_treasury,
        expected.buckets.project_treasury, expected.holders.treasury, 'created'),
      liquidity: coinBucket(direct.liquidity, expected.buckets.liquidity,
        expected.holders.liquidity_recipient, 'created'),
      community_contributors: coinBucket(direct.community_contributors,
        expected.buckets.community_contributors, expected.holders.distribution, 'created'),
      security_operations_reserve: {
        object_id: expected.initial_supply_coin_object_id,
        base_units: expected.buckets.security_operations_reserve,
        owner: expected.holders.treasury, change_kind: 'mutated',
        version: supply.output.version, digest: supply.output.digest,
        bcs_sha256: supply.output.coin.bcs_sha256,
      },
    },
    team_vesting_snapshot: {
      object_id: expected.team_vesting_object_id,
      version: vesting.current.version, digest: vesting.current.digest,
      previous_transaction: vesting.current.previous_transaction,
      total_base_units: vesting.snapshot.total_base_units,
      claimed_base_units: vesting.snapshot.claimed_base_units,
      remaining_base_units: vesting.snapshot.remaining_base_units,
      bcs_sha256: vesting.snapshot.bcs_sha256,
    },
  }
}

module.exports = { formatEvidence }
