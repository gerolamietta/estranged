use axum::{
    Json, Router,
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::IntoResponse,
    routing::post,
};
use axum_extra::{TypedHeader, typed_header::TypedHeaderRejection};
use estranged_headers::MaxBotApiSecret;
use estranged_types::{Secret, Update};

async fn authify(
    expected: State<Secret>,
    got: Result<TypedHeader<MaxBotApiSecret>, TypedHeaderRejection>,
    request: Request,
    next: Next,
) -> impl IntoResponse {
    match got {
        Ok(secret) if secret.0.0 == expected.0 => next.run(request).await,
        _ => StatusCode::UNAUTHORIZED.into_response(),
    }
}

pub fn router<
    S: 'static + Send + Sync + Clone,
    F: 'static + Send + Sync + Clone + Fn(Update) -> Fut,
    Fut: Send + Future<Output = ()>,
>(
    secret: Secret,
    callback: F,
) -> Router<S> {
    Router::new().route(
        "/",
        post(async move |Json(update)| {
            callback(update).await;
        })
        .layer(middleware::from_fn_with_state(secret, authify)),
    )
}
