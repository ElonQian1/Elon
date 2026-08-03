interface ConsumerCapabilityFilterFieldsProps {
  capabilityKind: string
  accessLevel: string
  onCapabilityKindChange: (value: string) => void
  onAccessLevelChange: (value: string) => void
}

export default function ConsumerCapabilityFilterFields({
  capabilityKind,
  accessLevel,
  onCapabilityKindChange,
  onAccessLevelChange,
}: ConsumerCapabilityFilterFieldsProps) {
  return (
    <>
      <label>
        能力类型
        <select value={capabilityKind} onChange={(event) => onCapabilityKindChange(event.target.value)}>
          <option value="">不限</option>
          <option value="query">信息查询</option>
          <option value="action">经营操作</option>
        </select>
      </label>
      <label>
        访问方式
        <select value={accessLevel} onChange={(event) => onAccessLevelChange(event.target.value)}>
          <option value="">不限</option>
          <option value="public">公开调用</option>
          <option value="authorized">需授权调用</option>
        </select>
      </label>
    </>
  )
}
