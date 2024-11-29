use chrono::{DateTime, Days, Duration, NaiveDate, Timelike, Utc};
use clap::Parser;
use reqwest::blocking::Client;
use reqwest::header::USER_AGENT;
use serde_json::Value;
use std::fs::File;
use std::io::prelude::*;

#[derive(Parser)]
struct Cli {
    ticker: String,
    year: i32,
    month: u32,
    filename: String,
}

fn unix_timestamp(date: NaiveDate) -> i64 {
    date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp()
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let start = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let end = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
            .unwrap()
            .checked_sub_days(Days::new(1))
            .unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
            .unwrap()
            .checked_sub_days(Days::new(1))
            .unwrap()
    };
    end.signed_duration_since(start).num_days() as u32 + 1
}

fn get_url(year: i32, month: u32, ticker: &str) -> String {
    let start = unix_timestamp(NaiveDate::from_ymd_opt(year, month, 1).unwrap());
    let end =
        unix_timestamp(NaiveDate::from_ymd_opt(year, month, days_in_month(year, month)).unwrap());
    format!(
        "https://query2.finance.yahoo.com/v8/finance/chart/{}?period1={}&period2={}&interval=1d&events=history&includeAdjustedClose=true", 
        ticker,
        start,
        end,
    )
}

fn parse_data(response: &Value) -> (Vec<DateTime<Utc>>, Vec<f64>) {
    let Value::Object(obj) = response else {
        unimplemented!()
    };
    let Value::Object(chart) = obj.get("chart").unwrap() else {
        unimplemented!()
    };
    let Value::Array(result) = chart.get("result").unwrap() else {
        unimplemented!()
    };
    let Value::Object(result) = result.first().unwrap() else {
        unimplemented!()
    };
    let Value::Object(indicators) = result.get("indicators").unwrap() else {
        unimplemented!()
    };
    let Value::Array(adjclose) = indicators.get("adjclose").unwrap() else {
        unimplemented!()
    };
    let Value::Object(adjclose) = adjclose.first().unwrap() else {
        unimplemented!()
    };
    let Value::Array(adjclose) = adjclose.get("adjclose").unwrap() else {
        unimplemented!()
    };
    let close = adjclose
        .iter()
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
        .iter()
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
    (timestamp, close)
}

fn main() {
    let cli = Cli::parse();
    let url = get_url(cli.year, cli.month, &cli.ticker);
    let client = Client::new();
    let response = client
        .get(url)
        .header(USER_AGENT, "curl/8.7.1")
        .send()
        .unwrap()
        .json::<serde_json::Value>()
        .unwrap();
    let (timestamp, close) = parse_data(&response);
    let mut file = File::create(cli.filename).unwrap();
    writeln!(file, "Date,Close").unwrap();
    for (time, close) in timestamp.into_iter().zip(close) {
        writeln!(file, "{},{}", time.format("%Y-%m-%d"), close).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_days_in_month() {
        assert_eq!(days_in_month(2024, 10), 31);
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2023, 2), 28);
        assert_eq!(days_in_month(2023, 12,), 31);
    }

    #[test]
    fn test_unix_timestamp() {
        assert_eq!(
            unix_timestamp(NaiveDate::from_ymd_opt(2024, 11, 24).unwrap()),
            1732406400
        );
    }

    #[test]
    fn test_get_url() {
        assert_eq!(
            get_url(2024, 11, "a"),
            "https://query2.finance.yahoo.com/v8/finance/chart/a?period1=1730419200&period2=1732924800&interval=1d&events=history&includeAdjustedClose=true",
        );
    }

    #[test]
    fn test_parse_data() {
        let timestamp0 = 1732406400;
        let date_time0 = NaiveDate::from_ymd_opt(2024, 11, 24)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();

        // This timestamp corresponds to 1 hr in the past.  The code adds 1 hour offset.
        let timestamp1 = 1732662000;
        let date_time1 = NaiveDate::from_ymd_opt(2024, 11, 27)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();

        let response = json!({
            "chart": {
                "result": [
                    {
                        "indicators": {
                            "adjclose": [
                                {
                                    "adjclose": [
                                        1,
                                        2,
                                    ]
                                },
                            ]
                        },
                        "timestamp": [
                            timestamp0,
                            timestamp1,
                        ]
                    },
                ]
            }
        });
        let expected_timestamps = vec![date_time0, date_time1];
        let expected_closes = vec![1f64, 2f64];
        let (timestamps, closes) = parse_data(&response);
        assert_eq!(expected_closes, closes);
        assert_eq!(expected_timestamps, timestamps);
    }
}
