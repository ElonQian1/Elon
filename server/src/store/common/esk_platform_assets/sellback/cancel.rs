use anyhow::Result;
use rusqlite::{params, TransactionBehavior};

use crate::{esk_asset::platform::sellback::*, store::Store};

use super::{
    authenticate, new_id, now,
    snapshot::{scan_on, Selection},
};

impl Store {
    pub(crate) fn cancel_esk_platform_sellback(
        &self,
        user: &str,
        token: &str,
        request_id: &str,
        config: &SellbackConfiguration,
    ) -> Result<SellbackResult> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        authenticate(&tx, user, token)?;
        if !valid_request_id(request_id) {
            return Err(SellbackError::InvalidInput.into());
        }
        let before = scan_on(&tx, user, token, config, Selection::Id(request_id))?;
        let record = before.selected.as_ref().ok_or(SellbackError::NotFound)?;
        if record.canceled_at.is_some() {
            authenticate(&tx, user, token)?;
            let result = before.result(true)?;
            tx.commit()?;
            return Ok(result);
        }
        let clock = now();
        // Never rewrite the audit time to conceal a wall-clock regression.
        if !timestamp_not_before(&clock, &record.created_at) {
            return Err(SellbackError::Corrupt.into());
        }
        tx.execute(
            "INSERT INTO esk_platform_sellback_cancellations(cancel_event_id,request_id,request_digest,canceled_by,created_at)
             VALUES(?1,?2,?3,?4,?5)",
            params![new_id("eskpsc"), request_id, record.request_digest, user, clock],
        )?;
        let result = scan_on(&tx, user, token, config, Selection::Id(request_id))?.result(false)?;
        authenticate(&tx, user, token)?;
        tx.commit()?;
        Ok(result)
    }
}
