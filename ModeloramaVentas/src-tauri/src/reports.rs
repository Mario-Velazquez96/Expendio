use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::State;

use crate::db::Db;

#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardDailyPoint {
    pub day: String,
    pub sales_cents: i64,
    pub profit_cents: i64,
    pub cash_difference_cents: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardTopProduct {
    pub product_id: i64,
    pub product_name: String,
    pub qty_sold: i64,
    pub sales_cents: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub range_days: i64,
    pub today_sales_cents: i64,
    pub today_profit_cents: i64,
    pub today_cash_difference_cents: i64,
    pub days: Vec<DashboardDailyPoint>,
    pub top_products: Vec<DashboardTopProduct>,
    pub top_products_today: Vec<DashboardTopProduct>,
}

/// Reportes básicos del MVP:
/// - Ventas por día
/// - Ganancia diaria
/// - Diferencia de caja
#[tauri::command]
pub async fn get_dashboard_summary(
    db: State<'_, Db>,
    days: i64,
) -> Result<DashboardSummary, String> {
    let range_days = days.clamp(1, 90);

    let sales_rows = sqlx::query(
        "SELECT DATE(s.created_at) as day, COALESCE(SUM(s.total_cents), 0) as sales_cents
         FROM sales s
         WHERE s.status = 'FINALIZED'
           AND DATE(s.created_at) >= DATE('now', printf('-%d day', ? - 1))
         GROUP BY DATE(s.created_at)",
    )
    .bind(range_days)
    .fetch_all(db.pool())
    .await
    .map_err(|e| format!("Error al calcular ventas por día: {}", e))?;

    let profit_rows = sqlx::query(
        "SELECT DATE(s.created_at) as day,
                COALESCE(SUM(sl.line_total_cents - (sl.qty * sl.cost_at_sale_cents)), 0) as profit_cents
         FROM sales s
         JOIN sale_lines sl ON sl.sale_id = s.id
         WHERE s.status = 'FINALIZED'
           AND DATE(s.created_at) >= DATE('now', printf('-%d day', ? - 1))
         GROUP BY DATE(s.created_at)",
    )
    .bind(range_days)
    .fetch_all(db.pool())
    .await
    .map_err(|e| format!("Error al calcular ganancia diaria: {}", e))?;

    let cash_diff_rows = sqlx::query(
        "SELECT DATE(cs.closed_at) as day, COALESCE(SUM(cs.difference_cents), 0) as cash_difference_cents
         FROM cash_sessions cs
         WHERE cs.status = 'CLOSED'
           AND cs.closed_at IS NOT NULL
           AND DATE(cs.closed_at) >= DATE('now', printf('-%d day', ? - 1))
         GROUP BY DATE(cs.closed_at)",
    )
    .bind(range_days)
    .fetch_all(db.pool())
    .await
    .map_err(|e| format!("Error al calcular diferencia de caja diaria: {}", e))?;

    let mut sales_by_day: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut profit_by_day: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut cash_diff_by_day: std::collections::HashMap<String, i64> = std::collections::HashMap::new();

    for row in &sales_rows {
        let day: String = row.get("day");
        let value: i64 = row.get("sales_cents");
        sales_by_day.insert(day, value);
    }
    for row in &profit_rows {
        let day: String = row.get("day");
        let value: i64 = row.get("profit_cents");
        profit_by_day.insert(day, value);
    }
    for row in &cash_diff_rows {
        let day: String = row.get("day");
        let value: i64 = row.get("cash_difference_cents");
        cash_diff_by_day.insert(day, value);
    }

    let day_rows = sqlx::query(
        "WITH RECURSIVE last_days(day) AS (
           SELECT DATE('now', printf('-%d day', ? - 1))
           UNION ALL
           SELECT DATE(day, '+1 day')
           FROM last_days
           WHERE day < DATE('now')
         )
         SELECT day FROM last_days ORDER BY day ASC",
    )
    .bind(range_days)
    .fetch_all(db.pool())
    .await
    .map_err(|e| format!("Error al construir rango de fechas: {}", e))?;

    let mut points = Vec::with_capacity(day_rows.len());
    for row in day_rows {
        let day: String = row.get("day");
        points.push(DashboardDailyPoint {
            sales_cents: *sales_by_day.get(&day).unwrap_or(&0),
            profit_cents: *profit_by_day.get(&day).unwrap_or(&0),
            cash_difference_cents: *cash_diff_by_day.get(&day).unwrap_or(&0),
            day,
        });
    }

    let today_day = sqlx::query_scalar::<_, String>("SELECT DATE('now')")
        .fetch_one(db.pool())
        .await
        .map_err(|e| format!("Error al resolver fecha actual: {}", e))?;

    let today_sales = *sales_by_day.get(&today_day).unwrap_or(&0);
    let today_profit = *profit_by_day.get(&today_day).unwrap_or(&0);
    let today_cash_diff = *cash_diff_by_day.get(&today_day).unwrap_or(&0);

    let top_product_rows = sqlx::query(
        "SELECT sl.product_id,
                p.name as product_name,
                COALESCE(SUM(sl.qty), 0) as qty_sold,
                COALESCE(SUM(sl.line_total_cents), 0) as sales_cents
         FROM sale_lines sl
         JOIN sales s ON s.id = sl.sale_id
         JOIN products p ON p.id = sl.product_id
         WHERE s.status = 'FINALIZED'
           AND DATE(s.created_at) >= DATE('now', printf('-%d day', ? - 1))
         GROUP BY sl.product_id, p.name
         ORDER BY qty_sold DESC, sales_cents DESC
         LIMIT 10",
    )
    .bind(range_days)
    .fetch_all(db.pool())
    .await
    .map_err(|e| format!("Error al calcular top productos: {}", e))?;

    let top_products: Vec<DashboardTopProduct> = top_product_rows
        .iter()
        .map(|r| DashboardTopProduct {
            product_id: r.get("product_id"),
            product_name: r.get("product_name"),
            qty_sold: r.get("qty_sold"),
            sales_cents: r.get("sales_cents"),
        })
        .collect();

    let top_today_rows = sqlx::query(
        "SELECT sl.product_id,
                p.name as product_name,
                COALESCE(SUM(sl.qty), 0) as qty_sold,
                COALESCE(SUM(sl.line_total_cents), 0) as sales_cents
         FROM sale_lines sl
         JOIN sales s ON s.id = sl.sale_id
         JOIN products p ON p.id = sl.product_id
         WHERE s.status = 'FINALIZED'
           AND DATE(s.created_at) = DATE('now')
         GROUP BY sl.product_id, p.name
         ORDER BY qty_sold DESC, sales_cents DESC
         LIMIT 10",
    )
    .fetch_all(db.pool())
    .await
    .map_err(|e| format!("Error al calcular top productos de hoy: {}", e))?;

    let top_products_today: Vec<DashboardTopProduct> = top_today_rows
        .iter()
        .map(|r| DashboardTopProduct {
            product_id: r.get("product_id"),
            product_name: r.get("product_name"),
            qty_sold: r.get("qty_sold"),
            sales_cents: r.get("sales_cents"),
        })
        .collect();

    Ok(DashboardSummary {
        range_days,
        today_sales_cents: today_sales,
        today_profit_cents: today_profit,
        today_cash_difference_cents: today_cash_diff,
        days: points,
        top_products,
        top_products_today,
    })
}
