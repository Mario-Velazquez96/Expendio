-- BeerPOS - Reset de datos operativos para salida a produccion
--
-- Conserva:
--   - users
--   - products
--   - price_rules
--   - inventory_balances (filas), pero deja on_hand = 0
--
-- Elimina:
--   - sale_lines
--   - sales
--   - cash_movements
--   - cash_sessions
--   - inventory_movements
--
-- Uso recomendado:
--   1) Cerrar la app
--   2) Respaldar app.db (+ app.db-wal / app.db-shm si existen)
--   3) Ejecutar este script sobre la base de datos objetivo

PRAGMA foreign_keys = ON;

-- 1) Limpiar historial de ventas
DELETE FROM sale_lines;
DELETE FROM sales;

-- 2) Limpiar historial de caja
DELETE FROM cash_movements;
DELETE FROM cash_sessions;

-- 3) Limpiar historial de inventario
DELETE FROM inventory_movements;

-- 4) Dejar stock en cero (manteniendo filas por producto)
UPDATE inventory_balances
SET on_hand = 0,
    updated_at = datetime('now');

-- Verificacion rapida (opcional):
-- SELECT COUNT(*) FROM sales;
-- SELECT COUNT(*) FROM cash_sessions;
-- SELECT COUNT(*) FROM inventory_movements;
-- SELECT COUNT(*) FROM products;
-- SELECT COUNT(*) FROM users;
