use anyhow::{bail, Result};
use rusqlite::{params, TransactionBehavior};

use super::{
    read::{request_by_id_on, request_receipt},
    review::{now_nanos, validate_digest, validate_exact},
    types::{CancelExternalPoolOnboardingRequest, ExternalPoolOnboardingRequestReceipt},
};
use crate::store::Store;

impl Store {
    pub(crate) fn cancel_external_pool_onboarding_request(
        &self,
        input: CancelExternalPoolOnboardingRequest,
    ) -> Result<ExternalPoolOnboardingRequestReceipt> {
        validate_exact(&input.owner_user_id, "onboarding owner", 160)?;
        validate_exact(&input.request_id, "onboarding request ID", 160)?;
        validate_digest(&input.expected_request_digest, "onboarding request digest")?;

        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = request_by_id_on(&transaction, &input.request_id)?
            .ok_or_else(|| anyhow::anyhow!("external-pool onboarding request does not exist"))?;
        if current.envelope.request.requested_by_owner_user_id != input.owner_user_id
            || current.envelope.request_digest != input.expected_request_digest
        {
            bail!("external-pool onboarding request is not owned or changed");
        }
        if current.status == "canceled" {
            let receipt = request_receipt(current, true);
            transaction.commit()?;
            return Ok(receipt);
        }
        if current.status != "submitted" {
            bail!("only a submitted external-pool onboarding request can be canceled");
        }

        let canceled_at = now_nanos();
        let changed = transaction.execute(
            "UPDATE compute_external_pool_onboarding_requests
                SET status='canceled', canceled_by_owner_user_id=?1,
                    canceled_at=?2, updated_at=?2
              WHERE request_id=?3 AND request_digest=?4
                AND provider_owner_account_id=?1 AND status='submitted'",
            params![
                input.owner_user_id,
                canceled_at,
                input.request_id,
                input.expected_request_digest,
            ],
        )?;
        if changed != 1 {
            bail!("external-pool onboarding request changed concurrently");
        }
        let stored = request_by_id_on(&transaction, &input.request_id)?
            .ok_or_else(|| anyhow::anyhow!("canceled onboarding request cannot be read"))?;
        let receipt = request_receipt(stored, false);
        transaction.commit()?;
        Ok(receipt)
    }
}
