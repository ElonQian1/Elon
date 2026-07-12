use anyhow::{bail, Result};
use chrono::Utc;
use std::sync::{Mutex, OnceLock};

use super::super::fit_run::{FitRunDocument, FitTrial};
use super::case_builder::translation_features;
use super::historical_evaluator::HistoricalAdjustmentEvaluator;
use super::prior_index::FitPriorIndex;
use super::promotion::{promote_priors, FitPromotionPolicy};
use super::store::FitLearningStore;
use super::types::{
    FitCase, FitCaseReview, FitPriorMatch, FitPriorQuery, FitRecordAndPromoteResult,
    FitUserDecision,
};

fn coordinator_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[derive(Debug, Clone)]
pub(crate) struct FitLearningCoordinator {
    store: FitLearningStore,
    evaluator: HistoricalAdjustmentEvaluator,
    policy: FitPromotionPolicy,
}

impl FitLearningCoordinator {
    pub(crate) fn for_run(run: &FitRunDocument) -> Result<Self> {
        Ok(Self {
            store: FitLearningStore::new(&run.project_root)?,
            evaluator: HistoricalAdjustmentEvaluator::default(),
            policy: FitPromotionPolicy {
                max_holdout_regression: 0.35,
                max_mean_loss_regression: 0.25,
                evaluate_training_evidence: true,
            },
        })
    }

    pub(crate) fn record_and_promote(
        &self,
        run: &FitRunDocument,
        trials: &[FitTrial],
        user_decision: FitUserDecision,
        note: Option<String>,
    ) -> Result<FitRecordAndPromoteResult> {
        if !run.phase.is_terminal() {
            bail!("只有终态 FitRun 可以沉淀学习案例");
        }
        let _guard = coordinator_lock()
            .lock()
            .map_err(|_| anyhow::anyhow!("fit learning 协调锁已损坏"))?;
        let case = FitCase::from_fit_run(
            run,
            trials,
            FitCaseReview {
                decision: user_decision,
                component_kind: infer_component_kind(run),
                decided_at: Some(Utc::now().to_rfc3339()),
                note,
            },
        );
        let cases = self.store.record_case(case.clone())?;
        let historical_holdouts = cases
            .cases
            .iter()
            .filter(|case| case.passes_promotion_gates())
            .cloned()
            .collect::<Vec<_>>();
        let promotion = promote_priors(
            &cases.cases,
            &historical_holdouts,
            &self.evaluator,
            &self.policy,
        )?;
        self.store.save_priors(&promotion.document)?;
        Ok(FitRecordAndPromoteResult {
            case,
            recorded_case_count: cases.cases.len(),
            promotion,
        })
    }

    pub(crate) fn top_k_for_run(
        &self,
        run: &FitRunDocument,
        limit: usize,
    ) -> Result<Vec<FitPriorMatch>> {
        let document = self.store.load_priors()?;
        Ok(
            FitPriorIndex::from_priors(document.priors).top_k(&FitPriorQuery {
                component_kind: infer_component_kind(run),
                definition_id: Some(run.pair.definition_id.clone()),
                properties: run.properties.clone(),
                density: run.environment.density,
                font_scale: run.environment.font_scale,
                theme: run.environment.theme.clone(),
                translation_features: translation_features(run),
                limit,
            }),
        )
    }
}

pub(crate) fn record_and_promote(
    run: &FitRunDocument,
    trials: &[FitTrial],
    user_decision: FitUserDecision,
    note: Option<String>,
) -> Result<FitRecordAndPromoteResult> {
    FitLearningCoordinator::for_run(run)?.record_and_promote(run, trials, user_decision, note)
}

pub(crate) fn top_k_for_run(run: &FitRunDocument, limit: usize) -> Result<Vec<FitPriorMatch>> {
    FitLearningCoordinator::for_run(run)?.top_k_for_run(run, limit)
}

fn infer_component_kind(run: &FitRunDocument) -> String {
    if let Some(kind) = run
        .pair
        .component_kind
        .as_deref()
        .map(str::trim)
        .filter(|kind| !kind.is_empty())
    {
        return kind.to_ascii_lowercase();
    }
    let definition = run.pair.definition_id.to_ascii_lowercase();
    let property_text = run.properties.join(" ").to_ascii_lowercase();
    for (kind, markers) in [
        (
            "button",
            &["button", "btn", "submit", "save", "pay", "cancel"][..],
        ),
        ("text", &["text", "label", "title", "subtitle"][..]),
        ("card", &["card", "surface", "panel"][..]),
        ("image", &["image", "icon", "avatar", "logo"][..]),
        ("input", &["input", "field", "edit", "search"][..]),
    ] {
        if markers
            .iter()
            .any(|marker| definition.contains(marker) || property_text.contains(marker))
        {
            return kind.to_string();
        }
    }
    "unknown".to_string()
}
