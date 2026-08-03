import type { ConsumerCandidateScope } from './openCommerceClientTypes'
import { badgeStyle, commerceStyles } from './openCommerceStyles'

export default function ConsumerCandidateScopeSummary({ scope }: { scope: ConsumerCandidateScope }) {
  return (
    <div style={commerceStyles.itemHeader}>
      <small style={commerceStyles.itemMeta}>
        目录候选 {scope.directory_candidate_count}/{scope.candidate_cap} · 合格 {scope.eligible_match_count} · 返回 {scope.returned_match_count} · 当前运营方目录，非全网穷尽
      </small>
      <span style={badgeStyle(scope.results_truncated ? 'warn' : 'neutral')} data-tone={scope.results_truncated ? 'warn' : 'neutral'}>
        {scope.results_truncated ? '结果已截断' : '本次合格结果已返回'}
      </span>
    </div>
  )
}
