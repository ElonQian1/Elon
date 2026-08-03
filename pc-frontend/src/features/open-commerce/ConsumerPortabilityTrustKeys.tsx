import { useCallback, useEffect, useState } from 'react'
import { KeyRound, RefreshCw, ShieldOff } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import type { ConsumerPortabilityTrustKey } from './openCommerceClientTypes'
import { errorText } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import { actionStyle, badgeStyle, commerceStyles, listItemStyle } from './openCommerceStyles'

export default function ConsumerPortabilityTrustKeys({ projectId }: { projectId: string }) {
  const [keys, setKeys] = useState<ConsumerPortabilityTrustKey[]>([])
  const [sourceOperator, setSourceOperator] = useState('')
  const [publicKeyPem, setPublicKeyPem] = useState('')
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    try {
      const response = await openCommerceClientApi.listConsumerPortabilityTrustKeys(projectId)
      setKeys(response.keys)
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [projectId])

  useEffect(() => {
    refresh()
  }, [refresh])

  async function addKey() {
    if (!sourceOperator.trim() || !publicKeyPem.trim()) {
      setMessage('请填写来源运营方和 RSA 公钥。')
      return
    }
    setBusy(true)
    setMessage('')
    try {
      await openCommerceClientApi.createConsumerPortabilityTrustKey(
        projectId,
        sourceOperator.trim(),
        publicKeyPem.trim(),
      )
      setPublicKeyPem('')
      setMessage('运营方公钥已加入当前用户的信任列表。')
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function revoke(key: ConsumerPortabilityTrustKey) {
    if (!window.confirm(`撤销“${key.source_operator}”的该公钥？`)) return
    setBusy(true)
    setMessage('')
    try {
      await openCommerceClientApi.revokeConsumerPortabilityTrustKey(projectId, key.id)
      setMessage('公钥已撤销，后续数据包不能再用该密钥建立新信任。')
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className={base.integrationSection}>
      <header>
        <span>
          <strong>运营方信任公钥</strong>
          <small>由消费者决定信任谁；撤销只阻止后续签名导入，不改写历史快照。</small>
        </span>
        <button style={actionStyle('icon', busy)} type="button" onClick={refresh} disabled={busy} title="刷新公钥">
          <RefreshCw size={14} />
        </button>
      </header>
      <div style={{ ...commerceStyles.list, padding: 12 }}>
        <input
          type="text"
          value={sourceOperator}
          maxLength={160}
          onChange={(event) => setSourceOperator(event.target.value)}
          placeholder="来源运营方标识"
          disabled={busy}
        />
        <textarea
          value={publicKeyPem}
          maxLength={16 * 1024}
          rows={5}
          onChange={(event) => setPublicKeyPem(event.target.value)}
          placeholder="-----BEGIN PUBLIC KEY-----"
          disabled={busy}
        />
        <button style={actionStyle('primary', busy)} type="button" onClick={addKey} disabled={busy}>
          <KeyRound size={14} />加入信任
        </button>
        {keys.map((key) => (
          <article key={key.id} style={listItemStyle()}>
            <header style={commerceStyles.itemHeader}>
              <strong style={commerceStyles.itemTitle}>{key.source_operator}</strong>
              <span style={badgeStyle(key.status === 'active' ? 'neutral' : 'danger')}>
                {key.status === 'active' ? '有效' : '已撤销'}
              </span>
            </header>
            <p style={{ ...commerceStyles.itemText, overflowWrap: 'anywhere' }}>{key.key_id}</p>
            {key.status === 'active' && (
              <footer style={{ ...commerceStyles.itemHeader, marginTop: 8 }}>
                <small style={commerceStyles.itemMeta}>{new Date(key.created_at).toLocaleString()}</small>
                <button style={actionStyle('danger', busy)} type="button" onClick={() => revoke(key)} disabled={busy}>
                  <ShieldOff size={13} />撤销
                </button>
              </footer>
            )}
          </article>
        ))}
        {keys.length === 0 && <p className={base.empty}>尚未信任任何运营方公钥。</p>}
      </div>
      {message && <div style={commerceStyles.message}>{message}</div>}
    </section>
  )
}
