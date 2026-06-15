可以，**很适合用 Rust 实现**。我建议你先做一个 **Tree-sitter 语法级符号索引 MVP**，再逐步接入 **rust-analyzer / LSP 的语义能力**。

最合理的路线是：

```text
第一版：Rust + tree-sitter-rust
目标：提取 struct / enum / trait / impl method / fn / const / mod 的定义、签名、行号
优点：快、简单、稳定、适合 repo map

第二版：加 relations
目标：contains / imports / implements / calls / type_uses
优点：能做影响分析和上下文排序

第三版：接 rust-analyzer 或 LSP
目标：精确 definition / references / implementations / call hierarchy
优点：真正语义级，能处理 trait、泛型、模块解析、宏展开后的很多情况
```

Aider 的 repo map 就是一个很好的参考：它会把仓库里的关键 class/function、类型和调用签名做成简洁 map，随用户请求一起给 LLM，并且包含关键定义行。([Aider][1]) Tree-sitter 官方也明确支持用 queries 做 code navigation，并用 `@definition.*`、`@reference.*`、`@name` 这类 capture 来标记可命名实体。([tree-sitter.github.io][2])

---

# 1. 适不适合用 Rust 写？

**适合，而且比 Python/JS 更适合做本地代码索引器。**

原因很简单：

```text
Rust 适合：
- 扫描大量文件
- 并行解析
- 增量更新
- 本地 CLI / daemon / MCP server
- SQLite / Tantivy / Kuzu / Qdrant 等索引后端
- Tree-sitter 原生 Rust bindings
- 和 rust-analyzer 生态更近
```

Tree-sitter 的 Rust binding 官方文档里就展示了如何设置 parser、加载语言 grammar、解析源码，并且 Tree-sitter 本身支持增量解析。([Docs.rs][3])

不过要注意一个边界：

**Tree-sitter 只能很好地做“语法级索引”，不能单独做完整的 Rust 语义解析。**

比如它能知道：

```rust
repo.find_by_email(email).await?
```

这里有一个调用表达式，大概叫 `find_by_email`。

但它不知道这个 `find_by_email` 精确绑定到哪个 trait、哪个 impl、哪个 crate。要做这个，就需要 rust-analyzer 或 LSP。rust-analyzer 的官方架构说明里说，它会从源码和 CrateGraph 生成 fully resolved 的语义模型，引用会绑定到声明，表达式也有类型信息。([rust-analyzer.github.io][4])

所以我的建议是：

```text
repo map / AI 上下文：Tree-sitter 足够开始
精准跳转 / 引用 / trait impl：接 rust-analyzer
标准化 code intelligence index：考虑 SCIP
```

SCIP 是 Sourcegraph 生态里的语言无关代码索引协议，用来支持 go to definition、find references、find implementations，并且官方仓库包含 Go 和 Rust bindings。([GitHub][5])

---

# 2. 功能应该怎么拆？

你的符号索引器可以拆成 5 层：

```text
Repo Scanner
  ↓
Parser
  ↓
Symbol Extractor
  ↓
Index Store
  ↓
Repo Map Renderer / Retrieval
```

## 第一层：Repo Scanner

负责找到项目里的源码文件。

Rust 里可以用：

```text
ignore crate
```

它能自动尊重 `.gitignore`，比自己 `std::fs::read_dir` 更适合代码仓库。

## 第二层：Parser

第一版用：

```text
tree-sitter
tree-sitter-rust
```

Tree-sitter 官方文档也说它可以和 query language 一起用于 code navigation，`tree-sitter tags` 就是把源码里的有趣语法节点 dump 出来。([tree-sitter.github.io][2])

## 第三层：Symbol Extractor

对 Rust，你先提取这些就够了：

```text
function_item
function_signature_item
struct_item
enum_item
trait_item
impl_item
type_item
const_item
static_item
mod_item
macro_definition
```

第一版重点不是“所有东西”，而是：

```text
名字
类型
文件路径
起止行号
签名
可见性
父级上下文
qualified_name
```

## 第四层：Index Store

MVP 先输出 JSON 文件即可。

成熟版建议用 SQLite：

```sql
CREATE TABLE symbols (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    qualified_name TEXT NOT NULL,
    kind TEXT NOT NULL,
    file_path TEXT NOT NULL,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    signature TEXT,
    visibility TEXT,
    parent TEXT
);

CREATE INDEX idx_symbols_name ON symbols(name);
CREATE INDEX idx_symbols_qname ON symbols(qualified_name);
CREATE INDEX idx_symbols_file ON symbols(file_path);
```

第二阶段再加：

```sql
CREATE TABLE relations (
    id TEXT PRIMARY KEY,
    from_symbol_id TEXT NOT NULL,
    to_symbol_id TEXT,
    to_name TEXT,
    relation_kind TEXT NOT NULL,
    file_path TEXT,
    line INTEGER,
    confidence REAL DEFAULT 1.0
);
```

## 第五层：Repo Map Renderer

最终喂给 AI 时，不要直接喂 JSON。

把索引渲染成：

````md
## src/service/auth_service.rs

Defines:
```rust
pub struct AuthService<R: UserRepository>;
impl<R: UserRepository> AuthService<R>;
pub async fn login(&self, email: &str, password: &str) -> Result<Token, AppError>;
````

Relations:

* AuthService::login uses UserRepository
* AuthService::login returns Token
* AuthService::login is called by login_handler

````

也就是：

```text
内部：SQLite / JSON / 图索引
外部给 AI：Markdown + Rust 签名 + 文件路径
````

---

# 3. Rust MVP：先做一个可跑的符号索引器

下面这个版本做的是：

```text
遍历仓库
解析 .rs 文件
提取 Rust 符号
输出 symbol_index.json
```

它是 **语法级索引器**，不是完整语义解析器。也就是说，它能很好地抽出定义和签名，但不会精确解析跨文件引用、trait dispatch、宏展开结果。

## Cargo.toml

```toml
[package]
name = "repo-symbol-index"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
ignore = "0.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tree-sitter = "0.26"
tree-sitter-rust = "0.24"
```

也可以直接用：

```bash
cargo add anyhow ignore serde --features serde/derive
cargo add serde_json tree-sitter tree-sitter-rust
```

## src/main.rs

```rust
use anyhow::{Context as AnyhowContext, Result};
use ignore::WalkBuilder;
use serde::Serialize;
use std::path::{Component, Path, PathBuf};
use tree_sitter::{Node, Parser};

#[derive(Debug, Serialize)]
struct RepoIndex {
    root: String,
    symbols: Vec<Symbol>,
}

#[derive(Debug, Clone, Serialize)]
struct Symbol {
    id: String,
    name: String,
    qualified_name: String,
    kind: String,
    file_path: String,
    start_line: usize,
    end_line: usize,
    start_byte: usize,
    end_byte: usize,
    signature: String,
    visibility: Option<String>,
    parent: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct ExtractContext {
    module_path: Vec<String>,
    impl_target: Option<String>,
    trait_name: Option<String>,
}

fn main() -> Result<()> {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?);

    let index = index_repo(&root)?;
    serde_json::to_writer_pretty(std::io::stdout().lock(), &index)?;
    println!();

    Ok(())
}

fn index_repo(root: &Path) -> Result<RepoIndex> {
    let mut symbols = Vec::new();

    let mut builder = WalkBuilder::new(root);
    builder.filter_entry(|entry| {
        let name = entry.file_name().to_string_lossy();
        !matches!(
            name.as_ref(),
            ".git" | "target" | "node_modules" | ".next" | "dist" | "build"
        )
    });

    for entry in builder.build() {
        let entry = entry?;
        let path = entry.path();

        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }

        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }

        let file_symbols = index_rust_file(root, path)
            .with_context(|| format!("failed to index {}", path.display()))?;

        symbols.extend(file_symbols);
    }

    Ok(RepoIndex {
        root: root.display().to_string(),
        symbols,
    })
}

fn index_rust_file(root: &Path, path: &Path) -> Result<Vec<Symbol>> {
    let source = std::fs::read_to_string(path)?;
    let rel_path = path.strip_prefix(root).unwrap_or(path);
    let rel_path_str = rel_path.to_string_lossy().replace('\\', "/");

    let mut parser = Parser::new();
    let language: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
    parser.set_language(&language)?;

    let tree = parser
        .parse(&source, None)
        .context("tree-sitter failed to parse source")?;

    let mut symbols = Vec::new();

    let context = ExtractContext {
        module_path: rust_module_path_from_file(rel_path),
        ..Default::default()
    };

    visit_node(
        tree.root_node(),
        &source,
        &rel_path_str,
        &context,
        &mut symbols,
    );

    Ok(symbols)
}

fn visit_node(
    node: Node<'_>,
    source: &str,
    file_path: &str,
    context: &ExtractContext,
    symbols: &mut Vec<Symbol>,
) {
    if let Some(symbol) = symbol_from_node(node, source, file_path, context) {
        symbols.push(symbol);
    }

    let mut child_context = context.clone();

    match node.kind() {
        "mod_item" => {
            if has_named_child_kind(node, "declaration_list") {
                if let Some(name_node) = node.child_by_field_name("name") {
                    child_context
                        .module_path
                        .push(node_text(name_node, source));
                }
            }
        }
        "impl_item" => {
            child_context.impl_target = impl_target_label(node, source);
        }
        "trait_item" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                child_context.trait_name = Some(node_text(name_node, source));
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        visit_node(child, source, file_path, &child_context, symbols);
    }
}

fn symbol_from_node(
    node: Node<'_>,
    source: &str,
    file_path: &str,
    context: &ExtractContext,
) -> Option<Symbol> {
    let node_kind = node.kind();

    let raw_kind = match node_kind {
        "function_item" => "function",
        "function_signature_item" => "function_signature",
        "struct_item" => "struct",
        "enum_item" => "enum",
        "trait_item" => "trait",
        "type_item" => "type_alias",
        "const_item" => "const",
        "static_item" => "static",
        "mod_item" => "module",
        "macro_definition" => "macro",
        _ => return None,
    };

    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source);

    let mut kind = raw_kind.to_string();
    let mut parent = None;

    let mut qname_parts = vec!["crate".to_string()];
    qname_parts.extend(context.module_path.iter().cloned());

    match node_kind {
        "function_item" => {
            if let Some(impl_target) = &context.impl_target {
                kind = "method".to_string();
                parent = Some(impl_target.clone());
                qname_parts.push(impl_target_for_qname(impl_target));
            } else if let Some(trait_name) = &context.trait_name {
                kind = "trait_method".to_string();
                parent = Some(trait_name.clone());
                qname_parts.push(trait_name.clone());
            }
            qname_parts.push(name.clone());
        }
        "function_signature_item" => {
            if let Some(trait_name) = &context.trait_name {
                kind = "trait_method".to_string();
                parent = Some(trait_name.clone());
                qname_parts.push(trait_name.clone());
            }
            qname_parts.push(name.clone());
        }
        _ => {
            qname_parts.push(name.clone());
        }
    }

    let qualified_name = qname_parts.join("::");
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;

    Some(Symbol {
        id: format!("{file_path}:{start_line}:{end_line}:{qualified_name}"),
        name,
        qualified_name,
        kind,
        file_path: file_path.to_string(),
        start_line,
        end_line,
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        signature: compact_signature(node, source),
        visibility: visibility(node, source),
        parent,
    })
}

fn rust_module_path_from_file(rel_path: &Path) -> Vec<String> {
    let without_ext = rel_path.with_extension("");

    let mut parts: Vec<String> = without_ext
        .components()
        .filter_map(|component| match component {
            Component::Normal(s) => s.to_str().map(|x| x.to_string()),
            _ => None,
        })
        .collect();

    if parts.first().map(String::as_str) == Some("src") {
        parts.remove(0);
    }

    if matches!(parts.last().map(String::as_str), Some("lib" | "main" | "mod"))
    {
        parts.pop();
    }

    parts
}

fn impl_target_label(node: Node<'_>, source: &str) -> Option<String> {
    let ty = node
        .child_by_field_name("type")
        .map(|n| compact_text(&node_text(n, source)));

    let tr = node
        .child_by_field_name("trait")
        .map(|n| compact_text(&node_text(n, source)));

    match (tr, ty) {
        (Some(tr), Some(ty)) => Some(format!("{ty} as {tr}")),
        (None, Some(ty)) => Some(ty),
        _ => Some("impl".to_string()),
    }
}

fn impl_target_for_qname(label: &str) -> String {
    label
        .split(" as ")
        .next()
        .unwrap_or(label)
        .trim()
        .to_string()
}

fn visibility(node: Node<'_>, source: &str) -> Option<String> {
    for i in 0..node.child_count() {
        let child = node.child(i)?;
        if child.kind() == "visibility_modifier" {
            return Some(compact_text(&node_text(child, source)));
        }
    }

    None
}

fn compact_signature(node: Node<'_>, source: &str) -> String {
    let text = node_text(node, source);

    let cutoff = text
        .find('{')
        .map(|i| i + 1)
        .or_else(|| text.find(';').map(|i| i + 1))
        .unwrap_or(text.len());

    let signature = compact_text(&text[..cutoff]);

    truncate_chars(&signature, 260)
}

fn compact_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let mut s: String = text.chars().take(max_chars.saturating_sub(3)).collect();
    s.push_str("...");
    s
}

fn node_text(node: Node<'_>, source: &str) -> String {
    node.utf8_text(source.as_bytes()).unwrap_or("").to_string()
}

fn has_named_child_kind(node: Node<'_>, kind: &str) -> bool {
    let mut cursor = node.walk();

    node.named_children(&mut cursor)
        .any(|child| child.kind() == kind)
}
```

## 运行

```bash
cargo run -- /path/to/your/repo > symbol_index.json
```

输出大概长这样：

```json
{
  "root": "/path/to/your/repo",
  "symbols": [
    {
      "id": "src/service/auth_service.rs:12:18:crate::service::auth_service::AuthService",
      "name": "AuthService",
      "qualified_name": "crate::service::auth_service::AuthService",
      "kind": "struct",
      "file_path": "src/service/auth_service.rs",
      "start_line": 12,
      "end_line": 18,
      "signature": "pub struct AuthService<R: UserRepository> {",
      "visibility": "pub",
      "parent": null
    },
    {
      "id": "src/service/auth_service.rs:25:72:crate::service::auth_service::AuthService<R>::login",
      "name": "login",
      "qualified_name": "crate::service::auth_service::AuthService<R>::login",
      "kind": "method",
      "file_path": "src/service/auth_service.rs",
      "start_line": 25,
      "end_line": 72,
      "signature": "pub async fn login(&self, email: &str, password: &str) -> Result<Token, AppError> {",
      "visibility": "pub",
      "parent": "AuthService<R>"
    }
  ]
}
```

---

# 4. 第二步：加关系索引

第一版只有 symbols。下一步加 relations。

你可以先做这些关系：

```text
contains
module contains symbol
trait contains trait_method
impl contains method

imports
file imports path

implements
impl Type for Trait

calls
function calls callee_name

uses_type
function / struct / enum uses type_name
```

关系结构可以先这样：

```rust
#[derive(Debug, Serialize)]
struct Relation {
    from_symbol_id: Option<String>,
    to_symbol_id: Option<String>,
    to_name: Option<String>,
    relation_kind: String,
    file_path: String,
    line: usize,
    confidence: f32,
}
```

第一版 `calls` 可以只做模糊关系：

```text
AuthService::login --calls_name--> find_by_email
AuthService::login --calls_name--> verify_password
AuthService::login --calls_name--> issue_token
```

这还不是精确绑定，但对 AI repo map 已经很有用。

第三版再把 `calls_name` 升级成：

```text
AuthService::login --calls--> UserRepository::find_by_email
```

这个升级需要 rust-analyzer / LSP。

LSP 标准里有 `textDocument/documentSymbol`，可以返回一个文件内的符号层级；还有 `workspace/symbol`，用于按 query 查全项目符号。([微软 GitHub][6]) LSP 也定义了 `textDocument/definition` 和 `textDocument/references`，分别用于跳转定义和查找项目级引用。([微软 GitHub][6])

---

# 5. 什么时候该接 rust-analyzer？

当你需要这些能力时，就该接：

```text
精确 find references
精确 go to definition
trait method 到 impl method 的绑定
泛型类型解析
pub use / re-export 解析
workspace crates 依赖解析
宏展开后的符号
cfg feature 影响
```

rust-analyzer 是 Rust 的官方主流 IDE 语义引擎，它提供 go-to-definition、find-all-references、refactorings、code completion 等能力，并且内部本身是一组分析 Rust 代码的库。([GitHub][7]) 它的 `ide` crate 提供面向 IDE 的 API，背后由 `RootDatabase`、`salsa` 和 `hir` 支撑。([Rust 语言][8])

实现方式有两种：

## 方式 A：把 rust-analyzer 当 LSP server 调

这是我更推荐的方式。

你的索引器启动：

```bash
rust-analyzer
```

然后通过 stdin/stdout 发 JSON-RPC 请求：

```text
initialize
initialized
textDocument/documentSymbol
workspace/symbol
textDocument/definition
textDocument/references
textDocument/implementation
textDocument/prepareCallHierarchy
callHierarchy/incomingCalls
callHierarchy/outgoingCalls
```

优点：

```text
不绑定 rust-analyzer 内部 unstable API
更像编辑器
比较稳
```

缺点：

```text
实现 JSON-RPC 麻烦一些
需要处理初始化、workspace、文件 URI、进程生命周期
```

## 方式 B：直接依赖 rust-analyzer 的 ra_ap_* crates

优点：

```text
纯 Rust library 方式
不需要跑外部进程
可以更深地拿 HIR / semantic info
```

缺点：

```text
API 复杂
稳定性不如 LSP
版本升级成本高
```

所以推荐顺序是：

```text
Tree-sitter MVP
  ↓
LSP 调 rust-analyzer
  ↓
必要时再深入 ra_ap_* crates
```

---

# 6. 开源项目值得参考哪些？

## Aider repo map

重点看它的思想：**不是把整个 repo 塞给模型，而是提取关键符号、签名、文件关系，再控制 token budget**。官方说明里明确说 repo map 包含文件列表、关键符号，以及这些符号如何定义。([Aider][1])

可以参考它的：

```text
符号提取
重要性排序
repo map 渲染
token budget 控制
```

## Tree-sitter / tree-sitter-rust

重点看：

```text
parser
node tree
query
tags.scm
capture convention
```

Tree-sitter 的 code navigation 文档里说，tagging 本质就是识别程序里可命名实体，并用 query capture 标出它的 role、kind 和 name。([tree-sitter.github.io][2])

## ast-grep

ast-grep 是 Rust 写的、基于 Tree-sitter 的结构化搜索和重写工具。它的价值不是 repo map，而是可以参考它如何把 Tree-sitter 用成一个高性能、跨语言的结构搜索系统。它官方介绍自己是 fast、polyglot、structural search/rewrite 工具，并且由 parallel Rust 驱动。([Ast Grep][9])

可以参考：

```text
Tree-sitter 多语言抽象
AST pattern
批量扫描
结构化 rewrite
CLI 体验
```

## Universal Ctags

ctags 是传统符号索引的经典参考。Universal Ctags 官方说明里说，它会为源文件里的 language objects 生成 index/tag file，方便编辑器和工具定位这些对象。([GitHub][10])

可以参考：

```text
符号类型设计
tag file 思路
多语言 symbol kind
编辑器跳转模型
```

## rust-analyzer

这是 Rust 语义索引最值得参考的项目。

重点不是直接抄，而是理解：

```text
CrateGraph
HIR
name resolution
find references
goto definition
call hierarchy
incremental database
```

rust-analyzer 官方架构说明里提到，HIR 相关 crates 是 rust-analyzer 的 “brain”，负责 name resolution、macro expansion、type inference。([rust-analyzer.github.io][4])

## SCIP

适合你以后想做标准化 code intelligence index。

SCIP 是语言无关的代码索引协议，用于 go to definition、find references、find implementations，并带有 Protobuf schema、CLI、Rust bindings。([GitHub][5])

适合参考：

```text
标准 index schema
symbol descriptor
occurrence
relationship
跨语言 code intelligence 数据格式
```

## tree-sitter-graph

这个项目可以参考“如何从 Tree-sitter AST 构造图”。它官方说明说这个库定义了一个 DSL，可以从 Tree-sitter 解析出的源码构造任意 graph structure。([GitHub][11])

适合参考：

```text
AST → graph
node / edge attribute
关系抽取 DSL
```

## CocoIndex Code

这是偏 AI coding agent 的参考。它官方说明是一个开源 semantic code search engine，会用 Tree-sitter 解析源码为 AST-aware chunks，增量 embedding，并通过 CLI 或 MCP server 提供结果。([CocoIndex][12])

适合参考：

```text
AST-aware chunk
incremental index
MCP server
agent search tool
```

---

# 7. 如果你要把现有实现“改写成 Rust”，对应关系是这样

```text
Python os.walk / glob
  → ignore crate

Python tree_sitter
  → tree-sitter + tree-sitter-rust

NetworkX / 自己的 graph dict
  → petgraph / slotmap / SQLite relations table

sqlite3
  → rusqlite / sqlx

JSON 输出
  → serde / serde_json

repo map 模板
  → 手写 Markdown renderer / tera / askama

全文搜索
  → tantivy / ripgrep 调用 / SQLite FTS5

向量搜索
  → qdrant client / lance / sqlite-vss / usearch

LSP client
  → lsp-types + jsonrpc / tower-lsp 相关生态
```

Aider 这类 repo map 的核心可以完全改写成 Rust：

```text
scan repo
  → parse files
  → extract symbols
  → build symbol graph
  → rank symbols
  → render markdown repo map
```

---

# 8. 我建议你的最终架构

```text
repo-indexer
├── scanner
│   └── 扫描文件，尊重 .gitignore
├── parser
│   ├── tree_sitter_rust
│   ├── tree_sitter_python
│   └── tree_sitter_typescript
├── extractor
│   ├── rust_symbols.rs
│   ├── rust_relations.rs
│   └── common.rs
├── store
│   ├── sqlite.rs
│   └── jsonl.rs
├── ranking
│   ├── text_match.rs
│   ├── graph_score.rs
│   └── recency_score.rs
├── renderer
│   ├── markdown.rs
│   └── xml_wrapped.rs
└── lsp
    └── rust_analyzer.rs
```

第一版只做：

```text
scanner
parser
extractor
json output
markdown renderer
```

第二版加：

```text
SQLite
relations
ranking
```

第三版加：

```text
rust-analyzer LSP
SCIP export
MCP server
```

---

# 9. 一句话方案

你现在最应该做的是：

```text
用 Rust + tree-sitter-rust 先做 syntax-level symbol index。
把结果存 JSON/SQLite。
repo map 渲染成 Markdown + Rust 签名。
等需要精准引用、trait impl、跨文件解析时，再接 rust-analyzer/LSP。
```

不要一开始就挑战完整 Rust 语义解析。先把 **符号定义索引** 做稳，再做 **关系索引**，最后做 **语义增强**。这条路线最现实，也最适合 AI repo map。

[1]: https://aider.chat/docs/repomap.html "Repository map | aider"
[2]: https://tree-sitter.github.io/tree-sitter/4-code-navigation.html "Code Navigation - Tree-sitter"
[3]: https://docs.rs/tree-sitter "tree_sitter - Rust"
[4]: https://rust-analyzer.github.io/book/contributing/architecture.html "Architecture - rust-analyzer"
[5]: https://github.com/scip-code/scip "GitHub - scip-code/scip: SCIP Code Intelligence Protocol · GitHub"
[6]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/ "Specification"
[7]: https://github.com/rust-lang/rust-analyzer "GitHub - rust-lang/rust-analyzer: A Rust compiler front-end for IDEs · GitHub"
[8]: https://rust-lang.github.io/rust-analyzer/ide/ "ide - Rust"
[9]: https://ast-grep.github.io/ "ast-grep | structural search/rewrite tool for many languages"
[10]: https://github.com/universal-ctags/ctags "GitHub - universal-ctags/ctags: A maintained ctags implementation · GitHub"
[11]: https://github.com/tree-sitter/tree-sitter-graph/ "GitHub - tree-sitter/tree-sitter-graph: Construct graphs from parsed source code · GitHub"
[12]: https://cocoindex.io/cocoindex-code "CocoIndex Code — speed up Claude Code, Cursor & Codex"
