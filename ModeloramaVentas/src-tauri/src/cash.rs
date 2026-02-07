use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::State;

use crate::db::Db;

// ─── Tipos de respuesta ───────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct CashSession {
    pub id: i64,
    pub status: String,
    pub opened_at: String,
    pub closed_at: Option<String>,
    pub opened_by_user_id: i64,
    pub closed_by_user_id: Option<i64>,
    pub opening_float_cents: i64,
    pub expected_cash_cents: Option<i64>,
    pub counted_cash_cents: Option<i64>,
    pub difference_cents: Option<i64>,
    pub notes_open: Option<String>,
    pub notes_close: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CashMovement {
    pub id: i64,
    pub cash_session_id: i64,
    #[serde(rename = "type")]
    pub movement_type: String,
    pub amount_cents: i64,
    pub reason: String,
    pub created_at: String,
    pub user_id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CashSessionSummary {
    pub session: CashSession,
    pub total_sales_cash_cents: i64,
    pub total_sales_count: i64,
    pub total_deposits_cents: i64,
    pub total_withdrawals_cents: i64,
    pub expected_cash_cents: i64,
    pub movements: Vec<CashMovement>,
}

// ─── Helpers internos ─────────────────────────────────────────────────────────

/// Construye un CashSession a partir de un Row de sqlx.
fn row_to_session(r: &sqlx::sqlite::SqliteRow) -> CashSession {
    CashSession {
        id: r.get("id"),
        status: r.get("status"),
        opened_at: r.get("opened_at"),
        closed_at: r.get("closed_at"),
        opened_by_user_id: r.get("opened_by_user_id"),
        closed_by_user_id: r.get("closed_by_user_id"),
        opening_float_cents: r.get("opening_float_cents"),
        expected_cash_cents: r.get("expected_cash_cents"),
        counted_cash_cents: r.get("counted_cash_cents"),
        difference_cents: r.get("difference_cents"),
        notes_open: r.get("notes_open"),
        notes_close: r.get("notes_close"),
    }
}

fn row_to_movement(r: &sqlx::sqlite::SqliteRow) -> CashMovement {
    CashMovement {
        id: r.get("id"),
        cash_session_id: r.get("cash_session_id"),
        movement_type: r.get("type"),
        amount_cents: r.get("amount_cents"),
        reason: r.get("reason"),
        created_at: r.get("created_at"),
        user_id: r.get("user_id"),
    }
}

/// Valida que un usuario exista, esté activo y (opcionalmente) tenga cierto rol.
async fn validate_user(
    pool: &sqlx::SqlitePool,
    user_id: i64,
    required_role: Option<&str>,
) -> Result<(), String> {
    let user = sqlx::query("SELECT id, role, active FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Error al validar usuario: {}", e))?
        .ok_or("Usuario no encontrado")?;

    let active: i64 = user.get("active");
    if active != 1 {
        return Err("El usuario no está activo".into());
    }

    if let Some(role) = required_role {
        let user_role: String = user.get("role");
        if user_role != role {
            return Err(format!(
                "Se requiere rol '{}', pero el usuario tiene '{}'",
                role, user_role
            ));
        }
    }

    Ok(())
}

// ─── Commands ─────────────────────────────────────────────────────────────────

/// Abre una nueva sesión de caja.
///
/// Validaciones:
///  - No debe existir ya una sesión OPEN.
///  - `opening_amount_cents` >= 0.
///  - Usuario válido y activo.
#[tauri::command]
pub async fn open_cash_session(
    db: State<'_, Db>,
    user_id: i64,
    opening_amount_cents: i64,
    note: Option<String>,
) -> Result<CashSession, String> {
    // 1. Monto inicial no negativo
    if opening_amount_cents < 0 {
        return Err("El monto inicial no puede ser negativo".into());
    }

    // 2. Usuario válido y activo (dueño o empleado)
    validate_user(db.pool(), user_id, None).await?;

    // 3. No debe haber una sesión abierta
    let existing = sqlx::query("SELECT id FROM cash_sessions WHERE status = 'OPEN'")
        .fetch_optional(db.pool())
        .await
        .map_err(|e| format!("Error al verificar sesión existente: {}", e))?;

    if existing.is_some() {
        return Err(
            "Ya existe una sesión de caja abierta. Ciérrala antes de abrir una nueva.".into(),
        );
    }

    // 4. Insertar dentro de una transacción
    let mut tx = db
        .pool()
        .begin()
        .await
        .map_err(|e| format!("Error al iniciar transacción: {}", e))?;

    let result = sqlx::query(
        "INSERT INTO cash_sessions (status, opened_by_user_id, opening_float_cents, notes_open)
         VALUES ('OPEN', ?, ?, ?)
         RETURNING id, status, opened_at, closed_at, opened_by_user_id, closed_by_user_id,
                   opening_float_cents, expected_cash_cents, counted_cash_cents,
                   difference_cents, notes_open, notes_close",
    )
    .bind(user_id)
    .bind(opening_amount_cents)
    .bind(&note)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| format!("Error al crear sesión de caja: {}", e))?;

    tx.commit()
        .await
        .map_err(|e| format!("Error al confirmar transacción: {}", e))?;

    Ok(row_to_session(&result))
}

/// Obtiene la sesión OPEN actual (si existe).
#[tauri::command]
pub async fn get_current_cash_session(
    db: State<'_, Db>,
) -> Result<Option<CashSession>, String> {
    let row = sqlx::query(
        "SELECT id, status, opened_at, closed_at, opened_by_user_id, closed_by_user_id,
                opening_float_cents, expected_cash_cents, counted_cash_cents,
                difference_cents, notes_open, notes_close
         FROM cash_sessions
         WHERE status = 'OPEN'
         LIMIT 1",
    )
    .fetch_optional(db.pool())
    .await
    .map_err(|e| format!("Error al consultar sesión: {}", e))?;

    Ok(row.as_ref().map(row_to_session))
}

/// Registra un movimiento manual de caja (depósito o retiro).
///
/// Validaciones:
///  - `movement_type` ∈ {DEPOSIT, WITHDRAWAL}.
///  - `amount_cents` > 0.
///  - `reason` no vacío.
///  - Sesión existente y en estado OPEN.
///  - Usuario válido y activo.
#[tauri::command]
pub async fn add_cash_movement(
    db: State<'_, Db>,
    session_id: i64,
    movement_type: String,
    amount_cents: i64,
    reason: String,
    user_id: i64,
) -> Result<CashMovement, String> {
    // 1. Tipo válido
    if movement_type != "DEPOSIT" && movement_type != "WITHDRAWAL" {
        return Err("Tipo de movimiento inválido. Usa 'DEPOSIT' o 'WITHDRAWAL'.".into());
    }

    // 2. Monto positivo
    if amount_cents <= 0 {
        return Err("El monto debe ser mayor a 0".into());
    }

    // 3. Razón obligatoria
    if reason.trim().is_empty() {
        return Err("Debes proporcionar una razón para el movimiento".into());
    }

    // 4. Sesión abierta
    let session = sqlx::query("SELECT id, status FROM cash_sessions WHERE id = ?")
        .bind(session_id)
        .fetch_optional(db.pool())
        .await
        .map_err(|e| format!("Error al verificar sesión: {}", e))?
        .ok_or("Sesión de caja no encontrada")?;

    let status: String = session.get("status");
    if status != "OPEN" {
        return Err("La sesión de caja no está abierta".into());
    }

    // 5. Usuario válido
    validate_user(db.pool(), user_id, None).await?;

    // 6. Insertar movimiento
    let result = sqlx::query(
        "INSERT INTO cash_movements (cash_session_id, type, amount_cents, reason, user_id)
         VALUES (?, ?, ?, ?, ?)
         RETURNING id, cash_session_id, type, amount_cents, reason, created_at, user_id",
    )
    .bind(session_id)
    .bind(&movement_type)
    .bind(amount_cents)
    .bind(reason.trim())
    .bind(user_id)
    .fetch_one(db.pool())
    .await
    .map_err(|e| format!("Error al registrar movimiento: {}", e))?;

    Ok(row_to_movement(&result))
}

/// Devuelve los movimientos de caja de una sesión.
#[tauri::command]
pub async fn get_cash_movements(
    db: State<'_, Db>,
    session_id: i64,
) -> Result<Vec<CashMovement>, String> {
    let rows = sqlx::query(
        "SELECT id, cash_session_id, type, amount_cents, reason, created_at, user_id
         FROM cash_movements
         WHERE cash_session_id = ?
         ORDER BY created_at ASC",
    )
    .bind(session_id)
    .fetch_all(db.pool())
    .await
    .map_err(|e| format!("Error al consultar movimientos: {}", e))?;

    Ok(rows.iter().map(row_to_movement).collect())
}

/// Calcula el resumen de la sesión (para la pantalla de cierre).
///
/// expected_cash = fondo_inicial + ventas_CASH + depósitos − retiros
#[tauri::command]
pub async fn get_cash_session_summary(
    db: State<'_, Db>,
    session_id: i64,
) -> Result<CashSessionSummary, String> {
    // Sesión
    let session_row = sqlx::query(
        "SELECT id, status, opened_at, closed_at, opened_by_user_id, closed_by_user_id,
                opening_float_cents, expected_cash_cents, counted_cash_cents,
                difference_cents, notes_open, notes_close
         FROM cash_sessions WHERE id = ?",
    )
    .bind(session_id)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| format!("Error al consultar sesión: {}", e))?
    .ok_or("Sesión de caja no encontrada")?;

    let session = row_to_session(&session_row);
    let opening_float: i64 = session_row.get("opening_float_cents");

    // Ventas en efectivo
    let sales_row = sqlx::query(
        "SELECT COALESCE(SUM(total_cents), 0) as total, COUNT(*) as cnt
         FROM sales
         WHERE cash_session_id = ? AND payment_method = 'CASH'",
    )
    .bind(session_id)
    .fetch_one(db.pool())
    .await
    .map_err(|e| format!("Error al calcular ventas: {}", e))?;

    let total_sales_cash_cents: i64 = sales_row.get("total");
    let total_sales_count: i64 = sales_row.get("cnt");

    // Depósitos
    let dep_row = sqlx::query(
        "SELECT COALESCE(SUM(amount_cents), 0) as total
         FROM cash_movements
         WHERE cash_session_id = ? AND type = 'DEPOSIT'",
    )
    .bind(session_id)
    .fetch_one(db.pool())
    .await
    .map_err(|e| format!("Error al calcular depósitos: {}", e))?;
    let total_deposits_cents: i64 = dep_row.get("total");

    // Retiros
    let wd_row = sqlx::query(
        "SELECT COALESCE(SUM(amount_cents), 0) as total
         FROM cash_movements
         WHERE cash_session_id = ? AND type = 'WITHDRAWAL'",
    )
    .bind(session_id)
    .fetch_one(db.pool())
    .await
    .map_err(|e| format!("Error al calcular retiros: {}", e))?;
    let total_withdrawals_cents: i64 = wd_row.get("total");

    let expected_cash_cents =
        opening_float + total_sales_cash_cents + total_deposits_cents - total_withdrawals_cents;

    // Movimientos
    let mov_rows = sqlx::query(
        "SELECT id, cash_session_id, type, amount_cents, reason, created_at, user_id
         FROM cash_movements
         WHERE cash_session_id = ?
         ORDER BY created_at ASC",
    )
    .bind(session_id)
    .fetch_all(db.pool())
    .await
    .map_err(|e| format!("Error al consultar movimientos: {}", e))?;

    let movements: Vec<CashMovement> = mov_rows.iter().map(row_to_movement).collect();

    Ok(CashSessionSummary {
        session,
        total_sales_cash_cents,
        total_sales_count,
        total_deposits_cents,
        total_withdrawals_cents,
        expected_cash_cents,
        movements,
    })
}

/// Cierra la sesión de caja.
///
/// Validaciones:
///  - Sesión existente y OPEN.
///  - Si hay diferencia (counted ≠ expected), `notes_close` es obligatoria.
///  - Si se proporciona `owner_auth_user_id`, debe ser rol OWNER y estar activo.
///
/// Cálculo:
///  expected = fondo_inicial + ventas_CASH + depósitos − retiros
///  difference = counted − expected
#[tauri::command]
pub async fn close_cash_session(
    db: State<'_, Db>,
    session_id: i64,
    user_id: i64,
    counted_cash_cents: i64,
    notes_close: Option<String>,
    owner_auth_user_id: Option<i64>,
) -> Result<CashSession, String> {
    // 1. Sesión abierta
    let session_row = sqlx::query(
        "SELECT id, status, opening_float_cents FROM cash_sessions WHERE id = ?",
    )
    .bind(session_id)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| format!("Error al verificar sesión: {}", e))?
    .ok_or("Sesión de caja no encontrada")?;

    let status: String = session_row.get("status");
    if status != "OPEN" {
        return Err("La sesión de caja ya está cerrada".into());
    }

    let opening_float: i64 = session_row.get("opening_float_cents");

    // 2. Usuario válido
    validate_user(db.pool(), user_id, None).await?;

    // 3. Calcular efectivo esperado
    let sales_row = sqlx::query(
        "SELECT COALESCE(SUM(total_cents), 0) as total
         FROM sales WHERE cash_session_id = ? AND payment_method = 'CASH'",
    )
    .bind(session_id)
    .fetch_one(db.pool())
    .await
    .map_err(|e| format!("Error al calcular ventas: {}", e))?;
    let sales_total: i64 = sales_row.get("total");

    let dep_row = sqlx::query(
        "SELECT COALESCE(SUM(amount_cents), 0) as total
         FROM cash_movements WHERE cash_session_id = ? AND type = 'DEPOSIT'",
    )
    .bind(session_id)
    .fetch_one(db.pool())
    .await
    .map_err(|e| format!("Error al calcular depósitos: {}", e))?;
    let deposits: i64 = dep_row.get("total");

    let wd_row = sqlx::query(
        "SELECT COALESCE(SUM(amount_cents), 0) as total
         FROM cash_movements WHERE cash_session_id = ? AND type = 'WITHDRAWAL'",
    )
    .bind(session_id)
    .fetch_one(db.pool())
    .await
    .map_err(|e| format!("Error al calcular retiros: {}", e))?;
    let withdrawals: i64 = wd_row.get("total");

    let expected_cash = opening_float + sales_total + deposits - withdrawals;
    let difference = counted_cash_cents - expected_cash;

    // 4. Nota obligatoria si hay diferencia
    if difference != 0
        && notes_close
            .as_ref()
            .map_or(true, |n| n.trim().is_empty())
    {
        return Err(format!(
            "Hay una diferencia de ${:.2}. Debes agregar una nota explicativa.",
            difference as f64 / 100.0
        ));
    }

    // 5. Autorización de dueño (si se proporcionó)
    if let Some(owner_id) = owner_auth_user_id {
        validate_user(db.pool(), owner_id, Some("OWNER")).await?;
    }

    // 6. Cerrar en transacción
    let mut tx = db
        .pool()
        .begin()
        .await
        .map_err(|e| format!("Error al iniciar transacción: {}", e))?;

    sqlx::query(
        "UPDATE cash_sessions SET
            status = 'CLOSED',
            closed_at = datetime('now'),
            closed_by_user_id = ?,
            expected_cash_cents = ?,
            counted_cash_cents = ?,
            difference_cents = ?,
            notes_close = ?,
            owner_auth_user_id = ?
         WHERE id = ?",
    )
    .bind(user_id)
    .bind(expected_cash)
    .bind(counted_cash_cents)
    .bind(difference)
    .bind(notes_close.as_deref().map(str::trim))
    .bind(owner_auth_user_id)
    .bind(session_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Error al cerrar sesión: {}", e))?;

    let updated = sqlx::query(
        "SELECT id, status, opened_at, closed_at, opened_by_user_id, closed_by_user_id,
                opening_float_cents, expected_cash_cents, counted_cash_cents,
                difference_cents, notes_open, notes_close
         FROM cash_sessions WHERE id = ?",
    )
    .bind(session_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| format!("Error al obtener sesión actualizada: {}", e))?;

    tx.commit()
        .await
        .map_err(|e| format!("Error al confirmar transacción: {}", e))?;

    Ok(row_to_session(&updated))
}
