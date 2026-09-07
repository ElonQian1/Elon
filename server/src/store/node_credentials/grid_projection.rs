//! Token and every private row share one transaction; public /quant is never involved.
use crate::{private_read_projection as contract, store::Store};
use anyhow::Result;
use serde_json::{json, Value};
impl Store {
    pub(crate) fn asset_access_grids(&self, token: &str, client: &str) -> Result<Value> {
        self.asset_access_private_read(token,client,contract::SCOPE,|tx,access| {
            let snapshots=super::private_projection::read_on(tx,access.user_id(),contract::now_ms())?;
            let value=json!({"schema":"yilong.private_read_projections.v1","audience":"yilong-quant",
              "subject":access.subject(),"client_id":access.client_id(),"grant_id":access.grant_id(),
              "expires_at":access.expires_at(),"snapshots":snapshots});
            if serde_json::to_vec(&value)?.len()>contract::MAX_BYTES {
                return Err(crate::esk_asset::platform::access::AccessError::Capacity.into());
            }
            Ok(value)
        })
    }
}
