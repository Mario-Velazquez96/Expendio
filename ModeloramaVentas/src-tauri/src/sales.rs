use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::State;

use crate::db::Db;

// ─── Tipos de respuesta ───────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct Product {
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SaleLineInput {
    pub product_id: i64,
    pub qty: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaleLineResult {
    pub id: i64,
    pub sale_id: i64,
    pub product_id: i64,
    pub product_name: String,
    pub qty: i64,
    pub unit_price_cents: i64,
    pub line_total_cents: i64,
    pub cost_at_sale_cents: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaleResult {
    pub id: i64,
    pub cash_session_id: i64,
    pub created_at: String,
    pub user_id: i64,
    pub total_cents: i64,
    pub payment_method: String,
    pub lines: Vec<SaleLineResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionSalesSummary {
    pub sales: Vec<SaleBrief>,
    pub count: i64,
    pub total_cents: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaleBrief {
    pub id: i64,
    pub created_at: String,
    pub total_cents: i64,
    pub payment_method: String,
    pub user_id: i64,
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn row_to_product(r: &sqlx::sqlite::SqliteRow) -> Product {
    Product {
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

const PRODUCT_SELECT: &str = "
    SELECT p.id, p.name, p.category, p.barcode, p.units_per_case,
           p.cost_cents, p.unit_price_cents, p.stock_min, p.active,
           COALESCE(ib.on_hand, 0) as on_hand
    FROM products p
    LEFT JOIN inventory_balances ib ON p.id = ib.product_id";

// ─── Commands ─────────────────────────────────────────────────────────────────

/// Busca productos activos por nombre o código de barras.
#[tauri::command]
pub async fn search_products(
    db: State<'_, Db>,
    query: String,
) -> Result<Vec<Product>, String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err("El criterio de búsqueda no puede estar vacío".into());
    }

    let search = format!("%{}%", trimmed);

    let sql = format!(
        "{} WHERE p.active = 1 AND (p.name LIKE ?1 OR p.barcode LIKE ?1) ORDER BY p.name LIMIT 50",
        PRODUCT_SELECT
    );

    let rows = sqlx::query(&sql)
        .bind(&search)
        .fetch_all(db.pool())
        .await
        .map_err(|e| format!("Error al buscar productos: {}", e))?;

    Ok(rows.iter().map(row_to_product).collect())
}

/// Obtiene un producto por código de barras exacto.
#[tauri::command]
pub async fn get_product_by_barcode(
    db: State<'_, Db>,
    barcode: String,
) -> Result<Option<Product>, String> {
    let sql = format!("{} WHERE p.active = 1 AND p.barcode = ? LIMIT 1", PRODUCT_SELECT);

    let row = sqlx::query(&sql)
        .bind(barcode.trim())
        .fetch_optional(db.pool())
        .await
        .map_err(|e| format!("Error al buscar producto: {}", e))?;

    Ok(row.as_ref().map(row_to_product))
}

/// Lista todos los productos activos (para el catálogo del POS).
#[tauri::command]
pub async fn list_products(
    db: State<'_, Db>,
) -> Result<Vec<Product>, String> {
    let sql = format!("{} WHERE p.active = 1 ORDER BY p.name", PRODUCT_SELECT);

    let rows = sqlx::query(&sql)
        .fetch_all(db.pool())
        .await
        .map_err(|e| format!("Error al listar productos: {}", e))?;

    Ok(rows.iter().map(row_to_product).collect())
}

/// Finaliza una venta: valida stock, descuenta inventario, registra todo en
/// una sola transacción.
///
/// Flujo transaccional:
///  1. Validar caja OPEN.
///  2. Validar stock suficiente para cada línea.
///  3. INSERT sales + sale_lines.
///  4. Congelar cost_at_sale.
///  5. UPDATE inventory_balances (descontar).
///  6. INSERT inventory_movements (tipo SALE).
#[tauri::command]
pub async fn finalize_sale(
    db: State<'_, Db>,
    session_id: i64,
    user_id: i64,
    lines: Vec<SaleLineInput>,
    payment_method: Option<String>,
) -> Result<SaleResult, String> {
    let payment = payment_method.unwrap_or_else(|| "CASH".to_string());

    // Validar método de pago
    if !["CASH", "TRANSFER", "EXTERNAL"].contains(&payment.as_str()) {
        return Err("Método de pago inválido. Usa 'CASH', 'TRANSFER' o 'EXTERNAL'.".into());
    }

    // Al menos una línea
    if lines.is_empty() {
        return Err("La venta debe tener al menos un producto".into());
    }

    // Sesión OPEN
    let session = sqlx::query("SELECT id, status FROM cash_sessions WHERE id = ?")
        .bind(session_id)
        .fetch_optional(db.pool())
        .await
        .map_err(|e| format!("Error al verificar sesión: {}", e))?
        .ok_or("Sesión de caja no encontrada")?;

    let status: String = session.get("status");
    if status != "OPEN" {
        return Err("La sesión de caja no está abierta. Abre una sesión para vender.".into());
    }

    // Usuario válido
    let user = sqlx::query("SELECT id, active FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(db.pool())
        .await
        .map_err(|e| format!("Error al validar usuario: {}", e))?
        .ok_or("Usuario no encontrado")?;

    let active: i64 = user.get("active");
    if active != 1 {
        return Err("El usuario no está activo".into());
    }

    // ── Iniciar transacción ──────────────────────────────────────────────
    let mut tx = db
        .pool()
        .begin()
        .await
        .map_err(|e| format!("Error al iniciar transacción: {}", e))?;

    // Pre-validar productos y stock
    struct LineDetail {
        product_id: i64,
        product_name: String,
        qty: i64,
        unit_price_cents: i64,
        cost_cents: i64,
        on_hand: i64,
    }

    let mut validated: Vec<LineDetail> = Vec::with_capacity(lines.len());
    let mut total_cents: i64 = 0;

    for line in &lines {
        if line.qty <= 0 {
            return Err(format!(
                "La cantidad debe ser mayor a 0 (producto ID {})",
                line.product_id
            ));
        }

        let product = sqlx::query(
            "SELECT p.id, p.name, p.unit_price_cents, p.cost_cents,
                    COALESCE(ib.on_hand, 0) as on_hand
             FROM products p
             LEFT JOIN inventory_balances ib ON p.id = ib.product_id
             WHERE p.id = ? AND p.active = 1",
        )
        .bind(line.product_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| format!("Error al validar producto {}: {}", line.product_id, e))?
        .ok_or(format!(
            "Producto {} no encontrado o inactivo",
            line.product_id
        ))?;

        let on_hand: i64 = product.get("on_hand");
        let name: String = product.get("name");

        if on_hand < line.qty {
            return Err(format!(
                "Stock insuficiente para '{}'. Disponible: {}, Solicitado: {}",
                name, on_hand, line.qty
            ));
        }

        let unit_price: i64 = product.get("unit_price_cents");
        let cost: i64 = product.get("cost_cents");
        let line_total = unit_price * line.qty;
        total_cents += line_total;

        validated.push(LineDetail {
            product_id: line.product_id,
            product_name: name,
            qty: line.qty,
            unit_price_cents: unit_price,
            cost_cents: cost,
            on_hand,
        });
    }

    // INSERT venta
    let sale_row = sqlx::query(
        "INSERT INTO sales (cash_session_id, user_id, total_cents, payment_method)
         VALUES (?, ?, ?, ?)
         RETURNING id, cash_session_id, created_at, user_id, total_cents, payment_method",
    )
    .bind(session_id)
    .bind(user_id)
    .bind(total_cents)
    .bind(&payment)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| format!("Error al crear venta: {}", e))?;

    let sale_id: i64 = sale_row.get("id");
    let created_at: String = sale_row.get("created_at");

    // INSERT líneas + actualizar inventario
    let mut result_lines: Vec<SaleLineResult> = Vec::with_capacity(validated.len());

    for detail in &validated {
        let line_total = detail.unit_price_cents * detail.qty;

        // Línea de venta
        let line_row = sqlx::query(
            "INSERT INTO sale_lines
                (sale_id, product_id, qty, unit_price_cents, line_total_cents, cost_at_sale_cents)
             VALUES (?, ?, ?, ?, ?, ?)
             RETURNING id",
        )
        .bind(sale_id)
        .bind(detail.product_id)
        .bind(detail.qty)
        .bind(detail.unit_price_cents)
        .bind(line_total)
        .bind(detail.cost_cents)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| format!("Error al crear línea de venta: {}", e))?;

        let line_id: i64 = line_row.get("id");

        // Descontar inventario (UPSERT)
        let new_on_hand = detail.on_hand - detail.qty;

        sqlx::query(
            "INSERT INTO inventory_balances (product_id, on_hand, updated_at)
             VALUES (?, ?, datetime('now'))
             ON CONFLICT(product_id) DO UPDATE
                SET on_hand = ?, updated_at = datetime('now')",
        )
        .bind(detail.product_id)
        .bind(new_on_hand)
        .bind(new_on_hand)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Error al actualizar inventario: {}", e))?;

        // Movimiento de inventario (auditoría)
        sqlx::query(
            "INSERT INTO inventory_movements
                (product_id, qty_delta, type, ref_table, ref_id, user_id, note)
             VALUES (?, ?, 'SALE', 'sales', ?, ?, ?)",
        )
        .bind(detail.product_id)
        .bind(-detail.qty) // negativo = salida
        .bind(sale_id)
        .bind(user_id)
        .bind(format!("Venta #{}", sale_id))
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Error al registrar movimiento de inventario: {}", e))?;

        result_lines.push(SaleLineResult {
            id: line_id,
            sale_id,
            product_id: detail.product_id,
            product_name: detail.product_name.clone(),
            qty: detail.qty,
            unit_price_cents: detail.unit_price_cents,
            line_total_cents: line_total,
            cost_at_sale_cents: detail.cost_cents,
        });
    }

    tx.commit()
        .await
        .map_err(|e| format!("Error al confirmar transacción: {}", e))?;

    Ok(SaleResult {
        id: sale_id,
        cash_session_id: session_id,
        created_at,
        user_id,
        total_cents,
        payment_method: payment,
        lines: result_lines,
    })
}

/// Devuelve las ventas de una sesión (resumen + lista).
#[tauri::command]
pub async fn get_session_sales(
    db: State<'_, Db>,
    session_id: i64,
) -> Result<SessionSalesSummary, String> {
    let rows = sqlx::query(
        "SELECT id, created_at, total_cents, payment_method, user_id
         FROM sales
         WHERE cash_session_id = ?
         ORDER BY created_at DESC",
    )
    .bind(session_id)
    .fetch_all(db.pool())
    .await
    .map_err(|e| format!("Error al consultar ventas: {}", e))?;

    let sales: Vec<SaleBrief> = rows
        .iter()
        .map(|r| SaleBrief {
            id: r.get("id"),
            created_at: r.get("created_at"),
            total_cents: r.get("total_cents"),
            payment_method: r.get("payment_method"),
            user_id: r.get("user_id"),
        })
        .collect();

    let total_cents: i64 = sales.iter().map(|s| s.total_cents).sum();
    let count = sales.len() as i64;

    Ok(SessionSalesSummary {
        sales,
        count,
        total_cents,
    })
}
