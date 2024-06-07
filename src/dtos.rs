#[derive(serde::Serialize)]
pub struct InitiatePaymentResult {
    pub session_id: String,
}

#[derive(serde::Serialize)]
pub struct StripeWebhookResult {
    pub received: bool,
}

#[derive(serde::Deserialize)]
pub struct CreatePaymentRequest {
    pub user_id: i32,
    pub amount: String,
}
