import { X } from 'lucide-react'
import type { AuthorizationRequest } from './openCommerceClientTypes'
import base from './OpenCommercePanel.module.css'
import { actionStyle, badgeStyle, commerceStyles, listItemStyle } from './openCommerceStyles'

interface Props {
  requests: AuthorizationRequest[]
  canEdit: boolean
  busy: boolean
  onCancel: (request: AuthorizationRequest) => Promise<void>
}

export default function OutboundAuthorizationRequests({ requests, canEdit, busy, onCancel }: Props) {
  return (
    <section className={base.integrationSection}>
      <header>
        <strong>我发出的授权申请</strong>
        <span style={badgeStyle('warn')}>{requests.filter((item) => item.status === 'pending').length} 待处理</span>
      </header>
      <div className={base.formCard} style={{ ...commerceStyles.sectionBody, ...commerceStyles.scrollArea }}>
        {requests.map((request) => (
          <article className={base.formCard} style={listItemStyle()} key={request.id}>
            <header style={commerceStyles.itemHeader}>
              <h3 style={commerceStyles.itemTitle}>{request.requester_app_id}</h3>
              <span style={badgeStyle(request.status === 'pending' ? 'warn' : 'neutral')}>{statusLabel(request.status)}</span>
            </header>
            <p style={commerceStyles.itemText}>{request.purpose}</p>
            <code style={commerceStyles.itemMeta}>{request.scopes.join(', ')}</code>
            <footer style={commerceStyles.itemHeader}>
              <small style={commerceStyles.itemMeta}>商户 {request.merchant_id}</small>
              {request.status === 'pending' && (
                <button
                  style={actionStyle('danger', !canEdit || busy)}
                  type="button"
                  onClick={() => onCancel(request)}
                  disabled={!canEdit || busy}
                >
                  <X size={13} />撤回申请
                </button>
              )}
            </footer>
          </article>
        ))}
        {requests.length === 0 && <p className={base.empty}>当前项目还没有向商户发出授权申请。</p>}
      </div>
    </section>
  )
}

function statusLabel(status: AuthorizationRequest['status']) {
  return {
    pending: '待商户处理',
    approved: '已批准',
    rejected: '已拒绝',
    canceled: '已撤回',
  }[status]
}
