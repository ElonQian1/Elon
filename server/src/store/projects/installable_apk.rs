pub(super) const EXISTS_SQL: &str = r#"(
  CASE WHEN p.id = 'yilong-quant' THEN EXISTS (
    SELECT 1 FROM project_releases quant_release
    WHERE quant_release.project_id = p.id
      AND quant_release.status = 'published'
      AND quant_release.package_name = 'com.elon.quant'
      AND quant_release.version_code >= 5
      AND quant_release.channel = 'paper'
      AND TRIM(COALESCE(quant_release.version_name, '')) != ''
      AND TRIM(COALESCE(quant_release.file_path, '')) != ''
      AND LOWER(quant_release.file_name) LIKE '%.apk'
      AND quant_release.size_bytes > 0
      AND LENGTH(quant_release.sha256) = 64
      AND quant_release.sha256 NOT GLOB '*[^0-9a-f]*'
      AND LENGTH(quant_release.source_git_sha) = 40
      AND quant_release.source_git_sha NOT GLOB '*[^0-9a-f]*'
      AND json_valid(quant_release.metadata_json)
      AND json_extract(quant_release.metadata_json, '$.schema')
          = 'yilong.official_quant_release_admission.v1'
      AND json_extract(
            quant_release.metadata_json,
            '$.apk_signing_block_structure_present'
          ) = 1
      AND json_extract(
            quant_release.metadata_json,
            '$.cryptographic_signature_verified'
          ) = 0
      AND json_extract(quant_release.metadata_json, '$.artifact_sha256')
          = quant_release.sha256
      AND json_extract(quant_release.metadata_json, '$.artifact_size_bytes')
          = quant_release.size_bytes
  ) ELSE (
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
  ) END
)"#;
