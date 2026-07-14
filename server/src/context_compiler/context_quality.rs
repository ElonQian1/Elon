use std::collections::HashSet;

use super::{
    model::{
        ContextQualityCoverage, ContextQualityGap, ContextQualityReport, ContextQualitySemantic,
        ContextQualitySeverity, RepoContextIndex, RustAnalyzerLspStatus, RustAnalyzerProbeStatus,
    },
    relevance::RelevantFile,
    validation::ValidationPlan,
};

const TOP_FILE_SAMPLE: usize = 12;
const TOP_SYMBOL_SAMPLE: usize = 20;
const MAX_GAPS: usize = 18;

pub(crate) fn build_context_quality_report(
    index: &RepoContextIndex,
    relevant_files: &[RelevantFile],
    validation_plan: &ValidationPlan,
) -> ContextQualityReport {
    let coverage = build_coverage(index, validation_plan);
    let semantic = build_semantic(index);
    let mut gaps = collect_gaps(index, relevant_files, &coverage, &semantic, validation_plan);
    gaps.truncate(MAX_GAPS);
    let score = score_quality(&coverage, &semantic, &gaps);
    let recommended_actions = recommended_actions(index, &coverage, &semantic, &gaps);

    ContextQualityReport {
        score,
        coverage,
        semantic,
        gaps,
        recommended_actions,
    }
}

#[path = "context_quality_impl.rs"]
mod impl_funcs;
use self::impl_funcs::*;
