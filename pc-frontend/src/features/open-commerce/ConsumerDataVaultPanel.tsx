import { useCallback, useEffect, useState } from 'react'
import { Download, Lock, Plus, RefreshCw, Save, Trash2, Unlock } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import type {
  ConsumerDataVaultItemKind,
  ConsumerDataVaultItemSummary,
} from './openCommerceClientTypes'
import {
  decryptConsumerDataVaultItem,
  encryptConsumerDataVaultItem,
} from './consumerDataVaultCrypto'
import { errorText } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import { actionStyle, badgeStyle, commerceStyles, listItemStyle } from './openCommerceStyles'

const ITEM_KINDS: Array<{ value: ConsumerDataVaultItemKind; label: string }> = [
  { value: 'private_note', label: '私密记录' },
  { value: 'identity', label: '身份资料' },
  { value: 'health', label: '健康资料' },
  { value: 'finance', label: '财务资料' },
  { value: 'credential_reference', label: '凭据参考' },
  { value: 'custom', label: '自定义' },
]

export default function ConsumerDataVaultPanel({ projectId }: { projectId: string }) {
  const [items, setItems] = useState<ConsumerDataVaultItemSummary[]>([])
  const [selected, setSelected] = useState<ConsumerDataVaultItemSummary | null>(null)
  const [label, setLabel] = useState('')
  const [itemKind, setItemKind] = useState<ConsumerDataVaultItemKind>('private_note')
  const [content, setContent] = useState('')
  const [passphrase, setPassphrase] = useState('')
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    try {
      const response = await openCommerceClientApi.listConsumerDataVaultItems(projectId)
      setItems(response.items)
      setSelected((current) =>
        current ? response.items.find((item) => item.id === current.id) ?? null : null
      )
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [projectId])

  useEffect(() => {
    refresh()
  }, [refresh])

  function resetEditor() {
    setSelected(null)
    setLabel('')
    setItemKind('private_note')
    setContent('')
    setPassphrase('')
  }

  async function save() {
    if (!label.trim() || !content || !passphrase) {
      setMessage('请填写非敏感标签、加密内容和本地口令。')
      return
    }
    setBusy(true)
    setMessage('')
    try {
      if (selected) {
        const envelope = await encryptConsumerDataVaultItem(
          selected.id,
          selected.revision + 1,
          content,
          passphrase,
        )
        await openCommerceClientApi.updateConsumerDataVaultItem(projectId, selected.id, {
          expected_revision: selected.revision,
          label: label.trim(),
          item_kind: itemKind,
          envelope,
        })
        setMessage('保险箱条目已加密更新。')
      } else {
        const id = crypto.randomUUID()
        const envelope = await encryptConsumerDataVaultItem(id, 1, content, passphrase)
        await openCommerceClientApi.createConsumerDataVaultItem(projectId, {
          id,
          label: label.trim(),
          item_kind: itemKind,
          envelope,
        })
        setMessage('保险箱条目已在本地加密并保存。')
      }
      resetEditor()
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function unlock(item: ConsumerDataVaultItemSummary) {
    if (!passphrase) {
      setMessage('请先输入该条目的本地口令。')
      return
    }
    setBusy(true)
    setMessage('')
    try {
      const fullItem = await openCommerceClientApi.getConsumerDataVaultItem(projectId, item.id)
      const plaintext = await decryptConsumerDataVaultItem(fullItem.envelope, passphrase)
      setSelected(fullItem)
      setLabel(fullItem.label)
      setItemKind(fullItem.item_kind)
      setContent(plaintext)
      setMessage('条目已在本机解锁，明文未发送到服务器。')
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function downloadEncrypted(item: ConsumerDataVaultItemSummary) {
    setBusy(true)
    setMessage('')
    try {
      const fullItem = await openCommerceClientApi.getConsumerDataVaultItem(projectId, item.id)
      const blob = new Blob([JSON.stringify(fullItem, null, 2)], { type: 'application/json' })
      const url = URL.createObjectURL(blob)
      const anchor = document.createElement('a')
      anchor.href = url
      anchor.download = `consumer-vault-${item.id}.json`
      document.body.appendChild(anchor)
      anchor.click()
      anchor.remove()
      window.setTimeout(() => URL.revokeObjectURL(url), 0)
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function remove(item: ConsumerDataVaultItemSummary) {
    if (!window.confirm(`永久删除保险箱条目“${item.label}”？平台无法找回密文或口令。`)) return
    setBusy(true)
    setMessage('')
    try {
      await openCommerceClientApi.deleteConsumerDataVaultItem(projectId, item.id, item.revision)
      if (selected?.id === item.id) resetEditor()
      setMessage('保险箱条目已删除。')
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
          <strong>消费者数据保险箱</strong>
          <small>敏感内容仅在本机加解密；标签、类型、密文大小和摘要对服务器可见。</small>
        </span>
        <div style={commerceStyles.headerActions}>
          <button style={actionStyle('icon', busy)} type="button" onClick={resetEditor} disabled={busy} title="新建条目">
            <Plus size={14} />
          </button>
          <button style={actionStyle('icon', busy)} type="button" onClick={refresh} disabled={busy} title="刷新保险箱">
            <RefreshCw size={14} />
          </button>
        </div>
      </header>
      <div style={{ ...commerceStyles.list, padding: 12 }}>
        <div style={{ display: 'grid', gap: 8 }}>
          <input
            type="text"
            value={label}
            maxLength={120}
            onChange={(event) => setLabel(event.target.value)}
            placeholder="非敏感标签"
            disabled={busy}
          />
          <select value={itemKind} onChange={(event) => setItemKind(event.target.value as ConsumerDataVaultItemKind)} disabled={busy}>
            {ITEM_KINDS.map((kind) => <option key={kind.value} value={kind.value}>{kind.label}</option>)}
          </select>
          <textarea
            value={content}
            rows={5}
            onChange={(event) => setContent(event.target.value)}
            placeholder="仅在本机出现的加密内容"
            disabled={busy}
          />
          <input
            type="password"
            value={passphrase}
            minLength={12}
            maxLength={256}
            onChange={(event) => setPassphrase(event.target.value)}
            placeholder="本地加密口令（不可找回）"
            disabled={busy}
          />
          <button style={actionStyle('primary', busy)} type="button" onClick={save} disabled={busy}>
            <Save size={14} />{selected ? `保存修订 ${selected.revision + 1}` : '加密并保存'}
          </button>
        </div>
        {items.map((item) => (
          <article key={item.id} style={listItemStyle(selected?.id === item.id)}>
            <header style={commerceStyles.itemHeader}>
              <strong style={commerceStyles.itemTitle}>{item.label}</strong>
              <span style={badgeStyle('neutral')}><Lock size={12} />修订 {item.revision}</span>
            </header>
            <small style={commerceStyles.itemMeta}>
              {itemKindLabel(item.item_kind)} · {formatBytes(item.ciphertext_bytes)} · {new Date(item.updated_at).toLocaleString()}
            </small>
            <p style={{ ...commerceStyles.itemText, overflowWrap: 'anywhere' }}>{item.ciphertext_sha256}</p>
            <footer style={{ ...commerceStyles.itemHeader, marginTop: 8 }}>
              <code style={commerceStyles.itemMeta}>{item.id}</code>
              <div style={commerceStyles.headerActions}>
                <button style={actionStyle('secondary', busy)} type="button" onClick={() => unlock(item)} disabled={busy} title="本机解锁">
                  <Unlock size={13} />
                </button>
                <button style={actionStyle('secondary', busy)} type="button" onClick={() => downloadEncrypted(item)} disabled={busy} title="下载密文">
                  <Download size={13} />
                </button>
                <button style={actionStyle('danger', busy)} type="button" onClick={() => remove(item)} disabled={busy} title="删除条目">
                  <Trash2 size={13} />
                </button>
              </div>
            </footer>
          </article>
        ))}
        {items.length === 0 && <p className={base.empty}>保险箱中尚无加密条目。</p>}
      </div>
      {message && <div style={commerceStyles.message}>{message}</div>}
    </section>
  )
}

function itemKindLabel(kind: ConsumerDataVaultItemKind) {
  return ITEM_KINDS.find((item) => item.value === kind)?.label ?? kind
}

function formatBytes(value: number) {
  return value < 1024 ? `${value} B` : `${(value / 1024).toFixed(1)} KiB`
}
