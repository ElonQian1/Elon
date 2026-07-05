pub(crate) fn pc_project_execution_prompt(
    user_message: &str,
    preflight_note: Option<&str>,
    cli_name: &str,
    model_label: Option<&str>,
    prompt_bootstrapped: bool,
) -> String {
    let model_line = model_label
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("当前选择：{value}\n"))
        .unwrap_or_default();
    let preflight = preflight_note
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("预检提示：{value}\n\n"))
        .unwrap_or_default();

    if prompt_bootstrapped {
        return format!(
            "继续当前一龙项目执行会话。完整执行规则已经在此原生 CLI 会话前序注入；继续遵守项目入口文档、Git/验证/commit/push/发布脚本规则。\n\
当前 CLI：{}\n\
{model_line}\
本轮仍是具体执行任务，不是计划草稿。不要只回复“已准备好/请提供任务/后续会处理”；读规则或查看状态后必须继续完成用户请求里的第一条未完成 direct task。\n\
如果用户请求包含 Hard rules、Required direct tasks、marker、commit、push、发布或线上验证契约，最终回复前必须确认对应可验证结果已经产生；真实阻塞时说明证据。\n\n\
{preflight}\
用户请求：\n\
{user_message}",
            pc_cli_progress_label(cli_name)
        );
    }

    format!(
        "本轮用户请求如下。它已经是具体执行任务，不是等待你确认的草稿；你必须完成后才能最终回复：\n\
<<<USER_REQUEST\n\
{user_message}\n\
USER_REQUEST\n\
>>>\n\n\
你是「一龙」平台调度到用户本机节点的项目执行助手，当前 CLI 是 {}。\n\
{model_line}\
当前请求已经被判定为项目开发、Git、构建或发布执行任务。请直接在当前工作区完成用户请求；不要只做计划，不要在读完 AGENTS、README 或规则文档后停下来要求用户再次说明任务。\n\
必须先读取并遵守工作区入口规则；读完规则后回到下面“用户请求”逐项执行。\n\
如果用户请求包含 Hard rules、Required direct tasks、End your final reply with marker 或类似完成契约，必须把它们当作本轮任务契约。\n\
用户已经给了具体任务。禁止把“可以继续给我具体任务”“请提供具体任务”“我已准备好”或同义内容作为最终回复；如果你准备这样回复，说明你还没执行任务，必须继续执行用户请求里的第一条未完成 direct task。\n\
如果用户显式要求发布、运行 publish-server.ps1/publish-apk.ps1 或验证线上版本，该要求优先于 docs-only、CodePushed 即可收尾、默认不发布等项目默认规则；即使本轮只改了文档或测试标记，也必须执行显式发布步骤，除非真实命令失败且无法恢复。\n\
不要先回复“收到”“我会处理”“后续任务我会…”之类的自然语言确认；在非交互 exec 里，这会直接结束本轮任务。\n\
第一步必须使用工具读取入口规则或运行诊断命令；只有读取规则、查看状态或声明工作区干净不算完成，之后必须继续完成用户请求里的文件、Git、构建或发布动作。\n\
仓库规则若要求开始前给用户一句说明，只把它理解为执行过程中的简短进度说明；不能因此停止，且不能替代真实工具执行。\n\
最终回复前必须确认任务契约已经产生可验证结果；如果用户要求创建文件、commit、push、发布或 marker，缺一项都不能宣称完成。\n\
只有遇到真实阻塞（权限、缺少密钥、冲突无法判断、命令失败且无法恢复）才停下，并说明已经完成的步骤和阻塞证据。\n\n\
{preflight}\
用户请求：\n\
{user_message}",
        pc_cli_progress_label(cli_name)
    )
}

pub(crate) fn pc_project_passthrough_prompt(user_message: &str) -> String {
    user_message.to_string()
}

pub(crate) fn pc_lightweight_chat_prompt(
    user_message: &str,
    _cli_name: &str,
    _model_label: Option<&str>,
) -> String {
    user_message.to_string()
}

pub(crate) fn pc_cli_progress_label(cli_name: &str) -> &'static str {
    match cli_name {
        "codex" => "Codex",
        "copilot" => "Copilot",
        "claude" => "Claude",
        "gemini" => "Gemini",
        "api-runtime" => "Route B",
        "server-runtime" => "Route C",
        _ => "PC AI",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        pc_lightweight_chat_prompt, pc_project_execution_prompt, pc_project_passthrough_prompt,
    };

    #[test]
    fn pc_lightweight_chat_prompt_is_raw_passthrough() {
        let prompt = pc_lightweight_chat_prompt("我有一个想法", "codex", Some("Codex"));

        assert_eq!(prompt, "我有一个想法");
        assert!(!prompt.contains(".github/copilot-instructions.md"));
        assert!(!prompt.contains("强制改代码"));
        assert!(!prompt.contains("必须完成后才能最终回复"));
        assert!(!prompt.contains("当前请求已经被判定为项目开发"));
        assert!(!prompt.contains("第一步必须使用工具"));
        assert!(!prompt.contains("缺一项都不能宣称完成"));
    }

    #[test]
    fn pc_project_passthrough_prompt_is_raw_user_message() {
        let prompt = pc_project_passthrough_prompt("你是 codex 吗？");

        assert_eq!(prompt, "你是 codex 吗？");
        assert!(!prompt.contains(".github/copilot-instructions.md"));
        assert!(!prompt.contains("强制改代码"));
        assert!(!prompt.contains("必须完成后才能最终回复"));
        assert!(!prompt.contains("当前请求已经被判定为项目开发"));
        assert!(!prompt.contains("第一步必须使用工具"));
        assert!(!prompt.contains("缺一项都不能宣称完成"));
    }

    #[test]
    fn pc_project_execution_prompt_requires_continuing_after_rules() {
        let prompt = pc_project_execution_prompt(
            "Create docs/e2e.md, commit, push, and end with marker mcp_git.",
            Some("workspace ok"),
            "codex",
            Some("Codex"),
            false,
        );

        assert!(prompt.contains("项目执行助手"));
        assert!(prompt.contains("不要只做计划"));
        assert!(prompt.contains("不要在读完 AGENTS"));
        assert!(prompt.contains("读完规则后回到下面"));
        assert!(prompt.contains("已经是具体执行任务"));
        assert!(prompt.contains("用户已经给了具体任务"));
        assert!(prompt.contains("可以继续给我具体任务"));
        assert!(prompt.contains("第一条未完成 direct task"));
        assert!(prompt.contains("显式要求发布"));
        assert!(prompt.contains("优先于 docs-only"));
        assert!(prompt.contains("不要先回复"));
        assert!(prompt.contains("第一步必须使用工具"));
        assert!(prompt.contains("查看状态或声明工作区干净不算完成"));
        assert!(prompt.contains("不能替代真实工具执行"));
        assert!(prompt.contains("缺一项都不能宣称完成"));
        assert!(prompt.contains("Hard rules"));
        assert!(prompt.contains("预检提示：workspace ok"));
        assert!(prompt.contains("Create docs/e2e.md"));
    }

    #[test]
    fn pc_project_execution_prompt_shortens_after_bootstrap() {
        let prompt = pc_project_execution_prompt(
            "继续修复 npm.ps1 问题并提交。",
            Some("workspace ok"),
            "codex",
            Some("Codex"),
            true,
        );

        assert!(prompt.contains("完整执行规则已经在此原生 CLI 会话前序注入"));
        assert!(prompt.contains("继续修复 npm.ps1"));
        assert!(prompt.contains("预检提示：workspace ok"));
        assert!(prompt.contains("第一条未完成 direct task"));
        assert!(!prompt.contains("本轮用户请求如下。它已经是具体执行任务"));
        assert!(!prompt.contains("如果用户显式要求发布、运行 publish-server.ps1/publish-apk.ps1"));
        assert!(prompt.chars().count() < 450);
    }
}
