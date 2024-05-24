use anyhow::Result;
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use clap::Args;
use futures::stream;
use influxdb2::models::DataPoint;
use influxdb2::Client;
use itertools::Itertools;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs::File;
use std::io::{self, BufRead};
use std::io::{BufReader, Seek};
use std::path::Path;

use crate::BirdDateAndTime;

#[derive(Debug, Args)]
pub struct Command {
    #[arg(short, long)]
    watch: bool,
    file: String,
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

pub async fn execute(cmd: Command) -> Result<()> {
    let log = BirdLog::new(cmd.file);

    if cmd.watch {
        log.watch()
    } else {
        log.publish_all().await
    }
}

pub struct BirdLog {
    path: String,
}

impl BirdLog {
    pub fn new(path: String) -> Self {
        Self { path }
    }

    pub fn watch(&self) -> Result<()> {
        let mut f = std::fs::File::open(&self.path)?;
        let mut pos = std::fs::metadata(&self.path)?.len();

        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
        watcher.watch(self.path.as_ref(), RecursiveMode::NonRecursive)?;

        for res in rx {
            match res {
                Ok(_event) => {
                    if f.metadata()?.len() == pos {
                        continue;
                    }

                    f.seek(std::io::SeekFrom::Start(pos + 1))?;

                    pos = f.metadata()?.len();

                    let reader = BufReader::new(&f);
                    for line in reader.lines() {
                        println!("> {:?}", line.unwrap());
                    }
                }
                Err(error) => println!("{error:?}"),
            }
        }

        Ok(())
    }

    fn parse_entry(&self, line: String) -> Result<LogEntry> {
        let fields = line.split(";").collect_vec();

        let date: NaiveDate = fields[0].parse()?;
        let time: NaiveTime = fields[1].parse()?;
        let scientific_name = fields[2].to_owned();
        let common_name = fields[3].to_owned();
        let confidence: f64 = fields[4].parse()?;
        let date_time = BirdDateAndTime::new_naive(date, time)?.into();

        Ok(LogEntry {
            date_time,
            common_name,
            scientific_name,
            confidence,
        })
    }

    pub async fn publish_all(&self) -> Result<()> {
        let lines = read_lines(&self.path)?;

        let host = std::env::var("INFLUXDB_HOST").unwrap();
        let org = std::env::var("INFLUXDB_ORG").unwrap();
        let token = std::env::var("INFLUXDB_TOKEN").unwrap();
        let client = Client::new(host, org, token);

        if true {
            for line in lines.skip(1) {
                if let Ok(line) = line {
                    let entry = self.parse_entry(line)?;

                    println!("{:?}", entry);

                    let dp: DataPoint = entry.into();

                    client.write("home", stream::iter(vec![dp])).await?;
                }
            }
        } else {
            let start = NaiveDate::from_ymd_opt(2020, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap();
            let stop = NaiveDate::from_ymd_opt(2024, 1, 1)
                .unwrap()
                .and_hms_opt(23, 59, 59)
                .unwrap();

            client
                .delete(
                    "home",
                    start,
                    stop,
                    Some("_measurement=\"birds\"".to_owned()),
                )
                .await?;
        }

        Ok(())
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct LogEntry {
    date_time: DateTime<Utc>,
    common_name: String,
    scientific_name: String,
    confidence: f64,
}

#[allow(dead_code)]
pub struct InfluxLineProtocol(String);

impl Into<DataPoint> for LogEntry {
    fn into(self) -> DataPoint {
        DataPoint::builder("birds")
            .tag("station", "backyard")
            .field(self.common_name, self.confidence)
            .build()
            .expect("Error building data point")
    }
}

impl Into<InfluxLineProtocol> for LogEntry {
    fn into(self) -> InfluxLineProtocol {
        InfluxLineProtocol(format!(
            "birds,station=backyard {}={} {}",
            self.common_name.replace(" ", "\\ "),
            self.confidence,
            self.date_time.timestamp_nanos()
        ))
    }
}
