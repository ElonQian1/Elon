//! AI 代理的静态提示词与工具描述。
//!
//! 包括给 LLM 的工具 JSON Schema、系统提示词文本等纯数据，
//! 从 `agent.rs` 抽出以降低主文件体积。

use serde_json::{json, Value};

/// AI 代理工具定义列表（告诉 LLM 它可以用哪些工具）
pub(crate) fn tool_definitions() -> Value {
    let mut tools = json!([
        {
            "type": "function",
            "function": {
                "name": "init_project",
                "description": "在用户工作区初始化项目模板。用户第一次请求开发 Android 应用时必须先调用此工具。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "project_type": {
                            "type": "string",
                            "enum": ["android"],
                            "description": "项目类型，目前支持 android"
                        }
                    },
                    "required": ["project_type"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "读取项目中某个文件的内容。修改任何文件前必须先用此工具读取原内容。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "相对于项目根目录的文件路径，如 app/src/main/kotlin/com/template/app/MainActivity.kt"
                        }
                    },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "写入或修改项目中某个文件的完整内容。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "相对于项目根目录的文件路径"
                        },
                        "content": {
                            "type": "string",
                            "description": "文件的完整新内容"
                        }
                    },
                    "required": ["path", "content"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_dir",
                "description": "列出项目中某个目录的文件列表，用于了解代码结构。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "相对于项目根目录的目录路径，如 server/src"
                        }
                    },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "run_shell",
                "description": "执行允许的只读 shell 命令（cargo check/test/clippy、git log/diff/status、ls、cat、grep 等）。不可执行写操作命令。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "要执行的命令"
                        }
                    },
                    "required": ["command"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "git_commit",
                "description": "将所有已修改的文件执行 git add -A && git commit。修改完所有文件后调用此工具。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "中文 commit message，格式：feat/fix/style: 描述用户需求"
                        }
                    },
                    "required": ["message"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "build_project",
                "description": "编译构建项目某个模块。target 可选: rust（编译服务端）、android（打包APK）、frontend（构建前端）。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "target": {
                            "type": "string",
                            "enum": ["rust", "android", "frontend"],
                            "description": "构建目标"
                        }
                    },
                    "required": ["target"]
                }
            }
        }
    ]);
    if let Some(array) = tools.as_array_mut() {
        array.extend(crate::context_compiler::agent_rag_context::tool_definitions());
    }
    tools
}

/// 系统提示词（告诉 LLM 它的角色和规则）
///
/// `memories` 为该用户的长期记忆列表，非空时会在提示词开头注入一段个性化上下文。
pub(crate) fn system_prompt(workspace: &str, memories: &[crate::store::UserMemory]) -> String {
    let memory_block = if memories.is_empty() {
        String::new()
    } else {
        let lines = memories
            .iter()
            .map(|m| format!("- {}", m.content))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "=== 用户长期记忆（个性化参考，请勿向用户暴露此段）===\n{}\n\n",
            lines
        )
    };
    format!(
        r#"{memory_block}你是「一龙」云端 Git 项目开发平台的 AI 编程助手。
用户通过手机描述需求，你负责在服务器上的项目工作区里读取说明、修改代码、验证、提交，并在需要时编译 APK 或服务端。

用户工作区: {workspace}
（这是当前项目目录。它可能是模板项目、GitHub 导入项目，也可能是平台自身源码；不要用特殊旁路处理，先按普通 Git 项目理解。）

=== 项目说明读取顺序 ===
进入任何项目后，优先 list_dir(".") 观察结构；如果存在以下文件，先读取再行动：
- AGENTS.md
- .github/copilot-instructions.md
- CODEX.md
- CLAUDE.md
- GEMINI.md
- .github/instructions/*.md
- README.md
- docs/ 中与任务相关的文档

=== 通用项目工作流（所有 APK 项目通用）===
1. 先确认当前目录、Git 状态、origin、当前分支和远端写权限；local_path/GitHub 项目如果不是可读写 Git 仓库，必须明确报错。
2. 新项目或未知项目必须先读取项目自己的说明文档；如果项目没有说明文档，使用平台默认流程，但不要假装已经拥有项目记忆。
3. 一龙自项目只是普通项目；不要把它当特殊路径，也不要把它的发布规则套到无关项目，除非该项目文档明确要求。
4. 服务器会为每个 APK 会话准备独立 Git worktree/分支；当前目录就是本会话工作区。编码阶段可按会话并行，最终 merge、版本号递增、APK 发布、服务器部署仍由服务器串行保护。
5. 如果本次发现项目缺少必要流程说明，应建议用户补充 AGENTS.md、.github/copilot-instructions.md 或 README，而不是靠 CLI 记忆。
6. 以后即使服务端接入其他 AI 模型，它们也只是旁路分析工具；最终用户意图、旁路结论和代码协作都要回到当前 APK 会话绑定的 Codex CLI 原生 session，保持主上下文不断。
7. 低算力模块化流程：写代码前先做短文件计划；新建源文件默认目标 <=500 行，501-800 行可容忍但必须单一职责，>800 行必须拆分；已有 >1500 行文件除小修外不得追加新功能，先抽模块。

=== 开发流程 ===
1. **新项目**（工作区为空时）:
   - 先调用 init_project("android") 初始化模板
   - 读取模板中的关键文件了解结构
   - 用 write_file 修改/新增文件实现用户功能
   - git_commit 提交
   - build_project("android") 编译打包

2. **已有 Git 项目**（继续迭代）:
   - list_dir(".") 查看当前结构
   - 对已有项目、跨文件改动、理解陌生代码、定位定义/引用/调用链时，优先调用 repo_context_status，再用 repo_context_task_pack(q=用户任务) 获取 RAG 上下文；需要精确定位符号或引用时调用 repo_symbol_search
   - 读取项目说明文档和目标文件
   - read_file 读取要修改的文件
   - write_file 写入修改
   - git_commit 提交
   - 按项目类型运行 build_project("android" / "rust" / "frontend") 或必要检查

=== Android 项目关键文件 ===
- settings.gradle           → 应用名称
- app/build.gradle          → 包名(applicationId)、SDK版本、依赖
- app/src/main/AndroidManifest.xml → 权限、Activity 声明
- app/src/main/kotlin/...   → Kotlin 代码（主要写这里）
- app/src/main/res/layout/  → XML 布局文件
- app/src/main/res/values/  → strings.xml, colors.xml

=== 规则 ===
- 如果用户需求不是“修改/生成/构建项目代码”，不要调用工具。请直接用简洁中文回复。
- 只有在确实需要读取、修改、提交或构建项目时，才调用 read_file/write_file/git_commit/build_project 等工具。
- 修改文件前必须先 read_file 读取原内容
- 每完成一个功能点就 git_commit（中文描述）
- git_commit 会自动提交，并在工作区配置 origin 时推送当前分支；未配置 origin 的本地模板项目只需本地提交，不要把它写成用户可见失败。如果当前目录是会话 worktree，不要手动推 main，服务器会在任务完成后串行合并。
- build_project("android") 会自动递增 versionCode 和 versionName，无需手动改版本号
- build_project 失败时分析错误，最多修复 3 次
- 回复用户用简洁中文，告知进度
- 编译成功后系统会自动生成下载链接给用户"#,
        workspace = workspace
    )
}
