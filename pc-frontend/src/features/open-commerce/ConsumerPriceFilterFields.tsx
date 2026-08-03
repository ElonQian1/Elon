interface ConsumerPriceFilterFieldsProps {
  maxPrice: string
  currency: string
  onMaxPriceChange: (value: string) => void
  onCurrencyChange: (value: string) => void
}

export default function ConsumerPriceFilterFields({
  maxPrice,
  currency,
  onMaxPriceChange,
  onCurrencyChange,
}: ConsumerPriceFilterFieldsProps) {
  return (
    <>
      <label>
        单位价格上限
        <input type="number" min="0" step="0.01" value={maxPrice} onChange={(event) => onMaxPriceChange(event.target.value)} />
      </label>
      <label>
        价格币种
        <input
          value={currency}
          maxLength={3}
          onChange={(event) => onCurrencyChange(event.target.value.toUpperCase())}
          placeholder="CNY"
          disabled={!maxPrice}
        />
      </label>
    </>
  )
}
