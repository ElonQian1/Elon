interface ConsumerPreferenceConstraintFieldsProps {
  requireCityMatch: boolean
  requireCategoryMatch: boolean
  requireAllTagsMatch: boolean
  onRequireCityMatchChange: (value: boolean) => void
  onRequireCategoryMatchChange: (value: boolean) => void
  onRequireAllTagsMatchChange: (value: boolean) => void
}

export default function ConsumerPreferenceConstraintFields({
  requireCityMatch,
  requireCategoryMatch,
  requireAllTagsMatch,
  onRequireCityMatchChange,
  onRequireCategoryMatchChange,
  onRequireAllTagsMatchChange,
}: ConsumerPreferenceConstraintFieldsProps) {
  return (
    <>
      <label>
        <span>城市必须匹配</span>
        <input type="checkbox" checked={requireCityMatch} onChange={(event) => onRequireCityMatchChange(event.target.checked)} />
      </label>
      <label>
        <span>经营类别必须匹配</span>
        <input type="checkbox" checked={requireCategoryMatch} onChange={(event) => onRequireCategoryMatchChange(event.target.checked)} />
      </label>
      <label>
        <span>全部偏好标签必须匹配</span>
        <input type="checkbox" checked={requireAllTagsMatch} onChange={(event) => onRequireAllTagsMatchChange(event.target.checked)} />
      </label>
    </>
  )
}
