use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
};
use tracing::info;
use wordbase_api::{Dictionary, DictionaryId, Record};

pub type DbPool = Pool<SqliteConnectionManager>;

#[derive(Clone)]
pub struct AppState {
    pub dictionaries: Arc<RwLock<HashMap<DictionaryId, Dictionary>>>,
    pub next_dict_id: Arc<RwLock<i64>>,
    pub pool: DbPool,
    pub data_dir: PathBuf,
    pub loading: Arc<AtomicBool>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct StoredRecord {
    pub dictionary_id: DictionaryId,
    pub record: Record,
    pub reading: Option<String>,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        if !data_dir.exists() {
            let _ = std::fs::create_dir_all(&data_dir);
        }
        let db_path = data_dir.join("yomitan.db");
        let manager = SqliteConnectionManager::file(&db_path);

        let pool = Pool::new(manager).expect("Failed to create DB pool");

        let conn = pool.get().expect("Failed to get DB connection");
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS terms (
                term TEXT NOT NULL,
                json TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_term ON terms(term);
             
             CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT
             );",
        )
        .expect("Failed to initialize database tables");

        info!("📂 [Yomitan] Database initialized at {:?}", db_path);

        Self {
            dictionaries: Arc::new(RwLock::new(HashMap::new())),
            next_dict_id: Arc::new(RwLock::new(1)),
            pool,
            data_dir,
            loading: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn set_loading(&self, val: bool) {
        self.loading.store(val, Ordering::SeqCst);
    }

    pub fn is_loading(&self) -> bool {
        self.loading.load(Ordering::Relaxed)
    }
}
