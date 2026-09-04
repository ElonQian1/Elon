use anyhow::Result;

use crate::{esk_asset::platform::sellback::*, store::Store};

use super::{
    authenticate,
    snapshot::{scan_on, Selection},
};

impl Store {
    pub(crate) fn esk_platform_sellback_page(
        &self,
        user: &str,
        token: &str,
        limit: usize,
        cursor: Option<&str>,
        config: &SellbackConfiguration,
    ) -> Result<SellbackPage> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        authenticate(&tx, user, token)?;
        if !(1..=MAX_PAGE_SIZE).contains(&limit) {
            return Err(SellbackError::InvalidInput.into());
        }
        let cursor = cursor.map(parse_cursor).transpose()?;
        let snapshot = scan_on(
            &tx,
            user,
            token,
            config,
            Selection::Page(limit, cursor.as_ref()),
        )?;
        tx.commit()?;
        Ok(snapshot.page)
    }

    pub(crate) fn esk_platform_sellback_request(
        &self,
        user: &str,
        token: &str,
        request_id: &str,
        config: &SellbackConfiguration,
    ) -> Result<SellbackResult> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        authenticate(&tx, user, token)?;
        if !valid_request_id(request_id) {
            return Err(SellbackError::InvalidInput.into());
        }
        let result = scan_on(&tx, user, token, config, Selection::Id(request_id))?.result(true)?;
        tx.commit()?;
        Ok(result)
    }

    /// A lookup miss says nothing about a request that might still be in flight.
    pub(crate) fn lookup_esk_platform_sellback(
        &self,
        user: &str,
        token: &str,
        key: &str,
        config: &SellbackConfiguration,
    ) -> Result<SellbackResult> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        authenticate(&tx, user, token)?;
        if !label(key, 96) {
            return Err(SellbackError::InvalidInput.into());
        }
        let result = scan_on(&tx, user, token, config, Selection::Key(key))?.result(true)?;
        tx.commit()?;
        Ok(result)
    }
}
