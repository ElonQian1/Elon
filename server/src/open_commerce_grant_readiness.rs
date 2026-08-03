//! Shared read-only Grant budget readiness used before a consumer invocation starts.

use crate::open_commerce_model::OpenCommerceGrant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpenCommerceGrantReadiness {
    Available,
    InvocationBudgetExhausted,
    AmountBudgetExhausted,
    BudgetCurrencyMismatch,
}

impl OpenCommerceGrantReadiness {
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::InvocationBudgetExhausted => "invocation_budget_exhausted",
            Self::AmountBudgetExhausted => "amount_budget_exhausted",
            Self::BudgetCurrencyMismatch => "budget_currency_mismatch",
        }
    }

    pub(crate) fn is_available(self) -> bool {
        self == Self::Available
    }
}

pub(crate) fn evaluate(
    grant: &OpenCommerceGrant,
    next_unit_price_micros: i64,
    capability_currency: &str,
) -> OpenCommerceGrantReadiness {
    if grant.max_invocations.is_some_and(|maximum| {
        grant
            .used_invocations
            .checked_add(1)
            .is_none_or(|next| next > maximum)
    }) {
        return OpenCommerceGrantReadiness::InvocationBudgetExhausted;
    }
    if grant.max_amount_micros.is_some()
        && grant.budget_currency.trim().to_ascii_uppercase()
            != capability_currency.trim().to_ascii_uppercase()
    {
        return OpenCommerceGrantReadiness::BudgetCurrencyMismatch;
    }
    if grant.max_amount_micros.is_some_and(|maximum| {
        grant
            .used_amount_micros
            .checked_add(next_unit_price_micros)
            .is_none_or(|next| next > maximum)
    }) {
        return OpenCommerceGrantReadiness::AmountBudgetExhausted;
    }
    OpenCommerceGrantReadiness::Available
}

pub(crate) fn select_best<'a>(
    grants: &'a [OpenCommerceGrant],
    next_unit_price_micros: i64,
    capability_currency: &str,
) -> Option<(&'a OpenCommerceGrant, OpenCommerceGrantReadiness)> {
    let mut first = None;
    for grant in grants {
        let readiness = evaluate(grant, next_unit_price_micros, capability_currency);
        first.get_or_insert((grant, readiness));
        if readiness.is_available() {
            return Some((grant, readiness));
        }
    }
    first
}
