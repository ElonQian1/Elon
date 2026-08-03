use std::collections::{BTreeMap, BTreeSet};

use super::{
    ComputeCapacityAccount, ComputeCapacityBucketBalance, ComputeCapacityBucketBinding,
    ComputeCapacityBucketStatus, ComputeCapacityCausalBinding, ComputeCapacityClaim,
    ComputeCapacityContractError, ComputeCapacityEventKind, ComputeCapacityLedgerLeg,
    ComputeCapacityLedgerTransaction, ComputeCapacityLegRole, ComputeCapacityMeterMode,
    ComputeCapacityMovementLine, ComputeCapacityPoolBinding,
};
use crate::compute_federation::market::ComputeDeliveryWindowBinding;

pub(crate) fn validate_capacity_claim(
    claim: &ComputeCapacityClaim,
) -> Result<(), ComputeCapacityContractError> {
    if claim.lines.is_empty() {
        return Err(ComputeCapacityContractError::EmptyClaim);
    }
    if claim.revision <= 0 {
        return Err(ComputeCapacityContractError::InvalidClaimRevision(
            claim.revision,
        ));
    }
    if claim.parent_claim_id.as_deref() == Some(claim.claim_id.as_str()) {
        return Err(ComputeCapacityContractError::SelfParentClaim(
            claim.claim_id.clone(),
        ));
    }

    validate_bound_lines(
        &claim.pool,
        &claim.delivery_window,
        claim
            .lines
            .iter()
            .map(|line| (line.line_no, &line.bucket, line.quantity_units)),
    )
}

pub(crate) fn validate_capacity_transaction(
    transaction: &ComputeCapacityLedgerTransaction,
) -> Result<(), ComputeCapacityContractError> {
    if transaction.movements.is_empty() {
        return Err(ComputeCapacityContractError::EmptyTransaction);
    }
    if transaction.ledger_sequence <= 0 {
        return Err(ComputeCapacityContractError::InvalidLedgerSequence(
            transaction.ledger_sequence,
        ));
    }
    validate_causal_binding(&transaction.causal_binding)?;
    validate_bound_lines(
        &transaction.pool,
        &transaction.delivery_window,
        transaction
            .movements
            .iter()
            .map(|line| (line.line_no, &line.bucket, line.quantity_units)),
    )?;

    for movement in &transaction.movements {
        if !is_allowed_transition(
            transaction.event_kind,
            movement.bucket.meter_mode,
            movement.from_account,
            movement.to_account,
        ) {
            return Err(ComputeCapacityContractError::InvalidAccountTransition {
                event_kind: transaction.event_kind,
                from: movement.from_account,
                to: movement.to_account,
            });
        }
    }

    // Expansion proves every bucket movement is exactly double-legged and zero-sum.
    let legs = expand_capacity_ledger_legs_unchecked(transaction);
    let mut bucket_totals = BTreeMap::<String, i128>::new();
    for leg in legs {
        let total = bucket_totals.entry(leg.bucket.bucket_id).or_default();
        *total = total
            .checked_add(i128::from(leg.delta_units))
            .ok_or(ComputeCapacityContractError::ArithmeticOverflow)?;
    }
    if bucket_totals.values().any(|total| *total != 0) {
        return Err(ComputeCapacityContractError::ArithmeticOverflow);
    }
    Ok(())
}

pub(crate) fn expand_capacity_ledger_legs(
    transaction: &ComputeCapacityLedgerTransaction,
) -> Result<Vec<ComputeCapacityLedgerLeg>, ComputeCapacityContractError> {
    validate_capacity_transaction(transaction)?;
    Ok(expand_capacity_ledger_legs_unchecked(transaction))
}

/// Applies every movement to a cloned projection and publishes it only after all buckets pass.
pub(crate) fn apply_capacity_transaction(
    balances: &mut BTreeMap<String, ComputeCapacityBucketBalance>,
    transaction: &ComputeCapacityLedgerTransaction,
) -> Result<(), ComputeCapacityContractError> {
    validate_capacity_transaction(transaction)?;
    let mut next = balances.clone();

    for movement in &transaction.movements {
        let balance = next.get_mut(&movement.bucket.bucket_id).ok_or_else(|| {
            ComputeCapacityContractError::MissingBucket(movement.bucket.bucket_id.clone())
        })?;
        if balance.binding != movement.bucket {
            return Err(ComputeCapacityContractError::BucketBindingMismatch(
                movement.bucket.bucket_id.clone(),
            ));
        }
        if balance.status != ComputeCapacityBucketStatus::Open
            && !matches!(
                transaction.event_kind,
                ComputeCapacityEventKind::ReservationReleased
                    | ComputeCapacityEventKind::ReservationExpired
                    | ComputeCapacityEventKind::AttemptReturned
            )
        {
            return Err(ComputeCapacityContractError::ClosedBucket(
                movement.bucket.bucket_id.clone(),
            ));
        }
        if let Some(previous) = balance.through_ledger_sequence {
            if transaction.ledger_sequence <= previous {
                return Err(ComputeCapacityContractError::NonMonotonicLedgerSequence {
                    bucket_id: movement.bucket.bucket_id.clone(),
                    previous,
                    current: transaction.ledger_sequence,
                });
            }
        }

        let mut work = CapacityBalanceWork::from(&*balance);
        work.apply(movement)?;
        work.validate(&movement.bucket)?;
        work.write_to(balance)?;
        balance.balance_revision = balance
            .balance_revision
            .checked_add(1)
            .ok_or(ComputeCapacityContractError::ArithmeticOverflow)?;
        balance.through_ledger_sequence = Some(transaction.ledger_sequence);
    }

    *balances = next;
    Ok(())
}

fn validate_bound_lines<'a>(
    pool: &ComputeCapacityPoolBinding,
    delivery_window: &ComputeDeliveryWindowBinding,
    lines: impl Iterator<Item = (i64, &'a ComputeCapacityBucketBinding, i64)>,
) -> Result<(), ComputeCapacityContractError> {
    let mut line_numbers = BTreeSet::new();
    let mut bucket_ids = BTreeSet::new();
    let mut meters = BTreeSet::new();

    for (line_no, bucket, quantity_units) in lines {
        if line_no < 0 {
            return Err(ComputeCapacityContractError::InvalidLineNumber(line_no));
        }
        if !line_numbers.insert(line_no) {
            return Err(ComputeCapacityContractError::DuplicateLineNumber(line_no));
        }
        if !bucket_ids.insert(bucket.bucket_id.as_str()) {
            return Err(ComputeCapacityContractError::DuplicateBucket(
                bucket.bucket_id.clone(),
            ));
        }
        if !meters.insert(bucket.meter.as_str()) {
            return Err(ComputeCapacityContractError::DuplicateMeter(
                bucket.meter.clone(),
            ));
        }
        if &bucket.pool != pool {
            return Err(ComputeCapacityContractError::PoolBindingMismatch(
                bucket.bucket_id.clone(),
            ));
        }
        if &bucket.delivery_window != delivery_window {
            return Err(ComputeCapacityContractError::DeliveryWindowMismatch(
                bucket.bucket_id.clone(),
            ));
        }
        if quantity_units <= 0 {
            return Err(ComputeCapacityContractError::NonPositiveQuantity(
                quantity_units,
            ));
        }
        if bucket.quantum_units <= 0 || quantity_units % bucket.quantum_units != 0 {
            return Err(ComputeCapacityContractError::InvalidQuantum {
                meter: bucket.meter.clone(),
                quantum_units: bucket.quantum_units,
                quantity_units,
            });
        }
    }
    Ok(())
}

fn validate_causal_binding(
    binding: &ComputeCapacityCausalBinding,
) -> Result<(), ComputeCapacityContractError> {
    if binding
        .offer
        .as_ref()
        .is_some_and(|offer| offer.offer_version <= 0)
    {
        return Err(ComputeCapacityContractError::InvalidCausalBinding);
    }
    if binding.reservation_id.is_some() && binding.job_id.is_none() {
        return Err(ComputeCapacityContractError::InvalidCausalBinding);
    }
    match (&binding.attempt_lease_id, binding.fencing_generation) {
        (None, None) => Ok(()),
        (Some(_), Some(generation))
            if generation > 0 && binding.reservation_id.is_some() && binding.job_id.is_some() =>
        {
            Ok(())
        }
        _ => Err(ComputeCapacityContractError::InvalidCausalBinding),
    }
}

fn is_allowed_transition(
    event_kind: ComputeCapacityEventKind,
    meter_mode: ComputeCapacityMeterMode,
    from: ComputeCapacityAccount,
    to: ComputeCapacityAccount,
) -> bool {
    use ComputeCapacityAccount::{Active, Available, Consumed, Held, Issuance, Retired};
    use ComputeCapacityEventKind::{
        AttemptActivated, AttemptReturned, ReservationExpired, ReservationHeld,
        ReservationReleased, SupplyAdded, SupplyWithdrawn, UsageConsumed,
    };

    match event_kind {
        SupplyAdded => (from, to) == (Issuance, Available),
        SupplyWithdrawn => (from, to) == (Available, Retired),
        ReservationHeld => (from, to) == (Available, Held),
        AttemptActivated => (from, to) == (Held, Active),
        AttemptReturned => (from, to) == (Active, Available),
        UsageConsumed => {
            meter_mode == ComputeCapacityMeterMode::Consumable && (from, to) == (Active, Consumed)
        }
        ReservationReleased | ReservationExpired => {
            matches!(from, Held | Active) && to == Available
        }
    }
}

fn expand_capacity_ledger_legs_unchecked(
    transaction: &ComputeCapacityLedgerTransaction,
) -> Vec<ComputeCapacityLedgerLeg> {
    let mut legs = Vec::with_capacity(transaction.movements.len() * 2);
    for movement in &transaction.movements {
        legs.push(ComputeCapacityLedgerLeg {
            line_no: movement.line_no,
            leg_role: ComputeCapacityLegRole::From,
            bucket: movement.bucket.clone(),
            account: movement.from_account,
            delta_units: -movement.quantity_units,
        });
        legs.push(ComputeCapacityLedgerLeg {
            line_no: movement.line_no,
            leg_role: ComputeCapacityLegRole::To,
            bucket: movement.bucket.clone(),
            account: movement.to_account,
            delta_units: movement.quantity_units,
        });
    }
    legs
}

struct CapacityBalanceWork {
    issued: i128,
    available: i128,
    held: i128,
    active: i128,
    consumed: i128,
    retired: i128,
}

impl From<&ComputeCapacityBucketBalance> for CapacityBalanceWork {
    fn from(balance: &ComputeCapacityBucketBalance) -> Self {
        Self {
            issued: i128::from(balance.issued_units),
            available: i128::from(balance.available_units),
            held: i128::from(balance.held_units),
            active: i128::from(balance.active_units),
            consumed: i128::from(balance.consumed_units),
            retired: i128::from(balance.retired_units),
        }
    }
}

impl CapacityBalanceWork {
    fn apply(
        &mut self,
        movement: &ComputeCapacityMovementLine,
    ) -> Result<(), ComputeCapacityContractError> {
        let quantity = i128::from(movement.quantity_units);
        if movement.from_account == ComputeCapacityAccount::Issuance {
            self.issued = self
                .issued
                .checked_add(quantity)
                .ok_or(ComputeCapacityContractError::ArithmeticOverflow)?;
        } else {
            let from = self.account_mut(movement.from_account);
            *from = from
                .checked_sub(quantity)
                .ok_or(ComputeCapacityContractError::ArithmeticOverflow)?;
        }
        let to = self.account_mut(movement.to_account);
        *to = to
            .checked_add(quantity)
            .ok_or(ComputeCapacityContractError::ArithmeticOverflow)?;
        Ok(())
    }

    fn account_mut(&mut self, account: ComputeCapacityAccount) -> &mut i128 {
        match account {
            ComputeCapacityAccount::Issuance => &mut self.issued,
            ComputeCapacityAccount::Available => &mut self.available,
            ComputeCapacityAccount::Held => &mut self.held,
            ComputeCapacityAccount::Active => &mut self.active,
            ComputeCapacityAccount::Consumed => &mut self.consumed,
            ComputeCapacityAccount::Retired => &mut self.retired,
        }
    }

    fn validate(
        &self,
        bucket: &ComputeCapacityBucketBinding,
    ) -> Result<(), ComputeCapacityContractError> {
        for (account, balance_units) in [
            (ComputeCapacityAccount::Available, self.available),
            (ComputeCapacityAccount::Held, self.held),
            (ComputeCapacityAccount::Active, self.active),
            (ComputeCapacityAccount::Consumed, self.consumed),
            (ComputeCapacityAccount::Retired, self.retired),
        ] {
            if balance_units < 0 {
                return Err(ComputeCapacityContractError::NegativeBalance {
                    bucket_id: bucket.bucket_id.clone(),
                    account,
                    balance_units,
                });
            }
        }
        if self.issued < 0 {
            return Err(ComputeCapacityContractError::NegativeBalance {
                bucket_id: bucket.bucket_id.clone(),
                account: ComputeCapacityAccount::Issuance,
                balance_units: self.issued,
            });
        }

        let projected = self
            .available
            .checked_add(self.held)
            .and_then(|value| value.checked_add(self.active))
            .and_then(|value| value.checked_add(self.retired))
            .ok_or(ComputeCapacityContractError::ArithmeticOverflow)?;
        let projected = match bucket.meter_mode {
            ComputeCapacityMeterMode::Consumable => projected
                .checked_add(self.consumed)
                .ok_or(ComputeCapacityContractError::ArithmeticOverflow)?,
            ComputeCapacityMeterMode::Reusable if self.consumed == 0 => projected,
            ComputeCapacityMeterMode::Reusable => {
                return Err(ComputeCapacityContractError::ReusableConsumedBalance {
                    bucket_id: bucket.bucket_id.clone(),
                    consumed_units: self.consumed,
                });
            }
        };
        if self.issued != projected {
            return Err(ComputeCapacityContractError::ConservationViolation {
                bucket_id: bucket.bucket_id.clone(),
                issued_units: self.issued,
                projected_units: projected,
            });
        }
        Ok(())
    }

    fn write_to(
        &self,
        balance: &mut ComputeCapacityBucketBalance,
    ) -> Result<(), ComputeCapacityContractError> {
        balance.issued_units = to_i64(self.issued)?;
        balance.available_units = to_i64(self.available)?;
        balance.held_units = to_i64(self.held)?;
        balance.active_units = to_i64(self.active)?;
        balance.consumed_units = to_i64(self.consumed)?;
        balance.retired_units = to_i64(self.retired)?;
        Ok(())
    }
}

fn to_i64(value: i128) -> Result<i64, ComputeCapacityContractError> {
    i64::try_from(value).map_err(|_| ComputeCapacityContractError::ArithmeticOverflow)
}
