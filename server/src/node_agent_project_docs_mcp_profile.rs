//! Profile metadata shared by the project-document MCP transport.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DescriptorProfile {
    Governance,
    Context,
    Feature,
    Receipt,
    WinControl,
}

impl DescriptorProfile {
    pub(crate) fn parse(value: Option<&str>) -> Result<Self> {
        match value.map(str::trim) {
            None | Some("") | Some("governance") => Ok(Self::Governance),
            Some("context") => Ok(Self::Context),
            Some("feature") => Ok(Self::Feature),
            Some("receipt") => Ok(Self::Receipt),
            Some("win_control") => Ok(Self::WinControl),
            Some(_) => bail!("profile 只支持 governance、context、feature、receipt 或 win_control"),
        }
    }

    pub(crate) fn query_name(self) -> Option<&'static str> {
        match self {
            Self::Governance => None,
            Self::Context => Some("context"),
            Self::Feature => Some("feature"),
            Self::Receipt => Some("receipt"),
            Self::WinControl => Some("win_control"),
        }
    }

    pub(crate) fn server_name(self) -> &'static str {
        match self {
            Self::Governance => "yilong_project_docs",
            Self::Context => "yilong_project_context",
            Self::Feature => "yilong_project_features",
            Self::Receipt => "yilong_project_receipt",
            Self::WinControl => "yilong_win_control",
        }
    }

    pub(crate) fn purpose(self) -> &'static str {
        match self {
            Self::Governance => "低 token 分析项目文档权威性、质量与联邦节点；支持项目 Git 工作区和平台托管版本的个人知识库。",
            Self::Context => "普通编码任务的单工具、零正文、revision 感知项目导航；真实搜索与读取继续使用代理原生工具。",
            Self::Feature => "普通编码任务的单工具功能需求生命周期；详细字段契约只在显式 describe 时按需返回。",
            Self::Receipt => "普通编码任务的单工具、只写候选回执；只保存摘要、主题和证据路径身份，不保存源码正文、聊天、prompt 或工具输出。",
            Self::WinControl => "项目绑定的 Win/Tauri 白名单语义控制与脱敏统一诊断时间线；不开放任意脚本、URL、command 或凭据。",
        }
    }

    pub(crate) fn supports_vault(self) -> bool {
        self == Self::Governance
    }

    pub(crate) fn is_governance(self) -> bool {
        self == Self::Governance
    }
}

#[cfg(test)]
mod tests {
    use super::DescriptorProfile;

    #[test]
    fn profiles_keep_read_write_and_governance_capabilities_separate() {
        assert_eq!(
            DescriptorProfile::parse(Some("context"))
                .unwrap()
                .server_name(),
            "yilong_project_context"
        );
        assert_eq!(
            DescriptorProfile::parse(Some("feature"))
                .unwrap()
                .server_name(),
            "yilong_project_features"
        );
        assert_eq!(
            DescriptorProfile::parse(Some("receipt"))
                .unwrap()
                .server_name(),
            "yilong_project_receipt"
        );
        assert!(!DescriptorProfile::Receipt.supports_vault());
        assert!(!DescriptorProfile::Feature.supports_vault());
        assert!(!DescriptorProfile::WinControl.supports_vault());
        assert_eq!(
            DescriptorProfile::parse(Some("win_control"))
                .unwrap()
                .server_name(),
            "yilong_win_control"
        );
        assert!(DescriptorProfile::Governance.supports_vault());
        assert!(DescriptorProfile::parse(Some("unknown")).is_err());
    }
}
