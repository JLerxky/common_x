pub use axum;

use std::{future::Future, net::SocketAddr, path::PathBuf, time::Duration};

use axum::{
    async_trait,
    extract::Host,
    handler::HandlerWithoutStateExt,
    http::{StatusCode, Uri},
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use color_eyre::eyre::{eyre, Error, Result};
use serde::Serialize;
use serde_json::json;
use tokio::{net::TcpListener, signal};
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use tracing::info;

use crate::signal::waiting_for_shutdown;

#[derive(Debug)]
pub struct RESTfulError {
    code: u16,
    err: Error,
}

impl IntoResponse for RESTfulError {
    fn into_response(self) -> Response {
        (
            StatusCode::from_u16(self.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            json!({
                "code": self.code,
                "message": self.err.to_string(),
            })
            .to_string(),
        )
            .into_response()
    }
}

impl<E> From<E> for RESTfulError
where
    E: Into<Error>,
{
    fn from(err: E) -> Self {
        Self {
            code: 500,
            err: err.into(),
        }
    }
}

async fn health() -> Result<impl IntoResponse, RESTfulError> {
    ok_simple()
}

pub async fn http_serve(port: u16, router: Router) -> Result<()> {
    let app = router
        .route("/health", get(health))
        .layer((
            TraceLayer::new_for_http(),
            // Graceful shutdown will wait for outstanding requests to complete. Add a timeout so
            // requests don't hang forever.
            TimeoutLayer::new(Duration::from_secs(10)),
        ))
        .fallback(|| async {
            (
                StatusCode::NOT_FOUND,
                json!({ "code": 404, "message": "Not Found" }).to_string(),
            )
                .into_response()
        });

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;

    info!("listening on 0.0.0.0:{port}");

    axum::serve(listener, app)
        .with_graceful_shutdown(waiting_for_shutdown())
        .await?;
    Ok(())
}

pub async fn https_serve(
    http_port: u16,
    https_port: u16,
    router: Router,
    cert_path: &str,
    key_path: &str,
) -> Result<()> {
    let handle = axum_server::Handle::new();
    let shutdown_future = shutdown_signal(handle.clone());
    tokio::spawn(redirect_http_to_https(
        http_port,
        https_port,
        shutdown_future,
    ));

    let config =
        RustlsConfig::from_pem_file(PathBuf::from(cert_path), PathBuf::from(key_path)).await?;

    let app = router.route("/health", get(health)).fallback(|| async {
        (
            StatusCode::NOT_FOUND,
            json!({ "code": 404, "message": "Not Found" }).to_string(),
        )
            .into_response()
    });

    let addr = SocketAddr::from(([0, 0, 0, 0], https_port));
    info!("listening on https {addr}");
    axum_server::bind_rustls(addr, config)
        .handle(handle)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct RESTfulResponse<T: Serialize> {
    code: u16,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
}

unsafe impl<T: Serialize> Send for RESTfulResponse<T> {}

#[async_trait]
impl<T: Serialize> IntoResponse for RESTfulResponse<T> {
    fn into_response(self) -> Response {
        (
            StatusCode::from_u16(self.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            if let Some(data) = self.data {
                json!({
                    "code": self.code,
                    "message": self.message,
                    "data": data,
                })
                .to_string()
            } else {
                json!({
                    "code": self.code,
                    "message": self.message,
                })
                .to_string()
            },
        )
            .into_response()
    }
}

pub fn ok<T: Serialize>(data: T) -> Result<impl IntoResponse, RESTfulError> {
    Ok(RESTfulResponse {
        code: 200,
        message: "OK".to_string(),
        data: Some(data),
    })
}

pub fn ok_simple() -> Result<impl IntoResponse, RESTfulError> {
    Ok(RESTfulResponse::<()> {
        code: 200,
        message: "OK".to_string(),
        data: None,
    })
}

pub fn err(code: u16, message: String) -> RESTfulError {
    RESTfulError {
        code,
        err: eyre!(message),
    }
}

async fn shutdown_signal(handle: axum_server::Handle) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Received termination signal shutting down");
    handle.graceful_shutdown(Some(Duration::from_secs(10))); // 10 secs is how long docker will wait
}

async fn redirect_http_to_https<F>(http_port: u16, https_port: u16, signal: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    fn make_https(host: String, uri: Uri, http_port: u16, https_port: u16) -> Result<Uri> {
        let mut parts = uri.into_parts();

        parts.scheme = Some(axum::http::uri::Scheme::HTTPS);

        if parts.path_and_query.is_none() {
            parts.path_and_query = Some("/".parse().unwrap());
        }

        let https_host = host.replace(&http_port.to_string(), &https_port.to_string());
        parts.authority = Some(https_host.parse()?);

        Ok(Uri::from_parts(parts)?)
    }

    let redirect = move |Host(host): Host, uri: Uri| async move {
        match make_https(host, uri, http_port, https_port) {
            Ok(uri) => Ok(Redirect::permanent(&uri.to_string())),
            Err(error) => {
                tracing::warn!(%error, "failed to convert URI to HTTPS");
                Err(StatusCode::BAD_REQUEST)
            }
        }
    };

    let addr = SocketAddr::from(([0, 0, 0, 0], http_port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::debug!("listening on {addr}");
    axum::serve(listener, redirect.into_make_service())
        .with_graceful_shutdown(signal)
        .await
        .unwrap();
}
