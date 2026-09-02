use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Duration};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};

use super::{
    esk_assets::{account_ledger_on, ensure_user_exists, validate_key},
    new_id, now,
};
use crate::{
    esk_asset::exchange::{
        EskExchangeAccountLedger, EskExchangeDirection, EskExchangeExecutionInput,
        EskExchangeExecutionRecord, EskExchangeQuoteInput, EskExchangeQuoteRecord,
        PaperUsdtCreditInput, PaperUsdtCreditReceipt,
    },
    store::Store,
};

impl Store {
    pub(crate) fn esk_exchange_account_ledger(
        &self,
        user_id: &str,
    ) -> Result<EskExchangeAccountLedger> {
        let conn = self.conn()?;
        exchange_account_on(&conn, user_id)
    }

    pub(crate) fn create_paper_usdt_credit(
        &self,
        input: &PaperUsdtCreditInput,
    ) -> Result<PaperUsdtCreditReceipt> {
        validate_credit(input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_user_exists(&tx, &input.user_id)?;
        if let Some(existing) = credit_by_idempotency(&tx, &input.idempotency_key)? {
            if existing.user_id != input.user_id
                || existing.amount_units != input.amount_units
                || existing.reference != input.reference
            {
                bail!("相同 USDT Paper 登记幂等键不能用于不同请求");
            }
            tx.commit()?;
            return Ok(PaperUsdtCreditReceipt {
                replayed: true,
                ..existing
            });
        }
        let receipt = PaperUsdtCreditReceipt {
            credit_id: new_id("usdtc"),
            user_id: input.user_id.clone(),
            amount_units: input.amount_units,
            reference: input.reference.clone(),
            idempotency_key: input.idempotency_key.clone(),
            created_at: now(),
            replayed: false,
        };
        insert_entry(
            &tx,
            "user",
            Some(&receipt.user_id),
            "USDT",
            receipt.amount_units,
            "paper_usdt_credit",
            &receipt.credit_id,
            &receipt.reference,
            Some(&receipt.idempotency_key),
            &receipt.created_at,
        )?;
        insert_entry(
            &tx,
            "platform",
            None,
            "USDT",
            -receipt.amount_units,
            "paper_credit_offset",
            &receipt.credit_id,
            &receipt.reference,
            None,
            &receipt.created_at,
        )?;
        tx.commit()?;
        Ok(receipt)
    }

    pub(crate) fn create_esk_exchange_quote(
        &self,
        input: &EskExchangeQuoteInput,
    ) -> Result<EskExchangeQuoteRecord> {
        validate_quote(input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_user_exists(&tx, &input.user_id)?;
        ensure_available(&tx, &input.user_id, input.direction, input.input_units)?;
        let created_at = now();
        let expires_at = (parse_time(&created_at)? + Duration::seconds(60)).to_rfc3339();
        let quote = EskExchangeQuoteRecord {
            quote_id: new_id("eskxq"),
            user_id: input.user_id.clone(),
            direction: input.direction.label().to_string(),
            input_units: input.input_units,
            price_units: input.price_units,
            fee_bps: input.fee_bps,
            config_revision: input.config_revision.clone(),
            gross_output_units: input.gross_output_units,
            fee_units: input.fee_units,
            net_output_units: input.net_output_units,
            created_at,
            expires_at,
        };
        tx.execute(
            "INSERT INTO esk_exchange_quotes (
               quote_id, user_id, direction, input_units, price_units, fee_bps,
               config_revision, gross_output_units, fee_units, net_output_units,
               created_at, expires_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                quote.quote_id,
                quote.user_id,
                quote.direction,
                quote.input_units,
                quote.price_units,
                quote.fee_bps,
                quote.config_revision,
                quote.gross_output_units,
                quote.fee_units,
                quote.net_output_units,
                quote.created_at,
                quote.expires_at,
            ],
        )?;
        tx.commit()?;
        Ok(quote)
    }

    pub(crate) fn execute_esk_exchange(
        &self,
        input: &EskExchangeExecutionInput,
    ) -> Result<EskExchangeExecutionRecord> {
        validate_execution(input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_user_exists(&tx, &input.user_id)?;
        if let Some(existing) =
            execution_by_idempotency(&tx, &input.user_id, &input.idempotency_key)?
        {
            if existing.quote.quote_id != input.quote_id {
                bail!("相同兑换幂等键不能用于不同报价");
            }
            tx.commit()?;
            return Ok(EskExchangeExecutionRecord {
                replayed: true,
                ..existing
            });
        }
        let quote = quote_by_id(&tx, &input.user_id, &input.quote_id)?
            .ok_or_else(|| anyhow!("兑换报价不存在"))?;
        if quote.config_revision != input.config_revision {
            bail!("兑换报价价格配置已经更新，请重新报价");
        }
        if execution_by_quote(&tx, &quote.quote_id)?.is_some() {
            bail!("兑换报价已经成交");
        }
        if parse_time(&quote.expires_at)? <= parse_time(&now())? {
            bail!("兑换报价已过期，请重新报价");
        }
        let direction = EskExchangeDirection::from_label(&quote.direction)?;
        ensure_available(&tx, &input.user_id, direction, quote.input_units)?;
        let execution_id = new_id("eskxe");
        let executed_at = now();
        tx.execute(
            "INSERT INTO esk_exchange_executions (
               execution_id, quote_id, user_id, idempotency_key, executed_at
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                execution_id,
                quote.quote_id,
                input.user_id,
                input.idempotency_key,
                executed_at
            ],
        )?;
        post_exchange(
            &tx,
            &execution_id,
            &input.user_id,
            &quote,
            direction,
            &executed_at,
        )?;
        let record =
            execution_by_id(&tx, &execution_id)?.ok_or_else(|| anyhow!("兑换成交写入后不可见"))?;
        tx.commit()?;
        Ok(record)
    }

    pub(crate) fn list_esk_exchanges(
        &self,
        user_id: &str,
        limit: usize,
    ) -> Result<Vec<EskExchangeExecutionRecord>> {
        let conn = self.conn()?;
        let mut statement = conn.prepare(&format!(
            "{} WHERE e.user_id = ?1 ORDER BY e.executed_at DESC, e.execution_id DESC LIMIT ?2",
            execution_select()
        ))?;
        let rows =
            statement.query_map(params![user_id, limit.clamp(1, 100) as i64], map_execution)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }
}

pub(super) fn user_esk_exchange_delta_on(
    conn: &Connection,
    user_id: &str,
) -> Result<(i64, i64, Option<String>)> {
    conn.query_row(
        "SELECT COALESCE(SUM(amount_units), 0), COUNT(*), MAX(created_at)
           FROM esk_exchange_ledger_entries
          WHERE owner_kind = 'user' AND user_id = ?1 AND asset = 'ESK'",
        params![user_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .map_err(Into::into)
}

fn exchange_account_on(conn: &Connection, user_id: &str) -> Result<EskExchangeAccountLedger> {
    let (total, count, updated_at): (i64, i64, Option<String>) = conn.query_row(
        "SELECT COALESCE(SUM(amount_units), 0), COUNT(*), MAX(created_at)
           FROM esk_exchange_ledger_entries
          WHERE owner_kind = 'user' AND user_id = ?1 AND asset = 'USDT'",
        params![user_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    if total < 0 {
        bail!("USDT Paper 余额状态无效");
    }
    Ok(EskExchangeAccountLedger {
        usdt_units: total,
        entry_count: count,
        updated_at,
    })
}

fn ensure_available(
    conn: &Connection,
    user_id: &str,
    direction: EskExchangeDirection,
    input_units: i64,
) -> Result<()> {
    let available = match direction {
        EskExchangeDirection::UsdtToEsk => exchange_account_on(conn, user_id)?.usdt_units,
        EskExchangeDirection::EskToUsdt => {
            let ledger = account_ledger_on(conn, user_id)?;
            ledger
                .total_base_units
                .checked_sub(ledger.reserved_base_units)
                .ok_or_else(|| anyhow!("ESK 可用余额状态无效"))?
        }
    };
    if input_units > available {
        let asset = direction.assets().0;
        bail!("兑换金额超过当前可用 {asset} Paper 余额");
    }
    Ok(())
}

fn post_exchange(
    tx: &Transaction<'_>,
    execution_id: &str,
    user_id: &str,
    quote: &EskExchangeQuoteRecord,
    direction: EskExchangeDirection,
    created_at: &str,
) -> Result<()> {
    let (source, target) = direction.assets();
    let postings = [
        (
            "user",
            Some(user_id),
            source,
            -quote.input_units,
            "exchange_user_debit",
        ),
        (
            "platform",
            None,
            source,
            quote.input_units,
            "exchange_market_credit",
        ),
        (
            "platform",
            None,
            target,
            -quote.gross_output_units,
            "exchange_market_debit",
        ),
        (
            "user",
            Some(user_id),
            target,
            quote.net_output_units,
            "exchange_user_credit",
        ),
    ];
    for (owner, user, asset, amount, kind) in postings {
        insert_entry(
            tx,
            owner,
            user,
            asset,
            amount,
            kind,
            execution_id,
            &quote.quote_id,
            None,
            created_at,
        )?;
    }
    if quote.fee_units > 0 {
        insert_entry(
            tx,
            "platform",
            None,
            target,
            quote.fee_units,
            "platform_fee",
            execution_id,
            &quote.quote_id,
            None,
            created_at,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn insert_entry(
    tx: &Transaction<'_>,
    owner_kind: &str,
    user_id: Option<&str>,
    asset: &str,
    amount_units: i64,
    entry_kind: &str,
    group_id: &str,
    reference: &str,
    idempotency_key: Option<&str>,
    created_at: &str,
) -> Result<()> {
    tx.execute(
        "INSERT INTO esk_exchange_ledger_entries (
           entry_id, owner_kind, user_id, asset, amount_units, entry_kind,
           posting_group_id, reference, idempotency_key, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            new_id("eskxl"),
            owner_kind,
            user_id,
            asset,
            amount_units,
            entry_kind,
            group_id,
            reference,
            idempotency_key,
            created_at
        ],
    )?;
    Ok(())
}

fn credit_by_idempotency(conn: &Connection, key: &str) -> Result<Option<PaperUsdtCreditReceipt>> {
    conn.query_row(
        "SELECT posting_group_id, user_id, amount_units, reference, idempotency_key, created_at
           FROM esk_exchange_ledger_entries
          WHERE entry_kind = 'paper_usdt_credit' AND idempotency_key = ?1",
        params![key],
        |row| {
            Ok(PaperUsdtCreditReceipt {
                credit_id: row.get(0)?,
                user_id: row.get(1)?,
                amount_units: row.get(2)?,
                reference: row.get(3)?,
                idempotency_key: row.get(4)?,
                created_at: row.get(5)?,
                replayed: false,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

fn quote_by_id(
    conn: &Connection,
    user_id: &str,
    quote_id: &str,
) -> Result<Option<EskExchangeQuoteRecord>> {
    conn.query_row(
        &format!("{} WHERE user_id = ?1 AND quote_id = ?2", quote_select()),
        params![user_id, quote_id],
        map_quote,
    )
    .optional()
    .map_err(Into::into)
}

fn execution_by_idempotency(
    conn: &Connection,
    user_id: &str,
    key: &str,
) -> Result<Option<EskExchangeExecutionRecord>> {
    conn.query_row(
        &format!(
            "{} WHERE e.user_id = ?1 AND e.idempotency_key = ?2",
            execution_select()
        ),
        params![user_id, key],
        map_execution,
    )
    .optional()
    .map_err(Into::into)
}

fn execution_by_quote(conn: &Connection, quote_id: &str) -> Result<Option<String>> {
    conn.query_row(
        "SELECT execution_id FROM esk_exchange_executions WHERE quote_id = ?1",
        params![quote_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

fn execution_by_id(
    conn: &Connection,
    execution_id: &str,
) -> Result<Option<EskExchangeExecutionRecord>> {
    conn.query_row(
        &format!("{} WHERE e.execution_id = ?1", execution_select()),
        params![execution_id],
        map_execution,
    )
    .optional()
    .map_err(Into::into)
}

fn quote_select() -> &'static str {
    "SELECT quote_id, user_id, direction, input_units, price_units, fee_bps,
            config_revision, gross_output_units, fee_units, net_output_units,
            created_at, expires_at FROM esk_exchange_quotes"
}

fn execution_select() -> String {
    "SELECT e.execution_id, e.idempotency_key, e.executed_at,
            q.quote_id, q.user_id, q.direction, q.input_units, q.price_units, q.fee_bps,
            q.config_revision, q.gross_output_units, q.fee_units, q.net_output_units,
            q.created_at, q.expires_at
       FROM esk_exchange_executions e
       JOIN esk_exchange_quotes q ON q.quote_id = e.quote_id"
        .to_string()
}

fn map_quote(row: &rusqlite::Row<'_>) -> rusqlite::Result<EskExchangeQuoteRecord> {
    Ok(EskExchangeQuoteRecord {
        quote_id: row.get(0)?,
        user_id: row.get(1)?,
        direction: row.get(2)?,
        input_units: row.get(3)?,
        price_units: row.get(4)?,
        fee_bps: row.get(5)?,
        config_revision: row.get(6)?,
        gross_output_units: row.get(7)?,
        fee_units: row.get(8)?,
        net_output_units: row.get(9)?,
        created_at: row.get(10)?,
        expires_at: row.get(11)?,
    })
}

fn map_execution(row: &rusqlite::Row<'_>) -> rusqlite::Result<EskExchangeExecutionRecord> {
    Ok(EskExchangeExecutionRecord {
        execution_id: row.get(0)?,
        idempotency_key: row.get(1)?,
        executed_at: row.get(2)?,
        quote: EskExchangeQuoteRecord {
            quote_id: row.get(3)?,
            user_id: row.get(4)?,
            direction: row.get(5)?,
            input_units: row.get(6)?,
            price_units: row.get(7)?,
            fee_bps: row.get(8)?,
            config_revision: row.get(9)?,
            gross_output_units: row.get(10)?,
            fee_units: row.get(11)?,
            net_output_units: row.get(12)?,
            created_at: row.get(13)?,
            expires_at: row.get(14)?,
        },
        replayed: false,
    })
}

fn parse_time(value: &str) -> Result<DateTime<chrono::FixedOffset>> {
    DateTime::parse_from_rfc3339(value).with_context(|| "兑换时间格式无效")
}

fn validate_credit(input: &PaperUsdtCreditInput) -> Result<()> {
    if input.amount_units <= 0 {
        bail!("USDT Paper 登记金额必须大于 0");
    }
    validate_key(&input.user_id, "用户 ID", 160)?;
    validate_key(&input.reference, "登记引用", 240)?;
    validate_key(&input.idempotency_key, "幂等键", 160)
}

fn validate_quote(input: &EskExchangeQuoteInput) -> Result<()> {
    validate_key(&input.user_id, "用户 ID", 160)?;
    validate_key(&input.config_revision, "价格配置修订", 64)?;
    if input.input_units <= 0
        || input.price_units <= 0
        || input.gross_output_units <= 0
        || input.fee_units < 0
        || input.net_output_units <= 0
        || input.gross_output_units.checked_sub(input.fee_units) != Some(input.net_output_units)
    {
        bail!("兑换报价金额无效");
    }
    if input.fee_bps > 1_000 {
        bail!("兑换手续费超出范围");
    }
    Ok(())
}

fn validate_execution(input: &EskExchangeExecutionInput) -> Result<()> {
    validate_key(&input.user_id, "用户 ID", 160)?;
    validate_key(&input.quote_id, "报价 ID", 160)?;
    validate_key(&input.idempotency_key, "幂等键", 160)
}
