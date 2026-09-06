//! Import production queue, contract, HTTP handlers and MCP dispatcher unchanged.
//! Only the outer runtime/request containers are reduced. This does not test the
//! project's descriptor authentication middleware or a running WebView host.
#![allow(dead_code)]

#[path = "../../../src/node_agent_browser_research.rs"]
mod node_agent_browser_research;
#[path = "../../../src/node_agent_browser_research_mcp.rs"]
mod node_agent_browser_research_mcp;

#[derive(Default)]
struct NodeRuntime {
    browser_research: node_agent_browser_research::BrowserResearchHub,
}

mod node_agent_project_docs_mcp {
    pub(crate) struct McpRequest {
        pub(crate) method: String,
        pub(crate) params: serde_json::Value,
    }
}

#[cfg(test)]
mod http_tests;
