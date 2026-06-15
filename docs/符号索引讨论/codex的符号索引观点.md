应该把“符号索引”当成 **repo map 的底层数据库/查询引擎**，而不是再做一个更大的 `repo_map.md`。

你现在已有基础：`RustSymbol`/`RustIndex` 在 [model.rs](/D:/rust/active-projects/elon%20cli/server/src/context_compiler/model.rs:42)，快速符号抽取在 [rust_symbols.rs](/D:/rust/active-projects/elon%20cli/server/src/context_compiler/rust_symbols.rs:10)，排序图在 [symbol_graph.rs](/D:/rust/active-projects/elon%20cli/server/src/context_compiler/symbol_graph.rs:18)，并且已经导出 `symbols.jsonl`、`edges.tsv`、`semantic_facts.jsonl`。下一步不是推翻，而是在它们上面加一层 `SymbolIndex`。

建议按这个顺序做：

1. 新建 `server/src/context_compiler/symbol_index.rs`
   - 定义 `SymbolIndex`、`SymbolRecord`、`SymbolEdge`、`SymbolQuery`
   - 从现有 `RustIndex + SymbolGraphSummary + RustAnalyzerLspReport` 构建
   - 建倒排表：`by_id`、`by_name`、`by_path`、`by_kind`、`outgoing_edges`、`incoming_edges`

2. 先提供 6 个核心查询能力
   - `search_symbols(query, limit)`
   - `get_symbol(id)`
   - `symbols_in_file(path)`
   - `references_to(symbol_id)`
   - `neighbors(symbol_id)`
   - `tests_for_symbol(symbol_id)`

3. 再把 rust-analyzer 语义事实合并进索引
   - `documentSymbol` 用来校准文件内层级
   - `definition` 用来修正定义位置
   - `references` 用来补强引用边
   - `workspace/symbol` 只做召回补充，不作为唯一来源

4. 输出新增 sidecar
   - `symbol_index.jsonl`：规范化符号记录
   - `symbol_edges.jsonl`：规范化边
   - `symbol_lookup.json`：轻量统计和索引摘要
   - 以后再升级为 SQLite 或 `.ai/cache/symbol-index/<workspace_hash>.json`

5. 接入上下文编译器
   - `repo map` 继续用于给 AI 快速扫结构
   - `SymbolIndex` 用于按任务切片：找相关符号、引用、测试、影响面
   - 最终 prompt 不塞全量索引，只塞 Top-K 符号和证据片段

第一刀我建议这样落文件：

```json
{
  "new_files": [
    "server/src/context_compiler/symbol_index.rs",
    "server/src/context_compiler/symbol_index_tests.rs"
  ],
  "modify_files": [
    "server/src/context_compiler/mod.rs",
    "server/src/context_compiler/artifact_exports.rs",
    "server/src/context_compiler/context_evidence.rs"
  ],
  "goal": "先把现有 RustSymbol/edges/LSP facts 汇总成可查询的 SymbolIndex，不改现有 repo map 行为"
}
```

关键判断：**不要只做 `workspace/symbol`**。那个更像 IDE 搜索接口，有数量限制和召回偏差。你的正确路线是：现有快速扫描做全量底座，`symbol_graph` 做引用/重要性，rust-analyzer 做 Top-K 精准增强，最后由 `SymbolIndex` 统一提供查询。

当前仓库状态我也核对了：`main` 已和 `origin/main` 对齐，最新提交是 `867e8c7d fix(server): 按优先级执行 repo map LSP 查询`。所以可以直接进入 `SymbolIndex Phase 1`，先实现内存索引和查询 API。