// ─── Tipos compartidos del proyecto BeerPOS ──────────────────────────────────

// ── Auth ─────────────────────────────────────────────────────────────────────

export interface User {
  id: number;
  name: string;
  role: 'OWNER' | 'EMPLOYEE';
}

export interface AuthState {
  user: User | null;
  isAuthenticated: boolean;
}

export interface LoginResult {
  success: boolean;
  user: User | null;
  error: string | null;
}

// ── Cash ─────────────────────────────────────────────────────────────────────

export interface CashSession {
  id: number;
  status: string;
  opened_at: string;
  closed_at: string | null;
  opened_by_user_id: number;
  closed_by_user_id: number | null;
  opening_float_cents: number;
  expected_cash_cents: number | null;
  counted_cash_cents: number | null;
  difference_cents: number | null;
  notes_open: string | null;
  notes_close: string | null;
}

export interface CashMovement {
  id: number;
  cash_session_id: number;
  type: string;
  amount_cents: number;
  reason: string;
  created_at: string;
  user_id: number;
}

export interface CashSessionSummary {
  session: CashSession;
  total_sales_cash_cents: number;
  total_sales_count: number;
  total_deposits_cents: number;
  total_withdrawals_cents: number;
  expected_cash_cents: number;
  movements: CashMovement[];
}

// ── Products ─────────────────────────────────────────────────────────────────

export interface Product {
  id: number;
  name: string;
  category: string;
  barcode: string | null;
  units_per_case: number;
  cost_cents: number;
  unit_price_cents: number;
  stock_min: number;
  active: number;
  on_hand: number;
}

// ── Sales ────────────────────────────────────────────────────────────────────

export interface CreateSaleResult {
  sale_id: number;
  cash_session_id: number;
  user_id: number;
}

export interface SaleLineResult {
  id: number;
  sale_id: number;
  product_id: number;
  product_name: string;
  qty: number;
  unit_price_cents: number;
  line_total_cents: number;
  cost_at_sale_cents: number;
  price_rule_id: number | null;
  rule_name: string | null;
  rule_required_qty: number | null;
  rule_bundle_price_cents: number | null;
}

export interface SaleResult {
  id: number;
  cash_session_id: number;
  created_at: string;
  user_id: number;
  total_cents: number;
  status: string;
  payment_method: string;
  lines: SaleLineResult[];
}

export interface SessionSalesSummary {
  sales: SaleBrief[];
  count: number;
  total_cents: number;
}

export interface PriceRule {
  id: number;
  product_id: number;
  name: string;
  required_qty: number;
  bundle_price_cents: number;
  start_at: string | null;
  end_at: string | null;
  enabled: number;
  priority: number;
}

export interface SaleBrief {
  id: number;
  created_at: string;
  total_cents: number;
  payment_method: string;
  user_id: number;
  status: string;
}

// ── Inventory (admin) ────────────────────────────────────────────────────────

export interface ProductDetail {
  id: number;
  name: string;
  category: string;
  barcode: string | null;
  units_per_case: number;
  cost_cents: number;
  unit_price_cents: number;
  stock_min: number;
  active: number;
  on_hand: number;
}

export interface StockResult {
  product_id: number;
  product_name: string;
  cases_added: number;
  units_added: number;
  new_on_hand: number;
}

export interface AdjustResult {
  product_id: number;
  product_name: string;
  units_removed: number;
  new_on_hand: number;
}
