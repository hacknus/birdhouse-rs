#![cfg(feature = "server")]

use chrono::{Duration as ChronoDuration, Utc};
use serde_json::Value;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_postgres::{Client, NoTls};

#[derive(Debug, Clone)]
pub enum TimeSeriesValue {
    Double(f64),
    Bool(bool),
    Text(String),
}

impl TimeSeriesValue {
    fn split(self) -> (Option<f64>, Option<bool>, Option<String>) {
        match self {
            Self::Double(v) => (Some(v), None, None),
            Self::Bool(v) => (None, Some(v), None),
            Self::Text(v) => (None, None, Some(v)),
        }
    }
}

#[derive(Clone)]
pub struct PostgresTimeSeriesStore {
    inner: Arc<Inner>,
}

struct Inner {
    table: String,
    bucket: String,
    dsn: String,
    auto_create: bool,
    connect_timeout_s: u64,
    connection: Mutex<Option<Arc<Client>>>,
}

impl PostgresTimeSeriesStore {
    pub fn from_env() -> Result<Self, String> {
        let table = std::env::var("POSTGRES_TABLE").unwrap_or_else(|_| "influx_points".to_string());
        if !is_valid_identifier(&table) {
            return Err(format!(
                "Invalid POSTGRES_TABLE {:?}. Use a simple SQL identifier.",
                table
            ));
        }

        let bucket = std::env::var("POSTGRES_BUCKET")
            .or_else(|_| std::env::var("INFLUXDB_BUCKET"))
            .unwrap_or_else(|_| "voegeli".to_string());
        let dsn = build_dsn_from_env()?;
        let auto_create = parse_bool(std::env::var("POSTGRES_AUTO_CREATE").ok(), true);
        let connect_timeout_s = std::env::var("POSTGRES_CONNECT_TIMEOUT")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(5);

        Ok(Self {
            inner: Arc::new(Inner {
                table,
                bucket,
                dsn,
                auto_create,
                connect_timeout_s,
                connection: Mutex::new(None),
            }),
        })
    }

    pub fn bucket(&self) -> &str {
        &self.inner.bucket
    }

    pub async fn close(&self) {
        let mut lock = self.inner.connection.lock().await;
        *lock = None;
    }

    pub async fn write_field(
        &self,
        measurement: &str,
        field: &str,
        value: TimeSeriesValue,
        unit: Option<&str>,
        location: Option<&str>,
        value_type: Option<&str>,
        bucket: Option<&str>,
    ) -> Result<(), String> {
        let client = self.ensure_connection().await?;
        let ts = Utc::now();
        let bucket_value = bucket.unwrap_or(&self.inner.bucket).to_string();
        let unit_value = unit.map(str::to_string);
        let location_value = location.map(str::to_string);
        let type_value = value_type.map(str::to_string);
        let (value_double, value_bool, value_text) = value.split();

        client
            .execute(
                &self.insert_sql(),
                &[
                    &bucket_value,
                    &ts,
                    &measurement,
                    &field,
                    &value_double,
                    &value_bool,
                    &value_text,
                    &unit_value,
                    &location_value,
                    &type_value,
                ],
            )
            .await
            .map_err(|e| format!("PostgreSQL insert failed: {e}"))?;

        Ok(())
    }

    pub async fn write_device_data(
        &self,
        device_data: &Value,
        measurement: Option<&str>,
    ) -> Result<(), String> {
        let root = device_data
            .as_object()
            .ok_or_else(|| "device_data must be an object.".to_string())?;

        let measurement_value = if let Some(m) = measurement {
            m.to_string()
        } else {
            root.get("device")
                .and_then(Value::as_str)
                .filter(|v| !v.trim().is_empty())
                .map(str::to_string)
                .ok_or_else(|| "device_data must contain a non-empty 'device' value.".to_string())?
        };

        let data = root
            .get("data")
            .and_then(Value::as_object)
            .ok_or_else(|| "device_data['data'] must be a dictionary/object.".to_string())?;

        let ts = Utc::now();
        let bucket = self.inner.bucket.clone();
        let sql = self.insert_sql();
        let client = self.ensure_connection().await?;

        for (field, value) in data {
            if field.ends_with("_unit") || field.ends_with("_location") || field.ends_with("_type") {
                continue;
            }
            if value.is_null() {
                continue;
            }

            let (value_double, value_bool, value_text) = split_json_value(value);
            let unit = data
                .get(&format!("{field}_unit"))
                .and_then(json_value_to_string);
            let location = data
                .get(&format!("{field}_location"))
                .and_then(json_value_to_string);
            let value_type = data
                .get(&format!("{field}_type"))
                .and_then(json_value_to_string);

            client
                .execute(
                    &sql,
                    &[
                        &bucket,
                        &ts,
                        &measurement_value,
                        &field,
                        &value_double,
                        &value_bool,
                        &value_text,
                        &unit,
                        &location,
                        &value_type,
                    ],
                )
                .await
                .map_err(|e| format!("PostgreSQL insert failed: {e}"))?;
        }

        Ok(())
    }

    pub async fn query_last(
        &self,
        data_since: &str,
        bucket: Option<&str>,
        field: &str,
        unit: Option<&str>,
    ) -> Result<Option<TimeSeriesValue>, String> {
        let since = Utc::now() - parse_duration(data_since)?;
        let query_bucket = bucket.unwrap_or(&self.inner.bucket);
        let client = self.ensure_connection().await?;

        let row = if let Some(unit) = unit {
            let sql = format!(
                "SELECT value_double, value_bool, value_text
                 FROM {}
                 WHERE bucket = $1
                   AND field = $2
                   AND ts >= $3
                   AND unit = $4
                 ORDER BY ts DESC
                 LIMIT 1",
                self.inner.table
            );
            client
                .query_opt(&sql, &[&query_bucket, &field, &since, &unit])
                .await
                .map_err(|e| format!("PostgreSQL query failed: {e}"))?
        } else {
            let sql = format!(
                "SELECT value_double, value_bool, value_text
                 FROM {}
                 WHERE bucket = $1
                   AND field = $2
                   AND ts >= $3
                 ORDER BY ts DESC
                 LIMIT 1",
                self.inner.table
            );
            client
                .query_opt(&sql, &[&query_bucket, &field, &since])
                .await
                .map_err(|e| format!("PostgreSQL query failed: {e}"))?
        };

        let Some(row) = row else {
            return Ok(None);
        };

        let value_double: Option<f64> = row.get(0);
        let value_bool: Option<bool> = row.get(1);
        let value_text: Option<String> = row.get(2);

        if let Some(v) = value_double {
            return Ok(Some(TimeSeriesValue::Double(v)));
        }
        if let Some(v) = value_bool {
            return Ok(Some(TimeSeriesValue::Bool(v)));
        }

        Ok(value_text.map(TimeSeriesValue::Text))
    }

    pub async fn query_last_f64(
        &self,
        data_since: &str,
        bucket: Option<&str>,
        field: &str,
        unit: Option<&str>,
    ) -> Result<Option<f64>, String> {
        let value = self.query_last(data_since, bucket, field, unit).await?;
        Ok(match value {
            Some(TimeSeriesValue::Double(v)) => Some(v),
            _ => None,
        })
    }

    async fn ensure_connection(&self) -> Result<Arc<Client>, String> {
        let mut lock = self.inner.connection.lock().await;
        let needs_connect = lock.as_ref().map(|c| c.is_closed()).unwrap_or(true);

        if needs_connect {
            let client = self.connect_new().await?;
            if self.inner.auto_create {
                self.initialize_schema(client.as_ref()).await?;
            }
            *lock = Some(client);
        }

        lock.as_ref()
            .map(Arc::clone)
            .ok_or_else(|| "PostgreSQL connection is not available.".to_string())
    }

    async fn connect_new(&self) -> Result<Arc<Client>, String> {
        let mut cfg = tokio_postgres::Config::from_str(&self.inner.dsn)
            .map_err(|e| format!("Invalid PostgreSQL DSN: {e}"))?;
        cfg.connect_timeout(Duration::from_secs(self.inner.connect_timeout_s));

        let (client, connection) = cfg
            .connect(NoTls)
            .await
            .map_err(|e| format!("Failed to connect to PostgreSQL: {e}"))?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("PostgreSQL connection task failed: {e}");
            }
        });

        Ok(Arc::new(client))
    }

    async fn initialize_schema(&self, client: &Client) -> Result<(), String> {
        let ts_idx = format!("{}_ts_idx", self.inner.table);
        let mf_idx = format!("{}_measurement_field_ts_idx", self.inner.table);

        let create_table_sql = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                id BIGSERIAL PRIMARY KEY,
                bucket TEXT NOT NULL,
                ts TIMESTAMPTZ NOT NULL,
                measurement TEXT NOT NULL,
                field TEXT NOT NULL,
                value_double DOUBLE PRECISION,
                value_bool BOOLEAN,
                value_text TEXT,
                unit TEXT,
                location TEXT,
                type TEXT,
                CHECK (num_nonnulls(value_double, value_bool, value_text) = 1)
            )
            "#,
            self.inner.table
        );
        let create_ts_idx_sql = format!(
            "CREATE INDEX IF NOT EXISTS {} ON {} (ts DESC)",
            ts_idx, self.inner.table
        );
        let create_mf_idx_sql = format!(
            "CREATE INDEX IF NOT EXISTS {} ON {} (measurement, field, ts DESC)",
            mf_idx, self.inner.table
        );

        client
            .execute(&create_table_sql, &[])
            .await
            .map_err(|e| format!("Failed to initialize PostgreSQL schema: {e}"))?;
        client
            .execute(&create_ts_idx_sql, &[])
            .await
            .map_err(|e| format!("Failed to create ts index: {e}"))?;
        client
            .execute(&create_mf_idx_sql, &[])
            .await
            .map_err(|e| format!("Failed to create measurement/field index: {e}"))?;

        Ok(())
    }

    fn insert_sql(&self) -> String {
        format!(
            "INSERT INTO {}
             (bucket, ts, measurement, field, value_double, value_bool, value_text, unit, location, type)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            self.inner.table
        )
    }
}

fn is_valid_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(c) if c == '_' || c.is_ascii_alphabetic() => {}
        _ => return false,
    }

    chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

fn parse_bool(value: Option<String>, default: bool) -> bool {
    value
        .map(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

fn build_dsn_from_env() -> Result<String, String> {
    if let Ok(dsn) = std::env::var("POSTGRES_DSN") {
        if !dsn.trim().is_empty() {
            return Ok(dsn);
        }
    }

    let host = std::env::var("POSTGRES_HOST").ok();
    let dbname = std::env::var("POSTGRES_DB").ok();
    let user = std::env::var("POSTGRES_USER").ok();
    let password = std::env::var("POSTGRES_PASSWORD").ok();
    let port = std::env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());

    match (host, dbname, user, password) {
        (Some(host), Some(dbname), Some(user), Some(password)) => Ok(format!(
            "host={} port={} dbname={} user={} password={}",
            host, port, dbname, user, password
        )),
        _ => Err(
            "Set POSTGRES_DSN or all of POSTGRES_HOST/POSTGRES_DB/POSTGRES_USER/POSTGRES_PASSWORD."
                .to_string(),
        ),
    }
}

fn split_json_value(value: &Value) -> (Option<f64>, Option<bool>, Option<String>) {
    match value {
        Value::Bool(v) => (None, Some(*v), None),
        Value::Number(v) => match v.as_f64() {
            Some(num) => (Some(num), None, None),
            None => (None, None, Some(v.to_string())),
        },
        Value::String(v) => (None, None, Some(v.clone())),
        _ => (None, None, Some(value.to_string())),
    }
}

fn json_value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(v) => Some(v.clone()),
        _ => Some(value.to_string()),
    }
}

fn parse_duration(duration: &str) -> Result<ChronoDuration, String> {
    let normalized: String = duration.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    if normalized.len() < 2 {
        return Err(format!(
            "Unsupported duration format: {:?}. Use e.g. 10s, 1m, 2h, 7d.",
            duration
        ));
    }

    let (amount_text, unit_text) = normalized.split_at(normalized.len() - 1);
    let amount = amount_text
        .parse::<i64>()
        .map_err(|_| format!("Unsupported duration format: {:?}.", duration))?;

    let unit = unit_text.to_ascii_lowercase();
    match unit.as_str() {
        "s" => Ok(ChronoDuration::seconds(amount)),
        "m" => Ok(ChronoDuration::minutes(amount)),
        "h" => Ok(ChronoDuration::hours(amount)),
        "d" => Ok(ChronoDuration::days(amount)),
        "w" => Ok(ChronoDuration::weeks(amount)),
        _ => Err(format!(
            "Unsupported duration format: {:?}. Use e.g. 10s, 1m, 2h, 7d.",
            duration
        )),
    }
}
