use anyhow::Result;
use axum::{
    extract::Path,
    http::{Method, StatusCode},
    routing::get,
    Extension, Json, Router,
};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc};
use chrono_tz::{Tz, US::Pacific};

use http_cache::{CACacheManager, CacheMode, HttpCache};
use http_cache_reqwest::Cache;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use rusqlite::Connection;
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing::info;
use tracing_subscriber::prelude::*;

mod flickr;

struct BirdDateAndTime {
    utc: DateTime<Utc>,
    local: DateTime<Tz>,
}

impl BirdDateAndTime {
    pub fn new_naive(date: NaiveDate, time: NaiveTime) -> Result<Self> {
        let no_tz = NaiveDateTime::new(date, time);
        let pacific = no_tz.and_local_timezone(Pacific);
        let best_case = pacific.single();
        let earliest = pacific.earliest();
        let riverside = best_case.or_else(|| earliest).expect("no idea time wise");

        Ok(Self {
            utc: riverside.with_timezone(&Utc),
            local: riverside,
        })
    }

    pub fn new(date: String, time: String) -> Result<Self> {
        let date_only = NaiveDate::parse_from_str(&date, "%Y-%m-%d")?;
        let time_only = NaiveTime::parse_from_str(&time, "%H:%M:%S")?;
        Self::new_naive(date_only, time_only)
    }

    fn new_date_only(date: String) -> Result<Self> {
        let date_only = NaiveDate::parse_from_str(&date, "%Y-%m-%d")?;
        Self::new_naive(date_only, NaiveTime::MIN)
    }
}

impl Into<DateTime<Utc>> for BirdDateAndTime {
    fn into(self) -> DateTime<Utc> {
        self.utc
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct Detection {
    when: DateTime<Utc>,
    common_name: String,
    scientific_name: String,
    confidence: f32,
    latitude: f32,
    longitude: f32,
    cutoff: f32,
    week: u32,
    sens: f32,
    overlap: f32,
    file_name: String,
}

#[derive(Serialize, Debug)]
struct Detections {
    when: DateTime<Utc>,
    common_name: String,
    scientific_name: String,
    average_confidence: f32,
}

#[derive(Serialize, Debug)]
struct DetectionsByCommonName {
    common_name: String,
    total: u32,
    average_confidence: f32,
    last_detection: DateTime<Utc>,
}

#[derive(Serialize, Debug)]
struct DetectionsByTimeAndCommonName {
    when: DateTime<Utc>,
    common_name: String,
    total: u32,
    average_confidence: f32,
}

#[derive(Serialize, Debug, Clone)]
struct FilesFor {
    when: DateTime<Utc>,
    confidence: f32,
    file_name: String,
    spectrogram_url: String,
    audio_url: String,
    available: Option<bool>,
}

impl FilesFor {
    fn into_with_available(self, available: bool) -> Self {
        Self {
            available: Some(available),
            ..self
        }
    }
}

#[derive(Serialize)]
pub struct DetectionsSummary {
    total: u64,
}

struct BirdDb {
    conn: Connection,
}

fn get_database() -> Result<String> {
    Ok(std::env::var("BIRDS_DB")?)
}

impl BirdDb {
    fn new() -> Result<Self> {
        Ok(Self {
            conn: Connection::open(get_database()?)?,
        })
    }

    fn common_name_to_scientific_name(&self) -> Result<HashMap<String, String>> {
        let mut stmt = self
            .conn
            .prepare(r"SELECT com_name, sci_name FROM detections GROUP BY com_name, sci_name")?;

        let res = stmt.query_map([], |row| {
            let common: String = row.get(0)?;
            let scientific: String = row.get(1)?;
            Ok((common, scientific))
        })?;

        Ok(res
            .into_iter()
            .map(|row| Ok(row?)) // Yeah yeah yeah TODO
            .collect::<Result<HashMap<_, _>>>()?)
    }

    fn by_day_and_common_name(&self) -> Result<Vec<DetectionsByTimeAndCommonName>> {
        let mut stmt = self.conn.prepare(
            r"SELECT
                date,
                com_name,
                COUNT(com_name) AS total,
                AVG(confidence) AS average_confidence
            FROM detections
            GROUP BY date, com_name",
        )?;

        let res = stmt.query_map([], |row| {
            let when = BirdDateAndTime::new_date_only(row.get(0)?).expect("invalid date and time");
            Ok(DetectionsByTimeAndCommonName {
                when: when.into(),
                common_name: row.get(1)?,
                total: row.get(2)?,
                average_confidence: row.get(3)?,
            })
        })?;

        Ok(res
            .into_iter()
            .map(|row| Ok(row?)) // Yeah yeah yeah TODO
            .collect::<Result<Vec<DetectionsByTimeAndCommonName>>>()?)
    }

    fn by_common_name(&self) -> Result<Vec<DetectionsByCommonName>> {
        let mut stmt = self.conn.prepare(
            r"SELECT
                com_name,
                COUNT(com_name) AS total,
                AVG(confidence) AS average_confidence,
                MAX(DATE) AS max_date,
                MAX(TIME) AS max_time
            FROM detections
            GROUP BY com_name",
        )?;

        let res = stmt.query_map([], |row| {
            let last_detection =
                BirdDateAndTime::new(row.get(3)?, row.get(4)?).expect("invalid date and time");
            Ok(DetectionsByCommonName {
                common_name: row.get(0)?,
                total: row.get(1)?,
                average_confidence: row.get(2)?,
                last_detection: last_detection.into(),
            })
        })?;

        Ok(res
            .into_iter()
            .map(|row| Ok(row?)) // Yeah yeah yeah TODO
            .collect::<Result<Vec<DetectionsByCommonName>>>()?)
    }

    fn detections(&self) -> Result<Vec<Detection>> {
        let mut stmt = self.conn.prepare(
            r"SELECT
                 date, time,
                 sci_name, com_name,
                 confidence,
                 lat, lon,
                 cutoff, week, sens, overlap, file_name
             FROM detections
             ORDER BY date, time, sci_name",
        )?;

        let entities = stmt.query_map([], |row| {
            let when = BirdDateAndTime::new(row.get(0)?, row.get(1)?).or_else(|_| {
                Err(rusqlite::Error::InvalidParameterName(
                    "DATE and TIME".into(),
                ))
            })?;

            Ok(Detection {
                when: when.into(),
                scientific_name: row.get(2)?,
                common_name: row.get(3)?,
                confidence: row.get(4)?,
                latitude: row.get(5)?,
                longitude: row.get(6)?,
                cutoff: row.get(7)?,
                week: row.get(8)?,
                sens: row.get(9)?,
                overlap: row.get(10)?,
                file_name: row.get(11)?,
            })
        })?;

        Ok(entities
            .into_iter()
            .map(|row| Ok(row?)) // Yeah yeah yeah TODO
            .collect::<Result<Vec<Detection>>>()?)
    }

    fn daily_detections(&self, common_name: &str) -> Result<Vec<Daily>> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT date, COUNT(*) FROM detections
            WHERE com_name = ?
            GROUP BY date
            ORDER BY date
            ",
        )?;

        let daily = stmt.query_map([common_name], |row| {
            let date = BirdDateAndTime::new_date_only(row.get(0)?).or_else(|_| {
                Err(rusqlite::Error::InvalidParameterName(
                    "DATE and TIME".into(),
                ))
            })?;

            Ok(Daily {
                date: date.into(),
                detections: row.get(1)?,
            })
        })?;

        Ok(daily
            .into_iter()
            .map(|row| Ok(row?))
            .collect::<Result<Vec<Daily>>>()?)
    }

    fn hourly_detections(&self, common_name: &str) -> Result<Vec<Hourly>> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT q.hour, COUNT(q.hour) FROM (
                SELECT com_name, strftime('%H:00:00', time) AS hour FROM detections
                WHERE com_name = ?
            ) AS q
            GROUP BY q.hour
            ORDER BY q.hour
            ",
        )?;

        let hourly = stmt.query_map([common_name], |row| {
            let time: String = row.get(0)?;
            let time = NaiveTime::parse_from_str(&time, "%H:%M:%S").or_else(|_| {
                Err(rusqlite::Error::InvalidParameterName(
                    "DATE and TIME".into(),
                ))
            })?;

            Ok(Hourly {
                time,
                number: time.hour(),
                detections: row.get(1)?,
            })
        })?;

        Ok(hourly
            .into_iter()
            .map(|row| Ok(row?))
            .collect::<Result<Vec<Hourly>>>()?)
    }

    fn summarize_detections(&self, common_name: &str) -> Result<DetectionsSummary> {
        let mut stmt = self
            .conn
            .prepare(r"SELECT COUNT(date) FROM detections WHERE com_name = ?")?;

        let total_detections: u64 = stmt.query_row([common_name], |row| row.get(0))?;

        Ok(DetectionsSummary {
            total: total_detections,
        })
    }

    fn files_for(&self, common_name: &str) -> Result<Vec<FilesFor>> {
        let mut stmt = self.conn.prepare(
            r"SELECT date, time, file_name, confidence
             FROM detections
             WHERE com_name = ?
             ORDER BY confidence DESC LIMIT 100",
        )?;

        let entities = stmt.query_map([common_name], |row| {
            let when = BirdDateAndTime::new(row.get(0)?, row.get(1)?).or_else(|_| {
                Err(rusqlite::Error::InvalidParameterName(
                    "DATE and TIME".into(),
                ))
            })?;

            let date_string = when.local.format("%Y-%m-%d");
            let file_name: String = row.get(2)?;

            fn urlify_string(i: &str) -> String {
                i.replace(" ", "_")
            }

            let audio_url = || -> Result<String, rusqlite::Error> {
                Ok(format!(
                    "http://192.168.0.164/By_Date/{}/{}/{}",
                    &date_string,
                    urlify_string(&common_name),
                    &file_name
                ))
            };

            let spectrogram_url =
                || -> Result<String, rusqlite::Error> { Ok(format!("{}.png", audio_url()?)) };

            let spectrogram_url = spectrogram_url()?;
            let audio_url = audio_url()?;
            let when = when.into();
            let confidence = row.get(3)?;

            Ok(FilesFor {
                when,
                file_name,
                confidence,
                spectrogram_url,
                audio_url,
                available: None,
            })
        })?;

        let files_for = entities
            .into_iter()
            .map(|row| Ok(row?)) // TODO Yeah, gross. Error types?
            .collect::<Result<Vec<FilesFor>>>()?;

        Ok(files_for)
    }
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

async fn check_file_available(file: FilesFor) -> FilesFor {
    match new_http_client().head(&file.spectrogram_url).send().await {
        Ok(r) => match r.status() {
            StatusCode::OK => file.into_with_available(true),
            _ => file.into_with_available(false),
        },
        Err(_) => file.into_with_available(false),
    }
}

async fn check_available(files: Vec<FilesFor>) -> Result<Vec<FilesFor>> {
    use futures::StreamExt;
    use tokio_stream::{self as stream};

    const CONCURRENT_REQUESTS: usize = 5;
    Ok(stream::iter(files.into_iter())
        .map(|row| check_file_available(row))
        .buffered(CONCURRENT_REQUESTS)
        .collect::<Vec<_>>()
        .await)
}

#[derive(Serialize)]
struct Daily {
    date: DateTime<Utc>,
    detections: u64,
}

#[derive(Serialize)]
struct Hourly {
    number: u32,
    time: NaiveTime,
    detections: u64,
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

    let files = check_available(files)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(FilesResponse { detections, files }))
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

struct AppState {}

fn get_rust_log() -> String {
    std::env::var("RUST_LOG").unwrap_or_else(|_| "birbs=info,tower_http=debug".into())
}

fn get_flickr_api_key() -> Result<String> {
    Ok(std::env::var("FLICKR_API_KEY")?)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(get_rust_log()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db = BirdDb::new()?;

    let _detections = db.detections()?;
    let _by_common_name = db.by_common_name()?;
    let _by_day_and_common_name = db.by_day_and_common_name()?;
    let _common_name_to_scientific_name = db.common_name_to_scientific_name()?;
    let _files_for = db.files_for("American Crow")?;
    let _hourly = db.hourly_detections("American Crow")?;
    let _daily = db.daily_detections("American Crow")?;

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
