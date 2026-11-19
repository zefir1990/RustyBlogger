use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Result};
use axum::extract::{Path, Query, Request, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::error;

use crate::date::format_unix_date;
use crate::model::{BlogSettings, LanguageRecord, PostRecord, PostSummary};
use crate::render::content_hash;
use crate::storage::{BlogRepository, RedbPostStore};
use crate::templates::{self, HomeItem, HomePage, LanguagesPage, MessagePage, PostPage};

const LANGUAGE_COOKIE_NAME: &str = "rustyblogger_language";
const LANGUAGE_COOKIE_MAX_AGE_SECONDS: u64 = 31_536_000;
const EDITOR_KEY_HEADER: &str = "rusty-blogger-post-editor-key";
const HTML_CACHE_CONTROL: &str = "private, no-cache";
const API_CACHE_CONTROL: &str = "private, no-cache";
const STYLESHEET_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";
const MEDIA_CACHE_CONTROL: &str = "public, max-age=604800";
const NO_STORE_CACHE_CONTROL: &str = "no-store";
const LOCALIZED_VARY: &str = "accept-language, cookie";

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
    categories: Arc<Vec<String>>,
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

#[derive(Debug, Default, Deserialize)]
struct LanguagesQuery {
    lang: Option<String>,
    next: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct DeleteQuery {
    lang: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PostWriteRequest {
    #[serde(default)]
    slug: String,
    language: String,
    title: String,
    #[serde(default)]
    categories: Vec<String>,
    body_html: String,
    #[serde(default)]
    source_path: String,
    #[serde(default)]
    published_unix_seconds: Option<u64>,
    #[serde(default)]
    published_date: Option<String>,
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

    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: message.into(),
        }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
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

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        AppError::from(anyhow::Error::from(error))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        no_store((self.status, self.message).into_response())
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
            "database {} contains no posts",
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
    let categories = collect_categories(repository.as_ref(), &languages)?;
    let site_title = options
        .site_title
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| settings.title.clone());
    let state = AppState {
        repository,
        site_title: Arc::from(site_title),
        default_language: Arc::from(default_language),
        languages: Arc::new(languages),
        categories: Arc::new(categories),
        settings,
    };
    let router = Router::new()
        .route("/", get(home))
        .route("/posts/{slug}", get(post_page))
        .route("/category/{category}", get(category_page))
        .route("/language/{code}", get(switch_language))
        .route("/languages", get(languages_page))
        .route("/api/posts", get(api_posts).post(create_post))
        .route(
            "/api/posts/{slug}",
            get(api_post).put(update_post).delete(delete_post),
        )
        .route("/api/languages", get(api_languages))
        .route("/api/languages/{language}/posts", get(api_language_posts))
        .route("/api/health", get(api_health))
        .route("/health", get(api_health))
        .route("/assets/style.css", get(stylesheet))
        .nest_service("/media", ServeDir::new(options.media_directory))
        .fallback(not_found_page)
        .method_not_allowed_fallback(method_not_allowed_page)
        .with_state(state)
        .layer(middleware::from_fn(add_media_cache_control))
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
    let base_url = request_base_url(&headers);
    let page = build_home_page(&state, &selected_language, None, query.page, &base_url)?;
    Ok(cached_html_response(
        &headers,
        StatusCode::OK,
        templates::render_home(&page),
    ))
}

async fn category_page(
    State(state): State<AppState>,
    Path(category): Path<String>,
    Query(query): Query<LanguageQuery>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let selected_language = resolve_language(&state, &query, &headers);
    let base_url = request_base_url(&headers);
    let page = build_home_page(
        &state,
        &selected_language,
        Some(&category),
        query.page,
        &base_url,
    )?;
    Ok(cached_html_response(
        &headers,
        StatusCode::OK,
        templates::render_home(&page),
    ))
}

async fn post_page(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<LanguageQuery>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let requested_language = resolve_language(&state, &query, &headers);
    let base_url = request_base_url(&headers);
    let Some(slug_record) = state.repository.get_slug(&slug)? else {
        return Ok(render_not_found(
            &state,
            &requested_language,
            &slug,
            &base_url,
            &headers,
        ));
    };
    let selected_language = if slug_record.languages.contains(&requested_language) {
        requested_language.clone()
    } else {
        slug_record.default_language.clone()
    };
    let Some(post) = state.repository.get_post(&slug, &selected_language)? else {
        return Ok(render_not_found(
            &state,
            &requested_language,
            &slug,
            &base_url,
            &headers,
        ));
    };
    let page = PostPage {
        site_title: state.site_title.as_ref().to_string(),
        selected_language: requested_language,
        categories: (*state.categories).clone(),
        base_url,
        post,
    };
    Ok(cached_html_response(
        &headers,
        StatusCode::OK,
        templates::render_post(&page),
    ))
}

fn render_not_found(
    state: &AppState,
    selected_language: &str,
    slug: &str,
    base_url: &str,
    headers: &HeaderMap,
) -> Response {
    let (title, message) = templates::post_not_found_text(selected_language, slug);
    render_message_page(
        state,
        selected_language,
        title,
        message,
        StatusCode::NOT_FOUND,
        base_url,
        headers,
    )
}

async fn not_found_page(
    State(state): State<AppState>,
    Query(query): Query<LanguageQuery>,
    headers: HeaderMap,
) -> Response {
    let selected_language = resolve_language(&state, &query, &headers);
    let base_url = request_base_url(&headers);
    let (title, message) = templates::page_not_found_text(&selected_language);
    render_message_page(
        &state,
        &selected_language,
        title.to_string(),
        message.to_string(),
        StatusCode::NOT_FOUND,
        &base_url,
        &headers,
    )
}

async fn method_not_allowed_page(
    State(state): State<AppState>,
    Query(query): Query<LanguageQuery>,
    headers: HeaderMap,
) -> Response {
    let selected_language = resolve_language(&state, &query, &headers);
    let base_url = request_base_url(&headers);
    let (title, message) = templates::method_not_allowed_text(&selected_language);
    render_message_page(
        &state,
        &selected_language,
        title.to_string(),
        message.to_string(),
        StatusCode::METHOD_NOT_ALLOWED,
        &base_url,
        &headers,
    )
}

fn render_message_page(
    state: &AppState,
    selected_language: &str,
    title: String,
    message: String,
    status: StatusCode,
    base_url: &str,
    headers: &HeaderMap,
) -> Response {
    let page = MessagePage {
        site_title: state.site_title.as_ref().to_string(),
        selected_language: selected_language.to_string(),
        categories: (*state.categories).clone(),
        base_url: base_url.to_string(),
        title,
        message,
    };
    cached_html_response(headers, status, templates::render_message(&page))
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
    no_store(response)
}

async fn languages_page(
    State(state): State<AppState>,
    Query(query): Query<LanguagesQuery>,
    headers: HeaderMap,
) -> Response {
    let language_query = LanguageQuery {
        lang: query.lang,
        page: None,
    };
    let selected_language = resolve_language(&state, &language_query, &headers);
    let page = LanguagesPage {
        site_title: state.site_title.as_ref().to_string(),
        selected_language,
        categories: (*state.categories).clone(),
        base_url: request_base_url(&headers),
        languages: (*state.languages).clone(),
        next_path: sanitize_redirect_target(query.next.as_deref()),
    };
    cached_html_response(&headers, StatusCode::OK, templates::render_languages(&page))
}

async fn api_posts(
    State(state): State<AppState>,
    Query(query): Query<LanguageQuery>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
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
        .collect::<Vec<_>>();
    cached_json_response(&headers, StatusCode::OK, &posts)
}

async fn api_post(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<LanguageQuery>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
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
    let response = ApiPostResponse {
        post,
        requested_language,
        available_languages: slug_record.languages,
    };
    cached_json_response(&headers, StatusCode::OK, &response)
}

async fn create_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<PostWriteRequest>,
) -> Result<Response, AppError> {
    require_editor_key(&state, &headers)?;
    let slug = request.slug.trim().to_string();
    if slug.is_empty() {
        return Err(AppError::bad_request("slug is required"));
    }
    let post = request.into_post_record(&slug)?;
    if state
        .repository
        .get_post(&post.slug, &post.language)?
        .is_some()
    {
        return Err(AppError::conflict("post already exists"));
    }
    state.repository.upsert_post(&post)?;
    Ok(no_store((StatusCode::CREATED, Json(post)).into_response()))
}

async fn update_post(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    Json(request): Json<PostWriteRequest>,
) -> Result<Response, AppError> {
    require_editor_key(&state, &headers)?;
    let normalized_slug = slug.trim().to_string();
    if normalized_slug.is_empty() {
        return Err(AppError::bad_request("slug is required"));
    }
    let requested_slug = request.slug.trim();
    if !requested_slug.is_empty() && requested_slug != normalized_slug {
        return Err(AppError::bad_request("slug in body must match path"));
    }
    let post = request.into_post_record(&normalized_slug)?;
    if state
        .repository
        .get_post(&post.slug, &post.language)?
        .is_none()
    {
        return Err(AppError::not_found("post not found"));
    }
    state.repository.upsert_post(&post)?;
    Ok(no_store(Json(post).into_response()))
}

async fn delete_post(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<DeleteQuery>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    require_editor_key(&state, &headers)?;
    let language = query
        .lang
        .as_deref()
        .map(str::trim)
        .filter(|language| !language.is_empty())
        .ok_or_else(|| AppError::bad_request("lang query parameter is required"))?
        .to_ascii_lowercase();
    let normalized_slug = slug.trim();
    if !state.repository.delete_post(normalized_slug, &language)? {
        return Err(AppError::not_found("post not found"));
    }
    Ok(no_store(StatusCode::NO_CONTENT.into_response()))
}

async fn api_languages(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let response = ApiLanguages {
        default_language: state.default_language.as_ref().to_string(),
        languages: (*state.languages).clone(),
    };
    cached_json_response(&headers, StatusCode::OK, &response)
}

async fn api_language_posts(
    State(state): State<AppState>,
    Path(language): Path<String>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let normalized_language = language.trim().to_ascii_lowercase();
    if !language_is_supported(&state, &normalized_language) {
        return Err(AppError::not_found(templates::language_not_found_message(
            state.default_language.as_ref(),
            &language,
        )));
    }
    let posts = state.repository.list_posts(&normalized_language)?;
    cached_json_response(&headers, StatusCode::OK, &posts)
}

async fn api_health(State(state): State<AppState>) -> Result<Response, AppError> {
    state.repository.check_health()?;
    Ok(no_store(
        Json(serde_json::json!({ "status": "ok" })).into_response(),
    ))
}

async fn stylesheet(headers: HeaderMap) -> Response {
    cached_text_response(
        &headers,
        StatusCode::OK,
        templates::STYLESHEET,
        "text/css; charset=utf-8",
        STYLESHEET_CACHE_CONTROL,
        None,
    )
}

fn build_home_page(
    state: &AppState,
    selected_language: &str,
    category: Option<&str>,
    requested_page: Option<usize>,
    base_url: &str,
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
        categories: (*state.categories).clone(),
        base_url: base_url.to_string(),
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

fn collect_categories(
    repository: &dyn BlogRepository,
    languages: &[LanguageRecord],
) -> Result<Vec<String>> {
    let mut categories = BTreeSet::new();
    for language in languages {
        for post in repository.list_posts(&language.code)? {
            for category in post.categories {
                let trimmed_category = category.trim();
                if !trimmed_category.is_empty() {
                    categories.insert(trimmed_category.to_string());
                }
            }
        }
    }
    Ok(categories.into_iter().collect())
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

fn cached_text_response(
    request_headers: &HeaderMap,
    status: StatusCode,
    body: &str,
    content_type: &'static str,
    cache_control: &'static str,
    vary: Option<&'static str>,
) -> Response {
    let etag = format!("W/\"{:016x}\"", content_hash(body.as_bytes()));
    let not_modified = status == StatusCode::OK && if_none_match_matches(request_headers, &etag);
    let mut response = if not_modified {
        StatusCode::NOT_MODIFIED.into_response()
    } else {
        (
            status,
            [(header::CONTENT_TYPE, content_type)],
            body.to_string(),
        )
            .into_response()
    };
    let response_headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(&etag) {
        response_headers.insert(header::ETAG, value);
    }
    response_headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(cache_control),
    );
    if let Some(vary) = vary {
        response_headers.append(header::VARY, HeaderValue::from_static(vary));
    }
    response
}

fn cached_html_response(request_headers: &HeaderMap, status: StatusCode, body: String) -> Response {
    cached_text_response(
        request_headers,
        status,
        &body,
        "text/html; charset=utf-8",
        HTML_CACHE_CONTROL,
        Some(LOCALIZED_VARY),
    )
}

fn cached_json_response<T: Serialize>(
    request_headers: &HeaderMap,
    status: StatusCode,
    value: &T,
) -> Result<Response, AppError> {
    let body = serde_json::to_string(value)?;
    Ok(cached_text_response(
        request_headers,
        status,
        &body,
        "application/json",
        API_CACHE_CONTROL,
        Some(LOCALIZED_VARY),
    ))
}

fn if_none_match_matches(request_headers: &HeaderMap, etag: &str) -> bool {
    let etag_value = etag.strip_prefix("W/").unwrap_or(etag);
    request_headers
        .get_all(header::IF_NONE_MATCH)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .any(|candidate| {
            if candidate == "*" {
                return true;
            }
            let candidate_value = candidate.strip_prefix("W/").unwrap_or(candidate);
            candidate_value == etag_value
        })
}

fn no_store(mut response: Response) -> Response {
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(NO_STORE_CACHE_CONTROL),
    );
    response
}

async fn add_media_cache_control(request: Request, next: Next) -> Response {
    let is_media = request.uri().path().starts_with("/media/");
    let mut response = next.run(request).await;
    if is_media
        && matches!(
            response.status(),
            StatusCode::OK | StatusCode::PARTIAL_CONTENT | StatusCode::NOT_MODIFIED
        )
    {
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static(MEDIA_CACHE_CONTROL),
        );
    }
    response
}

impl PostWriteRequest {
    fn into_post_record(self, slug: &str) -> Result<PostRecord, AppError> {
        let normalized_slug = slug.trim();
        let normalized_language = self.language.trim().to_ascii_lowercase();
        let normalized_title = self.title.trim();
        if normalized_slug.is_empty() {
            return Err(AppError::bad_request("slug is required"));
        }
        if !normalized_slug.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        }) {
            return Err(AppError::bad_request(
                "slug may contain only letters, digits, hyphens and underscores",
            ));
        }
        if normalized_language.is_empty() {
            return Err(AppError::bad_request("language is required"));
        }
        if !normalized_language
            .chars()
            .all(|character| character.is_ascii_alphabetic() || character == '-')
        {
            return Err(AppError::bad_request(
                "language may contain only letters and hyphens",
            ));
        }
        if normalized_title.is_empty() {
            return Err(AppError::bad_request("title is required"));
        }
        let mut categories = Vec::new();
        for category in self.categories {
            let normalized_category = category.trim().to_ascii_lowercase();
            if !normalized_category.is_empty()
                && !categories
                    .iter()
                    .any(|existing| existing == &normalized_category)
            {
                categories.push(normalized_category);
            }
        }
        let published_unix_seconds = self
            .published_unix_seconds
            .unwrap_or_else(current_unix_seconds);
        let published_date = self
            .published_date
            .map(|date| date.trim().to_string())
            .filter(|date| !date.is_empty())
            .unwrap_or_else(|| format_unix_date(published_unix_seconds));
        let source_path = if self.source_path.trim().is_empty() {
            format!("{normalized_slug}.txt")
        } else {
            self.source_path.trim().to_string()
        };
        Ok(PostRecord {
            slug: normalized_slug.to_string(),
            language: normalized_language,
            title: normalized_title.to_string(),
            categories,
            body_html: self.body_html,
            source_path,
            published_unix_seconds,
            published_date,
        })
    }
}

fn require_editor_key(state: &AppState, headers: &HeaderMap) -> Result<(), AppError> {
    let provided_key = headers
        .get(EDITOR_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let Some(provided_key) = provided_key else {
        return Err(AppError::unauthorized(
            "rusty-blogger-post-editor-key header is required",
        ));
    };
    let is_valid = state
        .settings
        .rusty_blogger_post_editor_keys
        .iter()
        .any(|key| constant_time_eq(key.as_bytes(), provided_key.as_bytes()));
    if is_valid {
        Ok(())
    } else {
        Err(AppError::unauthorized(
            "invalid rusty-blogger-post-editor-key header",
        ))
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (left_byte, right_byte) in left.iter().zip(right) {
        difference |= left_byte ^ right_byte;
    }
    difference == 0
}

fn current_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn request_base_url(headers: &HeaderMap) -> String {
    let scheme = request_scheme(headers);
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("localhost");
    format!("{scheme}://{host}")
}

fn request_scheme(headers: &HeaderMap) -> &'static str {
    if let Some(forwarded_proto) = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
    {
        if forwarded_proto.eq_ignore_ascii_case("https") {
            return "https";
        }
        if forwarded_proto.eq_ignore_ascii_case("http") {
            return "http";
        }
    }
    if let Some(forwarded) = headers
        .get("forwarded")
        .and_then(|value| value.to_str().ok())
    {
        for part in forwarded.split(';') {
            if let Some(proto) = part.trim().strip_prefix("proto=") {
                let proto = proto.trim_matches('"');
                if proto.eq_ignore_ascii_case("https") {
                    return "https";
                }
                if proto.eq_ignore_ascii_case("http") {
                    return "http";
                }
            }
        }
    }
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    if host.starts_with("localhost") || host.starts_with("127.0.0.1") || host.starts_with("[::1]") {
        "http"
    } else {
        "https"
    }
}
