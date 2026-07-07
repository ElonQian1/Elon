import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import { DownloadCloud, Handshake, KeyRound, ShieldCheck, Trash2, UploadCloud, XCircle } from 'lucide-react'
import { resolveApiUrl } from '../../api/runtime'
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
  const clean = (label || '').trim()
  if (!clean || clean === '全' || clean === '互') return '龙'
  return clean.slice(0, 1).toUpperCase()
}

interface SharingAlertLike {
  code?: string | null
  message?: string | null
  count?: number | null
  severity?: string | null
}

function sharingAlertCopy(alert: SharingAlertLike): { title: string; body: string; count: string } {
  const count = Number(alert.count ?? 0)
  if (alert.code === 'expired_uncleared_lease') {
    return {
      title: '共享会话已过期但未清理',
      body: '存在已过期的临时 Codex 会话，建议清理本机共享会话，避免继续占用或误用。',
      count: count > 0 ? `${count} 项` : '待处理',
    }
  }
  if (alert.code === 'missing_provider_vault') {
    return {
      title: '授权方还没有保存 Codex 账号',
      body: '对方需要先保存自己的 Codex 账号，授权关系才能被实际使用。',
      count: count > 0 ? `${count} 项` : '待处理',
    }
  }
  return {
    title: alert.severity === 'critical' ? '严重共享告警' : '共享告警',
    body: alert.message || '共享状态需要人工确认。',
    count: count > 0 ? `${count} 项` : '查看详情',
  }
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
  const usableIncomingCount = incomingGrants.filter((grant) => grant.status === 'active' && grant.provider_vault_available).length
  const activeOutgoingCount = outgoingGrants.filter((grant) => grant.status === 'active').length
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
          <h4>我的 Codex 账号保险箱</h4>
        </div>
        <span className={[styles.vaultState, stateTone].join(' ')}>{state}</span>
      </div>
      <p className={styles.vaultNote}>
        安全保存你的 Codex 登录态，并按授权临时共享给可信成员。这里同时展示账号状态、可用共享、授权关系和最近用量。
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
            <strong>{usableIncomingCount} 个可切换 · {activeOutgoingCount} 个正在共享</strong>
          </div>
          <span className={activeSharing ? styles.vaultOnline : sharingHealthTone}>
            {activeSharing ? `当前使用 ${robotLabel(activeSharedLease?.provider_nickname, activeSharedLease?.provider_account, activeSharedLease?.provider_user_id)}` : sharingHealthText}
          </span>
        </div>
        {sharingAlerts.length > 0 && (
          <div className={styles.vaultSlotList}>
            {sharingAlerts.slice(0, 3).map((alert) => {
              const copy = sharingAlertCopy(alert)
              return (
                <div key={alert.code ?? alert.message ?? 'sharing-alert'} className={[styles.vaultSlot, styles.vaultAlertSlot].join(' ')}>
                  <span>{copy.title}</span>
                  <strong>{copy.count}</strong>
                  <small>{copy.body}</small>
                </div>
              )
            })}
          </div>
        )}
        {activeSharing && (
          <p className={styles.shareHint}>
            当前正在使用 {robotLabel(activeSharedLease?.provider_nickname, activeSharedLease?.provider_account, activeSharedLease?.provider_user_id)} 共享的 Codex 账号；用量会按 shared_codex 记账，租约到期 {formatVaultTime(activeSharedLease?.expires_at)}。
          </p>
        )}
        <div className={styles.emergencyGrantForm}>
          <div className={styles.shareTargetPreview}>
            <strong>授权别人使用我的 Codex 账号</strong>
            <span>从项目成员、好友或全站用户中选择可信机器人</span>
          </div>
          <button
            className={[styles.btn, styles.iconBtn].join(' ')}
            type="button"
            onClick={() => setGrantPickerOpen(true)}
            disabled={busy}
            title="授权对方机器人使用本账号的 Codex 账号"
          >
            <Handshake size={15} strokeWidth={2.2} aria-hidden="true" />
            授权新成员
          </button>
        </div>
        <div className={styles.emergencyList}>
          <span className={styles.emergencyListTitle}>我可以使用的共享账号</span>
          {incomingGrants.length > 0 ? (
            <>
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
            </>
          ) : (
            <div className={styles.emergencyEmpty}>还没有别人共享给你的 Codex 账号。</div>
          )}
        </div>
        <div className={styles.emergencyList}>
          <span className={styles.emergencyListTitle}>我授权出去的成员</span>
          {outgoingGrants.length > 0 ? (
            <>
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
            </>
          ) : (
            <div className={styles.emergencyEmpty}>还没有对外授权。点击“授权新成员”开始共享。</div>
          )}
        </div>
        <div className={styles.emergencyList}>
          <span className={styles.emergencyListTitle}>最近共享用量</span>
          {leases.length > 0 ? (
            <>
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
            </>
          ) : (
            <div className={styles.emergencyEmpty}>暂无共享租约记录。</div>
          )}
        </div>
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
  const [imageFailed, setImageFailed] = useState(false)
  const directSrc = avatarDataUrl?.trim() || ''
  const fallbackSrc = userId ? resolveApiUrl('/api/users/' + encodeURIComponent(userId) + '/avatar') : ''
  const avatarSrc = imageFailed ? '' : (directSrc || fallbackSrc)

  useEffect(() => {
    setImageFailed(false)
  }, [directSrc, fallbackSrc])

  const content = avatarSrc ? (
    <img src={avatarSrc} alt="" onError={() => setImageFailed(true)} />
  ) : (
    <span className={styles.grantAvatarFallback}>{robotInitial(label)}</span>
  )
  const className = [styles.grantAvatarLink, avatarSrc ? styles.grantAvatarImage : styles.grantAvatarGenerated].join(' ')
  if (!userId) {
    return <span className={className} title={label}>{content}</span>
  }
  const profileTitle = '查看 ' + label + ' 的个人主页'
  return (
    <Link className={className} to={'/users/' + encodeURIComponent(userId)} title={profileTitle} aria-label={profileTitle}>
      {content}
    </Link>
  )
}
