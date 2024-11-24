use chrono::{DateTime, Duration, NaiveDate, Timelike};
use reqwest::blocking::Client;
use reqwest::header::USER_AGENT;
use serde_json::Value;
use std::fs::File;
use std::io::prelude::*;

fn unix_timestamp(date: NaiveDate) -> i64 {
    date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp()
}

fn main() {
    let start = unix_timestamp(NaiveDate::from_ymd_opt(2024, 10, 1).unwrap());
    let end = unix_timestamp(NaiveDate::from_ymd_opt(2024, 10, 31).unwrap());
    let ticker = "EURCHF=x";
    let output_file = "./eur.csv";
    let url = format!(
        "https://query2.finance.yahoo.com/v8/finance/chart/{}?period1={}&period2={}&interval=1d&events=history&includeAdjustedClose=true",
        ticker, start, end);
    let client = Client::new();
    let data = client
        .get(&url)
        .header(USER_AGENT, "curl/8.7.1")
        .send()
        .unwrap()
        .json::<serde_json::Value>()
        .unwrap();

    let Value::Object(obj) = data else {
        unimplemented!()
    };
    let Value::Object(chart) = obj.get("chart").unwrap() else {
        unimplemented!()
    };
    let Value::Array(result) = chart.get("result").unwrap() else {
        unimplemented!()
    };
    let Value::Object(result) = result.get(0).unwrap() else {
        unimplemented!()
    };
    let Value::Object(indicators) = result.get("indicators").unwrap() else {
        unimplemented!()
    };
    let Value::Array(adjclose) = indicators.get("adjclose").unwrap() else {
        unimplemented!()
    };
    let Value::Object(adjclose) = adjclose.get(0).unwrap() else {
        unimplemented!()
    };
    let Value::Array(adjclose) = adjclose.get("adjclose").unwrap() else {
        unimplemented!()
    };
    let close = adjclose
        .into_iter()
        .map(|value| {
            let Value::Number(value) = value else {
                unimplemented!()
            };
            value.as_f64().unwrap()
        })
        .collect::<Vec<_>>();
    let Value::Array(timestamp) = result.get("timestamp").unwrap() else {
        unimplemented!()
    };
    let timestamp = timestamp
        .into_iter()
        .map(|value| {
            let Value::Number(value) = value else {
                unimplemented!()
            };
            let timestamp = DateTime::from_timestamp(value.as_i64().unwrap(), 0).unwrap();
            if timestamp.hour() == 23 {
                timestamp + Duration::hours(1)
            } else {
                timestamp
            }
        })
        .collect::<Vec<_>>();

    let mut file = File::create(output_file).unwrap();
    writeln!(file, "Date,Close").unwrap();
    for (time, close) in timestamp.into_iter().zip(close) {
        writeln!(file, "{},{}", time.format("%Y-%m-%d"), close).unwrap();
    }
}
