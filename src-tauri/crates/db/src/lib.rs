use rusqlite::{Connection, Result};
use std::{
    ops::{Deref, DerefMut},
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use crate::db_executor::DbExecutor as DbExecutorTrait;
use tokio::sync::{
    Mutex as AsyncMutex, MutexGuard as AsyncMutexGuard, OwnedSemaphorePermit, Semaphore,
};
use tracing::{info, warn};

pub mod drive_stats_cache;
pub mod services;
pub mod files;
pub mod folders;
pub mod migrations;
pub mod provider_quota_cache;
pub mod tenant_meta;
pub mod upload_profile_rules;
pub mod backup;
pub mod backup_repo;
pub mod cache;
pub mod db_executor;
pub mod download_cache;
pub mod upload_cache;
pub mod upload_profiles;
pub mod repos;

pub struct Db {
    conn: Connection,
}

pub struct DbWriteQueue {
    db: AsyncMutex<Db>,
    queued: AtomicU64,
    completed: AtomicU64,
    total_wait_us: AtomicU64,
    total_hold_us: AtomicU64,
}

#[derive(Debug, Clone, Copy)]
pub struct DbWriteQueueStats {
    pub queued: u64,
    pub completed: u64,
    pub total_wait_us: u64,
    pub total_hold_us: u64,
}

pub struct DbWriteLease<'a> {
    guard: AsyncMutexGuard<'a, Db>,
    queue: &'a DbWriteQueue,
    hold_started: Instant,
    wait_duration: Duration,
}

pub struct ReadDbPool {
    connections: Mutex<Vec<Db>>,
    permits: Arc<Semaphore>,
    size: usize,
}

pub struct ReadDbLease<'a> {
    pool: &'a ReadDbPool,
    db: Option<Db>,
    _permit: OwnedSemaphorePermit,
}

const WRITE_QUEUE_WARN_AFTER: Duration = Duration::from_millis(250);

fn duration_micros(duration: Duration) -> u64 {
    duration.as_micros().min(u128::from(u64::MAX)) as u64
}

fn duration_millis(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

fn configure_connection(conn: &mut Connection) -> Result<()> {
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    conn.pragma_update(None, "cache_size", -65_536_i64)?;
    conn.pragma_update(None, "mmap_size", 1_073_741_824_i64)?;
    conn.set_prepared_statement_cache_capacity(256);

    // SQL query profiling is debug-build only and enabled with DEBUG=1.
    // The profile callback runs after each statement completes, so it can report elapsed time.
    #[cfg(debug_assertions)]
    if std::env::var("DEBUG").is_ok() {
        conn.profile(Some(|sql, duration| {
            tracing::info!(
                target: "db::sql",
                "[{:.6} ms] {}",
                duration.as_secs_f64() * 1000.0,
                sql
            );
        }));
    }

    Ok(())
}

impl Db {
    fn from_connection(conn: Connection) -> Self {
        Self { conn }
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    pub fn open(path: &Path) -> Result<Self> {
        let mut conn = Connection::open(path)?;
        configure_connection(&mut conn)?;

        let db = Self::from_connection(conn);
        migrations::run_migrations(db.conn())?;

        info!("SQLite Database opened at: {}", path.display());
        Ok(db)
    }

    pub fn open_no_migrate(path: &Path) -> Result<Self> {
        let mut conn = Connection::open(path)?;
        configure_connection(&mut conn)?;
        Ok(Self::from_connection(conn))
    }

    pub fn reopen(&mut self, path: &Path) -> Result<()> {
        let _ = self.conn.pragma_update(None, "wal_checkpoint", "TRUNCATE");
        let mut conn = Connection::open(path)?;
        configure_connection(&mut conn)?;
        migrations::run_migrations(&conn)?;
        self.conn = conn;
        info!("SQLite Database reopened at: {}", path.display());
        Ok(())
    }
}

impl DbWriteQueue {
    pub fn new(db: Db) -> Self {
        Self {
            db: AsyncMutex::new(db),
            queued: AtomicU64::new(0),
            completed: AtomicU64::new(0),
            total_wait_us: AtomicU64::new(0),
            total_hold_us: AtomicU64::new(0),
        }
    }

    pub async fn lock(&self) -> DbWriteLease<'_> {
        let queued_at_start = self.queued.fetch_add(1, Ordering::Relaxed) + 1;
        let wait_started = Instant::now();
        let guard = self.db.lock().await;
        let wait_duration = wait_started.elapsed();
        self.queued.fetch_sub(1, Ordering::Relaxed);
        self.record_wait(wait_duration, queued_at_start);

        DbWriteLease {
            guard,
            queue: self,
            hold_started: Instant::now(),
            wait_duration,
        }
    }

    pub fn blocking_lock(&self) -> DbWriteLease<'_> {
        let queued_at_start = self.queued.fetch_add(1, Ordering::Relaxed) + 1;
        let wait_started = Instant::now();
        let guard = self.db.blocking_lock();
        let wait_duration = wait_started.elapsed();
        self.queued.fetch_sub(1, Ordering::Relaxed);
        self.record_wait(wait_duration, queued_at_start);

        DbWriteLease {
            guard,
            queue: self,
            hold_started: Instant::now(),
            wait_duration,
        }
    }

    pub async fn with_write<R>(&self, f: impl FnOnce(&Connection) -> R) -> R {
        let db = self.lock().await;
        f(db.conn())
    }

    pub async fn with_write_mut<R>(&self, f: impl FnOnce(&mut Connection) -> R) -> R {
        let mut db = self.lock().await;
        f(db.conn_mut())
    }

    pub fn stats(&self) -> DbWriteQueueStats {
        DbWriteQueueStats {
            queued: self.queued.load(Ordering::Relaxed),
            completed: self.completed.load(Ordering::Relaxed),
            total_wait_us: self.total_wait_us.load(Ordering::Relaxed),
            total_hold_us: self.total_hold_us.load(Ordering::Relaxed),
        }
    }

    pub async fn reopen(&self, path: &Path) -> Result<()> {
        let mut db = self.lock().await;
        db.reopen(path)
    }

    fn record_wait(&self, wait_duration: Duration, queued_at_start: u64) {
        self.total_wait_us
            .fetch_add(duration_micros(wait_duration), Ordering::Relaxed);

        if wait_duration >= WRITE_QUEUE_WARN_AFTER {
            warn!(
                target: "db::write_queue",
                wait_ms = duration_millis(wait_duration),
                queued_at_start = queued_at_start,
                "SQLite writer queue waited before acquiring the write connection"
            );
        }
    }
}

impl Deref for DbWriteLease<'_> {
    type Target = Db;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

impl DerefMut for DbWriteLease<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.deref_mut()
    }
}

impl Drop for DbWriteLease<'_> {
    fn drop(&mut self) {
        let hold_duration = self.hold_started.elapsed();
        self.queue
            .total_hold_us
            .fetch_add(duration_micros(hold_duration), Ordering::Relaxed);
        self.queue.completed.fetch_add(1, Ordering::Relaxed);

        if hold_duration >= WRITE_QUEUE_WARN_AFTER {
            warn!(
                target: "db::write_queue",
                hold_ms = duration_millis(hold_duration),
                wait_ms = duration_millis(self.wait_duration),
                "SQLite writer queue held the write connection for a long duration"
            );
        }
    }
}

impl ReadDbPool {
    pub fn recommended_size() -> usize {
        std::thread::available_parallelism()
            .map(|parallelism| parallelism.get().clamp(4, 8))
            .unwrap_or(4)
    }

    pub fn open(path: &Path, size: usize) -> Result<Self> {
        let size = size.max(1);
        let mut connections = Vec::with_capacity(size);
        connections.push(Db::open_no_migrate(path)?);
        for _ in 1..size {
            connections.push(Db::open_no_migrate(path)?);
        }

        info!(
            "Opened SQLite read pool with {} connections for drive queries",
            size
        );

        Ok(Self {
            connections: Mutex::new(connections),
            permits: Arc::new(Semaphore::new(size)),
            size,
        })
    }

    pub async fn lock(&self) -> ReadDbLease<'_> {
        let permit = self
            .permits
            .clone()
            .acquire_owned()
            .await
            .unwrap_or_else(|_| panic!("SQLite read pool semaphore closed unexpectedly"));
        let db = {
            let mut guard = self
                .connections
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            match guard.pop() {
                Some(db) => db,
                None => panic!("SQLite read pool ran out of connections while permit was held"),
            }
        };

        ReadDbLease {
            pool: self,
            db: Some(db),
            _permit: permit,
        }
    }

    pub async fn with_read<T, F>(&self, f: F) -> T
    where
        F: FnOnce(&Connection) -> T + Send,
    {
        let lease = self.lock().await;
        f(lease.conn())
    }

    pub async fn reopen(&self, path: &Path) -> Result<()> {
        let permit = self
            .permits
            .clone()
            .acquire_many_owned(self.size as u32)
            .await
            .unwrap_or_else(|_| panic!("SQLite read pool semaphore closed unexpectedly"));

        let mut connections = Vec::with_capacity(self.size);
        connections.push(Db::open_no_migrate(path)?);
        for _ in 1..self.size {
            connections.push(Db::open_no_migrate(path)?);
        }

        let mut guard = self
            .connections
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        // Checkpoint + truncate WAL on all old connections before closing them
        for db in guard.iter() {
            let _ = db.conn().pragma_update(None, "wal_checkpoint", "TRUNCATE");
        }

        *guard = connections;
        drop(guard);
        drop(permit);

        info!(
            "Reopened SQLite read pool with {} connections for drive queries",
            self.size
        );

        Ok(())
    }
}

impl Deref for ReadDbLease<'_> {
    type Target = Db;

    fn deref(&self) -> &Self::Target {
        match self.db.as_ref() {
            Some(db) => db,
            None => panic!("SQLite read pool lease dereferenced after drop"),
        }
    }
}

impl Drop for ReadDbLease<'_> {
    fn drop(&mut self) {
        if let Some(db) = self.db.take() {
            let mut guard = self
                .pool
                .connections
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.push(db);
        }
    }
}

impl DbExecutorTrait for ReadDbPool {
    fn read(&self, f: &mut dyn FnMut(&Connection)) {
        let handle = tokio::runtime::Handle::current();
        let lease = tokio::task::block_in_place(|| handle.block_on(self.lock()));
        f(lease.conn());
    }

    fn write(&self, f: &mut dyn FnMut(&Connection)) {
        let handle = tokio::runtime::Handle::current();
        let lease = tokio::task::block_in_place(|| handle.block_on(self.lock()));
        f(lease.conn());
    }
}

impl DbExecutorTrait for DbWriteQueue {
    fn read(&self, f: &mut dyn FnMut(&Connection)) {
        let handle = tokio::runtime::Handle::current();
        let lease = tokio::task::block_in_place(|| handle.block_on(self.lock()));
        f(lease.conn());
    }

    fn write(&self, f: &mut dyn FnMut(&Connection)) {
        let handle = tokio::runtime::Handle::current();
        let lease = tokio::task::block_in_place(|| handle.block_on(self.lock()));
        f(lease.conn());
    }
}
