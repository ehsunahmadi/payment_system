use axum::{
    extract::State,
    http::StatusCode,
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
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod models;
mod schema;
// this embeds the migrations into the application binary
// the migration path is relative to the `CARGO_MANIFEST_DIR`
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

#[derive(Clone)]
struct AppState {
    pool: deadpool_diesel::sqlite::Pool,
    stripe_client: Arc<Client>,
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

    let app_state = AppState {
        pool,
        stripe_client,
    };

    let app = Router::new()
        .route("/user/list", get(list_users))
        .route("/user/create", post(create_user))
        .route("/payments/initiate", post(initiate_payment))
        .with_state(app_state.clone());

    // run it with hyper
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
    Json(new_payment): Json<models::NewPayment>,
) -> Result<Json<models::InitiatePaymentResult>, (StatusCode, String)> {
    let session = CheckoutSession::create(
        &state.stripe_client,
        CreateCheckoutSession {
            payment_method_types: Some(vec![CreateCheckoutSessionPaymentMethodTypes::Card]),
            line_items: Some(vec![CreateCheckoutSessionLineItems {
                price_data: Some(CreateCheckoutSessionLineItemsPriceData {
                    currency: Currency::USD,
                    ..Default::default()
                }),
                price: Some(new_payment.amount.clone()),
                ..Default::default()
            }]),
            mode: Some(stripe::CheckoutSessionMode::Payment),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let conn = state.pool.get().await.map_err(internal_error)?;
    conn.interact(|conn| {
        diesel::insert_into(schema::payments::table)
            .values(new_payment)
            .returning(models::Payment::as_returning())
            .get_result(conn)
    })
    .await
    .map_err(internal_error)?
    .map_err(internal_error)?;

    Ok(Json(models::InitiatePaymentResult {
        session_id: session.id.to_string(),
    }))
}

/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
