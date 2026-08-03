import { useId } from 'react'
import type { ConsumerSourceFilterOptions } from './openCommerceClientTypes'

interface ConsumerSourceFilterFieldsProps {
  providerKey: string
  dataDomain: string
  maxAgeMinutes: string
  options?: ConsumerSourceFilterOptions
  onProviderKeyChange: (value: string) => void
  onDataDomainChange: (value: string) => void
  onMaxAgeMinutesChange: (value: string) => void
}

export default function ConsumerSourceFilterFields({
  providerKey,
  dataDomain,
  maxAgeMinutes,
  options,
  onProviderKeyChange,
  onDataDomainChange,
  onMaxAgeMinutesChange,
}: ConsumerSourceFilterFieldsProps) {
  const providerListId = `${useId()}-source-providers`
  const dataDomainListId = `${useId()}-source-data-domains`
  return (
    <>
      <label>
        来源厂商标识
        <input list={providerListId} value={providerKey} onChange={(event) => onProviderKeyChange(event.target.value)} placeholder="可选，例如 meituan" />
        <datalist id={providerListId}>
          {options?.providers.map((option) => (
            <option key={option.value} value={option.value} label={`${option.capability_count} 项公开能力`} />
          ))}
        </datalist>
      </label>
      <label>
        来源数据域
        <input list={dataDomainListId} value={dataDomain} onChange={(event) => onDataDomainChange(event.target.value)} placeholder="可选，例如 inventory" />
        <datalist id={dataDomainListId}>
          {options?.data_domains.map((option) => (
            <option key={option.value} value={option.value} label={`${option.capability_count} 项公开能力`} />
          ))}
        </datalist>
      </label>
      <label>
        回执最长年龄（分钟）
        <input type="number" min="1" max="525600" step="1" value={maxAgeMinutes} onChange={(event) => onMaxAgeMinutesChange(event.target.value)} placeholder="可选，例如 30" />
      </label>
    </>
  )
}
