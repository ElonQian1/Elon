use crate::{
    open_commerce_developer_model::{DecideAuthorizationRequest, OpenCommerceAuthorizationRequest},
    open_commerce_model::CreateGrantRequest,
};

pub(crate) fn grant_request_for_authorization(
    request: &OpenCommerceAuthorizationRequest,
    decision: &DecideAuthorizationRequest,
) -> CreateGrantRequest {
    CreateGrantRequest {
        merchant_id: request.merchant_id.clone(),
        grantee_app_id: request.requester_app_id.clone(),
        scopes: request.scopes.clone(),
        purpose: request.purpose.clone(),
        expires_at: decision.expires_at.clone(),
        max_invocations: decision.max_invocations,
        max_amount_micros: decision.max_amount_micros,
        budget_currency: decision.budget_currency.clone(),
    }
}
