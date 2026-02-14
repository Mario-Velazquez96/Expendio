use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::State;

use crate::auth;
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

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSaleResult {
    pub sale_id: i64,
    pub cash_session_id: i64,
    pub user_id: i64,
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
    pub price_rule_id: Option<i64>,
    pub rule_name: Option<String>,
    pub rule_required_qty: Option<i64>,
    pub rule_bundle_price_cents: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaleResult {
    pub id: i64,
    pub cash_session_id: i64,
    pub created_at: String,
    pub user_id: i64,
    pub total_cents: i64,
    pub status: String,
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
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PriceRule {
    pub id: i64,
    pub product_id: i64,
    pub name: String,
    pub required_qty: i64,
    pub bundle_price_cents: i64,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub enabled: i64,
    pub priority: i64,
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

fn row_to_sale_line(r: &sqlx::sqlite::SqliteRow) -> SaleLineResult {
    SaleLineResult {
        id: r.get("id"),
        sale_id: r.get("sale_id"),
        product_id: r.get("product_id"),
        product_name: r.get("product_name"),
        qty: r.get("qty"),
        unit_price_cents: r.get("unit_price_cents"),
        line_total_cents: r.get("line_total_cents"),
        cost_at_sale_cents: r.get("cost_at_sale_cents"),
        price_rule_id: r.get("price_rule_id"),
        rule_name: r.get("rule_name"),
        rule_required_qty: r.get("rule_required_qty"),
        rule_bundle_price_cents: r.get("rule_bundle_price_cents"),
    }
}

const SALE_LINE_SELECT: &str = "
    SELECT sl.id, sl.sale_id, sl.product_id, p.name as product_name,
           sl.qty, sl.unit_price_cents, sl.line_total_cents, sl.cost_at_sale_cents,
           sl.price_rule_id, pr.name as rule_name, sl.rule_required_qty, sl.rule_bundle_price_cents
    FROM sale_lines sl
    JOIN products p ON sl.product_id = p.id
    LEFT JOIN price_rules pr ON sl.price_rule_id = pr.id";

fn calc_line_total(
    qty: i64,
    unit_price_cents: i64,
    rule_required_qty: Option<i64>,
    rule_bundle_price_cents: Option<i64>,
) -> Result<i64, String> {
    match (rule_required_qty, rule_bundle_price_cents) {
        (Some(required_qty), Some(bundle_price_cents)) => {
            if required_qty <= 1 {
                return Err("La regla de promoción tiene required_qty inválido".into());
            }
            if qty <= 0 || qty % required_qty != 0 {
                return Err(format!(
                    "Cantidad inválida para promo. qty={} debe ser múltiplo de {}",
                    qty, required_qty
                ));
            }
            Ok((qty / required_qty) * bundle_price_cents)
        }
        _ => Ok(unit_price_cents * qty),
    }
}

/// Valida un PIN y devuelve el user_id correspondiente.
async fn validate_pin(pool: &sqlx::SqlitePool, pin: &str) -> Result<i64, String> {
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

/// Valida que una venta exista, esté en DRAFT y su sesión siga OPEN.
/// Devuelve (sale_id, cash_session_id, user_id).
async fn validate_draft_sale(
    pool: &sqlx::SqlitePool,
    sale_id: i64,
) -> Result<(i64, i64, i64), String> {
    let row = sqlx::query(
        "SELECT s.id, s.cash_session_id, s.user_id, s.status, cs.status as session_status
         FROM sales s
         JOIN cash_sessions cs ON s.cash_session_id = cs.id
         WHERE s.id = ?",
    )
    .bind(sale_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Error al validar venta: {}", e))?
    .ok_or("Venta no encontrada")?;

    let sale_status: String = row.get("status");
    if sale_status != "DRAFT" {
        return Err("La venta ya fue finalizada".into());
    }

    let session_status: String = row.get("session_status");
    if session_status != "OPEN" {
        return Err("La sesión de caja está cerrada. No se puede modificar la venta.".into());
    }

    Ok((row.get("id"), row.get("cash_session_id"), row.get("user_id")))
}

// ─── Commands: Productos ──────────────────────────────────────────────────────

/// Busca productos activos. Si el texto es solo dígitos, asume escaneo de
/// barcode y hace match exacto; de lo contrario busca por nombre con LIKE.
#[tauri::command]
pub async fn search_products(
    db: State<'_, Db>,
    query: String,
) -> Result<Vec<Product>, String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err("El criterio de búsqueda no puede estar vacío".into());
    }

    // Heurística: si todo son dígitos, se trata como escaneo de barcode (match exacto).
    let is_barcode_scan = trimmed.chars().all(|c| c.is_ascii_digit());

    let (sql, bind_value) = if is_barcode_scan {
        (
            format!(
                "{} WHERE p.active = 1 AND p.barcode = ?1 ORDER BY p.name LIMIT 50",
                PRODUCT_SELECT
            ),
            trimmed.to_string(),
        )
    } else {
        (
            format!(
                "{} WHERE p.active = 1 AND p.name LIKE ?1 ORDER BY p.name LIMIT 50",
                PRODUCT_SELECT
            ),
            format!("%{}%", trimmed),
        )
    };

    let rows = sqlx::query(&sql)
        .bind(&bind_value)
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
pub async fn list_products(db: State<'_, Db>) -> Result<Vec<Product>, String> {
    let sql = format!("{} WHERE p.active = 1 ORDER BY p.name", PRODUCT_SELECT);

    let rows = sqlx::query(&sql)
        .fetch_all(db.pool())
        .await
        .map_err(|e| format!("Error al listar productos: {}", e))?;

    Ok(rows.iter().map(row_to_product).collect())
}

/// Lista reglas de promoción habilitadas para un producto.
#[tauri::command]
pub async fn list_product_price_rules(
    db: State<'_, Db>,
    product_id: i64,
) -> Result<Vec<PriceRule>, String> {
    let rows = sqlx::query(
        "SELECT id, product_id, name, required_qty, bundle_price_cents, start_at, end_at, enabled, priority
         FROM price_rules
         WHERE product_id = ?
           AND enabled = 1
           AND (start_at IS NULL OR start_at <= datetime('now'))
           AND (end_at IS NULL OR end_at >= datetime('now'))
         ORDER BY required_qty DESC, priority DESC, id DESC",
    )
    .bind(product_id)
    .fetch_all(db.pool())
    .await
    .map_err(|e| format!("Error al listar promociones: {}", e))?;

    Ok(rows
        .iter()
        .map(|r| PriceRule {
            id: r.get("id"),
            product_id: r.get("product_id"),
            name: r.get("name"),
            required_qty: r.get("required_qty"),
            bundle_price_cents: r.get("bundle_price_cents"),
            start_at: r.get("start_at"),
            end_at: r.get("end_at"),
            enabled: r.get("enabled"),
            priority: r.get("priority"),
        })
        .collect())
}

/// Aplica una promo manual a una línea normal (sin promo).
///
/// Regla:
///  - Debe existir línea DRAFT + sesión OPEN.
///  - La regla debe pertenecer al mismo producto y estar vigente.
///  - Si hay sobrante, se divide en línea promo + línea normal.
#[tauri::command]
pub async fn apply_price_rule_to_line(
    db: State<'_, Db>,
    pin: String,
    line_id: i64,
    rule_id: i64,
) -> Result<Vec<SaleLineResult>, String> {
    let _user_id = validate_pin(db.pool(), &pin).await?;

    let line_row = sqlx::query(
        "SELECT sl.id, sl.sale_id, sl.product_id, sl.qty, sl.unit_price_cents, sl.cost_at_sale_cents,
                sl.price_rule_id, p.name as product_name, COALESCE(ib.on_hand, 0) as on_hand
         FROM sale_lines sl
         JOIN products p ON sl.product_id = p.id
         LEFT JOIN inventory_balances ib ON p.id = ib.product_id
         WHERE sl.id = ?",
    )
    .bind(line_id)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| format!("Error al buscar línea: {}", e))?
    .ok_or("Línea de venta no encontrada")?;

    let sale_id: i64 = line_row.get("sale_id");
    let product_id: i64 = line_row.get("product_id");
    let qty: i64 = line_row.get("qty");
    let unit_price_cents: i64 = line_row.get("unit_price_cents");
    let cost_at_sale_cents: i64 = line_row.get("cost_at_sale_cents");
    let product_name: String = line_row.get("product_name");
    let on_hand: i64 = line_row.get("on_hand");
    let current_price_rule_id: Option<i64> = line_row.get("price_rule_id");

    if current_price_rule_id.is_some() {
        return Err("La línea ya tiene promoción aplicada".into());
    }

    validate_draft_sale(db.pool(), sale_id).await?;

    if on_hand < qty {
        return Err(format!(
            "Stock insuficiente para '{}'. Disponible: {}, En línea: {}",
            product_name, on_hand, qty
        ));
    }

    let rule_row = sqlx::query(
        "SELECT id, product_id, name, required_qty, bundle_price_cents
         FROM price_rules
         WHERE id = ?
           AND enabled = 1
           AND (start_at IS NULL OR start_at <= datetime('now'))
           AND (end_at IS NULL OR end_at >= datetime('now'))",
    )
    .bind(rule_id)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| format!("Error al buscar promoción: {}", e))?
    .ok_or("Promoción no encontrada o no vigente")?;

    let rule_product_id: i64 = rule_row.get("product_id");
    let rule_name: String = rule_row.get("name");
    let rule_required_qty: i64 = rule_row.get("required_qty");
    let rule_bundle_price_cents: i64 = rule_row.get("bundle_price_cents");

    if rule_product_id != product_id {
        return Err("La promoción no corresponde al producto de la línea".into());
    }

    let bundle_count = qty / rule_required_qty;
    if bundle_count <= 0 {
        return Err(format!(
            "La promo '{}' requiere mínimo {} piezas",
            rule_name, rule_required_qty
        ));
    }

    let promo_qty = bundle_count * rule_required_qty;
    let remainder_qty = qty - promo_qty;
    let promo_total = calc_line_total(
        promo_qty,
        unit_price_cents,
        Some(rule_required_qty),
        Some(rule_bundle_price_cents),
    )?;

    let mut tx = db
        .pool()
        .begin()
        .await
        .map_err(|e| format!("Error al iniciar transacción: {}", e))?;

    sqlx::query("DELETE FROM sale_lines WHERE id = ?")
        .bind(line_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Error al reemplazar línea: {}", e))?;

    sqlx::query(
        "INSERT INTO sale_lines
            (sale_id, product_id, qty, unit_price_cents, line_total_cents, cost_at_sale_cents,
             price_rule_id, rule_required_qty, rule_bundle_price_cents)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(sale_id)
    .bind(product_id)
    .bind(promo_qty)
    .bind(unit_price_cents)
    .bind(promo_total)
    .bind(cost_at_sale_cents)
    .bind(rule_id)
    .bind(rule_required_qty)
    .bind(rule_bundle_price_cents)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Error al crear línea promo: {}", e))?;

    if remainder_qty > 0 {
        let remainder_total = unit_price_cents * remainder_qty;
        sqlx::query(
            "INSERT INTO sale_lines
                (sale_id, product_id, qty, unit_price_cents, line_total_cents, cost_at_sale_cents)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(sale_id)
        .bind(product_id)
        .bind(remainder_qty)
        .bind(unit_price_cents)
        .bind(remainder_total)
        .bind(cost_at_sale_cents)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Error al crear línea sobrante: {}", e))?;
    }

    let line_rows = sqlx::query(&format!(
        "{} WHERE sl.sale_id = ? ORDER BY sl.id",
        SALE_LINE_SELECT
    ))
    .bind(sale_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| format!("Error al consultar líneas: {}", e))?;

    tx.commit()
        .await
        .map_err(|e| format!("Error al confirmar transacción: {}", e))?;

    Ok(line_rows.iter().map(row_to_sale_line).collect())
}

/// Quita promoción de una línea promo y regresa total a precio normal.
#[tauri::command]
pub async fn remove_price_rule_from_line(
    db: State<'_, Db>,
    pin: String,
    line_id: i64,
) -> Result<SaleLineResult, String> {
    let _user_id = validate_pin(db.pool(), &pin).await?;

    let line_row = sqlx::query(
        "SELECT sl.id, sl.sale_id, sl.product_id, sl.qty,
                sl.unit_price_cents, sl.cost_at_sale_cents,
                sl.price_rule_id, p.name as product_name
         FROM sale_lines sl
         JOIN products p ON sl.product_id = p.id
         WHERE sl.id = ?",
    )
    .bind(line_id)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| format!("Error al buscar línea: {}", e))?
    .ok_or("Línea de venta no encontrada")?;

    let sale_id: i64 = line_row.get("sale_id");
    validate_draft_sale(db.pool(), sale_id).await?;

    let price_rule_id: Option<i64> = line_row.get("price_rule_id");
    if price_rule_id.is_none() {
        return Err("La línea no tiene promoción aplicada".into());
    }

    let qty: i64 = line_row.get("qty");
    let unit_price_cents: i64 = line_row.get("unit_price_cents");
    let cost_at_sale_cents: i64 = line_row.get("cost_at_sale_cents");
    let line_total_cents = unit_price_cents * qty;
    let product_id: i64 = line_row.get("product_id");
    let product_name: String = line_row.get("product_name");

    sqlx::query(
        "UPDATE sale_lines
         SET price_rule_id = NULL,
             rule_required_qty = NULL,
             rule_bundle_price_cents = NULL,
             line_total_cents = ?
         WHERE id = ?",
    )
    .bind(line_total_cents)
    .bind(line_id)
    .execute(db.pool())
    .await
    .map_err(|e| format!("Error al quitar promoción: {}", e))?;

    Ok(SaleLineResult {
        id: line_id,
        sale_id,
        product_id,
        product_name,
        qty,
        unit_price_cents,
        line_total_cents,
        cost_at_sale_cents,
        price_rule_id: None,
        rule_name: None,
        rule_required_qty: None,
        rule_bundle_price_cents: None,
    })
}

// ─── Commands: Flujo de venta ─────────────────────────────────────────────────

/// Crea una venta vacía (DRAFT) amarrada a la sesión de caja abierta.
///
/// Validaciones:
///  - PIN válido → user_id.
///  - Debe existir una cash_session OPEN.
///  - Crea sale con total_cents = 0, status = 'DRAFT'.
#[tauri::command]
pub async fn create_sale(
    db: State<'_, Db>,
    pin: String,
) -> Result<CreateSaleResult, String> {
    let user_id = validate_pin(db.pool(), &pin).await?;

    // Sesión OPEN
    let session_row = sqlx::query("SELECT id FROM cash_sessions WHERE status = 'OPEN' LIMIT 1")
        .fetch_optional(db.pool())
        .await
        .map_err(|e| format!("Error al verificar sesión de caja: {}", e))?
        .ok_or("No hay sesión de caja abierta. Abre una para poder vender.")?;

    let cash_session_id: i64 = session_row.get("id");

    let row = sqlx::query(
        "INSERT INTO sales (cash_session_id, user_id, total_cents, status, payment_method)
         VALUES (?, ?, 0, 'DRAFT', 'CASH')
         RETURNING id",
    )
    .bind(cash_session_id)
    .bind(user_id)
    .fetch_one(db.pool())
    .await
    .map_err(|e| format!("Error al crear venta: {}", e))?;

    Ok(CreateSaleResult {
        sale_id: row.get("id"),
        cash_session_id,
        user_id,
    })
}

/// Agrega una línea a una venta DRAFT.
///
/// Si ya existe una línea del mismo producto (sin promo), incrementa la
/// cantidad en lugar de insertar una nueva línea (reduce clicks).
///
/// Validaciones:
///  - PIN válido.
///  - Venta en DRAFT con sesión OPEN.
///  - Producto activo.
///  - Stock estricto: on_hand >= qty total.
///  - qty > 0.
#[tauri::command]
pub async fn add_sale_line(
    db: State<'_, Db>,
    pin: String,
    sale_id: i64,
    product_id: i64,
    qty: i64,
) -> Result<SaleLineResult, String> {
    if qty <= 0 {
        return Err("La cantidad debe ser mayor a 0".into());
    }

    let _user_id = validate_pin(db.pool(), &pin).await?;
    validate_draft_sale(db.pool(), sale_id).await?;

    // Info del producto + stock
    let product = sqlx::query(
        "SELECT p.id, p.name, p.unit_price_cents, p.cost_cents,
                COALESCE(ib.on_hand, 0) as on_hand
         FROM products p
         LEFT JOIN inventory_balances ib ON p.id = ib.product_id
         WHERE p.id = ? AND p.active = 1",
    )
    .bind(product_id)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| format!("Error al buscar producto: {}", e))?
    .ok_or("Producto no encontrado o inactivo")?;

    let on_hand: i64 = product.get("on_hand");
    let unit_price: i64 = product.get("unit_price_cents");
    let cost: i64 = product.get("cost_cents");
    let product_name: String = product.get("name");

    // ¿Ya existe línea para este producto (sin promo)?
    let existing_line = sqlx::query(
        "SELECT id, qty FROM sale_lines
         WHERE sale_id = ? AND product_id = ? AND price_rule_id IS NULL",
    )
    .bind(sale_id)
    .bind(product_id)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| format!("Error al buscar línea existente: {}", e))?;

    let (line_id, final_qty) = if let Some(existing) = existing_line {
        // Upsert: incrementar qty
        let existing_id: i64 = existing.get("id");
        let existing_qty: i64 = existing.get("qty");
        let new_qty = existing_qty + qty;

        if on_hand < new_qty {
            return Err(format!(
                "Stock insuficiente para '{}'. Disponible: {}, Total solicitado: {}",
                product_name, on_hand, new_qty
            ));
        }

        let line_total = unit_price * new_qty;

        sqlx::query(
            "UPDATE sale_lines
             SET qty = ?, line_total_cents = ?, unit_price_cents = ?, cost_at_sale_cents = ?
             WHERE id = ?",
        )
        .bind(new_qty)
        .bind(line_total)
        .bind(unit_price)
        .bind(cost)
        .execute(db.pool())
        .await
        .map_err(|e| format!("Error al actualizar línea: {}", e))?;

        (existing_id, new_qty)
    } else {
        // Línea nueva
        if on_hand < qty {
            return Err(format!(
                "Stock insuficiente para '{}'. Disponible: {}, Solicitado: {}",
                product_name, on_hand, qty
            ));
        }

        let line_total = unit_price * qty;

        let row = sqlx::query(
            "INSERT INTO sale_lines
                (sale_id, product_id, qty, unit_price_cents, line_total_cents, cost_at_sale_cents)
             VALUES (?, ?, ?, ?, ?, ?)
             RETURNING id",
        )
        .bind(sale_id)
        .bind(product_id)
        .bind(qty)
        .bind(unit_price)
        .bind(line_total)
        .bind(cost)
        .fetch_one(db.pool())
        .await
        .map_err(|e| format!("Error al crear línea: {}", e))?;

        (row.get("id"), qty)
    };

    let final_line_total = unit_price * final_qty;

    Ok(SaleLineResult {
        id: line_id,
        sale_id,
        product_id,
        product_name,
        qty: final_qty,
        unit_price_cents: unit_price,
        line_total_cents: final_line_total,
        cost_at_sale_cents: cost,
        price_rule_id: None,
        rule_name: None,
        rule_required_qty: None,
        rule_bundle_price_cents: None,
    })
}

/// Actualiza la cantidad de una línea existente.
///
/// Validaciones:
///  - PIN válido.
///  - Venta DRAFT + sesión OPEN.
///  - new_qty > 0 (usar remove_sale_line para eliminar).
///  - Bloqueo estricto: si sube cantidad, on_hand >= new_qty.
#[tauri::command]
pub async fn update_sale_line_qty(
    db: State<'_, Db>,
    pin: String,
    line_id: i64,
    new_qty: i64,
) -> Result<SaleLineResult, String> {
    if new_qty <= 0 {
        return Err("La cantidad debe ser mayor a 0. Usa eliminar línea para quitar.".into());
    }

    let _user_id = validate_pin(db.pool(), &pin).await?;

    // Obtener línea con info del producto y stock
    let line_row = sqlx::query(
        "SELECT sl.id, sl.sale_id, sl.product_id, sl.qty as old_qty,
                sl.unit_price_cents, sl.cost_at_sale_cents,
                sl.price_rule_id, pr.name as rule_name,
                sl.rule_required_qty, sl.rule_bundle_price_cents,
                p.name as product_name,
                COALESCE(ib.on_hand, 0) as on_hand
         FROM sale_lines sl
         JOIN products p ON sl.product_id = p.id
         LEFT JOIN price_rules pr ON sl.price_rule_id = pr.id
         LEFT JOIN inventory_balances ib ON p.id = ib.product_id
         WHERE sl.id = ?",
    )
    .bind(line_id)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| format!("Error al buscar línea: {}", e))?
    .ok_or("Línea de venta no encontrada")?;

    let sale_id: i64 = line_row.get("sale_id");
    validate_draft_sale(db.pool(), sale_id).await?;

    let old_qty: i64 = line_row.get("old_qty");
    let on_hand: i64 = line_row.get("on_hand");
    let unit_price: i64 = line_row.get("unit_price_cents");
    let cost: i64 = line_row.get("cost_at_sale_cents");
    let product_name: String = line_row.get("product_name");
    let product_id: i64 = line_row.get("product_id");
    let price_rule_id: Option<i64> = line_row.get("price_rule_id");
    let rule_name: Option<String> = line_row.get("rule_name");
    let rule_required_qty: Option<i64> = line_row.get("rule_required_qty");
    let rule_bundle_price_cents: Option<i64> = line_row.get("rule_bundle_price_cents");

    if price_rule_id.is_some() {
        return Err("La línea tiene promoción. Elimínala o quita promo para cambiar cantidad.".into());
    }

    // Si sube la cantidad, validar stock
    if new_qty > old_qty && on_hand < new_qty {
        return Err(format!(
            "Stock insuficiente para '{}'. Disponible: {}, Solicitado: {}",
            product_name, on_hand, new_qty
        ));
    }

    let line_total = unit_price * new_qty;

    sqlx::query("UPDATE sale_lines SET qty = ?, line_total_cents = ? WHERE id = ?")
        .bind(new_qty)
        .bind(line_total)
        .bind(line_id)
        .execute(db.pool())
        .await
        .map_err(|e| format!("Error al actualizar línea: {}", e))?;

    Ok(SaleLineResult {
        id: line_id,
        sale_id,
        product_id,
        product_name,
        qty: new_qty,
        unit_price_cents: unit_price,
        line_total_cents: line_total,
        cost_at_sale_cents: cost,
        price_rule_id,
        rule_name,
        rule_required_qty,
        rule_bundle_price_cents,
    })
}

/// Elimina una línea de una venta DRAFT.
///
/// Validaciones:
///  - PIN válido.
///  - Venta DRAFT + sesión OPEN.
#[tauri::command]
pub async fn remove_sale_line(
    db: State<'_, Db>,
    pin: String,
    line_id: i64,
) -> Result<(), String> {
    let _user_id = validate_pin(db.pool(), &pin).await?;

    let line_row = sqlx::query("SELECT sale_id FROM sale_lines WHERE id = ?")
        .bind(line_id)
        .fetch_optional(db.pool())
        .await
        .map_err(|e| format!("Error al buscar línea: {}", e))?
        .ok_or("Línea de venta no encontrada")?;

    let sale_id: i64 = line_row.get("sale_id");
    validate_draft_sale(db.pool(), sale_id).await?;

    sqlx::query("DELETE FROM sale_lines WHERE id = ?")
        .bind(line_id)
        .execute(db.pool())
        .await
        .map_err(|e| format!("Error al eliminar línea: {}", e))?;

    Ok(())
}

/// Finaliza una venta DRAFT: recalcula totales, valida stock de TODAS las
/// líneas, descuenta inventario y registra movimientos de auditoría.
///
/// Todo ocurre en una sola transacción SQLite.
///
/// Reglas:
///  - No se permite si la caja está cerrada.
///  - La venta debe tener al menos una línea.
///  - Stock estricto por producto (agregado si hay varias líneas del mismo).
#[tauri::command]
pub async fn finalize_sale(
    db: State<'_, Db>,
    pin: String,
    sale_id: i64,
) -> Result<SaleResult, String> {
    let user_id = validate_pin(db.pool(), &pin).await?;

    // ── Iniciar transacción ──────────────────────────────────────────────
    let mut tx = db
        .pool()
        .begin()
        .await
        .map_err(|e| format!("Error al iniciar transacción: {}", e))?;

    // Validar venta DRAFT y sesión OPEN (dentro de la transacción)
    let sale_row = sqlx::query(
        "SELECT s.id, s.cash_session_id, s.user_id, s.status, s.created_at,
                s.payment_method, cs.status as session_status
         FROM sales s
         JOIN cash_sessions cs ON s.cash_session_id = cs.id
         WHERE s.id = ?",
    )
    .bind(sale_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Error al validar venta: {}", e))?
    .ok_or("Venta no encontrada")?;

    let sale_status: String = sale_row.get("status");
    if sale_status != "DRAFT" {
        return Err("La venta ya fue finalizada".into());
    }

    let session_status: String = sale_row.get("session_status");
    if session_status != "OPEN" {
        return Err("La sesión de caja está cerrada. No se puede finalizar la venta.".into());
    }

    let cash_session_id: i64 = sale_row.get("cash_session_id");
    let created_at: String = sale_row.get("created_at");
    let payment_method: String = sale_row.get("payment_method");

    // Obtener todas las líneas con info de producto y stock
    let line_rows = sqlx::query(
        "SELECT sl.id, sl.sale_id, sl.product_id, sl.qty,
                sl.unit_price_cents, sl.cost_at_sale_cents,
                sl.price_rule_id, pr.name as rule_name,
                sl.rule_required_qty, sl.rule_bundle_price_cents,
                p.name as product_name,
                COALESCE(ib.on_hand, 0) as on_hand
         FROM sale_lines sl
         JOIN products p ON sl.product_id = p.id
         LEFT JOIN price_rules pr ON sl.price_rule_id = pr.id
         LEFT JOIN inventory_balances ib ON p.id = ib.product_id
         WHERE sl.sale_id = ?",
    )
    .bind(sale_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| format!("Error al consultar líneas: {}", e))?;

    if line_rows.is_empty() {
        return Err("La venta no tiene líneas. Agrega al menos un producto.".into());
    }

    // Agregar qty por producto para validar stock
    // HashMap: product_id → (name, total_qty, on_hand)
    let mut product_totals: std::collections::HashMap<i64, (String, i64, i64)> =
        std::collections::HashMap::new();

    for row in &line_rows {
        let pid: i64 = row.get("product_id");
        let pname: String = row.get("product_name");
        let qty: i64 = row.get("qty");
        let on_hand: i64 = row.get("on_hand");

        let entry = product_totals
            .entry(pid)
            .or_insert((pname, 0, on_hand));
        entry.1 += qty;
    }

    // Validar stock por producto (agregado)
    for (_pid, (name, total_qty, on_hand)) in &product_totals {
        if *on_hand < *total_qty {
            return Err(format!(
                "Stock insuficiente para '{}'. Disponible: {}, Total en venta: {}",
                name, on_hand, total_qty
            ));
        }
    }

    // Recalcular totales de línea y acumular total de venta
    let mut total_cents: i64 = 0;
    let mut result_lines: Vec<SaleLineResult> = Vec::with_capacity(line_rows.len());

    for row in &line_rows {
        let lid: i64 = row.get("id");
        let pid: i64 = row.get("product_id");
        let pname: String = row.get("product_name");
        let qty: i64 = row.get("qty");
        let unit_price: i64 = row.get("unit_price_cents");
        let cost: i64 = row.get("cost_at_sale_cents");
        let price_rule_id: Option<i64> = row.get("price_rule_id");
        let rule_name: Option<String> = row.get("rule_name");
        let rule_required_qty: Option<i64> = row.get("rule_required_qty");
        let rule_bundle_price_cents: Option<i64> = row.get("rule_bundle_price_cents");

        let line_total = calc_line_total(qty, unit_price, rule_required_qty, rule_bundle_price_cents)?;
        total_cents += line_total;

        // Recalcular line_total_cents (no confiar en el valor previo)
        sqlx::query("UPDATE sale_lines SET line_total_cents = ? WHERE id = ?")
            .bind(line_total)
            .bind(lid)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Error al recalcular línea: {}", e))?;

        // Movimiento de inventario (auditoría) — uno por línea
        sqlx::query(
            "INSERT INTO inventory_movements
                (product_id, qty_delta, type, ref_table, ref_id, user_id, note)
             VALUES (?, ?, 'SALE', 'sales', ?, ?, ?)",
        )
        .bind(pid)
        .bind(-qty) // negativo = salida
        .bind(sale_id)
        .bind(user_id)
        .bind(format!("Venta #{}", sale_id))
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Error al registrar movimiento: {}", e))?;

        result_lines.push(SaleLineResult {
            id: lid,
            sale_id,
            product_id: pid,
            product_name: pname,
            qty,
            unit_price_cents: unit_price,
            line_total_cents: line_total,
            cost_at_sale_cents: cost,
            price_rule_id,
            rule_name,
            rule_required_qty,
            rule_bundle_price_cents,
        });
    }

    // Descontar inventario por producto (una vez por producto, con qty agregada)
    for (pid, (_name, total_qty, _on_hand)) in &product_totals {
        sqlx::query(
            "UPDATE inventory_balances
             SET on_hand = on_hand - ?, updated_at = datetime('now')
             WHERE product_id = ?",
        )
        .bind(total_qty)
        .bind(pid)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Error al actualizar inventario: {}", e))?;
    }

    // Actualizar venta: total + status
    sqlx::query("UPDATE sales SET total_cents = ?, status = 'FINALIZED' WHERE id = ?")
        .bind(total_cents)
        .bind(sale_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Error al actualizar venta: {}", e))?;

    // ── Commit ───────────────────────────────────────────────────────────
    tx.commit()
        .await
        .map_err(|e| format!("Error al confirmar transacción: {}", e))?;

    Ok(SaleResult {
        id: sale_id,
        cash_session_id,
        created_at,
        user_id,
        total_cents,
        status: "FINALIZED".to_string(),
        payment_method,
        lines: result_lines,
    })
}

// ─── Commands: Consultas ──────────────────────────────────────────────────────

/// Devuelve el detalle completo de una venta (DRAFT o FINALIZED).
#[tauri::command]
pub async fn get_sale_detail(
    db: State<'_, Db>,
    sale_id: i64,
) -> Result<SaleResult, String> {
    let sale_row = sqlx::query(
        "SELECT id, cash_session_id, created_at, user_id,
                total_cents, status, payment_method
         FROM sales WHERE id = ?",
    )
    .bind(sale_id)
    .fetch_optional(db.pool())
    .await
    .map_err(|e| format!("Error al consultar venta: {}", e))?
    .ok_or("Venta no encontrada")?;

    let line_rows = sqlx::query(&format!(
        "{} WHERE sl.sale_id = ? ORDER BY sl.id",
        SALE_LINE_SELECT
    ))
    .bind(sale_id)
    .fetch_all(db.pool())
    .await
    .map_err(|e| format!("Error al consultar líneas: {}", e))?;

    let lines: Vec<SaleLineResult> = line_rows.iter().map(row_to_sale_line).collect();

    Ok(SaleResult {
        id: sale_row.get("id"),
        cash_session_id: sale_row.get("cash_session_id"),
        created_at: sale_row.get("created_at"),
        user_id: sale_row.get("user_id"),
        total_cents: sale_row.get("total_cents"),
        status: sale_row.get("status"),
        payment_method: sale_row.get("payment_method"),
        lines,
    })
}

/// Devuelve las ventas FINALIZADAS de una sesión (resumen + lista).
#[tauri::command]
pub async fn get_session_sales(
    db: State<'_, Db>,
    session_id: i64,
) -> Result<SessionSalesSummary, String> {
    let rows = sqlx::query(
        "SELECT id, created_at, total_cents, payment_method, user_id, status
         FROM sales
         WHERE cash_session_id = ? AND status = 'FINALIZED'
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
            status: r.get("status"),
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
