use super::super::terminal_descriptor::TypedTerminalDescriptorV1;
use super::{LeafRecordV1, LeafSealV1};

/// A full static leaf observed during the same traversal that produces its verified compact seal.
pub(crate) enum StreamedLeafV1<'leaf> {
    Terminal {
        record: &'leaf LeafRecordV1,
        descriptor: &'leaf TypedTerminalDescriptorV1,
        seal: &'leaf LeafSealV1,
    },
    Excluded {
        record: &'leaf LeafRecordV1,
        seal: &'leaf LeafSealV1,
    },
}

impl StreamedLeafV1<'_> {
    pub(crate) const fn seal(&self) -> &LeafSealV1 {
        match self {
            Self::Terminal { seal, .. } | Self::Excluded { seal, .. } => seal,
        }
    }
}
