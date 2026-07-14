use std::{fs, path::Path};

use serde::Serialize;

const PROJECT_DOCS_TOTAL_BUDGET: usize = 8_000;
const TRUNCATION_MARKER: &str = "\n<!-- project doc truncated by projectDocs budget -->\n";

const PROJECT_DOC_SPECS: &[ProjectDocSpec] = &[
    ProjectDocSpec {
        path: "AI_PROJECT.md",
        max_chars: 2_400,
        role: "project_overview",
    },
    ProjectDocSpec {
        path: "AI_ARCHITECTURE.md",
        max_chars: 2_000,
        role: "architecture",
    },
    ProjectDocSpec {
        path: "AI_INDEX.md",
        max_chars: 1_600,
        role: "entry_index",
    },
    ProjectDocSpec {
        path: "AI_RULES.md",
        max_chars: 1_600,
        role: "project_rules",
    },
    ProjectDocSpec {
        path: "AI_TASK_TEMPLATE.md",
        max_chars: 1_200,
        role: "task_template",
    },
    ProjectDocSpec {
        path: ".aiignore",
        max_chars: 800,
        role: "ai_ignore",
    },
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentProjectDocsContext {
    pub(crate) total_budget_chars: usize,
    pub(crate) included_count: usize,
    pub(crate) missing_count: usize,
    pub(crate) truncated: bool,
    pub(crate) documents: Vec<AgentProjectDoc>,
    #[serde(skip_serializing)]
    pub(crate) pack_section: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentProjectDoc {
    pub(crate) path: String,
    pub(crate) role: String,
    pub(crate) exists: bool,
    pub(crate) included: bool,
    pub(crate) char_count: usize,
    pub(crate) snippet_char_count: usize,
    pub(crate) truncated: bool,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Copy)]
struct ProjectDocSpec {
    path: &'static str,
    max_chars: usize,
    role: &'static str,
}

pub(crate) fn load_agent_project_docs(workspace: &Path) -> AgentProjectDocsContext {
    let mut remaining_budget = PROJECT_DOCS_TOTAL_BUDGET;
    let mut documents = Vec::with_capacity(PROJECT_DOC_SPECS.len());
    let mut included_sections = Vec::new();
    let mut included_count = 0;
    let mut missing_count = 0;
    let mut truncated = false;

    for spec in PROJECT_DOC_SPECS {
        let doc_path = workspace.join(spec.path);
        let Ok(content) = fs::read_to_string(&doc_path) else {
            missing_count += 1;
            documents.push(AgentProjectDoc {
                path: spec.path.to_string(),
                role: spec.role.to_string(),
                exists: false,
                included: false,
                char_count: 0,
                snippet_char_count: 0,
                truncated: false,
                reason: "missing".to_string(),
            });
            continue;
        };

        let normalized = normalize_newlines(&content);
        let char_count = normalized.chars().count();
        if remaining_budget == 0 {
            truncated = true;
            documents.push(AgentProjectDoc {
                path: spec.path.to_string(),
                role: spec.role.to_string(),
                exists: true,
                included: false,
                char_count,
                snippet_char_count: 0,
                truncated: true,
                reason: "project_docs_budget_exhausted".to_string(),
            });
            continue;
        }

        let limit = spec.max_chars.min(remaining_budget);
        let (snippet, doc_truncated) = truncate_chars(&normalized, limit);
        let snippet_char_count = snippet.chars().count();
        remaining_budget = remaining_budget.saturating_sub(snippet_char_count);
        included_count += 1;
        truncated = truncated || doc_truncated;

        included_sections.push(render_doc_section(
            spec,
            &snippet,
            char_count,
            doc_truncated,
        ));
        documents.push(AgentProjectDoc {
            path: spec.path.to_string(),
            role: spec.role.to_string(),
            exists: true,
            included: true,
            char_count,
            snippet_char_count,
            truncated: doc_truncated,
            reason: if doc_truncated {
                "included_truncated"
            } else {
                "included"
            }
            .to_string(),
        });
    }

    let pack_section = render_project_docs_context(&included_sections);
    AgentProjectDocsContext {
        total_budget_chars: PROJECT_DOCS_TOTAL_BUDGET,
        included_count,
        missing_count,
        truncated,
        documents,
        pack_section,
    }
}

pub(crate) fn prepend_project_docs_to_pack(
    project_docs: &AgentProjectDocsContext,
    pack: &str,
) -> String {
    if project_docs.pack_section.is_empty() {
        return pack.to_string();
    }

    let mut out = String::with_capacity(project_docs.pack_section.len() + pack.len() + 2);
    out.push_str(&project_docs.pack_section);
    if !pack.starts_with('\n') {
        out.push('\n');
    }
    out.push_str(pack);
    out
}

fn render_project_docs_context(sections: &[String]) -> String {
    if sections.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    out.push_str("<project_docs_context format=\"xml-wrapped-markdown\">\n");
    out.push_str(
        "- purpose: persistent AI project rules, architecture, and entry index loaded before code retrieval.\n",
    );
    for section in sections {
        out.push_str(section);
    }
    out.push_str("</project_docs_context>\n");
    out
}

fn render_doc_section(
    spec: &ProjectDocSpec,
    snippet: &str,
    char_count: usize,
    truncated: bool,
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "<doc path=\"{}\" role=\"{}\" chars=\"{}\" truncated=\"{}\">\n",
        xml_escape(spec.path),
        xml_escape(spec.role),
        char_count,
        truncated
    ));
    out.push_str(snippet);
    if !snippet.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("</doc>\n");
    out
}

fn normalize_newlines(value: &str) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n")
}

fn truncate_chars(value: &str, max_chars: usize) -> (String, bool) {
    if value.chars().count() <= max_chars {
        return (value.to_string(), false);
    }

    let marker_chars = TRUNCATION_MARKER.chars().count();
    if max_chars <= marker_chars {
        return (value.chars().take(max_chars).collect(), true);
    }

    let keep_chars = max_chars - marker_chars;
    let mut truncated = value.chars().take(keep_chars).collect::<String>();
    truncated.push_str(TRUNCATION_MARKER);
    (truncated, true)
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
#[path = "agent_rag_project_docs_tests.rs"]
mod tests;
