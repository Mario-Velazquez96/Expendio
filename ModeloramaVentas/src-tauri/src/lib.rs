mod auth;
mod cash;
mod db;
mod inventory;
mod sales;

use db::{init_db, Db};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::{AppHandle, Manager, State};

#[derive(Debug, Serialize, Deserialize)]
pub struct TableInfo {
    name: String,
    sql: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DbTestResult {
    success: bool,
    message: String,
    details: Option<serde_json::Value>,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Obtiene la ruta donde está almacenada la base de datos
#[tauri::command]
fn get_db_path(app: AppHandle) -> Result<String, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| "No se pudo obtener el directorio de datos".to_string())?;
    
    let db_path = data_dir.join("beerpos").join("app.db");
    Ok(db_path.to_string_lossy().to_string())
}

/// Prueba la conexión a la base de datos
#[tauri::command]
async fn test_db_connection(db: State<'_, Db>) -> Result<DbTestResult, String> {
    match sqlx::query("SELECT 1 as test")
        .fetch_one(db.pool())
        .await
    {
        Ok(_) => Ok(DbTestResult {
            success: true,
            message: "✅ Conexión exitosa a la base de datos".to_string(),
            details: None,
        }),
        Err(e) => Ok(DbTestResult {
            success: false,
            message: format!("❌ Error de conexión: {}", e),
            details: None,
        }),
    }
}

/// Lista todas las tablas creadas en la base de datos
#[tauri::command]
async fn test_db_tables(db: State<'_, Db>) -> Result<DbTestResult, String> {
    let query = "
        SELECT name, sql 
        FROM sqlite_master 
        WHERE type='table' 
        AND name NOT LIKE 'sqlite_%'
        ORDER BY name;
    ";

    match sqlx::query(query).fetch_all(db.pool()).await {
        Ok(rows) => {
            let tables: Vec<TableInfo> = rows
                .iter()
                .map(|row| TableInfo {
                    name: row.get::<String, _>(0),
                    sql: row.get::<Option<String>, _>(1).unwrap_or_default(),
                })
                .collect();

            Ok(DbTestResult {
                success: true,
                message: format!("✅ Se encontraron {} tablas", tables.len()),
                details: Some(serde_json::json!({
                    "count": tables.len(),
                    "tables": tables.iter().map(|t| &t.name).collect::<Vec<_>>()
                })),
            })
        }
        Err(e) => Ok(DbTestResult {
            success: false,
            message: format!("❌ Error al listar tablas: {}", e),
            details: None,
        }),
    }
}

/// Prueba una consulta simple contando registros en cada tabla
#[tauri::command]
async fn test_db_query(db: State<'_, Db>) -> Result<DbTestResult, String> {
    let query = "
        SELECT name 
        FROM sqlite_master 
        WHERE type='table' 
        AND name NOT LIKE 'sqlite_%';
    ";

    match sqlx::query(query).fetch_all(db.pool()).await {
        Ok(rows) => {
            let mut results = serde_json::Map::new();
            
            for row in rows {
                let table_name: String = row.get(0);
                let count_query = format!("SELECT COUNT(*) FROM {}", table_name);
                
                match sqlx::query_scalar::<_, i64>(&count_query)
                    .fetch_one(db.pool())
                    .await
                {
                    Ok(count) => {
                        results.insert(table_name.clone(), serde_json::json!(count));
                    }
                    Err(e) => {
                        results.insert(table_name.clone(), serde_json::json!(format!("Error: {}", e)));
                    }
                }
            }

            Ok(DbTestResult {
                success: true,
                message: "✅ Consultas ejecutadas correctamente".to_string(),
                details: Some(serde_json::Value::Object(results)),
            })
        }
        Err(e) => Ok(DbTestResult {
            success: false,
            message: format!("❌ Error en la consulta: {}", e),
            details: None,
        }),
    }
}


/// Función principal para ejecutar la aplicación
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Inicializar la base de datos de forma síncrona (bloquea hasta terminar)
            let handle = app.handle().clone();
            tauri::async_runtime::block_on(async move {
                match init_db(&handle).await {
                    Ok(db) => {
                        if let Err(e) = auth::seed_users(&db).await {
                            eprintln!("Error al crear usuarios iniciales: {}", e);
                        }
                        handle.manage(db);
                    }
                    Err(e) => {
                        eprintln!("Error fatal: No se pudo inicializar la base de datos: {}", e);
                        std::process::exit(1);
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_db_path,
            test_db_connection,
            test_db_tables,
            test_db_query,
            // ── Auth ──
            auth::login_with_pin,
            auth::get_user_by_id,
            // ── Caja ──
            cash::open_cash_session,
            cash::get_current_cash_session,
            cash::add_cash_movement,
            cash::get_cash_movements,
            cash::get_cash_session_summary,
            cash::close_cash_session,
            // ── POS / Ventas ──
            sales::search_products,
            sales::get_product_by_barcode,
            sales::list_products,
            sales::create_sale,
            sales::add_sale_line,
            sales::update_sale_line_qty,
            sales::remove_sale_line,
            sales::finalize_sale,
            sales::get_sale_detail,
            sales::get_session_sales,
            // ── Inventario (admin) ──
            inventory::list_all_products_admin,
            inventory::create_product,
            inventory::add_stock,
            inventory::update_product,
            inventory::toggle_product_active,
            inventory::adjust_stock,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
