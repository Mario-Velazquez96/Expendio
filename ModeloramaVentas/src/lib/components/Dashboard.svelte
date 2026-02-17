<script lang="ts">
  import { onMount } from 'svelte';
  import { getDashboardSummary } from '$lib/api/reports';
  import { formatMoney } from '$lib/helpers/money';
  import type { DashboardSummary } from '$lib/types';

  let summary = $state<DashboardSummary | null>(null);
  let loading = $state(true);
  let error = $state('');
  let rangeDays = $state(7);
  let topProductsScope = $state<'range' | 'today'>('range');

  onMount(async () => {
    await loadDashboard();
  });

  async function loadDashboard() {
    loading = true;
    error = '';
    try {
      summary = await getDashboardSummary(rangeDays);
    } catch (e) {
      error = `Error al cargar dashboard: ${e}`;
    } finally {
      loading = false;
    }
  }

  async function changeRange(days: number) {
    rangeDays = days;
    await loadDashboard();
  }

  function maxValue(values: number[]): number {
    if (values.length === 0) return 1;
    return Math.max(...values, 1);
  }

  function maxAbsValue(values: number[]): number {
    if (values.length === 0) return 1;
    return Math.max(...values.map(v => Math.abs(v)), 1);
  }

  let salesTop = $derived(summary ? maxValue(summary.days.map(d => d.sales_cents)) : 1);
  let profitTop = $derived(summary ? maxValue(summary.days.map(d => d.profit_cents)) : 1);
  let cashDiffTop = $derived(summary ? maxAbsValue(summary.days.map(d => d.cash_difference_cents)) : 1);
  let activeTopProducts = $derived(
    summary
      ? (topProductsScope === 'today' ? summary.top_products_today : summary.top_products)
      : []
  );
  let topProductsMaxQty = $derived(maxValue(activeTopProducts.map(p => p.qty_sold)));
</script>

<div class="dashboard-layout">
  <div class="dash-header">
    <h2>📊 Dashboard</h2>
    <div class="range-pills">
      <button class="pill" class:active={rangeDays === 7} onclick={() => changeRange(7)}>7 días</button>
      <button class="pill" class:active={rangeDays === 14} onclick={() => changeRange(14)}>14 días</button>
      <button class="pill" class:active={rangeDays === 30} onclick={() => changeRange(30)}>30 días</button>
    </div>
  </div>

  {#if loading}
    <div class="loading-center">
      <div class="spinner"></div>
      <p>Cargando métricas...</p>
    </div>
  {:else if error}
    <div class="error-banner">⚠️ {error}</div>
  {:else if summary}
    <div class="kpis">
      <div class="kpi-card">
        <span class="kpi-label">Ventas de hoy</span>
        <span class="kpi-value">{formatMoney(summary.today_sales_cents)}</span>
      </div>
      <div class="kpi-card">
        <span class="kpi-label">Ganancia de hoy</span>
        <span class="kpi-value positive">{formatMoney(summary.today_profit_cents)}</span>
      </div>
      <div class="kpi-card">
        <span class="kpi-label">Diferencia caja hoy</span>
        <span class="kpi-value" class:positive={summary.today_cash_difference_cents > 0} class:negative={summary.today_cash_difference_cents < 0}>
          {formatMoney(summary.today_cash_difference_cents)}
        </span>
      </div>
    </div>

    <div class="mini-charts">
      <div class="chart-card">
        <h3>Ventas diarias</h3>
        <div class="sparkline">
          {#each summary.days as row (row.day)}
            <div class="bar-wrap" title={`${row.day}: ${formatMoney(row.sales_cents)}`}>
              <div class="bar sales" style={`height:${Math.max((row.sales_cents / salesTop) * 100, 2)}%`}></div>
            </div>
          {/each}
        </div>
      </div>

      <div class="chart-card">
        <h3>Ganancia diaria</h3>
        <div class="sparkline">
          {#each summary.days as row (row.day)}
            <div class="bar-wrap" title={`${row.day}: ${formatMoney(row.profit_cents)}`}>
              <div class="bar profit" style={`height:${Math.max((row.profit_cents / profitTop) * 100, 2)}%`}></div>
            </div>
          {/each}
        </div>
      </div>

      <div class="chart-card">
        <h3>Diferencia de caja</h3>
        <div class="sparkline sparkline-mid">
          {#each summary.days as row (row.day)}
            <div class="bar-wrap mid" title={`${row.day}: ${formatMoney(row.cash_difference_cents)}`}>
              {#if row.cash_difference_cents >= 0}
                <div
                  class="bar cash-pos"
                  style={`height:${Math.max((Math.abs(row.cash_difference_cents) / cashDiffTop) * 50, row.cash_difference_cents === 0 ? 0 : 2)}%; bottom:50%;`}
                ></div>
              {:else}
                <div
                  class="bar cash-neg"
                  style={`height:${Math.max((Math.abs(row.cash_difference_cents) / cashDiffTop) * 50, 2)}%; top:50%;`}
                ></div>
              {/if}
            </div>
          {/each}
          <div class="mid-line"></div>
        </div>
      </div>
    </div>

    <div class="table-card">
      <div class="top-products-header">
        <h3>Productos más vendidos</h3>
        <div class="top-scope-pills">
          <button class="pill" class:active={topProductsScope === 'range'} onclick={() => topProductsScope = 'range'}>
            Rango ({summary.range_days}d)
          </button>
          <button class="pill" class:active={topProductsScope === 'today'} onclick={() => topProductsScope = 'today'}>
            Solo hoy
          </button>
        </div>
      </div>
      {#if activeTopProducts.length === 0}
        <p class="empty-note">
          {topProductsScope === 'today' ? 'Hoy no hay ventas de productos.' : 'Sin ventas en el rango seleccionado.'}
        </p>
      {:else}
        <div class="top-products">
          {#each activeTopProducts as p (p.product_id)}
            <div class="top-product-row" title={`${p.product_name}: ${p.qty_sold} pzas · ${formatMoney(p.sales_cents)}`}>
              <div class="product-meta">
                <span class="product-name">{p.product_name}</span>
                <span class="product-sub">{p.qty_sold} pzas · {formatMoney(p.sales_cents)}</span>
              </div>
              <div class="product-bar-track">
                <div
                  class="product-bar-fill"
                  style={`width:${Math.max((p.qty_sold / topProductsMaxQty) * 100, 4)}%`}
                ></div>
              </div>
              <span class="product-qty">{p.qty_sold}</span>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <div class="table-card">
      <h3>Detalle diario ({summary.range_days} días)</h3>
      <table>
        <thead>
          <tr>
            <th>Día</th>
            <th class="right">Ventas</th>
            <th class="right">Ganancia</th>
            <th class="right">Diferencia caja</th>
          </tr>
        </thead>
        <tbody>
          {#each summary.days as row (row.day)}
            <tr>
              <td>{row.day}</td>
              <td class="right mono">{formatMoney(row.sales_cents)}</td>
              <td class="right mono positive">{formatMoney(row.profit_cents)}</td>
              <td class="right mono" class:positive={row.cash_difference_cents > 0} class:negative={row.cash_difference_cents < 0}>
                {formatMoney(row.cash_difference_cents)}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<style>
  .dashboard-layout {
    height: 100%;
    overflow-y: auto;
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .dash-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .dash-header h2 {
    margin: 0;
    color: #fff;
    font-size: 1.2rem;
  }

  .range-pills {
    display: flex;
    gap: 0.35rem;
    background: rgba(255,255,255,0.04);
    border-radius: 8px;
    padding: 0.2rem;
  }

  .pill {
    border: none;
    background: transparent;
    color: rgba(255,255,255,0.5);
    padding: 0.4rem 0.75rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.82rem;
  }

  .pill.active {
    background: rgba(245,158,11,0.2);
    color: #f59e0b;
    font-weight: 600;
  }

  .kpis {
    display: grid;
    grid-template-columns: repeat(3, minmax(180px, 1fr));
    gap: 0.75rem;
  }

  .kpi-card {
    background: rgba(255,255,255,0.04);
    border: 1px solid rgba(255,255,255,0.08);
    border-radius: 14px;
    padding: 0.95rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .kpi-label {
    color: rgba(255,255,255,0.5);
    font-size: 0.8rem;
  }

  .kpi-value {
    color: #fff;
    font-size: 1.45rem;
    font-weight: 700;
  }

  .positive { color: #22c55e; }
  .negative { color: #ef4444; }

  .table-card {
    background: rgba(255,255,255,0.04);
    border: 1px solid rgba(255,255,255,0.08);
    border-radius: 14px;
    padding: 1rem;
  }

  .mini-charts {
    display: grid;
    grid-template-columns: repeat(3, minmax(220px, 1fr));
    gap: 0.75rem;
  }

  .chart-card {
    background: rgba(255,255,255,0.04);
    border: 1px solid rgba(255,255,255,0.08);
    border-radius: 14px;
    padding: 0.85rem 0.9rem;
  }

  .chart-card h3 {
    margin: 0 0 0.65rem;
    color: rgba(255,255,255,0.78);
    font-size: 0.83rem;
    font-weight: 600;
  }

  .sparkline {
    height: 84px;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(4px, 1fr));
    gap: 3px;
    align-items: end;
    position: relative;
  }

  .sparkline-mid {
    align-items: stretch;
  }

  .mid-line {
    position: absolute;
    left: 0;
    right: 0;
    top: 50%;
    height: 1px;
    background: rgba(255,255,255,0.16);
    pointer-events: none;
  }

  .bar-wrap {
    height: 100%;
    display: flex;
    align-items: end;
    justify-content: center;
    position: relative;
  }

  .bar-wrap.mid {
    align-items: stretch;
  }

  .bar {
    width: 100%;
    border-radius: 3px 3px 0 0;
    min-height: 0;
    position: absolute;
    left: 0;
    right: 0;
  }

  .bar.sales {
    background: rgba(245,158,11,0.85);
  }

  .bar.profit {
    background: rgba(34,197,94,0.85);
  }

  .bar.cash-pos {
    background: rgba(59,130,246,0.85);
    border-radius: 3px 3px 0 0;
  }

  .bar.cash-neg {
    background: rgba(239,68,68,0.85);
    border-radius: 0 0 3px 3px;
  }

  .table-card h3 {
    margin: 0 0 0.75rem;
    color: #fff;
    font-size: 0.95rem;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th, td {
    border-bottom: 1px solid rgba(255,255,255,0.06);
    padding: 0.6rem;
    font-size: 0.88rem;
  }

  th {
    text-align: left;
    color: rgba(255,255,255,0.45);
    text-transform: uppercase;
    letter-spacing: 0.4px;
    font-size: 0.72rem;
  }

  .right { text-align: right; }
  .mono { font-family: 'SF Mono', 'Cascadia Code', monospace; }

  .loading-center {
    text-align: center;
    color: rgba(255,255,255,0.5);
    padding: 3rem 1rem;
  }

  .spinner {
    width: 30px;
    height: 30px;
    border: 3px solid rgba(255,255,255,0.1);
    border-top-color: #f59e0b;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin: 0 auto 0.8rem;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .error-banner {
    padding: 0.75rem 1rem;
    background: rgba(239,68,68,0.12);
    border: 1px solid rgba(239,68,68,0.25);
    border-radius: 8px;
    color: #ef4444;
    font-size: 0.9rem;
  }

  .top-products {
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
  }

  .top-products-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
    margin-bottom: 0.75rem;
  }

  .top-scope-pills {
    display: flex;
    gap: 0.3rem;
    background: rgba(255,255,255,0.04);
    border-radius: 8px;
    padding: 0.2rem;
  }

  .top-product-row {
    display: grid;
    grid-template-columns: minmax(220px, 1fr) minmax(120px, 2fr) 42px;
    align-items: center;
    gap: 0.6rem;
  }

  .product-meta {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }

  .product-name {
    color: #fff;
    font-size: 0.85rem;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .product-sub {
    color: rgba(255,255,255,0.48);
    font-size: 0.74rem;
  }

  .product-bar-track {
    height: 10px;
    border-radius: 999px;
    background: rgba(255,255,255,0.08);
    overflow: hidden;
  }

  .product-bar-fill {
    height: 100%;
    border-radius: 999px;
    background: linear-gradient(90deg, #f59e0b 0%, #fb7185 100%);
  }

  .product-qty {
    text-align: right;
    font-family: 'SF Mono', 'Cascadia Code', monospace;
    font-size: 0.8rem;
    color: rgba(255,255,255,0.8);
  }

  .empty-note {
    margin: 0;
    color: rgba(255,255,255,0.48);
    font-size: 0.85rem;
  }
</style>
