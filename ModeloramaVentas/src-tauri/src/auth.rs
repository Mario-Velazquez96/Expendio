use crate::db::Db;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub role: String, // "OWNER" | "EMPLOYEE"
}

#[derive(Debug, Serialize)]
pub struct LoginResult {
    pub success: bool,
    pub user: Option<User>,
    pub error: Option<String>,
}

/// Hash a PIN using SHA256
pub fn hash_pin(pin: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(pin.as_bytes());
    hex::encode(hasher.finalize())
}

/// Login with PIN - returns user if valid
#[tauri::command]
pub async fn login_with_pin(pin: String, db: State<'_, Db>) -> Result<LoginResult, String> {
    // Validate PIN format: must be exactly 6 digits
    if pin.len() != 6 || !pin.chars().all(|c| c.is_ascii_digit()) {
        return Ok(LoginResult {
            success: false,
            user: None,
            error: Some("El PIN debe ser exactamente 6 dígitos".to_string()),
        });
    }

    let pin_hash = hash_pin(&pin);

    let result = sqlx::query(
        "SELECT id, name, role FROM users WHERE pin_hash = ? AND active = 1"
    )
    .bind(&pin_hash)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| format!("Error de base de datos: {}", e))?;

    match result {
        Some(row) => {
            let user = User {
                id: row.get("id"),
                name: row.get("name"),
                role: row.get("role"),
            };
            Ok(LoginResult {
                success: true,
                user: Some(user),
                error: None,
            })
        }
        None => Ok(LoginResult {
            success: false,
            user: None,
            error: Some("PIN incorrecto o usuario inactivo".to_string()),
        }),
    }
}

/// Get user by ID (for session restoration)
#[tauri::command]
pub async fn get_user_by_id(user_id: i64, db: State<'_, Db>) -> Result<Option<User>, String> {
    let result = sqlx::query(
        "SELECT id, name, role FROM users WHERE id = ? AND active = 1"
    )
    .bind(user_id)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| format!("Error de base de datos: {}", e))?;

    Ok(result.map(|row| User {
        id: row.get("id"),
        name: row.get("name"),
        role: row.get("role"),
    }))
}

/// Seed initial users if none exist
/// Owner PIN: 123456
/// Employee PIN: 654321
pub async fn seed_users(db: &Db) -> Result<(), sqlx::Error> {
    // Check if users already exist
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(db.pool())
        .await?;

    if count > 0 {
        return Ok(()); // Users already seeded
    }

    // Create owner user (PIN: 123456)
    let owner_pin_hash = hash_pin("123456");
    sqlx::query(
        "INSERT INTO users (name, pin_hash, role, active) VALUES (?, ?, 'OWNER', 1)"
    )
    .bind("Dueño")
    .bind(&owner_pin_hash)
    .execute(db.pool())
    .await?;

    // Create employee user (PIN: 654321)
    let employee_pin_hash = hash_pin("654321");
    sqlx::query(
        "INSERT INTO users (name, pin_hash, role, active) VALUES (?, ?, 'EMPLOYEE', 1)"
    )
    .bind("Empleado")
    .bind(&employee_pin_hash)
    .execute(db.pool())
    .await?;

    println!("✅ Usuarios iniciales creados:");
    println!("   - Dueño (PIN: 123456)");
    println!("   - Empleado (PIN: 654321)");

    Ok(())
}
