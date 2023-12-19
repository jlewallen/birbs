use anyhow::Result;
use axum::extract::Path;
use axum::{http::Method, routing::get, Extension, Router};
use axum::{http::StatusCode, Json};
use http_cache::{CACacheManager, CacheMode, HttpCache};
use http_cache_reqwest::Cache;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use serde::Serialize;

use std::collections::HashMap;
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing::info;

use crate::{
    flickr, get_flickr_api_key, BirdDb, Daily, DetectionsByCommonName,
    DetectionsByTimeAndCommonName, DetectionsSummary, FilesFor, Hourly, Recently,
};

struct AppState {}

pub async fn execute() -> Result<()> {
    let db = BirdDb::new()?;

    let _detections = db.detections()?;
    let _by_common_name = db.by_common_name()?;
    let _by_day_and_common_name = db.by_day_and_common_name()?;
    let _common_name_to_scientific_name = db.common_name_to_scientific_name()?;
    let _files_for = db.files_for("American Crow")?;
    let _hourly = db.hourly_detections("American Crow")?;
    let _daily = db.daily_detections("American Crow")?;
    let _recently = db.recently()?;

    // let flickr = flickr::FlickrClient::new(&get_flickr_api_key()?);
    // let photos = flickr.search("Chestnut-rumped Thornbill").await?;
    // use futures::future;
    // let _photos = future::try_join_all(photos.iter().map(|p| flickr.image(p))).await?;

    let app_state = Arc::new(AppState {});

    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods([Method::GET, Method::POST])
        // allow requests from any origin
        .allow_origin(Any);

    let app = Router::new()
        .route("/", get(|| async { "hello, world!" }))
        .route(
            "/common-name-to-scientific-name.json",
            get(common_name_to_scientific_name),
        )
        .route("/recently.json", get(recently))
        .route("/by-common-name.json", get(by_common_name))
        .route("/by-day-and-common-name.json", get(by_day_and_common_name))
        .route("/:common-name/files.json", get(files_for))
        .route("/:common-name/hourly.json", get(hourly_for))
        .route("/:common-name/daily.json", get(daily_for))
        .route("/:common-name/photo.png", get(photo_for))
        .layer(cors)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(false)),
        )
        .layer(Extension(app_state));

    info!("listening on 0.0.0.0:3100");

    axum::Server::bind(&"0.0.0.0:3100".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

#[axum_macros::debug_handler]
async fn common_name_to_scientific_name() -> Result<Json<HashMap<String, String>>, StatusCode> {
    let db = BirdDb::new().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        db.common_name_to_scientific_name()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

#[axum_macros::debug_handler]
async fn by_common_name() -> Result<Json<Vec<DetectionsByCommonName>>, StatusCode> {
    let db = BirdDb::new().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        db.by_common_name()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

#[axum_macros::debug_handler]
async fn by_day_and_common_name() -> Result<Json<Vec<DetectionsByTimeAndCommonName>>, StatusCode> {
    let db = BirdDb::new().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        db.by_day_and_common_name()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

#[axum_macros::debug_handler]
async fn hourly_for(Path(common_name): Path<String>) -> Result<Json<Vec<Hourly>>, StatusCode> {
    let db = BirdDb::new().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let detections = db
        .hourly_detections(&common_name)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(detections))
}

#[axum_macros::debug_handler]
async fn daily_for(Path(common_name): Path<String>) -> Result<Json<Vec<Daily>>, StatusCode> {
    let db = BirdDb::new().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let detections = db
        .daily_detections(&common_name)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(detections))
}

#[derive(Serialize)]
struct FilesResponse {
    detections: DetectionsSummary,
    files: Vec<FilesFor>,
}

#[axum_macros::debug_handler]
async fn files_for(Path(common_name): Path<String>) -> Result<Json<FilesResponse>, StatusCode> {
    let db = BirdDb::new().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let detections = db
        .summarize_detections(&common_name)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let files = db
        .files_for(&common_name)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let files = check_files_available(files)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(FilesResponse { detections, files }))
}

#[derive(Serialize)]
struct RecentlyResponse {
    detections: Vec<Recently>,
}

#[axum_macros::debug_handler]
async fn recently() -> Result<Json<RecentlyResponse>, StatusCode> {
    let db = BirdDb::new().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let detections = db
        .recently()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let detections = check_recentlies_available(detections)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RecentlyResponse { detections }))
}

fn new_http_client() -> ClientWithMiddleware {
    return ClientBuilder::new(reqwest::Client::new())
        .with(Cache(HttpCache {
            mode: CacheMode::ForceCache,
            manager: CACacheManager::default(),
            options: None,
        }))
        .build();
}

async fn photo_for(Path(common_name): Path<String>) -> Result<Vec<u8>, StatusCode> {
    let flickr = flickr::FlickrClient::new(
        &get_flickr_api_key().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        new_http_client(),
    );
    let mut photos = flickr
        .search(&common_name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match photos.pop() {
        Some(photo) => Ok(flickr
            .image(&photo)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn head_url(url: &str) -> bool {
    match new_http_client().head(url).send().await {
        Ok(r) => match r.status() {
            StatusCode::OK => true,
            _ => false,
        },
        Err(_) => false,
    }
}

async fn check_recently_available(file: Recently) -> Recently {
    let available = head_url(&file.spectrogram_url).await && head_url(&file.audio_url).await;
    file.into_with_available(available)
}

async fn check_recentlies_available(files: Vec<Recently>) -> Result<Vec<Recently>> {
    use futures::StreamExt;
    use tokio_stream::{self as stream};

    const CONCURRENT_REQUESTS: usize = 5;
    Ok(stream::iter(files.into_iter())
        .map(|row| check_recently_available(row))
        .buffered(CONCURRENT_REQUESTS)
        .collect::<Vec<_>>()
        .await)
}

async fn check_file_available(file: FilesFor) -> FilesFor {
    let available = head_url(&file.spectrogram_url).await && head_url(&file.audio_url).await;
    file.into_with_available(available)
}

async fn check_files_available(files: Vec<FilesFor>) -> Result<Vec<FilesFor>> {
    use futures::StreamExt;
    use tokio_stream::{self as stream};

    const CONCURRENT_REQUESTS: usize = 5;
    Ok(stream::iter(files.into_iter())
        .map(|row| check_file_available(row))
        .buffered(CONCURRENT_REQUESTS)
        .collect::<Vec<_>>()
        .await)
}
