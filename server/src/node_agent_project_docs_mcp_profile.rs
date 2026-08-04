//! Profile metadata shared by the project-document MCP transport.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DescriptorProfile {
    Governance,
    Context,
    Receipt,
}

impl DescriptorProfile {
    pub(crate) fn parse(value: Option<&str>) -> Result<Self> {
        match value.map(str::trim) {
            None | Some("") | Some("governance") => Ok(Self::Governance),
            Some("context") => Ok(Self::Context),
            Some("receipt") => Ok(Self::Receipt),
            Some(_) => bail!("profile 只支持 governance、context 或 receipt"),
        }
    }

    pub(crate) fn query_name(self) -> Option<&'static str> {
        match self {
            Self::Governance => None,
            Self::Context => Some("context"),
            Self::Receipt => Some("receipt"),
        }
    }

    pub(crate) fn server_name(self) -> &'static str {
        match self {
            Self::Governance => "yilong_project_docs",
            Self::Context => "yilong_project_context",
            Self::Receipt => "yilong_project_receipt",
        }
    }

    pub(crate) fn purpose(self) -> &'static str {
        match self {
            Self::Governance => "低 token 分析项目文档权威性、质量与联邦节点；支持项目 Git 工作区和平台托管版本的个人知识库。",
            Self::Context => "普通编码任务的单工具、零正文、revision 感知项目导航；真实搜索与读取继续使用代理原生工具。",
            Self::Receipt => "普通编码任务的单工具、只写候选回执；只保存摘要、主题和证据路径身份，不保存源码正文、聊天、prompt 或工具输出。",
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
            DescriptorProfile::parse(Some("receipt"))
                .unwrap()
                .server_name(),
            "yilong_project_receipt"
        );
        assert!(!DescriptorProfile::Receipt.supports_vault());
        assert!(DescriptorProfile::Governance.supports_vault());
        assert!(DescriptorProfile::parse(Some("unknown")).is_err());
    }
}
