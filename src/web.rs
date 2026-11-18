use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Result};
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::error;

use crate::model::{BlogSettings, LanguageRecord, PostRecord, PostSummary};
use crate::storage::{BlogRepository, RedbPostStore};
use crate::templates::{self, HomeItem, HomePage, MessagePage, PostPage};

const LANGUAGE_COOKIE_NAME: &str = "rustyblogger_language";
const LANGUAGE_COOKIE_MAX_AGE_SECONDS: u64 = 31_536_000;

pub struct ServeOptions {
    pub database_path: PathBuf,
    pub media_directory: PathBuf,
    pub bind_address: String,
    pub site_title: Option<String>,
}

#[derive(Clone)]
struct AppState {
    repository: Arc<dyn BlogRepository>,
    site_title: Arc<str>,
    default_language: Arc<str>,
    languages: Arc<Vec<LanguageRecord>>,
    settings: BlogSettings,
}

#[derive(Debug, Default, Deserialize)]
struct LanguageQuery {
    lang: Option<String>,
    page: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
struct SwitchQuery {
    next: Option<String>,
}

#[derive(Debug)]
struct AppError {
    status: StatusCode,
    message: String,
}

impl AppError {
    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        error!("request failed: {:#}", error);
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "internal server error".to_string(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.status, self.message).into_response()
    }
}

#[derive(Serialize)]
struct ApiPostSummary {
    #[serde(flatten)]
    summary: PostSummary,
    languages: Vec<String>,
    requested_language: String,
    is_fallback: bool,
}

#[derive(Serialize)]
struct ApiPostResponse {
    #[serde(flatten)]
    post: PostRecord,
    requested_language: String,
    available_languages: Vec<String>,
}

#[derive(Serialize)]
struct ApiLanguages {
    default_language: String,
    languages: Vec<LanguageRecord>,
}

pub async fn serve(options: ServeOptions) -> Result<()> {
    let repository: Arc<dyn BlogRepository> =
        Arc::new(RedbPostStore::open(&options.database_path)?);
    let mut languages = repository.list_languages()?;
    if languages.is_empty() {
        bail!(
            "database {} contains no posts, run the import command first",
            options.database_path.display()
        );
    }
    let default_language = repository.default_language()?;
    if let Some(default_index) = languages
        .iter()
        .position(|language| language.code == default_language)
    {
        let default_language_record = languages.remove(default_index);
        languages.insert(0, default_language_record);
    }
    let settings = repository.settings()?;
    let site_title = options
        .site_title
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| settings.title.clone());
    let state = AppState {
        repository,
        site_title: Arc::from(site_title),
        default_language: Arc::from(default_language),
        languages: Arc::new(languages),
        settings,
    };
    let router = Router::new()
        .route("/", get(home))
        .route("/posts/{slug}", get(post_page))
        .route("/category/{category}", get(category_page))
        .route("/language/{code}", get(switch_language))
        .route("/api/posts", get(api_posts))
        .route("/api/posts/{slug}", get(api_post))
        .route("/api/languages", get(api_languages))
        .route("/api/languages/{language}/posts", get(api_language_posts))
        .route("/api/health", get(api_health))
        .route("/health", get(api_health))
        .route("/assets/style.css", get(stylesheet))
        .nest_service("/media", ServeDir::new(options.media_directory))
        .fallback(not_found_page)
        .method_not_allowed_fallback(method_not_allowed_page)
        .with_state(state)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http());
    let listener = tokio::net::TcpListener::bind(&options.bind_address).await?;
    tracing::info!("serving RustyBlogger on http://{}", options.bind_address);
    axum::serve(listener, router).await?;
    Ok(())
}

async fn home(
    State(state): State<AppState>,
    Query(query): Query<LanguageQuery>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let selected_language = resolve_language(&state, &query, &headers);
    let page = build_home_page(&state, &selected_language, None, query.page)?;
    Ok(Html(templates::render_home(&page)).into_response())
}

async fn category_page(
    State(state): State<AppState>,
    Path(category): Path<String>,
    Query(query): Query<LanguageQuery>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let selected_language = resolve_language(&state, &query, &headers);
    let page = build_home_page(&state, &selected_language, Some(&category), query.page)?;
    Ok(Html(templates::render_home(&page)).into_response())
}

async fn post_page(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<LanguageQuery>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let requested_language = resolve_language(&state, &query, &headers);
    let Some(slug_record) = state.repository.get_slug(&slug)? else {
        return Ok(render_not_found(&state, &requested_language, &slug));
    };
    let selected_language = if slug_record.languages.contains(&requested_language) {
        requested_language.clone()
    } else {
        slug_record.default_language.clone()
    };
    let Some(post) = state.repository.get_post(&slug, &selected_language)? else {
        return Ok(render_not_found(&state, &requested_language, &slug));
    };
    let page = PostPage {
        site_title: state.site_title.as_ref().to_string(),
        selected_language: requested_language,
        languages: (*state.languages).clone(),
        post,
        available_languages: slug_record.languages,
    };
    Ok(Html(templates::render_post(&page)).into_response())
}

fn render_not_found(state: &AppState, selected_language: &str, slug: &str) -> Response {
    let (title, message) = templates::post_not_found_text(selected_language, slug);
    render_message_page(
        state,
        selected_language,
        title,
        message,
        StatusCode::NOT_FOUND,
    )
}

async fn not_found_page(
    State(state): State<AppState>,
    Query(query): Query<LanguageQuery>,
    headers: HeaderMap,
) -> Response {
    let selected_language = resolve_language(&state, &query, &headers);
    let (title, message) = templates::page_not_found_text(&selected_language);
    render_message_page(
        &state,
        &selected_language,
        title.to_string(),
        message.to_string(),
        StatusCode::NOT_FOUND,
    )
}

async fn method_not_allowed_page(
    State(state): State<AppState>,
    Query(query): Query<LanguageQuery>,
    headers: HeaderMap,
) -> Response {
    let selected_language = resolve_language(&state, &query, &headers);
    let (title, message) = templates::method_not_allowed_text(&selected_language);
    render_message_page(
        &state,
        &selected_language,
        title.to_string(),
        message.to_string(),
        StatusCode::METHOD_NOT_ALLOWED,
    )
}

fn render_message_page(
    state: &AppState,
    selected_language: &str,
    title: String,
    message: String,
    status: StatusCode,
) -> Response {
    let page = MessagePage {
        site_title: state.site_title.as_ref().to_string(),
        selected_language: selected_language.to_string(),
        languages: (*state.languages).clone(),
        title,
        message,
    };
    (status, Html(templates::render_message(&page))).into_response()
}

async fn switch_language(
    State(state): State<AppState>,
    Path(code): Path<String>,
    Query(query): Query<SwitchQuery>,
) -> Response {
    let normalized_code = code.trim().to_ascii_lowercase();
    let destination = if language_is_supported(&state, &normalized_code) {
        sanitize_redirect_target(query.next.as_deref())
    } else {
        "/".to_string()
    };
    let cookie = format!(
        "{LANGUAGE_COOKIE_NAME}={}; Path=/; Max-Age={LANGUAGE_COOKIE_MAX_AGE_SECONDS}; SameSite=Lax",
        urlencoding::encode(&normalized_code)
    );
    let mut response = Redirect::to(&destination).into_response();
    if let Ok(header_value) = HeaderValue::from_str(&cookie) {
        response
            .headers_mut()
            .insert(header::SET_COOKIE, header_value);
    }
    response
}

async fn api_posts(
    State(state): State<AppState>,
    Query(query): Query<LanguageQuery>,
    headers: HeaderMap,
) -> Result<Json<Vec<ApiPostSummary>>, AppError> {
    let requested_language = resolve_language(&state, &query, &headers);
    let items = collect_home_items(&state, &requested_language, None)?;
    let posts = items
        .into_iter()
        .map(|item| ApiPostSummary {
            summary: item.post.summary(),
            languages: item.languages,
            requested_language: requested_language.clone(),
            is_fallback: item.is_fallback,
        })
        .collect();
    Ok(Json(posts))
}

async fn api_post(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<LanguageQuery>,
    headers: HeaderMap,
) -> Result<Json<ApiPostResponse>, AppError> {
    let requested_language = resolve_language(&state, &query, &headers);
    let slug_record = state.repository.get_slug(&slug)?.ok_or_else(|| {
        AppError::not_found(templates::post_not_found_message(
            &requested_language,
            &slug,
        ))
    })?;
    let selected_language = if slug_record.languages.contains(&requested_language) {
        requested_language.clone()
    } else {
        slug_record.default_language.clone()
    };
    let post = state
        .repository
        .get_post(&slug, &selected_language)?
        .ok_or_else(|| {
            AppError::not_found(templates::post_not_found_message(
                &requested_language,
                &slug,
            ))
        })?;
    Ok(Json(ApiPostResponse {
        post,
        requested_language,
        available_languages: slug_record.languages,
    }))
}

async fn api_languages(State(state): State<AppState>) -> Json<ApiLanguages> {
    Json(ApiLanguages {
        default_language: state.default_language.as_ref().to_string(),
        languages: (*state.languages).clone(),
    })
}

async fn api_language_posts(
    State(state): State<AppState>,
    Path(language): Path<String>,
) -> Result<Json<Vec<PostSummary>>, AppError> {
    let normalized_language = language.trim().to_ascii_lowercase();
    if !language_is_supported(&state, &normalized_language) {
        return Err(AppError::not_found(templates::language_not_found_message(
            state.default_language.as_ref(),
            &language,
        )));
    }
    Ok(Json(state.repository.list_posts(&normalized_language)?))
}

async fn api_health(State(state): State<AppState>) -> Result<Response, AppError> {
    state.repository.check_health()?;
    Ok(Json(serde_json::json!({ "status": "ok" })).into_response())
}

async fn stylesheet() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        templates::STYLESHEET,
    )
}

fn build_home_page(
    state: &AppState,
    selected_language: &str,
    category: Option<&str>,
    requested_page: Option<usize>,
) -> Result<HomePage, AppError> {
    let items = collect_home_items(state, selected_language, category)?;
    let total_items = items.len();
    let posts_per_page = state.settings.posts_per_page.max(1);
    let total_pages = total_items.div_ceil(posts_per_page).max(1);
    let current_page = requested_page.unwrap_or(1).clamp(1, total_pages);
    let has_requested_language_post = items.iter().any(|item| !item.is_fallback);
    let fallback_to_default = !items.is_empty()
        && !has_requested_language_post
        && selected_language != &*state.default_language;
    let page_start = (current_page - 1) * posts_per_page;
    let page_items: Vec<HomeItem> = items
        .into_iter()
        .skip(page_start)
        .take(posts_per_page)
        .collect();
    Ok(HomePage {
        site_title: state.site_title.as_ref().to_string(),
        selected_language: selected_language.to_string(),
        default_language: state.default_language.as_ref().to_string(),
        languages: (*state.languages).clone(),
        items: page_items,
        category: category.map(str::to_string),
        fallback_to_default,
        posts_per_page,
        current_page,
        total_pages,
    })
}

fn collect_home_items(
    state: &AppState,
    selected_language: &str,
    category: Option<&str>,
) -> Result<Vec<HomeItem>, AppError> {
    let mut items = Vec::new();
    for slug_record in state.repository.list_slugs()? {
        let item_language = if slug_record
            .languages
            .iter()
            .any(|language| language.as_str() == selected_language)
        {
            selected_language.to_string()
        } else {
            slug_record.default_language.clone()
        };
        let Some(post) = state
            .repository
            .get_post(&slug_record.slug, &item_language)?
        else {
            continue;
        };
        if let Some(category) = category {
            if !post
                .categories
                .iter()
                .any(|post_category| post_category.as_str() == category)
            {
                continue;
            }
        }
        items.push(HomeItem {
            is_fallback: item_language.as_str() != selected_language,
            post,
            languages: slug_record.languages,
        });
    }
    items.sort_by(|left, right| {
        right
            .post
            .published_unix_seconds
            .cmp(&left.post.published_unix_seconds)
            .then_with(|| left.post.title.cmp(&right.post.title))
    });
    Ok(items)
}

fn resolve_language(state: &AppState, query: &LanguageQuery, headers: &HeaderMap) -> String {
    if let Some(requested_language) = query
        .lang
        .as_deref()
        .map(str::trim)
        .map(str::to_ascii_lowercase)
    {
        if language_is_supported(state, &requested_language) {
            return requested_language;
        }
    }
    if let Some(cookie_language) = language_from_cookie(headers) {
        if language_is_supported(state, &cookie_language) {
            return cookie_language;
        }
    }
    if let Some(header_language) =
        language_from_accept_language(headers, state.languages.as_slice())
    {
        return header_language;
    }
    state.default_language.as_ref().to_string()
}

fn language_is_supported(state: &AppState, language: &str) -> bool {
    state
        .languages
        .iter()
        .any(|supported_language| supported_language.code.as_str() == language)
}

fn language_from_cookie(headers: &HeaderMap) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    for cookie in cookie_header.split(';') {
        let mut parts = cookie.trim().splitn(2, '=');
        let name = parts.next()?.trim();
        if name == LANGUAGE_COOKIE_NAME {
            return parts
                .next()
                .map(|value| value.trim().to_ascii_lowercase())
                .filter(|value| !value.is_empty());
        }
    }
    None
}

fn language_from_accept_language(
    headers: &HeaderMap,
    languages: &[LanguageRecord],
) -> Option<String> {
    let header_value = headers.get(header::ACCEPT_LANGUAGE)?.to_str().ok()?;
    for range in header_value.split(',') {
        let range = range.split(';').next()?.trim().to_ascii_lowercase();
        let range_text = range.as_str();
        let primary = range_text.split('-').next().unwrap_or(range_text);
        if let Some(language) = languages.iter().find(|language| {
            language.code.as_str() == range_text || language.code.as_str() == primary
        }) {
            return Some(language.code.clone());
        }
    }
    None
}

fn sanitize_redirect_target(target: Option<&str>) -> String {
    match target {
        Some(target) if target.starts_with('/') && !target.starts_with("//") => target.to_string(),
        _ => "/".to_string(),
    }
}
