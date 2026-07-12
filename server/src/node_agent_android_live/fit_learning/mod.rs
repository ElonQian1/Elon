//! 从已完成的 FitRun 中沉淀可审计案例，并晋升为项目级拟合先验。
//!
//! 本模块不参与运行时拟合编排。调用方显式传入用户决策和 holdout
//! 评测器；只有完整通过目标、源码一致性和用户接受门禁的案例才会晋升。

mod case_builder;
mod coordinator;
mod eval;
mod historical_evaluator;
mod prior_index;
mod promotion;
mod store;
mod types;

pub(crate) use coordinator::{record_and_promote, top_k_for_run};
pub(crate) use types::{FitPrior, FitUserDecision};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod coordinator_tests;
