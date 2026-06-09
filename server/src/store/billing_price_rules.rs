//! Configurable model and compute-unit pricing rules.
//!
//! The billing runtime falls back to the built-in conservative table when a
//! rule cannot be read. Operators can adjust this table without a code deploy.

use anyhow::{anyhow, Result};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::{new_id, now, Store};

#[derive(Debug, Clone, Serialize)]
pub struct BillingPriceRule {
    pub id: String,
    pub pattern: String,
    pub input_usd_per_m: f64,
    pub cached_usd_per_m: f64,
    pub output_usd_per_m: f64,
    pub priority: i64,
    pub enabled: bool,
    pub note: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BillingPriceRuleUpsert {
    pub pattern: String,
    pub input_usd_per_m: f64,
    pub cached_usd_per_m: f64,
    pub output_usd_per_m: f64,
    #[serde(default)]
    pub priority: i64,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub note: Option<String>,
}

fn default_enabled() -> bool {
    true
}

impl BillingPriceRule {
    pub fn price_tuple(&self) -> (f64, f64, f64) {
        (
            self.input_usd_per_m,
            self.cached_usd_per_m,
            self.output_usd_per_m,
        )
    }
}

impl Store {
    pub fn billing_list_price_rules(&self) -> Result<Vec<BillingPriceRule>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, pattern, input_usd_per_m, cached_usd_per_m, output_usd_per_m,
                    priority, enabled, note, updated_at
             FROM billing_price_rules
             ORDER BY enabled DESC, priority DESC, length(pattern) DESC, pattern ASC",
        )?;
        let rows = stmt
            .query_map([], read_rule_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn billing_find_price_rule(&self, model: &str) -> Result<Option<BillingPriceRule>> {
        let model = model.trim().to_ascii_lowercase();
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, pattern, input_usd_per_m, cached_usd_per_m, output_usd_per_m,
                    priority, enabled, note, updated_at
             FROM billing_price_rules
             WHERE enabled = 1
             ORDER BY priority DESC, length(pattern) DESC, pattern ASC",
        )?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let rule = read_rule_row(row)?;
            let pattern = rule.pattern.trim().to_ascii_lowercase();
            if pattern == "*" || (!pattern.is_empty() && model.contains(&pattern)) {
                return Ok(Some(rule));
            }
        }
        Ok(None)
    }

    pub fn billing_upsert_price_rule(
        &self,
        input: &BillingPriceRuleUpsert,
    ) -> Result<BillingPriceRule> {
        validate_price_rule(input)?;
        let pattern = input.pattern.trim();
        let note = input
            .note
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let ts = now();
        let id = new_id("bpr");
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO billing_price_rules (
               id, pattern, input_usd_per_m, cached_usd_per_m, output_usd_per_m,
               priority, enabled, note, updated_at
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)
             ON CONFLICT(pattern) DO UPDATE SET
               input_usd_per_m = excluded.input_usd_per_m,
               cached_usd_per_m = excluded.cached_usd_per_m,
               output_usd_per_m = excluded.output_usd_per_m,
               priority = excluded.priority,
               enabled = excluded.enabled,
               note = excluded.note,
               updated_at = excluded.updated_at",
            params![
                id,
                pattern,
                input.input_usd_per_m,
                input.cached_usd_per_m,
                input.output_usd_per_m,
                input.priority,
                if input.enabled { 1 } else { 0 },
                note,
                ts,
            ],
        )?;
        conn.query_row(
            "SELECT id, pattern, input_usd_per_m, cached_usd_per_m, output_usd_per_m,
                    priority, enabled, note, updated_at
             FROM billing_price_rules
             WHERE pattern = ?1",
            params![pattern],
            read_rule_row,
        )
        .map_err(Into::into)
    }
}

fn validate_price_rule(input: &BillingPriceRuleUpsert) -> Result<()> {
    let pattern = input.pattern.trim();
    if pattern.is_empty() {
        return Err(anyhow!("计价匹配规则不能为空"));
    }
    if pattern.len() > 120 {
        return Err(anyhow!("计价匹配规则过长"));
    }
    if !valid_price(input.input_usd_per_m)
        || !valid_price(input.cached_usd_per_m)
        || !valid_price(input.output_usd_per_m)
    {
        return Err(anyhow!("计价金额必须是非负有限数字"));
    }
    Ok(())
}

fn valid_price(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn read_rule_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BillingPriceRule> {
    let enabled: i64 = row.get(6)?;
    Ok(BillingPriceRule {
        id: row.get(0)?,
        pattern: row.get(1)?,
        input_usd_per_m: row.get(2)?,
        cached_usd_per_m: row.get(3)?,
        output_usd_per_m: row.get(4)?,
        priority: row.get(5)?,
        enabled: enabled != 0,
        note: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_store() -> (Store, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "elon_billing_price_rules_{}.db",
            Uuid::new_v4().simple()
        ));
        (Store::open(&path).expect("store should open"), path)
    }

    #[test]
    fn seeded_rules_match_known_models() {
        let (store, path) = temp_store();
        let rule = store
            .billing_find_price_rule("gpt-4o-mini-2024-07-18")
            .unwrap()
            .expect("rule should match");
        assert_eq!(rule.pattern, "gpt-4o-mini");
        assert_eq!(rule.price_tuple(), (0.15, 0.075, 0.60));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn higher_priority_custom_rule_wins() {
        let (store, path) = temp_store();
        store
            .billing_upsert_price_rule(&BillingPriceRuleUpsert {
                pattern: "gpt-4o-mini".to_string(),
                input_usd_per_m: 9.0,
                cached_usd_per_m: 8.0,
                output_usd_per_m: 7.0,
                priority: 999,
                enabled: true,
                note: Some("test override".to_string()),
            })
            .unwrap();
        let rule = store
            .billing_find_price_rule("gpt-4o-mini")
            .unwrap()
            .expect("rule should match");
        assert_eq!(rule.price_tuple(), (9.0, 8.0, 7.0));
        let _ = std::fs::remove_file(path);
    }
}
