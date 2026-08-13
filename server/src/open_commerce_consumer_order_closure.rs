mod api;
mod mcp;
mod model;
mod service;

#[cfg(test)]
mod support;
#[cfg(test)]
mod tests;

pub(crate) use api::routes;
pub(crate) use mcp::{call_if_handled, definitions};
