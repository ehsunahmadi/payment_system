use axum::{
    body::Body,
    extract::State,
    http::{header::HeaderMap, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use diesel::prelude::*;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use dotenvy::dotenv;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use stripe::{
    CheckoutSession, Client, CreateCheckoutSession, CreateCheckoutSessionLineItems,
    CreateCheckoutSessionLineItemsPriceData, CreateCheckoutSessionPaymentMethodTypes, Currency,
    EventObject, EventType, Webhook,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod dtos;
mod models;
mod schema;
// this embeds the migrations into the application binary
// the migration path is relative to the `CARGO_MANIFEST_DIR`
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

#[derive(Clone)]
struct AppState {
    pool: deadpool_diesel::sqlite::Pool,
    stripe_client: Arc<Client>,
    stripe_webhook_secret: String,
}

#[tokio::main]

async fn main() {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "example_tokio_sqlite=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url: String = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager =
        deadpool_diesel::sqlite::Manager::new(database_url, deadpool_diesel::Runtime::Tokio1);
    let pool = deadpool_diesel::sqlite::Pool::builder(manager)
        .build()
        .unwrap();

    {
        let conn = pool.get().await.unwrap();
        conn.interact(|conn| conn.run_pending_migrations(MIGRATIONS).map(|_| ()))
            .await
            .unwrap()
            .unwrap();
    }

    let stripe_token = env::var("STRIPE_SECRET_KEY").expect("STRIPE_SECRET_KEY must be set");

    let stripe_client: Arc<Client> = Arc::new(Client::new(stripe_token));

    let stripe_webhook_secret =
        env::var("STRIPE_WEBHOOK_SECRET").expect("STRIPE_WEBHOOK_SECRET must be set");

    let app_state = AppState {
        pool,
        stripe_client,
        stripe_webhook_secret,
    };

    let app = Router::new()
        .route("/user/list", get(list_users))
        .route("/user/create", post(create_user))
        .route("/payments/initiate", post(initiate_payment))
        .route("/webhook", post(handle_webhook))
        .with_state(app_state.clone());

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn create_user(
    State(state): State<AppState>,
    Json(new_user): Json<models::NewUser>,
) -> Result<Json<models::User>, (StatusCode, String)> {
    let conn = state.pool.get().await.map_err(internal_error)?;
    let res = conn
        .interact(|conn| {
            diesel::insert_into(schema::users::table)
                .values(new_user)
                .returning(models::User::as_returning())
                .get_result(conn)
        })
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;
    Ok(Json(res))
}

async fn list_users(
    State(state): State<AppState>,
) -> Result<Json<Vec<models::User>>, (StatusCode, String)> {
    let conn = state.pool.get().await.map_err(internal_error)?;
    let res = conn
        .interact(|conn| {
            schema::users::table
                .select(models::User::as_select())
                .load(conn)
        })
        .await
        .map_err(internal_error)?
        .map_err(internal_error)?;
    Ok(Json(res))
}

async fn initiate_payment(
    State(state): State<AppState>,
    Json(payment_request): Json<dtos::CreatePaymentRequest>,
) -> Result<Json<dtos::InitiatePaymentResult>, (StatusCode, String)> {
    let session = CheckoutSession::create(
        &state.stripe_client,
        CreateCheckoutSession {
            payment_method_types: Some(vec![CreateCheckoutSessionPaymentMethodTypes::Card]),
            line_items: Some(vec![CreateCheckoutSessionLineItems {
                price_data: Some(CreateCheckoutSessionLineItemsPriceData {
                    currency: Currency::USD,
                    ..Default::default()
                }),
                price: Some(payment_request.amount.clone()),
                ..Default::default()
            }]),
            mode: Some(stripe::CheckoutSessionMode::Payment),

            ..Default::default()
        },
    )
    .await
    .unwrap();

    let payment = models::NewPayment {
        session_id: session.id.to_string(),
        user_id: payment_request.user_id,
        amount: payment_request.amount.clone(),
    };

    let conn = state.pool.get().await.map_err(internal_error)?;
    conn.interact(|conn| {
        diesel::insert_into(schema::payments::table)
            .values(payment)
            .returning(models::Payment::as_returning())
            .get_result(conn)
    })
    .await
    .map_err(internal_error)?
    .map_err(internal_error)?;

    Ok(Json(dtos::InitiatePaymentResult {
        session_id: session.id.to_string(),
    }))
}

async fn handle_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Body,
) -> Result<Json<dtos::StripeWebhookResult>, (StatusCode, String)> {
    let bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err(internal_error)?; // (1
    let body_str = std::str::from_utf8(&bytes).unwrap();
    let sig_header = headers
        .get("Stripe-Signature")
        .ok_or((
            StatusCode::BAD_REQUEST,
            "Missing Stripe-Signature header".to_string(),
        ))?
        .to_str()
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "Invalid Stripe-Signature header".to_string(),
            )
        })?;
    let event = Webhook::construct_event(body_str, &sig_header, &state.stripe_webhook_secret)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    match event.type_ {
        EventType::CheckoutSessionCompleted => {
            match event.data.object {
                EventObject::CheckoutSession(session) => {
                    let conn = state.pool.get().await.map_err(internal_error)?;
                    let _res = conn
                        .interact(|conn| {
                            let session_id: stripe::CheckoutSessionId = session.id;

                            //fetch associated payment
                            let payment = schema::payments::table.filter(
                                schema::payments::session_id
                                    .clone()
                                    .eq(session_id.to_string()),
                            );
                            let payment = payment.load::<models::Payment>(conn).unwrap();
                            let payment = payment.first().unwrap();

                            //update payment status
                            diesel::update(
                                schema::payments::table.filter(
                                    schema::payments::session_id
                                        .clone()
                                        .eq(session_id.to_string()),
                                ),
                            )
                            .set(schema::payments::status.eq("completed"))
                            .execute(conn)
                            .unwrap();

                            //update user balance
                            let user_id = payment.user_id;
                            let user =
                                schema::users::table.filter(schema::users::id.clone().eq(user_id));
                            let user = user.load::<models::User>(conn).unwrap();
                            let user = user.first().unwrap();
                            let balance = user.balance.parse::<i32>().unwrap();
                            let new_balance = balance + payment.amount.parse::<i32>().unwrap();
                            diesel::update(
                                schema::users::table.filter(schema::users::id.clone().eq(user_id)),
                            )
                            .set(schema::users::balance.eq(new_balance.to_string()))
                            .execute(conn)
                            .unwrap();
                        })
                        .await;
                }
                _ => {
                    println!("Unexpected object type for CheckoutSessionCompleted event");
                }
            }

            // Call a method to handle the successful payment intent
            // handle_payment_intent_succeeded(payment_intent);
        }

        _ => {
            println!("Unhandled event type: {}", event.type_);
        }
    }

    Ok(Json(dtos::StripeWebhookResult { received: true }))
}

/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
