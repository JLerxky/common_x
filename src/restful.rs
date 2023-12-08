use color_eyre::eyre::{eyre, Error};
use salvo::{catcher::Catcher, prelude::*};
use serde::Serialize;
use serde_json::json;
use tracing::info;

pub type HttpServerHandle = salvo::server::ServerHandle;

#[derive(Debug)]
pub struct RESTfulError {
    code: u16,
    err: Error,
}

#[async_trait]
impl Writer for RESTfulError {
    async fn write(mut self, _req: &mut Request, _depot: &mut Depot, res: &mut Response) {
        res.status_code(
            StatusCode::from_u16(self.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        );
        res.render(Json(json!({
            "code": self.code,
            "message": self.err.to_string(),
        })));
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

#[handler]
async fn handle_http_error(
    &self,
    _req: &Request,
    _depot: &Depot,
    res: &mut Response,
    ctrl: &mut FlowCtrl,
) {
    if let Some(status_code) = res.status_code {
        match status_code {
            StatusCode::OK | StatusCode::INTERNAL_SERVER_ERROR => {}
            _ => {
                res.render(Json(json!({
                    "code": status_code.as_u16(),
                    "message": status_code.canonical_reason().unwrap_or_default(),
                })));
            }
        }
        ctrl.skip_rest();
    }
}

#[handler]
async fn health() -> impl Writer {
    ok_no_data()
}

pub async fn http_serve(service_name: &str, port: u16, router: Router) -> HttpServerHandle {
    let router = router.push(Router::with_path("health").get(health));

    let doc = OpenApi::new(format!("{} api", service_name), "0.0.1").merge_router(&router);

    let router = router
        .unshift(doc.into_router("/api-doc/openapi.json"))
        .unshift(SwaggerUi::new("/api-doc/openapi.json").into_router("swagger-ui"));

    let service = Service::new(router).catcher(Catcher::default().hoop(handle_http_error));

    let acceptor = TcpListener::new(format!("0.0.0.0:{}", port)).bind().await;

    let server = Server::new(acceptor);
    let handle = server.handle();
    server.serve(service).await;

    info!("{service_name} listening on 0.0.0.0:{port}");
    handle
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
impl<T: Serialize> Writer for RESTfulResponse<T> {
    async fn write(mut self, _req: &mut Request, _depot: &mut Depot, res: &mut Response) {
        res.status_code(
            StatusCode::from_u16(self.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        );
        if let Some(data) = self.data {
            res.render(Json(json!({
                "code": self.code,
                "message": self.message,
                "data": data,
            })));
        } else {
            res.render(Json(json!({
                "code": self.code,
                "message": self.message,
            })));
        }
    }
}

pub fn ok<T: Serialize>(data: T) -> Result<impl Writer, RESTfulError> {
    Ok(RESTfulResponse {
        code: 200,
        message: "OK".to_string(),
        data: Some(data),
    })
}

pub fn ok_no_data() -> Result<impl Writer, RESTfulError> {
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
