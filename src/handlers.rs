use axum::{
    body::Bytes,
    extract::Query,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
    Json,
};
use serde::Deserialize;

use crate::auto_decode::{auto_decode_text, auto_encode_text};
use crate::b64::{self, detect_file_type, guess_download_name, strip_data_url_prefix};

// Assets embedded at compile time for zero external dependency runtime
pub const INDEX_HTML: &str = include_str!("assets/index.html");
pub const STYLE_CSS: &str = include_str!("assets/style.css");
pub const APP_JS: &str = include_str!("assets/app.js");
pub const FAVICON_BYTES: &[u8] = include_bytes!("../favicon.ico");

/// GET / - Serve the single-page application
pub async fn index_handler() -> Html<&'static str> {
    Html(INDEX_HTML)
}

/// GET /static/style.css
pub async fn style_css_handler() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], STYLE_CSS)
}

/// GET /static/app.js
pub async fn app_js_handler() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "application/javascript; charset=utf-8")], APP_JS)
}

/// GET /favicon.ico
pub async fn favicon_handler() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "image/x-icon")], FAVICON_BYTES)
}

/// GET /health
pub async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "runtime": "rust-axum",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

#[derive(Debug, Deserialize)]
pub struct EncodeRequest {
    pub data: Option<String>,
    #[serde(default)]
    pub urlsafe: bool,
}

#[derive(Debug, Deserialize)]
pub struct DecodeRequest {
    pub data: Option<String>,
    #[serde(default)]
    pub urlsafe: bool,
}

#[derive(Debug, Deserialize)]
pub struct QueryOptions {
    pub urlsafe: Option<String>,
}

fn is_urlsafe_param(opt: Option<String>, default: bool) -> bool {
    match opt {
        Some(val) => {
            let v = val.to_lowercase();
            v == "1" || v == "true" || v == "yes"
        }
        None => default,
    }
}

/// POST /api/encode
pub async fn api_encode_handler(
    headers: HeaderMap,
    Query(query): Query<QueryOptions>,
    body: Bytes,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Case 1: Multipart form (file upload)
    if content_type.starts_with("multipart/form-data") {
        let boundary = multer::parse_boundary(content_type)
            .map_err(|e| (StatusCode::BAD_REQUEST, Json(serde_json::json!({"ok": false, "error": e.to_string()}))))?;
        
        let stream = futures_util::stream::once(async move { Result::<_, std::io::Error>::Ok(body) });
        let mut multipart = multer::Multipart::new(stream, boundary);
        let mut file_bytes = None;
        let mut file_name = None;
        let mut form_data_text = None;
        let mut urlsafe_form = None;

        while let Ok(Some(field)) = multipart.next_field().await {
            let name = field.name().unwrap_or("").to_string();
            if name == "file" || name == "input_file" {
                if let Some(fname) = field.file_name() {
                    file_name = Some(fname.to_string());
                }
                if let Ok(bytes) = field.bytes().await {
                    file_bytes = Some(bytes.to_vec());
                }
            } else if name == "data" || name == "input_text" {
                if let Ok(text) = field.text().await {
                    form_data_text = Some(text);
                }
            } else if name == "urlsafe" {
                if let Ok(val) = field.text().await {
                    urlsafe_form = Some(val);
                }
            }
        }

        let urlsafe = is_urlsafe_param(urlsafe_form, is_urlsafe_param(query.urlsafe, false));

        if let Some(bytes) = file_bytes {
            if !bytes.is_empty() {
                let encoded = b64::encode(&bytes, urlsafe);
                return Ok(Json(serde_json::json!({
                    "ok": true,
                    "mode": "file",
                    "filename": file_name.unwrap_or_else(|| "arquivo".to_string()),
                    "base64": encoded,
                    "urlsafe": urlsafe
                })));
            }
        }

        if let Some(text) = form_data_text {
            if !text.is_empty() {
                let encoded = b64::encode(text.as_bytes(), urlsafe);
                return Ok(Json(serde_json::json!({
                    "ok": true,
                    "mode": "text",
                    "base64": encoded,
                    "urlsafe": urlsafe
                })));
            }
        }

        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"ok": false, "error": "Nenhum arquivo ou texto enviado."})),
        ));
    }

    // Case 2: JSON payload
    if let Ok(req) = serde_json::from_slice::<EncodeRequest>(&body) {
        if let Some(data) = req.data {
            let urlsafe = req.urlsafe || is_urlsafe_param(query.urlsafe, false);
            let encoded = b64::encode(data.as_bytes(), urlsafe);
            return Ok(Json(serde_json::json!({
                "ok": true,
                "mode": "text",
                "base64": encoded,
                "urlsafe": urlsafe
            })));
        }
    }

    // Case 3: Form urlencoded
    if let Ok(pairs) = serde_urlencoded::from_bytes::<Vec<(String, String)>>(&body) {
        let mut data_text = None;
        let mut urlsafe_val = false;
        for (k, v) in pairs {
            if k == "data" || k == "input_text" {
                data_text = Some(v);
            } else if k == "urlsafe" {
                urlsafe_val = is_urlsafe_param(Some(v), false);
            }
        }
        if let Some(text) = data_text {
            let urlsafe = urlsafe_val || is_urlsafe_param(query.urlsafe, false);
            let encoded = b64::encode(text.as_bytes(), urlsafe);
            return Ok(Json(serde_json::json!({
                "ok": true,
                "mode": "text",
                "base64": encoded,
                "urlsafe": urlsafe
            })));
        }
    }

    Err((
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"ok": false, "error": "Faltou 'data' (texto) ou 'file'."})),
    ))
}

/// POST /api/decode
pub async fn api_decode_handler(
    headers: HeaderMap,
    Query(query): Query<QueryOptions>,
    body: Bytes,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Case 1: Multipart form (file upload)
    if content_type.starts_with("multipart/form-data") {
        let boundary = multer::parse_boundary(content_type)
            .map_err(|e| (StatusCode::BAD_REQUEST, Json(serde_json::json!({"ok": false, "error": e.to_string()}))))?;
        
        let stream = futures_util::stream::once(async move { Result::<_, std::io::Error>::Ok(body) });
        let mut multipart = multer::Multipart::new(stream, boundary);
        let mut file_bytes = None;
        let mut form_data_text = None;
        let mut urlsafe_form = None;

        while let Ok(Some(field)) = multipart.next_field().await {
            let name = field.name().unwrap_or("").to_string();
            if name == "file" || name == "input_file" {
                if let Ok(bytes) = field.bytes().await {
                    file_bytes = Some(bytes.to_vec());
                }
            } else if name == "data" || name == "input_text" {
                if let Ok(text) = field.text().await {
                    form_data_text = Some(text);
                }
            } else if name == "urlsafe" {
                if let Ok(val) = field.text().await {
                    urlsafe_form = Some(val);
                }
            }
        }

        let urlsafe = is_urlsafe_param(urlsafe_form, is_urlsafe_param(query.urlsafe, false));

        let raw_to_decode = if let Some(bytes) = file_bytes {
            String::from_utf8_lossy(&bytes).to_string()
        } else if let Some(text) = form_data_text {
            text
        } else {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"ok": false, "error": "Nenhum arquivo ou texto enviado."})),
            ));
        };

        return process_decode_string(&raw_to_decode, urlsafe);
    }

    // Case 2: JSON payload
    if let Ok(req) = serde_json::from_slice::<DecodeRequest>(&body) {
        if let Some(data) = req.data {
            let urlsafe = req.urlsafe || is_urlsafe_param(query.urlsafe, false);
            return process_decode_string(&data, urlsafe);
        }
    }

    // Case 3: Form urlencoded
    if let Ok(pairs) = serde_urlencoded::from_bytes::<Vec<(String, String)>>(&body) {
        let mut data_text = None;
        let mut urlsafe_val = false;
        for (k, v) in pairs {
            if k == "data" || k == "input_text" {
                data_text = Some(v);
            } else if k == "urlsafe" {
                urlsafe_val = is_urlsafe_param(Some(v), false);
            }
        }
        if let Some(text) = data_text {
            let urlsafe = urlsafe_val || is_urlsafe_param(query.urlsafe, false);
            return process_decode_string(&text, urlsafe);
        }
    }

    Err((
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"ok": false, "error": "Faltou 'data' (Base64) ou 'file'."})),
    ))
}

fn process_decode_string(
    input: &str,
    urlsafe: bool,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let (data_part, mime_hint) = strip_data_url_prefix(input);
    let decoded_bytes = match b64::decode(data_part, urlsafe) {
        Ok(bytes) => bytes,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"ok": false, "error": e})),
            ));
        }
    };

    // Try decoding as valid UTF-8 text first
    if let Ok(text) = String::from_utf8(decoded_bytes.clone()) {
        return Ok(Json(serde_json::json!({
            "ok": true,
            "mode": "text",
            "text": text,
            "urlsafe": urlsafe
        })));
    }

    // Binary content detected
    let file_info = detect_file_type(&decoded_bytes);
    let mut mime_str = file_info.mime.to_string();
    let mut ext_str = file_info.extension.to_string();
    let mut label_str = file_info.label.to_string();

    if let Some(hint) = mime_hint {
        mime_str = hint.clone();
        label_str = hint.to_uppercase();
        if hint == "application/pdf" {
            ext_str = ".pdf".to_string();
        } else if hint == "image/png" {
            ext_str = ".png".to_string();
        } else if hint == "image/jpeg" {
            ext_str = ".jpg".to_string();
        } else if hint == "image/gif" {
            ext_str = ".gif".to_string();
        } else if hint == "image/webp" {
            ext_str = ".webp".to_string();
        }
    }

    let b64_safe = b64::encode(&decoded_bytes, false);

    Ok(Json(serde_json::json!({
        "ok": true,
        "mode": "binary",
        "note": "Conteúdo não é texto UTF-8; codificado em Base64 para tráfego seguro.",
        "decoded_base64": b64_safe,
        "mime": mime_str,
        "extension": ext_str,
        "label": label_str,
        "is_image": file_info.is_image,
        "is_audio": file_info.is_audio,
        "size_bytes": decoded_bytes.len(),
        "urlsafe": urlsafe
    })))
}

#[derive(Debug, Deserialize)]
pub struct DownloadTextForm {
    pub filename: Option<String>,
    pub content: Option<String>,
}

/// POST /download_text
pub async fn download_text_handler(
    _headers: HeaderMap,
    body: Bytes,
) -> Result<Response, (StatusCode, &'static str)> {
    let mut filename = "resultado.txt".to_string();
    let mut content = String::new();

    if let Ok(req) = serde_json::from_slice::<DownloadTextForm>(&body) {
        if let Some(f) = req.filename {
            filename = f;
        }
        if let Some(c) = req.content {
            content = c;
        }
    } else if let Ok(pairs) = serde_urlencoded::from_bytes::<Vec<(String, String)>>(&body) {
        for (k, v) in pairs {
            if k == "filename" && !v.is_empty() {
                filename = v;
            } else if k == "content" {
                content = v;
            }
        }
    }

    let disposition = format!("attachment; filename=\"{}\"", filename);
    let mut res = Response::new(content.into());
    res.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    res.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&disposition).unwrap_or(HeaderValue::from_static("attachment")),
    );
    Ok(res)
}

#[derive(Debug, Deserialize)]
pub struct DownloadBinaryForm {
    pub filename: Option<String>,
    pub content_b64: Option<String>,
    pub mime: Option<String>,
}

/// POST /download_binary
pub async fn download_binary_handler(
    _headers: HeaderMap,
    body: Bytes,
) -> Result<Response, (StatusCode, &'static str)> {
    let mut filename = "resultado.bin".to_string();
    let mut content_b64 = String::new();
    let mut mime = "application/octet-stream".to_string();

    if let Ok(req) = serde_json::from_slice::<DownloadBinaryForm>(&body) {
        if let Some(f) = req.filename {
            filename = f;
        }
        if let Some(c) = req.content_b64 {
            content_b64 = c;
        }
        if let Some(m) = req.mime {
            mime = m;
        }
    } else if let Ok(pairs) = serde_urlencoded::from_bytes::<Vec<(String, String)>>(&body) {
        for (k, v) in pairs {
            if k == "filename" && !v.is_empty() {
                filename = v;
            } else if k == "content_b64" {
                content_b64 = v;
            } else if k == "mime" && !v.is_empty() {
                mime = v;
            }
        }
    }

    let raw = b64::decode(&content_b64, false)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Base64 inválido para download."))?;

    let disposition = format!("attachment; filename=\"{}\"", filename);
    let mut res = Response::new(raw.into());
    res.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&mime).unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    res.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&disposition).unwrap_or(HeaderValue::from_static("attachment")),
    );
    Ok(res)
}

/// POST /process - Legacy form compatibility & direct file decode download
pub async fn process_handler(
    headers: HeaderMap,
    _query: Query<QueryOptions>,
    body: Bytes,
) -> Response {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // If it's multipart
    if content_type.starts_with("multipart/form-data") {
        if let Ok(boundary) = multer::parse_boundary(content_type) {
            let stream = futures_util::stream::once(async move { Result::<_, std::io::Error>::Ok(body) });
            let mut multipart = multer::Multipart::new(stream, boundary);
            let mut file_bytes = None;
            let mut file_name = None;
            let mut action = "encode".to_string();
            let mut urlsafe = false;

            while let Ok(Some(field)) = multipart.next_field().await {
                let name = field.name().unwrap_or("").to_string();
                if name == "input_file" || name == "file" {
                    if let Some(fname) = field.file_name() {
                        file_name = Some(fname.to_string());
                    }
                    if let Ok(bytes) = field.bytes().await {
                        file_bytes = Some(bytes.to_vec());
                    }
                } else if name == "action" {
                    if let Ok(act) = field.text().await {
                        action = act;
                    }
                } else if name == "urlsafe" {
                    if let Ok(val) = field.text().await {
                        urlsafe = is_urlsafe_param(Some(val), false);
                    }
                }
            }

            // If action is decode AND file was uploaded, trigger direct file download (same as Python original)
            if action == "decode" {
                if let Some(bytes) = file_bytes {
                    if !bytes.is_empty() {
                        let text = String::from_utf8_lossy(&bytes);
                        let (data_part, _) = strip_data_url_prefix(&text);
                        if let Ok(decoded) = b64::decode(data_part, urlsafe) {
                            let fname = guess_download_name(file_name.as_deref(), "decode", None);
                            let disposition = format!("attachment; filename=\"{}\"", fname);
                            let mut res = Response::new(decoded.into());
                            res.headers_mut().insert(
                                header::CONTENT_TYPE,
                                HeaderValue::from_static("application/octet-stream"),
                            );
                            res.headers_mut().insert(
                                header::CONTENT_DISPOSITION,
                                HeaderValue::from_str(&disposition).unwrap_or(HeaderValue::from_static("attachment")),
                            );
                            return res;
                        }
                    }
                }
            }
        }
    }

    // Default: redirect back to UI or serve index
    Html(INDEX_HTML).into_response()
}

#[derive(Debug, Deserialize)]
pub struct AutoDecodeRequest {
    pub text: Option<String>,
    pub data: Option<String>,
    #[serde(default = "default_true")]
    pub convert_k8s: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct AutoDecodeQuery {
    pub raw: Option<String>,
    pub convert_k8s: Option<String>,
}

/// POST /api/auto-decode
/// Automatically discovers Base64 tokens in full text, YAML Secret (type: Opaque), or JSON and decodes them.
pub async fn api_auto_decode_handler(
    headers: HeaderMap,
    Query(query): Query<AutoDecodeQuery>,
    body: Bytes,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let _content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let mut text_to_process = None;
    let mut convert_k8s = true;

    if let Some(c) = query.convert_k8s {
        convert_k8s = c.to_lowercase() == "1" || c.to_lowercase() == "true";
    }

    // 1. Try parsing JSON
    if let Ok(req) = serde_json::from_slice::<AutoDecodeRequest>(&body) {
        text_to_process = req.text.or(req.data);
        convert_k8s = req.convert_k8s;
    }

    // 2. Try parsing URL-encoded form
    if text_to_process.is_none() {
        if let Ok(pairs) = serde_urlencoded::from_bytes::<Vec<(String, String)>>(&body) {
            for (k, v) in pairs {
                if k == "text" || k == "data" || k == "input_text" {
                    text_to_process = Some(v);
                } else if k == "convert_k8s" {
                    convert_k8s = v.to_lowercase() == "1" || v.to_lowercase() == "true";
                }
            }
        }
    }

    // 3. Raw text or YAML body
    if text_to_process.is_none() && (!body.is_empty()) {
        text_to_process = Some(String::from_utf8_lossy(&body).to_string());
    }

    let input_text = match text_to_process {
        Some(t) if !t.trim().is_empty() => t,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "ok": false,
                    "error": "Forneça o texto completo no corpo da requisição ou via campo 'text'."
                })),
            ));
        }
    };

    let result = auto_decode_text(&input_text, convert_k8s);

    let wants_raw = query.raw.as_deref() == Some("true")
        || query.raw.as_deref() == Some("1")
        || headers.get(header::ACCEPT).and_then(|v| v.to_str().ok()) == Some("text/plain");

    if wants_raw {
        let mut res = Response::new(result.decoded_text.into());
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        );
        return Ok(res);
    }

    let json_bytes = serde_json::to_vec(&result)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"ok": false, "error": e.to_string()}))))?;

    let mut res = Response::new(json_bytes.into());
    res.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    Ok(res)
}

#[derive(Debug, Deserialize)]
pub struct AutoEncodeRequest {
    pub text: Option<String>,
    pub data: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AutoEncodeQuery {
    pub raw: Option<String>,
}

/// POST /api/auto-encode
/// Converts YAML Secret with `stringData:` back to `data:` with Base64 values.
pub async fn api_auto_encode_handler(
    headers: HeaderMap,
    Query(query): Query<AutoEncodeQuery>,
    body: Bytes,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let mut text_to_process = None;

    if let Ok(req) = serde_json::from_slice::<AutoEncodeRequest>(&body) {
        text_to_process = req.text.or(req.data);
    }

    if text_to_process.is_none() {
        if let Ok(pairs) = serde_urlencoded::from_bytes::<Vec<(String, String)>>(&body) {
            for (k, v) in pairs {
                if k == "text" || k == "data" || k == "input_text" {
                    text_to_process = Some(v);
                }
            }
        }
    }

    if text_to_process.is_none() && (!body.is_empty()) {
        text_to_process = Some(String::from_utf8_lossy(&body).to_string());
    }

    let input_text = match text_to_process {
        Some(t) if !t.trim().is_empty() => t,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "ok": false,
                    "error": "Forneça o texto com stringData no corpo da requisição."
                })),
            ));
        }
    };

    let result = auto_encode_text(&input_text);

    let wants_raw = query.raw.as_deref() == Some("true")
        || query.raw.as_deref() == Some("1")
        || headers.get(header::ACCEPT).and_then(|v| v.to_str().ok()) == Some("text/plain");

    if wants_raw {
        let mut res = Response::new(result.encoded_text.into());
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        );
        return Ok(res);
    }

    let json_bytes = serde_json::to_vec(&result)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"ok": false, "error": e.to_string()}))))?;

    let mut res = Response::new(json_bytes.into());
    res.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    Ok(res)
}

/// POST /api/diff
pub async fn api_diff_handler(
    _headers: HeaderMap,
    body: Bytes,
) -> Result<Json<crate::diff::DiffResult>, (StatusCode, Json<serde_json::Value>)> {
    if let Ok(req) = serde_json::from_slice::<crate::diff::DiffRequest>(&body) {
        let res = crate::diff::compare_texts(
            &req.original,
            &req.modified,
            req.ignore_case,
            req.ignore_whitespace,
        );
        return Ok(Json(res));
    }

    if let Ok(pairs) = serde_urlencoded::from_bytes::<Vec<(String, String)>>(&body) {
        let mut original = String::new();
        let mut modified = String::new();
        let mut ignore_case = false;
        let mut ignore_whitespace = false;

        for (k, v) in pairs {
            if k == "original" || k == "text_a" {
                original = v;
            } else if k == "modified" || k == "text_b" {
                modified = v;
            } else if k == "ignore_case" {
                ignore_case = v.to_lowercase() == "1" || v.to_lowercase() == "true";
            } else if k == "ignore_whitespace" {
                ignore_whitespace = v.to_lowercase() == "1" || v.to_lowercase() == "true";
            }
        }

        let res = crate::diff::compare_texts(&original, &modified, ignore_case, ignore_whitespace);
        return Ok(Json(res));
    }

    Err((
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({
            "ok": false,
            "error": "Forneça os campos 'original' e 'modified' no corpo da requisição."
        })),
    ))
}



