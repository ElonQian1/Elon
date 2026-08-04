use serde_json::{json, Value};

pub(crate) const CHAT_MEMORY_LOCAL_CLI_NOTE: &str = "=== PC 本机 CLI 使用规则 ===
普通聊天本身不能直接执行用户电脑命令，也不能直接读取 C 盘、D 盘或其它本机文件。
当用户询问本机目录、Windows 命令、cmd、PowerShell、文件读写、Win 端 CLI 或“为什么网页端不能访问我的电脑”时，不要只回答“我无法访问你的电脑”。
应明确告诉用户：在 PC 工作台里，AI 回复下方会出现“本机开发 CLI”快捷卡，用户可以点击“检测 Win 端”“使用默认目录”或由项目 owner/管理员确认“开启完整命令行”；账号绑定会在这些动作里自动使用当前网页账号完成，不需要单独步骤。
只有本机 Win 端节点绑定到当前网页账号、项目默认目录已准备，并且项目开发频道真实返回了本机工具执行结果后，才可以声称已经执行命令或读写文件。
如果当前对话还没有本机工具结果，只能引导用户完成授权流程，或说明需要到项目开发频道继续。";

pub(crate) fn append_system_prompt_note(messages: &mut Vec<Value>, note: &str) {
    let has_system = messages.first().and_then(|m| m["role"].as_str()) == Some("system");
    if has_system {
        if let Some(sys) = messages.first_mut() {
            let orig = sys["content"].as_str().unwrap_or("").to_string();
            sys["content"] = json!(format!("{orig}\n\n{note}"));
        }
    } else {
        messages.insert(0, json!({"role": "system", "content": note}));
    }
}
