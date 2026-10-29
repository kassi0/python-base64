mod auto_decode;
mod b64;
mod diff;
mod handlers;

use std::net::SocketAddr;
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tracing initialization
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,rust_base64=debug".into()),
        )
        .init();

    // Environment variables
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let max_mb: usize = std::env::var("MAX_MB")
        .ok()
        .and_then(|m| m.parse().ok())
        .unwrap_or(10);

    let max_bytes = max_mb * 1024 * 1024;

    // CORS Layer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build Axum Router
    let app = Router::new()
        .route("/", get(handlers::index_handler))
        .route("/diff", get(handlers::index_handler))
        .route("/static/style.css", get(handlers::style_css_handler))
        .route("/static/app.js", get(handlers::app_js_handler))
        .route("/favicon.ico", get(handlers::favicon_handler))
        .route("/health", get(handlers::health_handler))
        .route("/api/health", get(handlers::health_handler))
        .route("/process", post(handlers::process_handler))
        .route("/download_text", post(handlers::download_text_handler))
        .route("/download_binary", post(handlers::download_binary_handler))
        .route("/api/encode", post(handlers::api_encode_handler))
        .route("/api/decode", post(handlers::api_decode_handler))
        .route("/api/auto-decode", post(handlers::api_auto_decode_handler))
        .route("/api/secret-decode", post(handlers::api_auto_decode_handler))
        .route("/api/auto-encode", post(handlers::api_auto_encode_handler))
        .route("/api/secret-encode", post(handlers::api_auto_encode_handler))
        .route("/api/diff", post(handlers::api_diff_handler))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(RequestBodyLimitLayer::new(max_bytes));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("🦀 Base64 Studio (Rust Edition) rodando em http://0.0.0.0:{} (limite: {} MB)", port, max_mb);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("falha ao registrar listener de Ctrl+C");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("falha ao registrar listener de SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Recebido sinal de encerramento, finalizando servidor graciosamente...");
}
