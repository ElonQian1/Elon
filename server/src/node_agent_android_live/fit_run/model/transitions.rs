use super::FitRunPhase;

pub(super) fn legal_transition(from: FitRunPhase, to: FitRunPhase) -> bool {
    use FitRunPhase::*;
    match from {
        Created => matches!(to, Baselining | Paused | Failed | Cancelled),
        Baselining => matches!(to, LocalSolving | Paused | Failed | Cancelled),
        LocalSolving => matches!(
            to,
            CandidateReady | AwaitingCodex | Plateau | Paused | Failed | Cancelled
        ),
        AwaitingCodex => matches!(to, CodexRunning | Paused | Plateau | Cancelled),
        CodexRunning => matches!(to, Rebuilding | AwaitingCodex | Paused | Failed | Cancelled),
        Rebuilding => matches!(to, Evaluating | Paused | Failed | Cancelled),
        Evaluating => matches!(
            to,
            LocalSolving | CandidateReady | AwaitingCodex | Plateau | Paused | Failed | Cancelled
        ),
        CandidateReady => matches!(to, SourceVerifying | LocalSolving | Paused | Cancelled),
        SourceVerifying => matches!(
            to,
            Accepted | LocalSolving | AwaitingCodex | Paused | Failed | Cancelled
        ),
        Paused => matches!(to, Cancelled),
        Accepted | Plateau | Failed | Cancelled => false,
    }
}
