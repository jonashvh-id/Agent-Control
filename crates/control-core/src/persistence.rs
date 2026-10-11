use crate::{
    error::{Error, Result},
    security::{files, keys::Keys},
};
use rand::{RngCore, rngs::OsRng};
use sqlx::{
    Row, Sqlite, SqlitePool, Transaction,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use std::{path::Path, sync::Arc, time::Duration};
pub struct Database {
    pub pool: SqlitePool,
    pub keys: Arc<Keys>,
    _lock: std::fs::File,
}
impl Database {
    pub async fn open(dir: &Path) -> Result<Self> {
        if dir.exists()
            && (std::fs::symlink_metadata(dir)?.file_type().is_symlink()
                || !dir.join(".agent-control-rust").exists()
                    && std::fs::read_dir(dir)?.next().is_some())
        {
            return Err(Error::new("use_empty_rust_data_directory", 503));
        }
        files::private_directory(dir)?;
        let marker = dir.join(".agent-control-rust");
        if !marker.exists() {
            if std::fs::read_dir(dir)?.next().is_some() {
                return Err(Error::new("use_empty_rust_data_directory", 503));
            }
            files::create_private(&marker, b"agent-control-rust-v1\n")?;
        }
        if files::private_read(&marker, 100)? != b"agent-control-rust-v1\n" {
            return Err(Error::new("invalid_runtime_marker", 503));
        }
        let lockpath = dir.join("runtime.lock");
        if !lockpath.exists() {
            files::create_private(&lockpath, b"")?;
        }
        let lock = files::private_open(&lockpath, 0)?;
        fs2::FileExt::try_lock_exclusive(&lock)
            .map_err(|_| Error::new("runtime_already_open", 503))?;
        let keypath = dir.join("token.key");
        let path = dir.join("agent-control.sqlite3");
        if !keypath.exists() {
            if path.exists() {
                return Err(Error::new("missing_runtime_key", 503));
            }
            let mut key = [0u8; 48];
            OsRng.fill_bytes(&mut key);
            files::create_private(&keypath, &key)?;
        }
        let keys = Arc::new(Keys::new(files::private_read(&keypath, 48)?)?);
        if path.exists() {
            files::private_open(&path, u64::MAX)?;
        } else {
            files::create_private(&path, b"")?;
        }
        let options = SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(false)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(options)
            .await?;
        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .map_err(|_| Error::new("migration_failed", 503))?;
        // In-flight writes on a previous process may have reached the provider. Never retry them.
        crate::execution::recover(&pool).await?;
        Ok(Self {
            pool,
            keys,
            _lock: lock,
        })
    }
    pub async fn write(&self) -> Result<Transaction<'_, Sqlite>> {
        Ok(self.pool.begin_with("BEGIN IMMEDIATE").await?)
    }
}
pub fn id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}
pub fn epoch() -> f64 {
    chrono::Utc::now().timestamp_millis() as f64 / 1000.0
}
pub fn row_json(row: &sqlx::sqlite::SqliteRow) -> serde_json::Value {
    use sqlx::{Column, TypeInfo, ValueRef};
    let mut map = serde_json::Map::new();
    for column in row.columns() {
        let name = column.name();
        let raw = row.try_get_raw(name).expect("column exists");
        let value = if raw.is_null() {
            serde_json::Value::Null
        } else {
            match raw.type_info().name() {
                "INTEGER" => serde_json::json!(row.get::<i64, _>(name)),
                "REAL" => serde_json::json!(row.get::<f64, _>(name)),
                _ => {
                    let s: String = row.get(name);
                    if matches!(
                        name,
                        "scopes" | "input_summary" | "result_summary" | "result_receipt"
                    ) {
                        serde_json::from_str(&s).unwrap_or(serde_json::Value::Null)
                    } else {
                        serde_json::json!(s)
                    }
                }
            }
        };
        map.insert(name.into(), value);
    }
    serde_json::Value::Object(map)
}
