use std::{env, sync::Arc};

use axum::{
    Json, Router,
    extract::{
        DefaultBodyLimit, Form, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{
        HeaderMap, HeaderValue, StatusCode, Uri,
        header::{HOST, ORIGIN},
        uri::Authority,
    },
    response::{Html, IntoResponse, Response},
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use leptos::prelude::*;
use leptos::tachys::view::RenderHtml;
use maud::{DOCTYPE, Markup, html};
use next_loggers::{JsonObject, Logger, OpenTelemetryTransport, Options, Value as LogValue, json};
use sea_orm::{Database, DatabaseConnection};
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, broadcast};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

const MAX_REQUEST_BODY_BYTES: usize = 16 * 1024;
const MAX_WEBSOCKET_FRAME_BYTES: usize = 8 * 1024;
const MAX_WEBSOCKET_MESSAGE_BYTES: usize = 16 * 1024;
const MAX_ITEMS: usize = 1_024;
const MAX_TITLE_CHARS: usize = 160;
const MAX_DETAIL_CHARS: usize = 4_000;
const COMPONENT_CONTRACT: &str = "hhm.component.v1";
const HTMX_INTEGRITY: &str =
    "sha384-HGfztofotfshcF7+8n44JQL2oJmowVChPTg48S+jvZoztPfvwD79OC/LTtG6dMp+";

#[derive(Clone)]
struct AppState {
    db: Option<DatabaseConnection>,
    items: Arc<RwLock<Vec<Item>>>,
    events: broadcast::Sender<String>,
    supabase_url: Option<String>,
    demo_writes_enabled: bool,
    observability: Observability,
}

#[derive(Clone)]
struct Item {
    id: Uuid,
    title: String,
    detail: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewItem {
    title: String,
    detail: String,
}

#[derive(Debug, Eq, PartialEq)]
enum InputError {
    InvalidTitle,
    InvalidDetail,
}

#[derive(Serialize)]
struct ComponentManifest {
    contract: &'static str,
    components: [ComponentDescriptor; 2],
}

#[derive(Serialize)]
struct ComponentDescriptor {
    id: &'static str,
    href: &'static str,
    media_type: &'static str,
    runtimes: [&'static str; 2],
    executable: bool,
}

#[derive(Clone)]
struct Observability {
    logger: Logger,
}

impl Observability {
    fn new() -> Self {
        let transport = OpenTelemetryTransport::new(|record| {
            tracing::info!(
                target: "ores_otel",
                otel_body = %record.body,
                otel_severity_text = %record.severity_text,
                otel_severity_number = record.severity_number,
                otel_attributes = ?record.attributes,
                "Ores structured log"
            );
            Ok(())
        });
        Self {
            logger: Logger::new(Options {
                app_name: "hhm-mash-web".to_owned(),
                console: false,
                transports: vec![Arc::new(transport)],
                ..Options::default()
            }),
        }
    }

    fn event(&self, name: &'static str, outcome: &'static str) {
        let fields = JsonObject::from_iter([
            ("event.name".to_owned(), LogValue::String(name.to_owned())),
            (
                "event.outcome".to_owned(),
                LogValue::String(outcome.to_owned()),
            ),
        ]);
        let _ = self
            .logger
            .info(vec![json!("HHM web transition")])
            .add_fields(fields)
            .send();
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let db = match env::var("DATABASE_URL") {
        Ok(url) if !url.is_empty() => Some(Database::connect(url).await?),
        _ => None,
    };
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let demo_writes_requested = matches!(
        non_empty_env("ALLOW_UNAUTHENTICATED_DEMO_WRITES").as_deref(),
        Some("true")
    );
    let demo_writes_enabled = should_enable_demo_writes(demo_writes_requested, &host);
    if demo_writes_requested && !demo_writes_enabled {
        tracing::warn!(
            bind_host = %host,
            "ignoring ALLOW_UNAUTHENTICATED_DEMO_WRITES on a non-loopback bind host"
        );
    }
    let (events, _) = broadcast::channel(256);
    let state = AppState {
        db,
        items: Arc::new(RwLock::new(seed_items())),
        events,
        supabase_url: non_empty_env("SUPABASE_URL"),
        demo_writes_enabled,
        observability: Observability::new(),
    };
    let app = Router::new()
        .route("/", get(index))
        .route("/healthz", get(health))
        .route("/v1/components", get(component_manifest))
        .route("/v1/components/reservations", get(reservation_component))
        .route(
            "/v1/components/leptos-capabilities",
            get(leptos_capability_component),
        )
        .route(
            "/partials/reservations",
            get(items_partial).post(create_item),
        )
        .route("/ws", get(ws_upgrade))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BODY_BYTES))
        .layer(TraceLayer::new_for_http())
        .with_state(state);
    let port = env::var("PORT").unwrap_or_else(|_| "8081".into());
    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}")).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn non_empty_env(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn is_loopback_bind_host(host: &str) -> bool {
    host == "127.0.0.1" || host == "::1" || host.eq_ignore_ascii_case("localhost")
}

fn should_enable_demo_writes(requested: bool, host: &str) -> bool {
    requested && is_loopback_bind_host(host)
}

async fn index(State(state): State<AppState>) -> Response {
    let items = state.items.read().await.clone();
    let nonce = csp_nonce();
    document_response(layout(items_markup(&items), &nonce).into_string(), &nonce)
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "database": state.db.is_some(),
        "supabase": state.supabase_url.is_some(),
        "demo_writes_enabled": state.demo_writes_enabled,
        "component_contract": COMPONENT_CONTRACT,
        "state_durable": false
    }))
}

async fn component_manifest() -> Response {
    let manifest = ComponentManifest {
        contract: COMPONENT_CONTRACT,
        components: [
            ComponentDescriptor {
                id: "reservations",
                href: "/v1/components/reservations",
                media_type: "text/html; charset=utf-8",
                runtimes: ["browser", "flutter_webview"],
                executable: false,
            },
            ComponentDescriptor {
                id: "leptos-capabilities",
                href: "/v1/components/leptos-capabilities",
                media_type: "text/html; charset=utf-8",
                runtimes: ["browser", "flutter_webview"],
                executable: false,
            },
        ],
    };
    let mut response = Json(manifest).into_response();
    response.headers_mut().insert(
        "cache-control",
        HeaderValue::from_static("public, max-age=60"),
    );
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    response
}

async fn reservation_component(State(state): State<AppState>) -> Response {
    state
        .observability
        .event("component.reservations", "served");
    fragment_response(items_markup(&state.items.read().await).into_string())
}

async fn leptos_capability_component(State(state): State<AppState>) -> Response {
    state
        .observability
        .event("component.leptos_capabilities", "served");
    fragment_response(leptos_capability_markup())
}

async fn items_partial(State(state): State<AppState>) -> Response {
    fragment_response(items_markup(&state.items.read().await).into_string())
}

async fn create_item(State(state): State<AppState>, Form(input): Form<NewItem>) -> Response {
    if !state.demo_writes_enabled {
        state
            .observability
            .event("reservation.demo_create", "denied");
        return message_response(
            StatusCode::FORBIDDEN,
            "Demo writes are disabled until authenticated persistence is configured.",
        );
    }

    let input = match normalize_item(input) {
        Ok(input) => input,
        Err(_) => {
            state
                .observability
                .event("reservation.demo_create", "invalid");
            return message_response(StatusCode::UNPROCESSABLE_ENTITY, "Invalid reservation.");
        }
    };

    let item = Item {
        id: Uuid::new_v4(),
        title: input.title,
        detail: input.detail,
    };
    let mut items = state.items.write().await;
    if items.len() >= MAX_ITEMS {
        state
            .observability
            .event("reservation.demo_create", "capacity_reached");
        return message_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "Reservation demo capacity reached.",
        );
    }
    items.push(item.clone());
    let rendered = items_markup(&items).into_string();
    drop(items);

    let _ = state.events.send(format!("created:{}", item.id));
    state
        .observability
        .event("reservation.demo_create", "accepted");
    fragment_response(rendered)
}

fn normalize_item(mut input: NewItem) -> Result<NewItem, InputError> {
    input.title = input.title.trim().to_owned();
    input.detail = input.detail.trim().to_owned();
    if !valid_text(&input.title, MAX_TITLE_CHARS) {
        return Err(InputError::InvalidTitle);
    }
    if !valid_text(&input.detail, MAX_DETAIL_CHARS) {
        return Err(InputError::InvalidDetail);
    }
    Ok(input)
}

fn valid_text(value: &str, maximum_chars: usize) -> bool {
    !value.is_empty()
        && value.chars().count() <= maximum_chars
        && !value.chars().any(char::is_control)
}

fn csp_nonce() -> String {
    Uuid::new_v4().simple().to_string()
}

fn document_response(body: String, nonce: &str) -> Response {
    let mut response = Html(body).into_response();
    add_no_store_headers(&mut response);
    let policy = format!(
        "default-src 'self'; base-uri 'none'; object-src 'none'; frame-ancestors 'none'; form-action 'self'; script-src 'self' https://unpkg.com 'nonce-{nonce}'; style-src 'unsafe-inline'; connect-src 'self' ws: wss:"
    );
    response.headers_mut().insert(
        "content-security-policy",
        HeaderValue::try_from(policy).expect("generated CSP must be a valid header value"),
    );
    response
}

fn fragment_response(body: String) -> Response {
    let mut response = Html(body).into_response();
    add_no_store_headers(&mut response);
    response.headers_mut().insert(
        "content-security-policy",
        HeaderValue::from_static(
            "default-src 'none'; base-uri 'none'; form-action 'none'; object-src 'none'; sandbox",
        ),
    );
    response.headers_mut().insert(
        "x-hhm-component-contract",
        HeaderValue::from_static(COMPONENT_CONTRACT),
    );
    response
}

fn message_response(status: StatusCode, message: &'static str) -> Response {
    let mut response = fragment_response(html! { p { (message) } }.into_string());
    *response.status_mut() = status;
    response
}

fn add_no_store_headers(response: &mut Response) {
    response.headers_mut().insert(
        "cache-control",
        HeaderValue::from_static("private, no-store"),
    );
    response
        .headers_mut()
        .insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
}

fn layout(content: Markup, script_nonce: &str) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "Hacker House Medellín" }
                script
                    src="https://unpkg.com/htmx.org@2.0.4/dist/htmx.min.js"
                    integrity=(HTMX_INTEGRITY)
                    crossorigin="anonymous" {}
                style { "body{font-family:system-ui;max-width:900px;margin:3rem auto;padding:0 1rem} form{display:grid;gap:.75rem} .card{border:1px solid #ddd;border-radius:12px;padding:1rem;margin:.75rem 0}" }
            }
            body {
                header {
                    h1 { "Hacker House Medellín" }
                    p { "Operations and community software for an entrepreneur-focused coliving and coworking house in Medellín, Colombia." }
                }
                form hx-post="/partials/reservations" hx-target="#items" hx-swap="innerHTML" {
                    input type="text" name="title" placeholder="Title" maxlength=(MAX_TITLE_CHARS) required;
                    textarea name="detail" placeholder="Details" maxlength=(MAX_DETAIL_CHARS) required {}
                    button type="submit" { "Create Reservation" }
                }
                section id="items" { (content) }
                script nonce=(script_nonce) { (maud::PreEscaped("const proto=location.protocol==='https:'?'wss':'ws';const ws=new WebSocket(proto+'://'+location.host+'/ws');ws.onmessage=()=>htmx.ajax('GET','/partials/reservations',{target:'#items'});")) }
            }
        }
    }
}

fn items_markup(items: &[Item]) -> Markup {
    html! {
        @for item in items {
            article class="card" data-id=(item.id) {
                h2 { (item.title) }
                p { (item.detail) }
            }
        }
    }
}

#[component]
fn MashCapabilityCard() -> impl IntoView {
    view! {
        <article class="card hhm-leptos-ssr">
            <h2>"MASH component delivery"</h2>
            <p>"Rendered on the server with Leptos and delivered as inert HTML."</p>
            <ul>
                <li>"Maud remains the primary document and reservation renderer."</li>
                <li>"HTMX and WebSockets remain the browser interaction layer."</li>
                <li>"Flutter clients may display this fragment in a sandboxed WebView."</li>
            </ul>
        </article>
    }
}

fn leptos_capability_markup() -> String {
    view! { <MashCapabilityCard/> }.to_html()
}

fn seed_items() -> Vec<Item> {
    vec![Item {
        id: Uuid::new_v4(),
        title: "Foundation ready".into(),
        detail: "Maud + Axum + SeaORM + Supabase configuration + HTMX + WebSockets".into(),
    }]
}

async fn ws_upgrade(
    headers: HeaderMap,
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    if !websocket_origin_matches_host(&headers) {
        let mut response = StatusCode::FORBIDDEN.into_response();
        add_no_store_headers(&mut response);
        return response;
    }

    ws.max_frame_size(MAX_WEBSOCKET_FRAME_BYTES)
        .max_message_size(MAX_WEBSOCKET_MESSAGE_BYTES)
        .on_upgrade(move |socket| websocket(socket, state))
        .into_response()
}

fn websocket_origin_matches_host(headers: &HeaderMap) -> bool {
    if headers.get_all(HOST).iter().count() != 1 || headers.get_all(ORIGIN).iter().count() != 1 {
        return false;
    }

    let Some(host) = headers.get(HOST).and_then(|value| value.to_str().ok()) else {
        return false;
    };
    let Some(origin) = headers.get(ORIGIN).and_then(|value| value.to_str().ok()) else {
        return false;
    };
    if host.contains('@') || origin == "null" {
        return false;
    }

    let Ok(host_authority) = host.parse::<Authority>() else {
        return false;
    };
    let Ok(origin_uri) = origin.parse::<Uri>() else {
        return false;
    };
    let Some(scheme) = origin_uri.scheme_str() else {
        return false;
    };
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return false;
    }
    if origin_uri.path() != "/" || origin_uri.query().is_some() {
        return false;
    }
    let Some(origin_authority) = origin_uri.authority() else {
        return false;
    };
    if origin_authority.as_str().contains('@')
        || !host_authority
            .host()
            .eq_ignore_ascii_case(origin_authority.host())
    {
        return false;
    }

    effective_port(&host_authority, scheme) == effective_port(origin_authority, scheme)
}

fn effective_port(authority: &Authority, scheme: &str) -> Option<u16> {
    authority.port_u16().or_else(|| {
        if scheme.eq_ignore_ascii_case("http") {
            Some(80)
        } else if scheme.eq_ignore_ascii_case("https") {
            Some(443)
        } else {
            None
        }
    })
}

async fn websocket(socket: WebSocket, state: AppState) {
    let (mut tx, mut rx) = socket.split();
    let mut events = state.events.subscribe();
    loop {
        tokio::select! {
            message = rx.next() => match message {
                Some(Ok(Message::Close(_))) | None => break,
                _ => {}
            },
            event = events.recv() => match event {
                Ok(event) => if tx.send(Message::Text(event.into())).await.is_err() {
                    break;
                },
                Err(_) => break,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_and_bounds_demo_input() {
        let normalized = normalize_item(NewItem {
            title: "  Desk  ".to_owned(),
            detail: "  Near the window  ".to_owned(),
        })
        .unwrap();
        assert_eq!(normalized.title, "Desk");
        assert_eq!(normalized.detail, "Near the window");

        assert!(matches!(
            normalize_item(NewItem {
                title: "bad\nname".to_owned(),
                detail: "valid".to_owned(),
            }),
            Err(InputError::InvalidTitle)
        ));
        assert!(matches!(
            normalize_item(NewItem {
                title: "valid".to_owned(),
                detail: "x".repeat(MAX_DETAIL_CHARS + 1),
            }),
            Err(InputError::InvalidDetail)
        ));
    }

    #[test]
    fn component_markup_escapes_untrusted_text() {
        let markup = items_markup(&[Item {
            id: Uuid::nil(),
            title: "<script>alert(1)</script>".to_owned(),
            detail: "<img src=x onerror=alert(1)>".to_owned(),
        }])
        .into_string();
        assert!(!markup.contains("<script>alert"));
        assert!(!markup.contains("<img src=x"));
        assert!(markup.contains("&lt;script&gt;"));
        assert!(markup.contains("&lt;img"));
    }

    #[test]
    fn fragment_has_inert_component_headers() {
        let response = fragment_response("<p>safe</p>".to_owned());
        assert_eq!(
            response.headers().get("x-hhm-component-contract").unwrap(),
            COMPONENT_CONTRACT
        );
        assert_eq!(
            response.headers().get("cache-control").unwrap(),
            "private, no-store"
        );
        assert!(
            response
                .headers()
                .get("content-security-policy")
                .unwrap()
                .to_str()
                .unwrap()
                .contains("sandbox")
        );
    }

    #[test]
    fn html_errors_use_the_inert_component_contract() {
        let response = message_response(StatusCode::UNPROCESSABLE_ENTITY, "Invalid.");
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            response.headers().get("x-hhm-component-contract").unwrap(),
            COMPONENT_CONTRACT
        );
        assert_eq!(
            response.headers().get("cache-control").unwrap(),
            "private, no-store"
        );
        assert!(
            response
                .headers()
                .get("content-security-policy")
                .unwrap()
                .to_str()
                .unwrap()
                .contains("sandbox")
        );
    }

    #[test]
    fn demo_writes_only_allow_explicit_loopback_hosts() {
        assert!(should_enable_demo_writes(true, "127.0.0.1"));
        assert!(should_enable_demo_writes(true, "::1"));
        assert!(should_enable_demo_writes(true, "localhost"));
        assert!(should_enable_demo_writes(true, "LOCALHOST"));
        assert!(!should_enable_demo_writes(false, "127.0.0.1"));
        assert!(!should_enable_demo_writes(true, "0.0.0.0"));
        assert!(!should_enable_demo_writes(true, "127.0.0.2"));
        assert!(!should_enable_demo_writes(true, "example.com"));
    }

    #[test]
    fn document_csp_uses_the_inline_script_nonce() {
        let nonce = "0123456789abcdef";
        let body = layout(items_markup(&[]), nonce).into_string();
        assert!(body.contains("nonce=\"0123456789abcdef\""));

        let response = document_response(body, nonce);
        let policy = response
            .headers()
            .get("content-security-policy")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(policy.contains("'nonce-0123456789abcdef'"));
        assert!(!policy.contains("script-src 'unsafe-inline'"));
        assert_ne!(csp_nonce(), csp_nonce());
    }

    #[test]
    fn websocket_origin_must_match_the_single_host_header() {
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_static("example.com"));
        headers.insert(ORIGIN, HeaderValue::from_static("https://example.com"));
        assert!(websocket_origin_matches_host(&headers));

        headers.insert(HOST, HeaderValue::from_static("example.com:443"));
        assert!(websocket_origin_matches_host(&headers));

        headers.insert(HOST, HeaderValue::from_static("[::1]:8081"));
        headers.insert(ORIGIN, HeaderValue::from_static("http://[::1]:8081"));
        assert!(websocket_origin_matches_host(&headers));

        headers.insert(ORIGIN, HeaderValue::from_static("https://attacker.test"));
        assert!(!websocket_origin_matches_host(&headers));

        headers.insert(ORIGIN, HeaderValue::from_static("not a URI"));
        assert!(!websocket_origin_matches_host(&headers));

        headers.remove(ORIGIN);
        assert!(!websocket_origin_matches_host(&headers));
    }

    #[test]
    fn websocket_origin_rejects_non_origin_uris_and_duplicate_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_static("example.com"));
        headers.insert(ORIGIN, HeaderValue::from_static("https://example.com/path"));
        assert!(!websocket_origin_matches_host(&headers));

        headers.insert(ORIGIN, HeaderValue::from_static("https://example.com"));
        headers.append(ORIGIN, HeaderValue::from_static("https://example.com"));
        assert!(!websocket_origin_matches_host(&headers));
    }

    #[test]
    fn leptos_component_is_inert_server_rendered_html() {
        let markup = leptos_capability_markup();
        assert!(markup.contains("hhm-leptos-ssr"));
        assert!(markup.contains("Rendered on the server with Leptos"));
        assert!(markup.contains("Maud remains the primary"));
        assert!(!markup.contains("<script"));
    }
}
