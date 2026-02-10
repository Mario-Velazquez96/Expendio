use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::State;

use crate::auth;
use crate::db::Db;

// ─── Tipos de respuesta ───────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductDetail {
    pub id: i64,
    pub name: String,
    pub category: String,
    pub barcode: Option<String>,
    pub units_per_case: i64,
    pub cost_cents: i64,
    pub unit_price_cents: i64,
    pub stock_min: i64,
    pub active: i64,
    pub on_hand: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StockResult {
    pub product_id: i64,
    pub product_name: String,
    pub cases_added: i64,
    pub units_added: i64,
    pub new_on_hand: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdjustResult {
    pub product_id: i64,
    pub product_name: String,
    pub units_removed: i64,
    pub new_on_hand: i64,
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn row_to_product_detail(r: &sqlx::sqlite::SqliteRow) -> ProductDetail {
    ProductDetail {
        id: r.get("id"),
        name: r.get("name"),
        category: r.get("category"),
        barcode: r.get("barcode"),
        units_per_case: r.get("units_per_case"),
        cost_cents: r.get("cost_cents"),
        unit_price_cents: r.get("unit_price_cents"),
        stock_min: r.get("stock_min"),
        active: r.get("active"),
        on_hand: r.get("on_hand"),
    }
}

const PRODUCT_DETAIL_SELECT: &str = "
    SELECT p.id, p.name, p.category, p.barcode, p.units_per_case,
           p.cost_cents, p.unit_price_cents, p.stock_min, p.active,
           COALESCE(ib.on_hand, 0) as on_hand
    FROM products p
    LEFT JOIN inventory_balances ib ON p.id = ib.product_id";

/// Valida que el PIN corresponda a un usuario OWNER activo.
/// Devuelve el user_id.
async fn validate_owner(pool: &sqlx::SqlitePool, pin: &str) -> Result<i64, String> {
    if pin.len() != 6 || !pin.chars().all(|c| c.is_ascii_digit()) {
        return Err("El PIN debe ser exactamente 6 dígitos".into());
    }
    let pin_hash = auth::hash_pin(pin);
    let row = sqlx::query("SELECT id, role FROM users WHERE pin_hash = ? AND active = 1")
        .bind(&pin_hash)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Error al validar PIN: {}", e))?
        .ok_or("PIN incorrecto o usuario inactivo")?;

    let role: String = row.get("role");
    if role != "OWNER" {
        return Err("Solo el dueño puede realizar esta operación".into());
    }
    Ok(row.get("id"))
}

/// Valida que el PIN corresponda a cualquier usuario activo (OWNER o EMPLOYEE).
/// Devuelve el user_id.
async fn validate_authenticated(pool: &sqlx::SqlitePool, pin: &str) -> Result<i64, String> {
    if pin.len() != 6 || !pin.chars().all(|c| c.is_ascii_digit()) {
        return Err("El PIN debe ser exactamente 6 dígitos".into());
    }
    let pin_hash = auth::hash_pin(pin);
    let row = sqlx::query("SELECT id FROM users WHERE pin_hash = ? AND active = 1")
        .bind(&pin_hash)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Error al validar PIN: {}", e))?
        .ok_or("PIN incorrecto o usuario inactivo")?;

    Ok(row.get("id"))
}

// ─── Commands ─────────────────────────────────────────────────────────────────

/// Lista TODOS los productos (activos e inactivos) con stock.
/// Disponible para cualquier usuario autenticado (OWNER o EMPLOYEE).
#[tauri::command]
pub async fn list_all_products_admin(
    db: State<'_, Db>,
    pin: String,
) -> Result<Vec<ProductDetail>, String> {
    let _user_id = validate_authenticated(db.pool(), &pin).await?;

    let sql = format!("{} ORDER BY p.active DESC, p.name", PRODUCT_DETAIL_SELECT);
    let rows = sqlx::query(&sql)
        .fetch_all(db.pool())
        .await
        .map_err(|e| format!("Error al listar productos: {}", e))?;

    Ok(rows.iter().map(row_to_product_detail).collect())
}

/// Crea un nuevo producto con stock inicial opcional.
///
/// `initial_stock_qty` se interpreta como:
///   - **cajas** si `units_per_case > 0` (el sistema desglosa: cajas × unidades_por_caja)
///   - **unidades** si `units_per_case == 0`
///
/// Solo OWNER.
#[tauri::command]
pub async fn create_product(
    db: State<'_, Db>,
    pin: String,
    name: String,
    category: String,
    barcode: Option<String>,
    units_per_case: i64,
    cost_cents: i64,
    unit_price_cents: i64,
    stock_min: i64,
    initial_stock_qty: i64,
) -> Result<ProductDetail, String> {
    let user_id = validate_owner(db.pool(), &pin).await?;

    // ── Validaciones ─────────────────────────────────────────────────────
    if name.trim().is_empty() {
        return Err("El nombre del producto no puede estar vacío".into());
    }
    if category != "BEER" && category != "PRODUCT" {
        return Err("La categoría debe ser 'BEER' o 'PRODUCT'".into());
    }
    if cost_cents < 0 {
        return Err("El costo no puede ser negativo".into());
    }
    if unit_price_cents <= 0 {
        return Err("El precio de venta debe ser mayor a 0".into());
    }
    if stock_min < 0 {
        return Err("El stock mínimo no puede ser negativo".into());
    }
    if initial_stock_qty < 0 {
        return Err("La cantidad inicial no puede ser negativa".into());
    }
    if units_per_case < 0 {
        return Err("Las unidades por caja no pueden ser negativas".into());
    }

    // Desglose automático: cajas → piezas
    let initial_units = if units_per_case > 0 {
        initial_stock_qty * units_per_case
    } else {
        initial_stock_qty
    };

    // ── Transacción ──────────────────────────────────────────────────────
    let mut tx = db
        .pool()
        .begin()
        .await
        .map_err(|e| format!("Error al iniciar transacción: {}", e))?;

    // Insertar producto
    let product_row = sqlx::query(
        "INSERT INTO products (name, category, barcode, units_per_case, cost_cents, unit_price_cents, stock_min)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(name.trim())
    .bind(&category)
    .bind(
        barcode
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty()),
    )
    .bind(units_per_case)
    .bind(cost_cents)
    .bind(unit_price_cents)
    .bind(stock_min)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "Ya existe un producto con ese código de barras".to_string()
        } else {
            format!("Error al crear producto: {}", e)
        }
    })?;

    let product_id: i64 = product_row.get("id");

    // Crear balance de inventario
    sqlx::query("INSERT INTO inventory_balances (product_id, on_hand) VALUES (?, ?)")
        .bind(product_id)
        .bind(initial_units)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Error al crear balance de inventario: {}", e))?;

    // Registrar movimiento inicial si hay stock
    if initial_units > 0 {
        sqlx::query(
            "INSERT INTO inventory_movements (product_id, qty_delta, type, user_id, note)
             VALUES (?, ?, 'INITIAL', ?, ?)",
        )
        .bind(product_id)
        .bind(initial_units)
        .bind(user_id)
        .bind(format!(
            "Stock inicial: {} {}",
            initial_stock_qty,
            if units_per_case > 0 {
                "cajas"
            } else {
                "unidades"
            }
        ))
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Error al registrar movimiento: {}", e))?;
    }

    tx.commit()
        .await
        .map_err(|e| format!("Error al confirmar transacción: {}", e))?;

    Ok(ProductDetail {
        id: product_id,
        name: name.trim().to_string(),
        category,
        barcode,
        units_per_case,
        cost_cents,
        unit_price_cents,
        stock_min,
        active: 1,
        on_hand: initial_units,
    })
}

/// Agrega stock a un producto existente (compra / reposición).
///
/// `qty` se interpreta como cajas si `units_per_case > 0`, unidades si no.
///
/// Disponible para OWNER y EMPLOYEE.
#[tauri::command]
pub async fn add_stock(
    db: State<'_, Db>,
    pin: String,
    product_id: i64,
    qty: i64,
    note: Option<String>,
) -> Result<StockResult, String> {
    let user_id = validate_authenticated(db.pool(), &pin).await?;

    if qty <= 0 {
        return Err("La cantidad debe ser mayor a 0".into());
    }

    // Info del producto
    let product = sqlx::query(
        "SELECT id, name, units_per_case, active FROM products WHERE id = ?",
    )
    .bind(product_id)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| format!("Error al buscar producto: {}", e))?
    .ok_or("Producto no encontrado")?;

    let active: i64 = product.get("active");
    if active != 1 {
        return Err("El producto está inactivo".into());
    }

    let product_name: String = product.get("name");
    let units_per_case: i64 = product.get("units_per_case");

    let units = if units_per_case > 0 {
        qty * units_per_case
    } else {
        qty
    };

    // ── Transacción ──────────────────────────────────────────────────────
    let mut tx = db
        .pool()
        .begin()
        .await
        .map_err(|e| format!("Error al iniciar transacción: {}", e))?;

    // Upsert balance de inventario
    sqlx::query(
        "INSERT INTO inventory_balances (product_id, on_hand)
         VALUES (?, ?)
         ON CONFLICT(product_id) DO UPDATE SET
            on_hand = on_hand + ?,
            updated_at = datetime('now')",
    )
    .bind(product_id)
    .bind(units)
    .bind(units)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Error al actualizar inventario: {}", e))?;

    // Registrar movimiento de compra
    let note_text = note
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| {
            format!(
                "Compra: {} {}",
                qty,
                if units_per_case > 0 {
                    "cajas"
                } else {
                    "unidades"
                }
            )
        });

    sqlx::query(
        "INSERT INTO inventory_movements (product_id, qty_delta, type, user_id, note)
         VALUES (?, ?, 'PURCHASE', ?, ?)",
    )
    .bind(product_id)
    .bind(units)
    .bind(user_id)
    .bind(&note_text)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Error al registrar movimiento: {}", e))?;

    // Obtener balance actualizado
    let balance_row =
        sqlx::query("SELECT on_hand FROM inventory_balances WHERE product_id = ?")
            .bind(product_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| format!("Error al consultar balance: {}", e))?;

    let new_on_hand: i64 = balance_row.get("on_hand");

    tx.commit()
        .await
        .map_err(|e| format!("Error al confirmar transacción: {}", e))?;

    Ok(StockResult {
        product_id,
        product_name,
        cases_added: qty,
        units_added: units,
        new_on_hand,
    })
}

/// Actualiza la información de un producto existente.
///
/// Solo OWNER.
#[tauri::command]
pub async fn update_product(
    db: State<'_, Db>,
    pin: String,
    product_id: i64,
    name: String,
    category: String,
    barcode: Option<String>,
    units_per_case: i64,
    cost_cents: i64,
    unit_price_cents: i64,
    stock_min: i64,
) -> Result<ProductDetail, String> {
    let _user_id = validate_owner(db.pool(), &pin).await?;

    if name.trim().is_empty() {
        return Err("El nombre no puede estar vacío".into());
    }
    if category != "BEER" && category != "PRODUCT" {
        return Err("Categoría inválida. Usa 'BEER' o 'PRODUCT'.".into());
    }
    if unit_price_cents <= 0 {
        return Err("El precio de venta debe ser mayor a 0".into());
    }
    if cost_cents < 0 {
        return Err("El costo no puede ser negativo".into());
    }

    // Verificar existencia
    sqlx::query("SELECT id FROM products WHERE id = ?")
        .bind(product_id)
        .fetch_optional(db.pool())
        .await
        .map_err(|e| format!("Error: {}", e))?
        .ok_or("Producto no encontrado")?;

    sqlx::query(
        "UPDATE products SET
            name = ?, category = ?, barcode = ?, units_per_case = ?,
            cost_cents = ?, unit_price_cents = ?, stock_min = ?
         WHERE id = ?",
    )
    .bind(name.trim())
    .bind(&category)
    .bind(
        barcode
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty()),
    )
    .bind(units_per_case)
    .bind(cost_cents)
    .bind(unit_price_cents)
    .bind(stock_min)
    .bind(product_id)
    .execute(db.pool())
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "Ya existe un producto con ese código de barras".to_string()
        } else {
            format!("Error al actualizar: {}", e)
        }
    })?;

    // Devolver producto actualizado con stock
    let row = sqlx::query(&format!("{} WHERE p.id = ?", PRODUCT_DETAIL_SELECT))
        .bind(product_id)
        .fetch_one(db.pool())
        .await
        .map_err(|e| format!("Error al obtener producto: {}", e))?;

    Ok(row_to_product_detail(&row))
}

/// Activa o desactiva un producto (toggle).
///
/// Solo OWNER.
#[tauri::command]
pub async fn toggle_product_active(
    db: State<'_, Db>,
    pin: String,
    product_id: i64,
) -> Result<ProductDetail, String> {
    let _user_id = validate_owner(db.pool(), &pin).await?;

    sqlx::query(
        "UPDATE products SET active = CASE WHEN active = 1 THEN 0 ELSE 1 END WHERE id = ?",
    )
    .bind(product_id)
    .execute(db.pool())
    .await
    .map_err(|e| format!("Error al cambiar estado: {}", e))?;

    let row = sqlx::query(&format!("{} WHERE p.id = ?", PRODUCT_DETAIL_SELECT))
        .bind(product_id)
        .fetch_one(db.pool())
        .await
        .map_err(|e| format!("Error al obtener producto: {}", e))?;

    Ok(row_to_product_detail(&row))
}

/// Ajusta (resta) stock de un producto por pérdida, merma, etc.
///
/// `qty` siempre es positivo y se interpreta en **piezas** (unidades individuales).
/// El sistema lo restará del inventario.
///
/// Solo OWNER.
#[tauri::command]
pub async fn adjust_stock(
    db: State<'_, Db>,
    pin: String,
    product_id: i64,
    qty: i64,
    reason: String,
) -> Result<AdjustResult, String> {
    let user_id = validate_owner(db.pool(), &pin).await?;

    if qty <= 0 {
        return Err("La cantidad debe ser mayor a 0".into());
    }
    if reason.trim().is_empty() {
        return Err("Debes indicar el motivo del ajuste".into());
    }

    // Info del producto y stock actual
    let row = sqlx::query(
        "SELECT p.id, p.name, COALESCE(ib.on_hand, 0) as on_hand
         FROM products p
         LEFT JOIN inventory_balances ib ON p.id = ib.product_id
         WHERE p.id = ?",
    )
    .bind(product_id)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| format!("Error al buscar producto: {}", e))?
    .ok_or("Producto no encontrado")?;

    let product_name: String = row.get("name");
    let current_on_hand: i64 = row.get("on_hand");

    if qty > current_on_hand {
        return Err(format!(
            "No se pueden restar {} piezas. Stock actual: {} piezas",
            qty, current_on_hand
        ));
    }

    // ── Transacción ──────────────────────────────────────────────────────
    let mut tx = db
        .pool()
        .begin()
        .await
        .map_err(|e| format!("Error al iniciar transacción: {}", e))?;

    // Restar del balance
    sqlx::query(
        "UPDATE inventory_balances SET on_hand = on_hand - ?, updated_at = datetime('now')
         WHERE product_id = ?",
    )
    .bind(qty)
    .bind(product_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Error al actualizar inventario: {}", e))?;

    // Registrar movimiento de ajuste (qty_delta negativo)
    sqlx::query(
        "INSERT INTO inventory_movements (product_id, qty_delta, type, user_id, note)
         VALUES (?, ?, 'ADJUSTMENT', ?, ?)",
    )
    .bind(product_id)
    .bind(-qty) // negativo para reflejar salida
    .bind(user_id)
    .bind(reason.trim())
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Error al registrar movimiento: {}", e))?;

    // Obtener balance actualizado
    let balance_row =
        sqlx::query("SELECT on_hand FROM inventory_balances WHERE product_id = ?")
            .bind(product_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| format!("Error al consultar balance: {}", e))?;

    let new_on_hand: i64 = balance_row.get("on_hand");

    tx.commit()
        .await
        .map_err(|e| format!("Error al confirmar transacción: {}", e))?;

    Ok(AdjustResult {
        product_id,
        product_name,
        units_removed: qty,
        new_on_hand,
    })
}
