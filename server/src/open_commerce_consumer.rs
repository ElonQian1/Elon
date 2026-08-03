use anyhow::{bail, Result};
use chrono::Utc;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{
    open_commerce_consumer_model::{
        ConsumerAuthorizationState, ConsumerDiscoveryMatch, ConsumerDiscoveryRequest,
        ConsumerDiscoveryResponse, ConsumerPreferences, ConsumerRankingReceipt,
    },
    open_commerce_consumer_preference_service,
    open_commerce_consumer_ranking::{self, ConsumerRankingPolicy},
    open_commerce_developer_model::{CreateAuthorizationRequest, OpenCommerceAuthorizationRequest},
    open_commerce_directory_model::{
        OpenCommerceDirectoryCapability, OpenCommerceDirectoryMerchantDetail,
    },
    open_commerce_directory_service,
    open_commerce_model::{ACCESS_AUTHORIZED, ACCESS_PUBLIC},
    store::Store,
};

pub(crate) fn discover(
    store: &Store,
    user_id: &str,
    mut request: ConsumerDiscoveryRequest,
) -> Result<ConsumerDiscoveryResponse> {
    if request.requester_app_id.trim().is_empty() {
        request.requester_app_id = "pc-web".to_string();
    }
    ensure_app_owned_by_user(store, user_id, &request.requester_app_id)?;
    let ranking_is_user_selected = request
        .ranking_policy
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());
    let ranking_policy = ConsumerRankingPolicy::parse(request.ranking_policy.as_deref())?;
    request.preferences =
        open_commerce_consumer_preference_service::normalize_preferences(request.preferences)?;
    let candidate_limit = request.limit.clamp(1, 50).saturating_mul(4).min(100);
    let candidates = open_commerce_directory_service::discover_merchants(
        store,
        request.query.as_deref(),
        request.capability_key.as_deref(),
        candidate_limit,
    )?;
    let directory_candidate_count = candidates.len();
    let mut matches = candidates
        .into_iter()
        .filter_map(|detail| best_match(store, detail, &request, ranking_policy).transpose())
        .collect::<Result<Vec<_>>>()?;
    let eligible_match_count = matches.len();
    ranking_policy.sort_matches(&mut matches);
    matches.truncate(request.limit.clamp(1, 50));
    let ranking_receipt = request
        .include_ranking_receipt
        .then(|| {
            build_ranking_receipt(
                &request,
                ranking_policy,
                candidate_limit,
                directory_candidate_count,
                eligible_match_count,
                &matches,
            )
        })
        .transpose()?;
    Ok(ConsumerDiscoveryResponse {
        schema: "open_commerce.consumer_discovery.v1",
        capability_contract_profile: "open_commerce.capability_schema.v1",
        requester_app_id: request.requester_app_id,
        ranking_policy: ranking_policy.key().to_string(),
        ranking_policy_label: ranking_policy.label().to_string(),
        ranking_explanation: ranking_policy.explanation().to_string(),
        ranking_is_paid: false,
        ranking_is_user_selected,
        freshness_requirement: freshness_requirement(request.require_current_declaration),
        source_requirement: source_requirement(request.require_internal_sync_receipt),
        available_ranking_policies: open_commerce_consumer_ranking::available_ranking_policies(),
        ranking_receipt,
        matches,
    })
}

fn build_ranking_receipt(
    request: &ConsumerDiscoveryRequest,
    ranking_policy: ConsumerRankingPolicy,
    candidate_limit: usize,
    directory_candidate_count: usize,
    eligible_match_count: usize,
    matches: &[ConsumerDiscoveryMatch],
) -> Result<ConsumerRankingReceipt> {
    let request_fingerprint_payload = json!({
        "schema": "open_commerce.consumer_discovery_input.v1",
        "query": normalized_optional(request.query.as_deref()),
        "capability_key": normalized_optional(request.capability_key.as_deref()),
        "requester_app_id": request.requester_app_id.as_str(),
        "ranking_policy": ranking_policy.key(),
        "require_current_declaration": request.require_current_declaration,
        "require_internal_sync_receipt": request.require_internal_sync_receipt,
        "preferences": &request.preferences,
        "limit": request.limit.clamp(1, 50)
    });
    let request_fingerprint_json = serde_json::to_string(&request_fingerprint_payload)?;
    let ordered_results = matches
        .iter()
        .enumerate()
        .map(|(index, item)| {
            json!({
                "position": index + 1,
                "merchant_id": item.merchant.id.as_str(),
                "capability_key": item.capability.capability_key.as_str(),
                "capability_version": item.capability.version,
                "score": item.score,
                "access_level": item.capability.access_level.as_str(),
                "unit_price_micros": item.capability.unit_price_micros,
                "currency": item.capability.currency.as_str(),
                "directory_updated_at": item.capability.updated_at.as_str(),
                "source": &item.capability.source,
                "freshness": &item.capability.freshness
            })
        })
        .collect::<Vec<_>>();
    let payload = json!({
        "schema": "open_commerce.consumer_ranking_receipt_payload.v1",
        "generated_at": Utc::now().to_rfc3339(),
        "candidate_scope": {
            "kind": "current_operator_public_directory.v1",
            "operator_exhaustive": false,
            "candidate_cap": candidate_limit,
            "directory_candidate_count": directory_candidate_count
        },
        "ranking": {
            "policy": ranking_policy.key(),
            "label": ranking_policy.label(),
            "explanation": ranking_policy.explanation(),
            "paid_placement": false
        },
        "freshness_requirement": freshness_requirement(request.require_current_declaration),
        "source_requirement": source_requirement(request.require_internal_sync_receipt),
        "request_fingerprint_sha256": sha256_hex(&request_fingerprint_json),
        "eligible_match_count": eligible_match_count,
        "returned_match_count": matches.len(),
        "ordered_results": ordered_results
    });
    let canonical_payload_json = serde_json::to_string(&payload)?;
    Ok(ConsumerRankingReceipt {
        schema: "open_commerce.consumer_ranking_receipt.v1",
        hash_algorithm: "sha256",
        payload_sha256: sha256_hex(&canonical_payload_json),
        canonical_payload_json,
        signed_by_operator: false,
    })
}

fn normalized_optional(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn sha256_hex(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

pub(crate) fn create_authorization_request(
    store: &Store,
    user_id: &str,
    request: CreateAuthorizationRequest,
) -> Result<OpenCommerceAuthorizationRequest> {
    if request.requester_app_id.trim() == "pc-web" {
        bail!("公共 pc-web 身份只能用于发现，申请授权前请注册独立开发者应用");
    }
    ensure_app_owned_by_user(store, user_id, &request.requester_app_id)?;
    crate::open_commerce_app_block_service::ensure_app_allowed(
        store,
        &request.merchant_id,
        &request.requester_app_id,
        false,
    )?;
    store.create_open_commerce_authorization_request(user_id, request)
}

pub(crate) fn ensure_app_owned_by_user(store: &Store, user_id: &str, app_id: &str) -> Result<()> {
    if app_id.trim() == "pc-web" {
        return Ok(());
    }
    store.ensure_open_commerce_developer_app_owned_by_user(app_id, user_id)?;
    Ok(())
}

fn best_match(
    store: &Store,
    detail: OpenCommerceDirectoryMerchantDetail,
    request: &ConsumerDiscoveryRequest,
    ranking_policy: ConsumerRankingPolicy,
) -> Result<Option<ConsumerDiscoveryMatch>> {
    let candidates = detail
        .capabilities
        .iter()
        .filter(|capability| {
            request
                .capability_key
                .as_deref()
                .map(|key| capability.capability_key == key)
                .unwrap_or(true)
        })
        .filter(|capability| {
            request
                .preferences
                .max_unit_price_micros
                .map(|maximum| capability.unit_price_micros <= maximum)
                .unwrap_or(true)
        })
        .filter(|capability| {
            !request.require_current_declaration || capability.freshness.status == "current"
        })
        .filter(|capability| {
            !request.require_internal_sync_receipt
                || capability.source.kind == "integration_sync_receipt"
        })
        .collect::<Vec<_>>();
    let selected = ranking_policy
        .select_capability(candidates, &request.preferences, capability_score)
        .cloned();
    let Some(capability) = selected else {
        return Ok(None);
    };
    let (score, mut reasons) = score_match(&detail, &capability, &request.preferences);
    reasons.push(ranking_policy.ranking_reason());
    if request.require_current_declaration {
        reasons.push("符合消费者要求的商户声明有效期".to_string());
    }
    if request.require_internal_sync_receipt {
        reasons.push("已关联商户项目内部业务同步回执".to_string());
    }
    let authorization =
        authorization_state(store, &detail, &capability, &request.requester_app_id)?;
    Ok(Some(ConsumerDiscoveryMatch {
        merchant: detail.merchant,
        capability,
        score,
        reasons,
        authorization,
    }))
}

fn freshness_requirement(required: bool) -> &'static str {
    if required {
        "current_declaration"
    } else {
        "any_declaration"
    }
}

fn source_requirement(required: bool) -> &'static str {
    if required {
        "internal_sync_receipt"
    } else {
        "any_merchant_source"
    }
}

fn score_match(
    detail: &OpenCommerceDirectoryMerchantDetail,
    capability: &OpenCommerceDirectoryCapability,
    preferences: &ConsumerPreferences,
) -> (i64, Vec<String>) {
    let mut score = 40;
    let mut reasons = vec![format!("提供 {} 能力", capability.display_name)];
    let profile = &detail.merchant.public_profile;
    let category = profile.get("category").and_then(|value| value.as_str());
    if category
        .map(|value| contains_ignore_case(&preferences.categories, value))
        .unwrap_or(false)
    {
        score += 20;
        reasons.push(format!("经营类别匹配 {}", category.unwrap_or_default()));
    }
    let city = profile.get("city").and_then(|value| value.as_str());
    if city
        .zip(preferences.city.as_deref())
        .map(|(left, right)| left.eq_ignore_ascii_case(right))
        .unwrap_or(false)
    {
        score += 15;
        reasons.push(format!("所在城市匹配 {}", city.unwrap_or_default()));
    }
    let merchant_tags = profile
        .get("tags")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>();
    let tag_matches = merchant_tags
        .iter()
        .filter(|tag| contains_ignore_case(&preferences.tags, tag))
        .count();
    if tag_matches > 0 {
        let points = (tag_matches as i64 * 5).min(20);
        score += points;
        reasons.push(format!("匹配 {} 个偏好标签", tag_matches));
    }
    if preferences.prefer_public && capability.access_level == ACCESS_PUBLIC {
        score += 5;
        reasons.push("无需额外授权即可调用".to_string());
    }
    if capability.unit_price_micros == 0 {
        score += 3;
        reasons.push("当前能力调用价格为 0".to_string());
    }
    (score, reasons)
}

fn authorization_state(
    store: &Store,
    detail: &OpenCommerceDirectoryMerchantDetail,
    capability: &OpenCommerceDirectoryCapability,
    app_id: &str,
) -> Result<ConsumerAuthorizationState> {
    match capability.access_level.as_str() {
        ACCESS_PUBLIC => Ok(ConsumerAuthorizationState {
            required: false,
            status: "not_required".to_string(),
            grant_id: None,
            request_id: None,
        }),
        ACCESS_AUTHORIZED => {
            if app_id == "pc-web" {
                return Ok(ConsumerAuthorizationState {
                    required: true,
                    status: "app_registration_required".to_string(),
                    grant_id: None,
                    request_id: None,
                });
            }
            let grant_id = store.active_open_commerce_grant_for_app_capability(
                &detail.merchant.id,
                app_id,
                &capability.capability_key,
            )?;
            if grant_id.is_some() {
                return Ok(ConsumerAuthorizationState {
                    required: true,
                    status: "granted".to_string(),
                    grant_id,
                    request_id: None,
                });
            }
            let request_id = store.pending_authorization_for_app_capability(
                &detail.merchant.id,
                app_id,
                &capability.capability_key,
            )?;
            Ok(ConsumerAuthorizationState {
                required: true,
                status: if request_id.is_some() {
                    "pending"
                } else {
                    "request_required"
                }
                .to_string(),
                grant_id: None,
                request_id,
            })
        }
        _ => bail!("能力访问级别无效"),
    }
}

fn capability_score(
    capability: &OpenCommerceDirectoryCapability,
    preferences: &ConsumerPreferences,
) -> i64 {
    let access = match capability.access_level.as_str() {
        ACCESS_PUBLIC if preferences.prefer_public => 30,
        ACCESS_PUBLIC => 20,
        ACCESS_AUTHORIZED => 10,
        _ => 0,
    };
    access - capability.unit_price_micros.min(1_000_000) / 100_000
}

fn contains_ignore_case(values: &[String], needle: &str) -> bool {
    values
        .iter()
        .any(|value| value.trim().eq_ignore_ascii_case(needle.trim()))
}
