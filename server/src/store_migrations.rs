use anyhow::Result;
use rusqlite::Connection;

#[cfg(test)]
mod migration_tests;
mod migrations_v17_v34;
mod migrations_v1_v16;
mod migrations_v35_v53;
mod migrations_v54_v70;
mod migrations_v71_v82;

use migrations_v17_v34::*;
use migrations_v1_v16::*;
use migrations_v35_v53::*;
use migrations_v54_v70::*;
use migrations_v71_v82::*;

#[rustfmt::skip]
pub(crate) static MIGRATIONS: &[(u32, &str, fn(&Connection) -> Result<()>)] = &[
    (1, "初始全量表结构（幂等）", migration_v1),
    (2, "补充缺失列与辅助索引（幂等）", migration_v2),
    (3, "将所有现有项目设为公开可见（一次性）", migration_v3),
    (4, "好友会话已读状态与未读提醒", migration_v4),
    (5, "用户头像数据（个人资料上传）", migration_v5),
    (6, "好友群聊基础表与未读状态", migration_v6),
    (7, "项目空间频道与共享频道消息", migration_v7),
    (8, "好友与群聊消息附件引用", migration_v8),
    (9, "同一用户禁止重名活跃项目", migration_v9),
    (
        10,
        "tasks.codex_thread_id + conversation_timeline 视图",
        migration_v10,
    ),
    (
        11,
        "projects 构建缓存（last_build_sha / last_build_apk_url）",
        migration_v11,
    ),
    (12, "好友聊天 EL 助手上下文消息", migration_v12),
    (13, "每日编译配额表（build_quota）", migration_v13),
    (14, "项目意见频道建议状态", migration_v14),
    (15, "用户长期记忆表", migration_v15),
    (16, "token 用量事件表 + 用户 token 配额表", migration_v16),
    (
        17,
        "人民币预存计费：用户余额、充值记录、扣费明细、计费配置",
        migration_v17,
    ),
    (18, "微信支付订单表", migration_v18),
    (19, "项目加入申请表（approval 审批流程）", migration_v19),
    (20, "PC 本地项目绑定节点 ID", migration_v20),
    (21, "分布式节点积分账本与节点凭证表", migration_v21),
    (
        22,
        "conversations.locked_agent_name 会话首次 CLI 锁定",
        migration_v22,
    ),
    (
        23,
        "node_credentials.device_name PC 设备展示名",
        migration_v23,
    ),
    (24, "user_memories 记忆作用域", migration_v24),
    (25, "收紧一龙自项目默认成员与加入权限", migration_v25),
    (26, "指定钱一龙账号为一龙自项目管理员", migration_v26),
    (27, "项目成员会话人类讨论消息", migration_v27),
    (28, "项目成员个人会话公开状态", migration_v28),
    (29, "PC 项目执行会话与工作区状态", migration_v29),
    (30, "token 用量与扣费事件原子对账字段", migration_v30),
    (31, "token 用量可信记账幂等键", migration_v31),
    (32, "PC 项目执行会话 token 用量字段", migration_v32),
    (33, "计费调用预授权冻结与对账摘要", migration_v33),
    (34, "非 CLI 算力预授权配置", migration_v34),
    (35, "PC 项目工作区健康快照", migration_v35),
    (36, "算力多单位计量明细账本", migration_v36),
    (37, "模型与算力计价规则配置表", migration_v37),
    (38, "计费自动对账告警表", migration_v38),
    (39, "扣费计价规则版本与价格快照", migration_v39),
    (40, "节点收益流水绑定真实扣费事件", migration_v40),
    (41, "PC 节点硬件画像快照", migration_v41),
    (42, "节点收益提现申请表", migration_v42),
    (43, "节点收益整数资金账本", migration_v43),
    (44, "节点算力执行证明与质量评分基础表", migration_v44),
    (45, "项目 APK 图标数据", migration_v45),
    (46, "project channel message reply parent", migration_v46),
    (47, "指定钱一龙为一龙自项目创建者与 owner", migration_v47),
    (48, "PC 硬盘节点项目仓库绑定", migration_v48),
    (49, "PC 硬盘节点 owner checkout 路径", migration_v49),
    (50, "项目展示别名", migration_v50),
    (51, "一龙自项目公开展示并审批加入", migration_v51),
    (52, "项目级 AI 运行权限授权", migration_v52),
    (53, "群聊 AI 文档、Context Pack 与总结帖", migration_v53),
    (54, "外部应用账号、默认群映射与授权码", migration_v54),
    (55, "项目代码身份去重索引", migration_v55),
    (56, "所有用户默认加入指定联合开发项目", migration_v56),
    (57, "fb2 外部应用 AI 回复试用额度配置", migration_v57),
    (58, "项目首页 landing manifest 云端快照", migration_v58),
    (59, "项目首页上传凭证", migration_v59),
    (60, "外部应用工具执行审计", migration_v60),
    (61, "普通新用户 AI 试用额度配置", migration_v61),
    (62, "停止默认加入联合项目并清理旧成员关系", migration_v62),
    (63, "项目开发命令自动识别元数据", migration_v63),
    (64, "Route C 服务器模型每日预算调用审计", migration_v64),
    (65, "Route C 服务器模型用户日预算索引", migration_v65),
    (66, "Route C 服务器模型调用完成态审计", migration_v66),
    (67, "BB64A external app AI reply trial credit config", migration_v67),
    (68, "项目级 AI danger_full_access 权限", migration_v68),
    (69, "项目成员管理审计日志", migration_v69),
    (70, "项目成员禁言与封禁状态", migration_v70),
    (71, "项目自定义角色与成员权限矩阵", migration_v71),
    (72, "项目成员多角色绑定", migration_v72),
    (73, "项目频道角色权限覆盖", migration_v73),
    (74, "项目频道成员权限覆盖", migration_v74),
    (75, "项目频道分类与分类权限继承", migration_v75),
    (76, "用户展示在线状态与项目邀请链接", migration_v76),
    (77, "项目空间商店截图列表", migration_v77),
    (78, "群体 AI 开发 Matter 与节点授权骨架", migration_v78),
    (79, "群体 AI 产物上传与人工合并队列", migration_v79),
    (80, "项目 PC 节点级工作区绑定", migration_v80),
    (81, "用户 Codex Pro 凭据保险箱", migration_v81),
    (82, "项目成员昵称与管理员备注", migration_v82),
    (83, "releases", crate::project_release_migration::migration_v83),
    (84, "新用户默认免费额度提升到 30000 分", crate::billing_trial_credit_migration::migration_v84),
    (85, "token 用量资源来源与自有 Codex 免扣费标记", crate::billing_usage_source_migration::migration_v85),
    (86, "PC 节点安装实例幂等注册", crate::node_install_id_migration::migration_v86), (87, "用户子项目 APK release 元数据与项目首页同步", crate::project_release_migration::migration_v87),
    (88, "Codex auth.json 保险箱多账号槽位", crate::codex_vault_slot_migration::migration_v88),
];

// ── 内部工具 ───────────────────────────────────────────────────────────────────

pub(crate) fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<()> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
    let exists = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .any(|name| name == column);
    if !exists {
        conn.execute(
            &format!("ALTER TABLE {} ADD COLUMN {}", table, definition),
            [],
        )?;
    }
    Ok(())
}
