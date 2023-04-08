use anyhow::Result;
use axum::{
    extract::Path,
    http::{Method, StatusCode},
    routing::get,
    Extension, Json, Router,
};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use rusqlite::Connection;
use std::{collections::HashMap, sync::Arc};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing::info;
use tracing_subscriber::prelude::*;

struct BirdDb {
    conn: Connection,
}

struct BirdDateAndTime {
    date_time: DateTime<Utc>,
}

use chrono_tz::US::Pacific;
use serde::Serialize;

impl BirdDateAndTime {
    pub fn new_naive(date: NaiveDate, time: NaiveTime) -> Result<Self> {
        let no_tz = NaiveDateTime::new(date, time);
        let pacific = no_tz.and_local_timezone(Pacific);
        let best_case = pacific.single();
        let earliest = pacific.earliest();
        let riverside = best_case.or_else(|| earliest).expect("no idea time wise");

        Ok(Self {
            date_time: riverside.with_timezone(&Utc),
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
        self.date_time
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

#[derive(Serialize, Debug)]
struct FilesFor {
    when: DateTime<Utc>,
    confidence: f32,
    file_name: String,
}

impl BirdDb {
    fn new() -> Result<Self> {
        Ok(Self {
            conn: Connection::open("birds.db")?,
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

            Ok(FilesFor {
                when: when.into(),
                file_name: row.get(2)?,
                confidence: row.get(3)?,
            })
        })?;

        Ok(entities
            .into_iter()
            .map(|row| Ok(row?)) // Yeah yeah yeah TODO
            .collect::<Result<Vec<FilesFor>>>()?)
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

#[axum_macros::debug_handler]
async fn files_for(Path(common_name): Path<String>) -> Result<Json<Vec<FilesFor>>, StatusCode> {
    let db = BirdDb::new().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        db.files_for(&common_name)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

struct AppState {}

fn get_rust_log() -> String {
    std::env::var("RUST_LOG").unwrap_or_else(|_| "birbs=info,tower_http=debug".into())
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
        .layer(cors)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(false)),
        )
        .layer(Extension(app_state));

    info!("listening on 0.0.0.0:3000");

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
