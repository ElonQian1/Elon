import { useCallback, useEffect, useState } from 'react'
import { Download, KeyRound, RefreshCw, ShieldCheck, Trash2 } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import type { MerchantIdentityKey } from './openCommerceClientTypes'
import {
  downloadMerchantIdentityPrivateKey,
  generateMerchantIdentityProof,
  type GeneratedMerchantIdentityProof,
} from './merchantIdentityProof'
import { errorText } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import { actionStyle, badgeStyle, commerceStyles, listItemStyle } from './openCommerceStyles'

export default function MerchantPortableIdentityPanel({
  projectId,
  merchantId,
  canEdit,
}: {
  projectId: string
  merchantId: string
  canEdit: boolean
}) {
  const [keys, setKeys] = useState<MerchantIdentityKey[]>([])
  const [pending, setPending] = useState<GeneratedMerchantIdentityProof | null>(null)
  const [privateKeySaved, setPrivateKeySaved] = useState(false)
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    try {
      const response = await openCommerceClientApi.listMerchantIdentityKeys(projectId, merchantId)
      setKeys(response.keys)
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [merchantId, projectId])

  useEffect(() => {
    refresh()
  }, [refresh])

  async function generate() {
    if (!window.confirm('私钥只会在本机内存中生成，平台无法找回。继续吗？')) return
    setBusy(true)
    setMessage('')
    try {
      setPending(await generateMerchantIdentityProof(projectId, merchantId))
      setPrivateKeySaved(false)
      setMessage('密钥已在本机生成。请先下载私钥，再发布公钥指纹。')
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  function downloadPrivateKey() {
    if (!pending) return
    downloadMerchantIdentityPrivateKey(merchantId, pending)
    setPrivateKeySaved(true)
    setMessage('私钥已触发本地下载。请确认保存后再发布。')
  }

  async function publish() {
    if (!pending || !privateKeySaved) return
    if (!window.confirm('发布经私钥持有证明的公钥指纹到商户目录？')) return
    setBusy(true)
    try {
      await openCommerceClientApi.createMerchantIdentityKey(projectId, merchantId, {
        public_key_pem: pending.publicKeyPem,
        proof_signature_base64: pending.proofSignatureBase64,
      })
      setPending(null)
      setPrivateKeySaved(false)
      setMessage('商户可携带身份指纹已发布。')
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function revoke(key: MerchantIdentityKey) {
    if (!window.confirm('撤销该公钥指纹？历史记录保留，且不能重新启用。')) return
    setBusy(true)
    try {
      await openCommerceClientApi.revokeMerchantIdentityKey(projectId, merchantId, key.id)
      setMessage('商户身份公钥已撤销。')
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
          <strong>可携带商户身份</strong>
          <small>私钥留在商户手中；公开目录只发布经验证的公钥指纹。</small>
        </span>
        <button style={actionStyle('icon', busy)} type="button" onClick={refresh} disabled={busy} title="刷新身份公钥">
          <RefreshCw size={14} />
        </button>
      </header>
      <div style={{ ...commerceStyles.list, padding: 12 }}>
        {canEdit && !pending && (
          <button style={actionStyle('secondary', busy)} type="button" onClick={generate} disabled={busy}>
            <KeyRound size={14} />在本机生成身份密钥
          </button>
        )}
        {pending && (
          <article style={listItemStyle()}>
            <strong>待发布指纹 {pending.keyId.slice(0, 16)}…</strong>
            <p style={{ ...commerceStyles.itemText, overflowWrap: 'anywhere' }}>{pending.keyId}</p>
            <footer style={{ ...commerceStyles.itemHeader, marginTop: 8 }}>
              <button style={actionStyle('secondary', busy)} type="button" onClick={downloadPrivateKey} disabled={busy}>
                <Download size={13} />下载私钥
              </button>
              <button style={actionStyle('primary', busy || !privateKeySaved)} type="button" onClick={publish} disabled={busy || !privateKeySaved}>
                <ShieldCheck size={13} />发布指纹
              </button>
            </footer>
          </article>
        )}
        {keys.map((key) => (
          <article key={key.id} style={listItemStyle()}>
            <header style={commerceStyles.itemHeader}>
              <strong style={commerceStyles.itemTitle}>{key.key_id.slice(0, 20)}…</strong>
              <span style={badgeStyle(key.status === 'active' ? 'neutral' : 'warn')}>
                {key.status === 'active' ? '已验证' : '已撤销'}
              </span>
            </header>
            <p style={{ ...commerceStyles.itemText, overflowWrap: 'anywhere' }}>{key.key_id}</p>
            <small>持有证明 {new Date(key.proof_verified_at).toLocaleString('zh-CN')}</small>
            {canEdit && key.status === 'active' && (
              <footer style={{ marginTop: 8 }}>
                <button style={actionStyle('danger', busy)} type="button" onClick={() => revoke(key)} disabled={busy}>
                  <Trash2 size={13} />撤销
                </button>
              </footer>
            )}
          </article>
        ))}
        {keys.length === 0 && !pending && <small>尚未发布商户身份指纹。</small>}
      </div>
      {message && <div style={commerceStyles.message}>{message}</div>}
    </section>
  )
}
