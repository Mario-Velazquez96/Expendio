use sqlx::{sqlite::{SqlitePoolOptions, SqliteConnectOptions}, Executor, SqlitePool};
use std::str::FromStr;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use thiserror::Error;

#[derive(Clone)]
pub struct Db(pub SqlitePool);

#[derive(Debug, Error)]
pub enum DbError {
  #[error("sqlx error: {0}")]
  Sqlx(#[from] sqlx::Error),

  #[error("io error: {0}")]
  Io(#[from] std::io::Error),

  #[error("failed to resolve app data dir")]
  NoAppDataDir,

  #[error("schema file not found at: {0}")]
  SchemaNotFound(String),
}

impl Db {
  /// Get the underlying pool.
  pub fn pool(&self) -> &SqlitePool {
    &self.0
  }
}

/// Initializes the SQLite database:
/// - creates the app data directory
/// - opens/creates the sqlite file
/// - applies pragmas (WAL, foreign keys, busy_timeout)
/// - runs schema.sql (idempotent)
pub async fn init_db(app: &AppHandle) -> Result<Db, DbError> {
  let db_path = resolve_db_path(app)?;
  ensure_parent_dir(&db_path)?;
  
  // Verify directory was created and is writable
  if let Some(parent) = db_path.parent() {
    if !parent.exists() {
      return Err(DbError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("No se pudo crear el directorio: {}", parent.display()),
      )));
    }
    
    // Test write permissions by creating a test file
    let test_path = parent.join(format!(".tauri_db_test_{}", std::process::id()));
    match std::fs::File::create(&test_path) {
      Ok(mut file) => {
        use std::io::Write;
        if file.write_all(b"test").is_err() {
          return Err(DbError::Io(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!("No se tienen permisos de escritura en: {}", parent.display()),
          )));
        }
        let _ = std::fs::remove_file(&test_path);
      }
      Err(e) => {
        return Err(DbError::Io(std::io::Error::new(
          std::io::ErrorKind::PermissionDenied,
          format!("No se tienen permisos de escritura en: {}: {}", parent.display(), e),
        )));
      }
    }
  }

  // Convert Windows path separators to forward slashes for SQLite URL
  let db_path_str = db_path.to_string_lossy().replace('\\', "/");
  
  // Use SqliteConnectOptions for better Windows compatibility
  let connect_options = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path_str))
    .map_err(|e| {
      DbError::Sqlx(sqlx::Error::Configuration(format!("Invalid connection string: {}", e).into()))
    })?
    .create_if_missing(true);
  
  let pool = SqlitePoolOptions::new()
    .max_connections(5)
    .acquire_timeout(std::time::Duration::from_secs(5))
    .connect_with(connect_options)
    .await?;

  // Recommended pragmas for a local POS-style app
  // - WAL improves concurrency and durability
  // - foreign_keys enforces relational integrity
  // - busy_timeout helps avoid "database is locked" under quick operations
  pool.execute("PRAGMA journal_mode = WAL;").await?;
  pool.execute("PRAGMA foreign_keys = ON;").await?;
  pool.execute("PRAGMA busy_timeout = 5000;").await?;
  // Optional: performance tuning (reasonable defaults)
  pool.execute("PRAGMA synchronous = NORMAL;").await?;

  apply_schema(app, &pool).await?;

  // ── Migraciones idempotentes ──────────────────────────────────────────
  // Agregar columna status a sales (para flujo DRAFT → FINALIZED).
  // Si la columna ya existe, el ALTER TABLE falla silenciosamente.
  let _ = pool.execute(
    "ALTER TABLE sales ADD COLUMN status TEXT NOT NULL DEFAULT 'FINALIZED' CHECK(status IN ('DRAFT','FINALIZED'))"
  ).await;

  Ok(Db(pool))
}

/// Resolve database file path: <app_data_dir>/beerpos/app.db
fn resolve_db_path(app: &AppHandle) -> Result<PathBuf, DbError> {
  let data_dir = app
    .path()
    .app_data_dir()
    .map_err(|_| DbError::NoAppDataDir)?;

  Ok(data_dir.join("beerpos").join("app.db"))
}

fn ensure_parent_dir(db_path: &Path) -> Result<(), DbError> {
  if let Some(parent) = db_path.parent() {
    std::fs::create_dir_all(parent)?;
  }
  Ok(())
}

/// Loads and runs schema.sql. Must be idempotent (CREATE TABLE IF NOT EXISTS).
async fn apply_schema(app: &AppHandle, pool: &SqlitePool) -> Result<(), DbError> {
  // Preferred: bundle schema.sql as a resource file
  // You can place it at src-tauri/resources/schema.sql and reference it via path resolver.
  let schema_path = app
    .path()
    .resolve("resources/schema.sql", tauri::path::BaseDirectory::Resource)
    .map_err(|_| DbError::SchemaNotFound("resources/schema.sql".into()))?;

  if !schema_path.exists() {
    return Err(DbError::SchemaNotFound(schema_path.to_string_lossy().to_string()));
  }

  let schema_sql = std::fs::read_to_string(&schema_path)?;

  // Parse SQL: handle comments and multi-line statements properly
  let mut tx = pool.begin().await?;

  // Remove single-line comments (-- ...) but preserve the structure
  let mut cleaned_sql = String::new();
  
  for line in schema_sql.lines() {
    let trimmed = line.trim();
    
    // Skip empty lines
    if trimmed.is_empty() {
      cleaned_sql.push('\n');
      continue;
    }
    
    // Check if line is a comment
    if trimmed.starts_with("--") {
      cleaned_sql.push('\n');
      continue;
    }
    
    // Remove inline comments (-- at end of line)
    let line_without_comment = if let Some(comment_pos) = line.find("--") {
      // Check if -- is inside a string (simple check)
      let before_comment = &line[..comment_pos];
      let quote_count = before_comment.matches('"').count() + before_comment.matches('\'').count();
      if quote_count % 2 == 0 {
        // Not inside a string, remove comment
        line[..comment_pos].trim_end()
      } else {
        // Inside a string, keep the line
        line
      }
    } else {
      line
    };
    
    cleaned_sql.push_str(&line_without_comment);
    cleaned_sql.push('\n');
  }

  // Split by semicolon to get individual statements
  let statements: Vec<String> = cleaned_sql
    .split(';')
    .map(|s| s.trim().to_string())
    .filter(|s| {
      // Filter out empty strings and lines that are just whitespace
      !s.is_empty() && !s.chars().all(|c| c.is_whitespace())
    })
    .collect();

  // Execute each statement
  for stmt in statements {
    tx.execute(stmt.as_str()).await?;
  }

  tx.commit().await?;
  Ok(())
}

/// Helper: start a transaction.
/// Use this for finalize_sale / close_cash_session, etc.
pub async fn begin_tx(db: &Db) -> Result<sqlx::Transaction<'_, sqlx::Sqlite>, DbError> {
  Ok(db.pool().begin().await?)
}
