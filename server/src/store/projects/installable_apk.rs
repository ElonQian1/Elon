pub(super) const EXISTS_SQL: &str = r#"(
  EXISTS (
    SELECT 1 FROM tasks t_apk
    WHERE t_apk.project_id = p.id
      AND t_apk.apk_url IS NOT NULL
      AND t_apk.apk_url != ''
  )
  OR EXISTS (
    SELECT 1
    FROM json_each(
      CASE
        WHEN json_valid(p.landing_json) THEN p.landing_json
        ELSE '{"downloads":[]}'
      END,
      '$.downloads'
    ) landing_download
    WHERE LOWER(COALESCE(json_extract(landing_download.value, '$.platform'), '')) = 'android'
      AND LOWER(COALESCE(json_extract(landing_download.value, '$.status'), ''))
          IN ('available', 'external')
      AND (
        LOWER(COALESCE(json_extract(landing_download.value, '$.url'), '')) LIKE 'https://%'
        OR LOWER(COALESCE(json_extract(landing_download.value, '$.url'), '')) LIKE 'http://%'
      )
  )
)"#;
