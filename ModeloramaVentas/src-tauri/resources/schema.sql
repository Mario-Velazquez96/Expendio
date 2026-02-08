-- Usuarios (PIN)
CREATE TABLE IF NOT EXISTS users (
  id              INTEGER PRIMARY KEY,
  name            TEXT NOT NULL,
  pin_hash        TEXT NOT NULL,
  role            TEXT NOT NULL CHECK(role IN ('OWNER','EMPLOYEE')),
  active          INTEGER NOT NULL DEFAULT 1,
  created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Productos
CREATE TABLE IF NOT EXISTS products (
  id              INTEGER PRIMARY KEY,
  name            TEXT NOT NULL,
  category        TEXT NOT NULL CHECK(category IN ('BEER','PRODUCT')),
  barcode         TEXT,
  units_per_case  INTEGER NOT NULL DEFAULT 0, -- BEER: 12 o 24; PRODUCT: 0 o 1
  cost_cents      INTEGER NOT NULL DEFAULT 0, -- costo por pieza (promedio actual o último)
  unit_price_cents INTEGER NOT NULL DEFAULT 0,
  stock_min       INTEGER NOT NULL DEFAULT 0,
  active          INTEGER NOT NULL DEFAULT 1
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_products_barcode
ON products(barcode) WHERE barcode IS NOT NULL AND barcode <> '';

-- Stock actual
CREATE TABLE IF NOT EXISTS inventory_balances (
  product_id      INTEGER PRIMARY KEY,
  on_hand         INTEGER NOT NULL DEFAULT 0,
  updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
  FOREIGN KEY(product_id) REFERENCES products(id)
);

-- Sesión de caja (apertura/cierre)
CREATE TABLE IF NOT EXISTS cash_sessions (
  id                  INTEGER PRIMARY KEY,
  status              TEXT NOT NULL CHECK(status IN ('OPEN','CLOSED')),
  opened_at           TEXT NOT NULL DEFAULT (datetime('now')),
  closed_at           TEXT,
  opened_by_user_id   INTEGER NOT NULL,
  closed_by_user_id   INTEGER,
  opening_float_cents INTEGER NOT NULL, -- fondo inicial
  expected_cash_cents INTEGER,          -- calculado al cerrar
  counted_cash_cents  INTEGER,          -- capturado al cerrar
  difference_cents    INTEGER,          -- counted - expected
  notes_open          TEXT,
  notes_close         TEXT,
  owner_auth_user_id  INTEGER,          -- si requieres autorización de dueño
  FOREIGN KEY(opened_by_user_id) REFERENCES users(id),
  FOREIGN KEY(closed_by_user_id) REFERENCES users(id),
  FOREIGN KEY(owner_auth_user_id) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_cash_sessions_status ON cash_sessions(status);

-- Movimientos manuales de caja (retiros/depósitos)
CREATE TABLE IF NOT EXISTS cash_movements (
  id              INTEGER PRIMARY KEY,
  cash_session_id INTEGER NOT NULL,
  type            TEXT NOT NULL CHECK(type IN ('DEPOSIT','WITHDRAWAL')),
  amount_cents    INTEGER NOT NULL CHECK(amount_cents > 0),
  reason          TEXT NOT NULL,
  created_at      TEXT NOT NULL DEFAULT (datetime('now')),
  user_id         INTEGER NOT NULL,
  FOREIGN KEY(cash_session_id) REFERENCES cash_sessions(id),
  FOREIGN KEY(user_id) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_cash_movements_session ON cash_movements(cash_session_id);

-- Ventas (efectivo, transferencia, externo)
CREATE TABLE IF NOT EXISTS sales (
  id              INTEGER PRIMARY KEY,
  cash_session_id INTEGER NOT NULL,
  created_at      TEXT NOT NULL DEFAULT (datetime('now')),
  user_id         INTEGER NOT NULL,
  total_cents     INTEGER NOT NULL CHECK(total_cents >= 0),
  status          TEXT NOT NULL DEFAULT 'FINALIZED' CHECK(status IN ('DRAFT','FINALIZED')),
  payment_method  TEXT NOT NULL DEFAULT 'CASH' CHECK(payment_method IN ('CASH','TRANSFER','EXTERNAL')),
  FOREIGN KEY(cash_session_id) REFERENCES cash_sessions(id),
  FOREIGN KEY(user_id) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_sales_session ON sales(cash_session_id);

-- Líneas de venta
CREATE TABLE IF NOT EXISTS sale_lines (
  id                  INTEGER PRIMARY KEY,
  sale_id             INTEGER NOT NULL,
  product_id          INTEGER NOT NULL,
  qty                 INTEGER NOT NULL CHECK(qty > 0),
  unit_price_cents    INTEGER NOT NULL CHECK(unit_price_cents >= 0),
  line_total_cents    INTEGER NOT NULL CHECK(line_total_cents >= 0),
  cost_at_sale_cents  INTEGER NOT NULL CHECK(cost_at_sale_cents >= 0),
  price_rule_id       INTEGER,
  rule_required_qty   INTEGER,
  rule_bundle_price_cents INTEGER,
  FOREIGN KEY(sale_id) REFERENCES sales(id),
  FOREIGN KEY(product_id) REFERENCES products(id)
);

CREATE INDEX IF NOT EXISTS idx_sale_lines_sale ON sale_lines(sale_id);

-- Reglas de promo por producto exacto
CREATE TABLE IF NOT EXISTS price_rules (
  id              INTEGER PRIMARY KEY,
  product_id      INTEGER NOT NULL,
  name            TEXT NOT NULL,
  required_qty    INTEGER NOT NULL CHECK(required_qty > 1),
  bundle_price_cents INTEGER NOT NULL CHECK(bundle_price_cents >= 0),
  start_at        TEXT,
  end_at          TEXT,
  enabled         INTEGER NOT NULL DEFAULT 1,
  priority        INTEGER NOT NULL DEFAULT 0,
  FOREIGN KEY(product_id) REFERENCES products(id)
);

CREATE INDEX IF NOT EXISTS idx_price_rules_product ON price_rules(product_id);

-- Movimientos de inventario (auditoría)
CREATE TABLE IF NOT EXISTS inventory_movements (
  id              INTEGER PRIMARY KEY,
  product_id      INTEGER NOT NULL,
  qty_delta       INTEGER NOT NULL, -- + entrada, - salida
  type            TEXT NOT NULL CHECK(type IN ('PURCHASE','SALE','ADJUSTMENT','INITIAL')),
  ref_table       TEXT, -- 'sales', 'purchases', etc. (opcional)
  ref_id          INTEGER, -- id relacionado (opcional)
  created_at      TEXT NOT NULL DEFAULT (datetime('now')),
  user_id         INTEGER NOT NULL,
  note            TEXT,
  FOREIGN KEY(product_id) REFERENCES products(id),
  FOREIGN KEY(user_id) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_inventory_movements_product ON inventory_movements(product_id);
