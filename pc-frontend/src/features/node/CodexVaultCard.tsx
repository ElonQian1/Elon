import { useState } from 'react'
import { Link } from 'react-router-dom'
import { DownloadCloud, Handshake, KeyRound, ShieldCheck, Trash2, UploadCloud, XCircle } from 'lucide-react'
import type { CodexVaultEmergencyGrant, CodexVaultLocalStatus, CodexVaultStatusResponse } from './types'
import type { CodexVaultEmergencyActions } from './codexVaultEmergencyActions'
import UserPickerDrawer, { type UserPickerUser } from '../users/UserPickerDrawer'
import styles from './NodePage.module.css'

function formatVaultTime(value?: string | null): string {
  if (!value) return '无'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleString()
}

function isFutureTime(value?: string | null): boolean {
  if (!value) return true
  const time = new Date(value).getTime()
  return Number.isFinite(time) && time > Date.now()
}

function authStateText(auth?: CodexVaultLocalStatus['default_auth']): string {
  if (!auth?.present) return '未找到'
  if (auth.problem) return '读取异常'
  if (auth.auth_mode && auth.auth_mode !== 'chatgpt') return auth.auth_mode
  return auth.has_refresh_token ? 'ChatGPT / Pro' : '缺少 refresh_token'
}

function robotLabel(nickname?: string | null, account?: string | null, userId?: string | null): string {
  return nickname || account || userId || '机器人账号'
}

function robotInitial(label?: string | null): string {
  return (label || '龙').trim().slice(0, 1).toUpperCase() || '龙'
}

function formatFen(value?: number | null): string {
  const fen = Number(value ?? 0)
  if (!Number.isFinite(fen) || fen <= 0) return '0.00 元'
  return `${(fen / 100).toFixed(2)} 元`
}

export default function CodexVaultCard({
  status,
  cloud,
  busy,
  onBackup,
  onRestore,
  onClear,
  onDeleteCloud,
  emergencyActions,
  onRefresh,
  currentUserId,
}: {
  status: CodexVaultLocalStatus | null
  cloud?: CodexVaultStatusResponse['cloud']
  busy: boolean
  onBackup: () => void
  onRestore: () => void
  onClear: () => void
  onDeleteCloud: () => void
  emergencyActions: CodexVaultEmergencyActions
  onRefresh: () => void
  currentUserId?: string
}) {
  const [grantPickerOpen, setGrantPickerOpen] = useState(false)
  const vault = cloud?.vault
  const sharing = cloud?.sharing ?? cloud?.emergency
  const cloudSlots = vault?.slots ?? []
  const localSlots = status?.managed_slots ?? []
  const grants = sharing?.grants ?? []
  const leases = sharing?.leases ?? []
  const sharingHealth = sharing?.health
  const sharingAlerts = sharingHealth?.alerts ?? []
  const incomingGrants = grants.filter((grant) => grant.consumer_user_id === currentUserId)
  const outgoingGrants = grants.filter((grant) => grant.provider_user_id === currentUserId)
  const cloudReady = !!vault?.configured
  const bound = !!vault?.bound
  const defaultAuth = status?.default_auth
  const managedAuth = status?.managed_auth
  const activeManaged = !!status?.active_home_managed
  const canBackup = !!defaultAuth?.present
    && defaultAuth.auth_mode !== 'api_key'
    && !!defaultAuth.has_refresh_token
  const canRestore = cloudReady && bound
  const canClear = activeManaged || !!managedAuth?.present
  const state = cloud?.error
    ? '云端不可用'
    : !cloud
      ? '读取中'
      : !cloudReady
        ? '服务器未配置'
        : bound
          ? `已保存 ${vault?.available_count ?? (cloudSlots.length || 1)} 个账号`
          : '未保存'
  const stateTone = cloudReady && bound ? styles.vaultOnline : cloud?.error ? styles.vaultOffline : styles.vaultChecking
  const activeSharedLease = leases.find((lease) => lease.consumer_user_id === currentUserId && lease.status === 'active' && isFutureTime(lease.expires_at))
  const activeSharing = !!activeSharedLease
  const sharingHealthText = sharingHealth?.status === 'critical'
    ? `严重 · ${sharingHealth.alert_count ?? sharingAlerts.length} 项`
    : sharingHealth?.status === 'warning'
      ? `告警 · ${sharingHealth.alert_count ?? sharingAlerts.length} 项`
      : `正常 · ${sharingHealth?.active_lease_count ?? 0} 个活动租约`
  const sharingHealthTone = sharingHealth?.status === 'critical'
    ? styles.vaultOffline
    : sharingHealth?.status === 'warning'
      ? styles.vaultChecking
      : styles.vaultOnline
  async function grantSelectedUsers(users: UserPickerUser[]) {
    for (const user of users) {
      await emergencyActions.onCreateEmergencyGrant(user.id)
    }
    setGrantPickerOpen(false)
  }
  function grantStatusText(grant: CodexVaultEmergencyGrant): string {
    if (grant.status !== 'active') return '已撤销'
    if (!grant.provider_vault_available) return '账号未保存'
    return grant.reciprocal_active ? '互授权' : '单向授权'
  }
  return (
    <section className={styles.vaultCard}>
      <div className={styles.vaultHead}>
        <div>
          <span className={styles.codexLabel}>Codex 账号保险箱</span>
          <h4>保存并分享自己的 Codex 账号</h4>
        </div>
        <span className={[styles.vaultState, stateTone].join(' ')}>{state}</span>
      </div>
      <p className={styles.vaultNote}>
        Codex 账号默认只给账号所有者自己的节点保存和切换；你显式授权后，其他机器人才能使用共享 Codex 账号，页面会记录共享租约、token 和收益。
      </p>
      <div className={styles.vaultGrid}>
        <div>
          <span>本机 Codex 账号</span>
          <strong>{authStateText(defaultAuth)}</strong>
        </div>
        <div>
          <span>本机共享会话</span>
          <strong>{activeManaged ? `当前生效${status?.active_account_hint_hash ? ` · ${status.active_account_hint_hash}` : ''}` : managedAuth?.present ? '已写入' : '未写入'}</strong>
        </div>
        <div>
          <span>最近保存</span>
          <strong>{formatVaultTime(vault?.last_backup_at)}</strong>
        </div>
        <div>
          <span>最近恢复</span>
          <strong>{formatVaultTime(vault?.last_lease_at)}</strong>
        </div>
      </div>
      {cloud?.error && <p className={styles.codexFixHint}>{cloud.error}</p>}
      {defaultAuth?.problem && <p className={styles.codexFixHint}>{defaultAuth.problem}</p>}
      {cloudSlots.length > 0 && (
        <div className={styles.vaultSlotList}>
          {cloudSlots.map((slot) => (
            <div key={slot.slot_id ?? slot.account_hint_hash ?? 'slot'} className={styles.vaultSlot}>
              <span>{slot.account_hint_hash ?? slot.slot_id ?? 'Codex 账号'}</span>
              <strong>{slot.status === 'degraded' ? '备用受限' : '可用'} · v{slot.credential_version ?? 1}</strong>
              {slot.last_error && <small>{slot.last_error}</small>}
            </div>
          ))}
        </div>
      )}
      {localSlots.length > 0 && (
        <div className={styles.vaultSlotList}>
          {localSlots.map((slot) => (
            <div key={slot.slot_id ?? slot.home ?? 'local'} className={styles.vaultSlot}>
              <span>{slot.active ? '当前本机槽位' : '本机备用槽位'}</span>
              <strong>
                {slot.account_hint_hash ?? slot.slot_id ?? '未知账号'}
              </strong>
            </div>
          ))}
        </div>
      )}
      <div className={styles.emergencyPanel}>
        <div className={styles.emergencyHead}>
          <div>
            <span>机器人授权共享</span>
            <strong>{incomingGrants.length} 个可使用 · {outgoingGrants.length} 个已共享</strong>
          </div>
          <span className={activeSharing ? styles.vaultOnline : sharingHealthTone}>
            {activeSharing ? `当前使用 ${robotLabel(activeSharedLease?.provider_nickname, activeSharedLease?.provider_account, activeSharedLease?.provider_user_id)}` : sharingHealthText}
          </span>
        </div>
        {sharingAlerts.length > 0 && (
          <div className={styles.vaultSlotList}>
            {sharingAlerts.slice(0, 3).map((alert) => (
              <div key={alert.code ?? alert.message ?? 'sharing-alert'} className={styles.vaultSlot}>
                <span>{alert.severity === 'critical' ? '严重告警' : '共享告警'} · {alert.code}</span>
                <strong>{alert.count ?? 0} 项</strong>
                <small>{alert.message}</small>
              </div>
            ))}
          </div>
        )}
        {activeSharing && (
          <p className={styles.shareHint}>
            当前 Codex CLI 用量按 shared_codex 记账，资源提供方为 {robotLabel(activeSharedLease?.provider_nickname, activeSharedLease?.provider_account, activeSharedLease?.provider_user_id)}，租约到期 {formatVaultTime(activeSharedLease?.expires_at)}。
          </p>
        )}
        <div className={styles.emergencyGrantForm}>
          <div className={styles.shareTargetPreview}>
            <strong>选择授权对象</strong>
            <span>从项目成员、好友或全站用户中勾选机器人账号</span>
          </div>
          <button
            className={[styles.btn, styles.iconBtn].join(' ')}
            type="button"
            onClick={() => setGrantPickerOpen(true)}
            disabled={busy}
            title="授权对方机器人使用本账号的 Codex 账号"
          >
            <Handshake size={15} strokeWidth={2.2} aria-hidden="true" />
            选择并授权
          </button>
        </div>
        {incomingGrants.length > 0 && (
          <div className={styles.emergencyList}>
            <span className={styles.emergencyListTitle}>别人共享给我</span>
            {incomingGrants.map((grant) => (
              <div className={styles.emergencyRow} key={grant.id ?? `${grant.provider_user_id}-in`}>
                <GrantAvatarLink
                  userId={grant.provider_user_id}
                  label={robotLabel(grant.provider_nickname, grant.provider_account, grant.provider_user_id)}
                  avatarDataUrl={grant.provider_avatar_data_url}
                />
                <div>
                  <strong>{grantStatusText(grant)}</strong>
                  <span>到期 {formatVaultTime(grant.expires_at)} · {robotLabel(grant.provider_nickname, grant.provider_account, grant.provider_user_id)}</span>
                </div>
                <button
                  className={[styles.btn, styles.iconBtn].join(' ')}
                  type="button"
                  disabled={busy || grant.status !== 'active' || !grant.provider_vault_available || !grant.provider_user_id}
                  onClick={() => grant.provider_user_id && void emergencyActions.onEmergencyRestore(grant.provider_user_id)}
                  title="切换到该授权机器人的共享 Codex 账号"
                >
                  <KeyRound size={15} strokeWidth={2.2} aria-hidden="true" />
                  使用共享
                </button>
              </div>
            ))}
          </div>
        )}
        {outgoingGrants.length > 0 && (
          <div className={styles.emergencyList}>
            <span className={styles.emergencyListTitle}>我共享出去</span>
            {outgoingGrants.map((grant) => (
              <div className={styles.emergencyRow} key={grant.id ?? `${grant.consumer_user_id}-out`}>
                <GrantAvatarLink
                  userId={grant.consumer_user_id}
                  label={robotLabel(grant.consumer_nickname, grant.consumer_account, grant.consumer_user_id)}
                  avatarDataUrl={grant.consumer_avatar_data_url}
                />
                <div>
                  <strong>{grantStatusText(grant)}</strong>
                  <span>到期 {formatVaultTime(grant.expires_at)} · {robotLabel(grant.consumer_nickname, grant.consumer_account, grant.consumer_user_id)}</span>
                </div>
                <button
                  className={[styles.btn, styles.iconBtn].join(' ')}
                  type="button"
                  disabled={busy || grant.status !== 'active' || !grant.id}
                  onClick={() => grant.id && void emergencyActions.onRevokeEmergencyGrant(grant.id)}
                  title="撤销这个机器人账号的共享使用权限"
                >
                  <XCircle size={15} strokeWidth={2.2} aria-hidden="true" />
                  撤销
                </button>
              </div>
            ))}
          </div>
        )}
        {leases.length > 0 && (
          <div className={styles.emergencyList}>
            <span className={styles.emergencyListTitle}>最近业务往来</span>
            {leases.slice(0, 4).map((lease) => {
              const usingIncoming = lease.consumer_user_id === currentUserId
              const counterparty = usingIncoming
                ? robotLabel(lease.provider_nickname, lease.provider_account, lease.provider_user_id)
                : robotLabel(lease.consumer_nickname, lease.consumer_account, lease.consumer_user_id)
              const counterpartyId = usingIncoming ? lease.provider_user_id : lease.consumer_user_id
              const counterpartyAvatar = usingIncoming ? lease.provider_avatar_data_url : lease.consumer_avatar_data_url
              const direction = usingIncoming ? '我使用' : '对方使用'
              return (
                <div className={styles.emergencyRow} key={lease.id ?? lease.leased_at}>
                  <GrantAvatarLink userId={counterpartyId} label={counterparty} avatarDataUrl={counterpartyAvatar} />
                  <div>
                    <strong>{direction}</strong>
                    <span>{counterparty} · {lease.total_tokens ?? 0} tokens · 扣费 {formatFen(lease.billed_cost_rmb_fen)} · {formatVaultTime(lease.leased_at)}</span>
                  </div>
                  <small>{lease.accounting_status ?? lease.status ?? 'active'}</small>
                </div>
              )
            })}
          </div>
        )}
      </div>
      {status?.managed_home && <code className={styles.codexPath}>{status.managed_home}</code>}
      <div className={styles.vaultActions}>
        <button
          className={[styles.btn, styles.primary, styles.iconBtn].join(' ')}
          onClick={onBackup}
          disabled={busy || !canBackup}
          title="把这台电脑的 Codex 账号加密保存到云端账号保险箱"
        >
          <UploadCloud size={15} strokeWidth={2.2} aria-hidden="true" />
          保存本机账号
        </button>
        <button
          className={[styles.btn, styles.iconBtn].join(' ')}
          onClick={onRestore}
          disabled={busy || !canRestore}
          title="把云端账号保险箱切换为本机临时 Codex 会话"
        >
          <DownloadCloud size={15} strokeWidth={2.2} aria-hidden="true" />
          临时切换
        </button>
        <button
          className={[styles.btn, styles.iconBtn].join(' ')}
          onClick={onClear}
          disabled={busy || !canClear}
          title="删除本机共享 Codex 会话，不影响默认账号"
        >
          <Trash2 size={15} strokeWidth={2.2} aria-hidden="true" />
          清理本机
        </button>
        <button
          className={[styles.btn, styles.iconBtn].join(' ')}
          onClick={onDeleteCloud}
          disabled={busy || !bound}
          title="删除云端账号保险箱中的 Codex 账号记录，不影响本机登录"
        >
          <Trash2 size={15} strokeWidth={2.2} aria-hidden="true" />
          删除云端
        </button>
        <button
          className={[styles.btn, styles.iconBtn].join(' ')}
          onClick={onRefresh}
          disabled={busy}
          title="刷新 Codex 账号保险箱状态"
        >
          <ShieldCheck size={15} strokeWidth={2.2} aria-hidden="true" />
          刷新
        </button>
      </div>
      <UserPickerDrawer
        open={grantPickerOpen}
        title="机器人授权共享"
        subtitle="选择允许使用本账号 Codex 共享能力的成员、好友或全站用户。"
        busy={busy}
        currentUserId={currentUserId}
        disabledUserIds={new Set(outgoingGrants.filter((grant) => grant.status === 'active').map((grant) => grant.consumer_user_id ?? '').filter(Boolean))}
        onClose={() => setGrantPickerOpen(false)}
        onConfirm={grantSelectedUsers}
      />
    </section>
  )
}


function GrantAvatarLink({
  userId,
  label,
  avatarDataUrl,
}: {
  userId?: string | null
  label: string
  avatarDataUrl?: string | null
}) {
  const content = avatarDataUrl ? (
    <img src={avatarDataUrl} alt="" />
  ) : (
    <span className={styles.grantAvatarFallback}>{robotInitial(label)}</span>
  )
  const className = styles.grantAvatarLink
  if (!userId) {
    return <span className={className} title={label}>{content}</span>
  }
  return (
    <Link className={className} to={'/users/' + encodeURIComponent(userId)} title={label} aria-label={'查看 ' + label + ' 的主页'}>
      {content}
    </Link>
  )
}
