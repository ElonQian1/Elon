use super::{fail, model::text, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RichCard {
    pub kind: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trend: Option<String>,
    #[serde(default)]
    pub periods: Vec<Period>,
    #[serde(default)]
    pub metrics: Vec<Metric>,
    #[serde(default)]
    pub series: Vec<Series>,
    #[serde(default)]
    pub points: Vec<Point>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Period {
    pub id: String,
    pub label: String,
    pub selected: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Metric {
    pub label: String,
    pub value: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Series {
    pub key: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_suffix: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Point {
    pub label: String,
    pub values: Vec<f64>,
}

impl RichCard {
    pub(super) fn validate(&self) -> Result<()> {
        text(&self.title, 120)?;
        for (value, bound) in [
            (&self.description, 240),
            (&self.symbol, 24),
            (&self.primary_value, 64),
            (&self.secondary_value, 96),
        ] {
            if let Some(value) = value {
                text(value, bound)?;
            }
        }
        if self.title.trim().is_empty()
            || self.periods.len() > 12
            || self.metrics.len() > 16
            || self.series.len() > 4
            || self.points.len() > 512
        {
            return Err(fail(400, "Invalid rich card bounds"));
        }
        match self.kind.as_str() {
            "finance" => {
                if !self
                    .primary_value
                    .as_deref()
                    .is_some_and(|v| !v.trim().is_empty())
                    || !matches!(
                        self.trend.as_deref(),
                        Some("positive" | "negative" | "neutral")
                    )
                    || self.series.len() > 1
                {
                    return Err(fail(400, "Invalid finance card"));
                }
            }
            "chart" => {
                if self.series.is_empty()
                    || self.points.len() < 2
                    || self.points.len() > 256
                    || self.primary_value.is_some()
                    || self.secondary_value.is_some()
                    || self.trend.is_some()
                    || self.symbol.is_some()
                    || !self.periods.is_empty()
                    || !self.metrics.is_empty()
                {
                    return Err(fail(400, "Invalid line chart"));
                }
            }
            _ => return Err(fail(400, "Unsupported rich card")),
        }
        for period in &self.periods {
            text(&period.id, 16)?;
            text(&period.label, 16)?;
        }
        for metric in &self.metrics {
            text(&metric.label, 64)?;
            text(&metric.value, 96)?;
        }
        let mut keys = std::collections::BTreeSet::new();
        for series in &self.series {
            text(&series.key, 48)?;
            text(&series.label, 64)?;
            if series.key.is_empty() || !keys.insert(&series.key) {
                return Err(fail(400, "Invalid chart series key"));
            }
            if let Some(v) = &series.value_prefix {
                text(v, 16)?;
            }
            if let Some(v) = &series.value_suffix {
                text(v, 16)?;
            }
        }
        for point in &self.points {
            text(&point.label, 64)?;
            let width = if self.kind == "finance" {
                1
            } else {
                self.series.len()
            };
            if point.values.len() != width || point.values.iter().any(|v| !v.is_finite()) {
                return Err(fail(400, "Invalid chart point"));
            }
        }
        Ok(())
    }
}
