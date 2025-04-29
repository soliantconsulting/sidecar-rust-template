use async_trait::async_trait;
use lambda_http::http::{header, StatusCode};
use lambda_http::{run, service_fn, tracing, Body, IntoResponse, Request, Response};
use serde::{Deserialize, Serialize, Serializer};
use serde_path_to_error::{Path, Segment};
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HttpErrorSource {
    Pointer(String),
    Parameter(String),
    Header(String),
}

impl From<&Path> for HttpErrorSource {
    fn from(path: &Path) -> Self {
        let mut pointer = String::new();

        for segment in path.iter() {
            match segment {
                Segment::Seq { index } => pointer.push_str(&format!("/{}", index)),
                Segment::Map { key } | Segment::Enum { variant: key } => {
                    pointer.push_str(&format!("/{}", key))
                }
                Segment::Unknown => continue,
            }
        }

        HttpErrorSource::Pointer(pointer)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpErrorResponse {
    #[serde(serialize_with = "status_code_as_string")]
    status: StatusCode,
    code: String,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<HttpErrorSource>,
}

fn status_code_as_string<S: Serializer>(
    status: &StatusCode,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(status.as_str())
}

impl HttpErrorResponse {
    pub fn new(status: StatusCode, code: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            status,
            code: code.into(),
            title: title.into(),
            detail: None,
            source: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn with_source(mut self, source: HttpErrorSource) -> Self {
        self.source = Some(source);
        self
    }

    pub fn boxed(self) -> Box<Self> {
        Box::new(self)
    }
}

pub type ResponseFuture = Pin<Box<dyn Future<Output = Response<Body>> + Send>>;

impl IntoResponse for HttpErrorResponse {
    fn into_response(self) -> ResponseFuture {
        (
            self.status,
            serde_json::to_value(self).expect("Failed to convert error to JSON"),
        )
            .into_response()
    }
}

#[derive(Error, Debug)]
pub enum ParseBodyError {
    #[error("Content type header must be application/json or application/x-www-form-urlencoded")]
    UnsupportedContentType,
    #[error("Invalid content type header")]
    InvalidContentType,
    #[error("Missing content type header")]
    MissingContentType,
    #[error("Invalid JSON body")]
    InvalidJsonBody(#[source] serde_path_to_error::Error<serde_json::Error>),
    #[error("Invalid form body")]
    InvalidFormBody(#[source] serde_path_to_error::Error<serde_html_form::de::Error>),
}

impl From<ParseBodyError> for HttpErrorResponse {
    fn from(error: ParseBodyError) -> Self {
        match error {
            ParseBodyError::UnsupportedContentType => HttpErrorResponse::new(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "unsupported content type",
                "Unsupported content type",
            )
            .with_detail(error.to_string())
            .with_source(HttpErrorSource::Header("Content-Type".to_string())),
            ParseBodyError::InvalidContentType => HttpErrorResponse::new(
                StatusCode::BAD_REQUEST,
                "invalid_content_type",
                "Invalid content type",
            )
            .with_detail(error.to_string())
            .with_source(HttpErrorSource::Header("Content-Type".to_string())),
            ParseBodyError::MissingContentType => HttpErrorResponse::new(
                StatusCode::BAD_REQUEST,
                "missing_content_type",
                "Missing content type",
            )
            .with_detail(error.to_string())
            .with_source(HttpErrorSource::Header("Content-Type".to_string())),
            ParseBodyError::InvalidJsonBody(source) => {
                let mut http_error = HttpErrorResponse::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "invalid_body",
                    "Invalid body",
                )
                .with_detail(source.to_string());

                if source.path().iter().next().is_some() {
                    http_error = http_error.with_source(source.path().into());
                }

                http_error
            }
            ParseBodyError::InvalidFormBody(source) => {
                let mut http_error = HttpErrorResponse::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "invalid_body",
                    "Invalid body",
                )
                .with_detail(source.to_string());

                if source.path().iter().next().is_some() {
                    http_error = http_error.with_source(source.path().into());
                }

                http_error
            }
        }
    }
}

pub fn parse_body<'a, T: Deserialize<'a>>(request: &'a Request) -> Result<T, ParseBodyError> {
    let mime_type = match request.headers().get(header::CONTENT_TYPE) {
        Some(ct) => match ct.to_str() {
            Ok(s) => mime::Mime::from_str(s).map_err(|_| ParseBodyError::InvalidContentType)?,
            Err(_) => return Err(ParseBodyError::InvalidContentType),
        },
        None => mime::APPLICATION_JSON,
    };

    match (mime_type.type_(), mime_type.subtype()) {
        (mime::APPLICATION, mime::JSON) => {
            let deserializer = &mut serde_json::Deserializer::from_slice(request.body().as_ref());
            let result: Result<T, _> = serde_path_to_error::deserialize(deserializer);

            result.map_err(ParseBodyError::InvalidJsonBody)
        }
        (mime::APPLICATION, mime::WWW_FORM_URLENCODED) => {
            let deserializer = serde_html_form::Deserializer::from_bytes(request.body().as_ref());
            let result: Result<T, _> = serde_path_to_error::deserialize(deserializer);

            result.map_err(ParseBodyError::InvalidFormBody)
        }
        _ => Err(ParseBodyError::InvalidContentType),
    }
}

#[derive(Debug)]
pub enum HttpError {
    Server(anyhow::Error),
    Client(Box<HttpErrorResponse>),
}

impl<E> From<E> for HttpError
where
    E: Into<anyhow::Error>,
    Result<(), E>: anyhow::Context<(), E>,
{
    fn from(error: E) -> Self {
        HttpError::Server(error.into())
    }
}

pub fn e500<T>(error: T) -> HttpError
where
    T: Into<anyhow::Error>,
{
    HttpError::Server(error.into())
}

pub fn e400<T>(error: T) -> HttpError
where
    T: Into<HttpErrorResponse>,
{
    HttpError::Client(Box::new(error.into()))
}

pub fn json_response(
    status: StatusCode,
    body: impl Serialize,
) -> Result<Response<Body>, anyhow::Error> {
    let response = Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::Text(serde_json::to_string(&body)?))?;

    Ok(response)
}

pub fn empty_response(status: StatusCode) -> Result<Response<Body>, anyhow::Error> {
    let response = Response::builder().status(status).body(Body::Empty)?;

    Ok(response)
}

#[async_trait]
pub trait HttpHandler {
    async fn handle(&self, request: Request) -> Result<Response<Body>, HttpError>;
}

async fn handle_request<T>(request: Request, handler: &T) -> Result<Response<Body>, Infallible>
where
    T: HttpHandler + Sync + Send + 'static,
{
    match handler.handle(request).await {
        Ok(response) => Ok(response.into_response().await),
        Err(HttpError::Client(error)) => Ok(error.into_response().await),
        Err(HttpError::Server(error)) => {
            tracing::error!("Internal server error: {:#}", error);

            Ok(HttpErrorResponse::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_server_error",
                "Internal server error",
            )
            .into_response()
            .await)
        }
    }
}

pub async fn start_http_service<T>(handler: T) -> Result<(), lambda_http::Error>
where
    T: HttpHandler + Send + Sync + 'static,
{
    let handler_ref = &handler;

    run(service_fn(move |request| {
        handle_request(request, handler_ref)
    }))
    .await?;

    Ok(())
}
