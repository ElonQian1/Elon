import { useState, type ReactNode } from 'react'
import { UiInspectorTabs, type UiInspectorTab } from '../inspector/UiInspectorTabs'
import type { SourcePreviewMode, SourcePreviewNode, SourcePreviewPatch, SourcePreviewSaveState } from './types'
import panelStyles from './SourcePreviewInspector.module.css'
import styles from './SourcePreview.module.css'

interface Props {
  node: SourcePreviewNode | null
  pendingCount: number
  saveState: SourcePreviewSaveState
  onChange: (patch: SourcePreviewPatch) => void
  onSave: () => void
  onModeChange: (mode: SourcePreviewMode) => void
}

export function SourcePreviewInspector({ node, pendingCount, saveState, onChange, onSave, onModeChange }: Props) {
  const [activeTab, setActiveTab] = useState<UiInspectorTab>('design')
  const [copyNotice, setCopyNotice] = useState('')
  if (!node) return <aside className={styles.sourceInspector}><UiInspectorTabs value={activeTab} onChange={setActiveTab} /><div className={styles.inspectorEmpty}>在画布或组件树中选择一个组件。</div></aside>
  const update = (property: string, value: string | number) => onChange({ nodeKey: node.key, property, value })
  const supportsText = ['text', 'button', 'input'].includes(node.kind)
  const supportsBoxStyle = node.kind !== 'spacer'
  const copyContext = async () => {
    await navigator.clipboard.writeText(JSON.stringify({
      intent: '修改当前 UI 组件',
      nodeKey: node.key,
      resourceId: node.resourceId,
      component: node.name,
      kind: node.kind,
      layoutFile: node.source.layoutFile,
      editable: node.editable,
      layout: node.layout,
      style: node.style,
    }, null, 2))
    setCopyNotice('已复制当前组件的精简上下文')
  }
  return (
    <aside className={styles.sourceInspector} data-testid="source-preview-inspector">
      <UiInspectorTabs value={activeTab} onChange={setActiveTab} />
      <header><strong>{node.name}</strong><small>{componentLabel(node)}</small><code>{node.resourceId ?? node.key}</code></header>

      {activeTab === 'design' && <>
        {supportsText && <section><h3>文字</h3>
          <FullField label="文案"><input data-preview-property="text" value={node.style.text} onChange={(event) => update('text', event.target.value)} /></FullField>
          <div className={styles.fieldGrid}>
            <ColorField property="textColor" label="文字颜色" value={node.style.textColor} onChange={(v) => update('textColor', v)} />
            <NumberField property="fontSize" label="字号" value={node.style.fontSize} onChange={(v) => update('fontSize', v)} />
          </div>
        </section>}
        {supportsBoxStyle && <section><h3>外观</h3><div className={styles.fieldGrid}>
          <ColorField property="background" label="背景颜色" value={node.style.background} onChange={(v) => update('background', v)} />
          <NumberField property="borderRadius" label="圆角" value={node.style.borderRadius} onChange={(v) => update('borderRadius', v)} />
          <NumberField property="opacity" label="透明度" value={node.style.opacity} step={0.05} min={0} max={1} onChange={(v) => update('opacity', v)} />
        </div></section>}
        <section><h3>尺寸与对齐</h3><div className={styles.fieldGrid}>
          <TextField property="width" label="宽度" value={node.layout.width} onChange={(v) => update('width', v)} />
          <TextField property="height" label="高度" value={node.layout.height} onChange={(v) => update('height', v)} />
        </div><FullField label="对齐"><select value={node.layout.gravity} onChange={(event) => update('gravity', event.target.value)}><option value="">默认</option><option value="center">居中</option><option value="center_horizontal">水平居中</option><option value="center_vertical">垂直居中</option><option value="end">末端</option><option value="bottom">底部</option></select></FullField></section>
        <EdgeSection title="内边距" prefix="padding" edges={node.layout.padding} onChange={update} />
        <EdgeSection title="外边距" prefix="margin" edges={node.layout.margin} onChange={update} />
      </>}

      {activeTab === 'ai' && <section className={panelStyles.aiPanel}>
        <h3>AI 按需修改</h3>
        <strong>只发送当前组件，不重复读取整棵源码树</strong>
        <p>上下文包含节点 ID、布局文件、可编辑属性和当前样式；结构变化时再让 Codex 按需读取父组件源码。</p>
        <button type="button" onClick={() => { void copyContext() }}>复制组件上下文</button>
        <button type="button" onClick={() => onModeChange('evidence')}>转到 Android 真帧校准</button>
        {copyNotice && <small>{copyNotice}</small>}
      </section>}

      {activeTab === 'inspect' && <>
        <section className={styles.sourceInfo}><h3>源码绑定</h3><p>{node.source.layoutFile}</p><p>起始标签：{node.source.startTagStart}–{node.source.startTagEnd}</p><p>修改会确定性写回这个 XML 节点；父子布局在草稿中即时重排。</p></section>
        <section className={panelStyles.rawInfo}><h3>节点信息</h3><dl><dt>标签</dt><dd>{node.tag}</dd><dt>节点 Key</dt><dd>{node.key}</dd><dt>布局模式</dt><dd>{node.layout.mode} / {node.layout.orientation}</dd><dt>可编辑属性</dt><dd>{node.editable.join('、') || '由适配器决定'}</dd></dl></section>
      </>}

      <div className={styles.inspectorFooter}><span>{pendingCount ? `${pendingCount} 个组件待写回` : saveState === 'saved' ? '源码已保存' : '尚无修改'}</span><button onClick={onSave} disabled={!pendingCount}>确认写回源码</button></div>
    </aside>
  )
}

function componentLabel(node: SourcePreviewNode) {
  const labels: Record<string, string> = { group: '布局容器', list: '列表', text: '文字', button: '按钮', input: '输入框', image: '图片', spacer: '间隔' }
  return `${labels[node.kind] ?? '组件'} · ${node.tag}`
}

function EdgeSection({ title, prefix, edges, onChange }: { title: string; prefix: 'padding' | 'margin'; edges: SourcePreviewNode['layout']['padding']; onChange: (property: string, value: number) => void }) {
  return <section><h3>{title}</h3><div className={styles.fieldGrid}>{(['start', 'top', 'end', 'bottom'] as const).map((edge) => { const property = `${prefix}${edge[0].toUpperCase()}${edge.slice(1)}`; return <NumberField key={edge} property={property} label={{ start: '左', top: '上', end: '右', bottom: '下' }[edge]} value={edges[edge]} onChange={(value) => onChange(property, value)} /> })}</div></section>
}

function FullField({ label, children }: { label: string; children: ReactNode }) { return <label className={styles.fullField}><span>{label}</span>{children}</label> }
function TextField({ property, label, value, onChange }: { property: string; label: string; value: string; onChange: (value: string) => void }) { return <label className={styles.field}><span>{label}</span><input data-preview-property={property} value={value} onChange={(event) => onChange(event.target.value)} /></label> }
function NumberField({ property, label, value, onChange, min, max, step = 1 }: { property: string; label: string; value: number; onChange: (value: number) => void; min?: number; max?: number; step?: number }) { return <label className={styles.field}><span>{label}</span><input data-preview-property={property} type="number" value={value} min={min} max={max} step={step} onChange={(event) => onChange(Number(event.target.value))} /></label> }
function ColorField({ property, label, value, onChange }: { property: string; label: string; value: string; onChange: (value: string) => void }) { const safe = /^#[0-9a-f]{6}$/i.test(value) ? value : '#ffffff'; return <label className={styles.field}><span>{label}</span><div className={styles.colorField}><input data-preview-property={`${property}Picker`} type="color" value={safe} onChange={(event) => onChange(event.target.value)} /><input data-preview-property={property} value={value} onChange={(event) => onChange(event.target.value)} /></div></label> }
