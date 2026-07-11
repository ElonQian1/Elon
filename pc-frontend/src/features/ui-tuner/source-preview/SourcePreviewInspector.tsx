import type { ReactNode } from 'react'
import type { SourcePreviewNode, SourcePreviewPatch, SourcePreviewSaveState } from './types'
import styles from './SourcePreview.module.css'

interface Props { node: SourcePreviewNode | null; pendingCount: number; saveState: SourcePreviewSaveState; onChange: (patch: SourcePreviewPatch) => void; onSave: () => void }

export function SourcePreviewInspector({ node, pendingCount, saveState, onChange, onSave }: Props) {
  if (!node) return <aside className={styles.sourceInspector}><div className={styles.inspectorEmpty}>在画布或组件树中选择一个组件。</div></aside>
  const update = (property: string, value: string | number) => onChange({ nodeKey: node.key, property, value })
  return (
    <aside className={styles.sourceInspector} data-testid="source-preview-inspector">
      <header><strong>{node.name}</strong><small>{node.tag}</small><code>{node.key}</code></header>
      <section><h3>内容与外观</h3>
        <FullField label="文字"><input data-preview-property="text" value={node.style.text} onChange={(event) => update('text', event.target.value)} /></FullField>
        <div className={styles.fieldGrid}><ColorField property="textColor" label="文字颜色" value={node.style.textColor} onChange={(v) => update('textColor', v)} /><ColorField property="background" label="背景颜色" value={node.style.background} onChange={(v) => update('background', v)} />
          <NumberField property="fontSize" label="字号" value={node.style.fontSize} onChange={(v) => update('fontSize', v)} /><NumberField property="borderRadius" label="圆角" value={node.style.borderRadius} onChange={(v) => update('borderRadius', v)} />
          <NumberField property="opacity" label="透明度" value={node.style.opacity} step={0.05} min={0} max={1} onChange={(v) => update('opacity', v)} /></div>
      </section>
      <section><h3>布局尺寸</h3><div className={styles.fieldGrid}>
        <TextField property="width" label="宽度" value={node.layout.width} onChange={(v) => update('width', v)} /><TextField property="height" label="高度" value={node.layout.height} onChange={(v) => update('height', v)} />
      </div><FullField label="对齐"><select value={node.layout.gravity} onChange={(event) => update('gravity', event.target.value)}><option value="">默认</option><option value="center">居中</option><option value="center_horizontal">水平居中</option><option value="center_vertical">垂直居中</option><option value="end">末端</option><option value="bottom">底部</option></select></FullField></section>
      <EdgeSection title="内边距" prefix="padding" edges={node.layout.padding} onChange={update} />
      <EdgeSection title="外边距" prefix="margin" edges={node.layout.margin} onChange={update} />
      <section className={styles.sourceInfo}><h3>源码绑定</h3><p>{node.source.layoutFile}</p><p>起始标签：{node.source.startTagStart}–{node.source.startTagEnd}</p><p>修改会直接写回这个 XML 节点；父子布局会在画布中实时重排。</p></section>
      <div className={styles.inspectorFooter}><span>{pendingCount ? `${pendingCount} 个组件待写回` : saveState === 'saved' ? '源码已保存' : '尚无修改'}</span><button onClick={onSave} disabled={!pendingCount}>确认写回源码</button></div>
    </aside>
  )
}

function EdgeSection({ title, prefix, edges, onChange }: { title: string; prefix: 'padding' | 'margin'; edges: SourcePreviewNode['layout']['padding']; onChange: (property: string, value: number) => void }) {
  return <section><h3>{title}</h3><div className={styles.fieldGrid}>{(['start', 'top', 'end', 'bottom'] as const).map((edge) => { const property = `${prefix}${edge[0].toUpperCase()}${edge.slice(1)}`; return <NumberField key={edge} property={property} label={{ start: '左', top: '上', end: '右', bottom: '下' }[edge]} value={edges[edge]} onChange={(value) => onChange(property, value)} /> })}</div></section>
}

function FullField({ label, children }: { label: string; children: ReactNode }) { return <label className={styles.fullField}><span>{label}</span>{children}</label> }
function TextField({ property, label, value, onChange }: { property: string; label: string; value: string; onChange: (value: string) => void }) { return <label className={styles.field}><span>{label}</span><input data-preview-property={property} value={value} onChange={(event) => onChange(event.target.value)} /></label> }
function NumberField({ property, label, value, onChange, min, max, step = 1 }: { property: string; label: string; value: number; onChange: (value: number) => void; min?: number; max?: number; step?: number }) { return <label className={styles.field}><span>{label}</span><input data-preview-property={property} type="number" value={value} min={min} max={max} step={step} onChange={(event) => onChange(Number(event.target.value))} /></label> }
function ColorField({ property, label, value, onChange }: { property: string; label: string; value: string; onChange: (value: string) => void }) { const safe = /^#[0-9a-f]{6}$/i.test(value) ? value : '#ffffff'; return <label className={styles.field}><span>{label}</span><div className={styles.colorField}><input data-preview-property={`${property}Picker`} type="color" value={safe} onChange={(event) => onChange(event.target.value)} /><input data-preview-property={property} value={value} onChange={(event) => onChange(event.target.value)} /></div></label> }
