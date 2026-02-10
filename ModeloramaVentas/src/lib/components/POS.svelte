<script lang="ts">
  import { onMount } from 'svelte';
  import { authStore } from '$lib/stores/auth.svelte';
  import {
    listProducts,
    createSale,
    addSaleLine,
    updateSaleLineQty,
    removeSaleLine,
    finalizeSale
  } from '$lib/api/sales';
  import { formatMoney } from '$lib/helpers/money';
  import type { CashSession, Product, SaleLineResult } from '$lib/types';

  let { session }: { session: CashSession } = $props();

  // ── Estado de productos ──
  let allProducts = $state<Product[]>([]);
  let productsLoading = $state(true);
  let searchQuery = $state('');

  // ── Estado de la venta actual ──
  let currentSaleId = $state<number | null>(null);
  let saleLines = $state<SaleLineResult[]>([]);
  let operationLoading = $state(false);
  let error = $state('');

  // ── Retroalimentación de venta completada ──
  let lastSaleTotal = $state<number | null>(null);
  let lastSaleTimeout: ReturnType<typeof setTimeout> | null = null;

  // ── Computed ──
  let total = $derived(saleLines.reduce((sum, l) => sum + l.line_total_cents, 0));

  let filteredProducts = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return allProducts;
    return allProducts.filter(p =>
      p.name.toLowerCase().includes(q) ||
      (p.barcode && p.barcode.includes(searchQuery.trim()))
    );
  });

  // ── Lifecycle ──
  onMount(async () => {
    await loadProducts();
  });

  // ── Funciones ──
  async function loadProducts() {
    productsLoading = true;
    try {
      allProducts = await listProducts();
    } catch (e) {
      error = `Error al cargar productos: ${e}`;
    } finally {
      productsLoading = false;
    }
  }

  async function handleAddProduct(product: Product) {
    if (operationLoading) return;
    if (product.on_hand <= 0) {
      error = `Sin stock disponible para "${product.name}"`;
      return;
    }

    operationLoading = true;
    error = '';

    try {
      // Auto-crear venta si no hay una activa
      if (currentSaleId === null) {
        const result = await createSale(authStore.pin);
        currentSaleId = result.sale_id;
      }

      // Agregar línea (el backend hace upsert si el producto ya existe)
      const line = await addSaleLine(authStore.pin, currentSaleId, product.id, 1);

      // Actualizar estado local
      const idx = saleLines.findIndex(l => l.id === line.id);
      if (idx >= 0) {
        saleLines[idx] = line;
      } else {
        saleLines.push(line);
      }
    } catch (e) {
      error = String(e);
    } finally {
      operationLoading = false;
    }
  }

  async function handleIncrement(line: SaleLineResult) {
    if (operationLoading) return;
    operationLoading = true;
    error = '';

    try {
      const updated = await updateSaleLineQty(authStore.pin, line.id, line.qty + 1);
      const idx = saleLines.findIndex(l => l.id === line.id);
      if (idx >= 0) saleLines[idx] = updated;
    } catch (e) {
      error = String(e);
    } finally {
      operationLoading = false;
    }
  }

  async function handleDecrement(line: SaleLineResult) {
    if (operationLoading) return;
    operationLoading = true;
    error = '';

    try {
      if (line.qty <= 1) {
        // Eliminar línea
        await removeSaleLine(authStore.pin, line.id);
        saleLines = saleLines.filter(l => l.id !== line.id);
      } else {
        const updated = await updateSaleLineQty(authStore.pin, line.id, line.qty - 1);
        const idx = saleLines.findIndex(l => l.id === line.id);
        if (idx >= 0) saleLines[idx] = updated;
      }
    } catch (e) {
      error = String(e);
    } finally {
      operationLoading = false;
    }
  }

  async function handleRemoveLine(line: SaleLineResult) {
    if (operationLoading) return;
    operationLoading = true;
    error = '';

    try {
      await removeSaleLine(authStore.pin, line.id);
      saleLines = saleLines.filter(l => l.id !== line.id);
    } catch (e) {
      error = String(e);
    } finally {
      operationLoading = false;
    }
  }

  async function handleFinalize() {
    if (operationLoading || !currentSaleId || saleLines.length === 0) return;
    operationLoading = true;
    error = '';

    try {
      const result = await finalizeSale(authStore.pin, currentSaleId);

      // Mostrar confirmación
      lastSaleTotal = result.total_cents;
      if (lastSaleTimeout) clearTimeout(lastSaleTimeout);
      lastSaleTimeout = setTimeout(() => {
        lastSaleTotal = null;
      }, 3000);

      // Resetear para nueva venta
      currentSaleId = null;
      saleLines = [];

      // Recargar productos para stock actualizado
      await loadProducts();
    } catch (e) {
      error = String(e);
    } finally {
      operationLoading = false;
    }
  }

  function handleCancel() {
    // Abandonar venta DRAFT (queda en BD pero no afecta nada)
    currentSaleId = null;
    saleLines = [];
    error = '';
  }
</script>

<div class="pos-layout">
  <!-- ═══ Panel izquierdo: Productos ═══ -->
  <div class="products-panel">
    <div class="search-section">
      <div class="search-box">
        <span class="search-icon">🔍</span>
        <input
          type="text"
          placeholder="Buscar producto o escanear código..."
          bind:value={searchQuery}
          class="search-input"
        />
        {#if searchQuery}
          <button class="clear-search" onclick={() => searchQuery = ''}>✕</button>
        {/if}
      </div>
    </div>

    <div class="products-grid">
      {#if productsLoading}
        <div class="products-empty">
          <div class="spinner"></div>
          <p>Cargando productos...</p>
        </div>
      {:else if filteredProducts.length === 0}
        <div class="products-empty">
          <p>😕</p>
          <p>No se encontraron productos</p>
        </div>
      {:else}
        {#each filteredProducts as product (product.id)}
          <button
            class="product-card"
            class:out-of-stock={product.on_hand <= 0}
            disabled={product.on_hand <= 0 || operationLoading}
            onclick={() => handleAddProduct(product)}
          >
            <span class="product-name">{product.name}</span>
            <span class="product-price">{formatMoney(product.unit_price_cents)}</span>
            <span class="product-stock" class:low={product.on_hand <= product.stock_min && product.on_hand > 0}>
              {product.on_hand <= 0 ? 'Sin stock' : `Stock: ${product.on_hand}`}
            </span>
          </button>
        {/each}
      {/if}
    </div>
  </div>

  <!-- ═══ Panel derecho: Carrito ═══ -->
  <div class="cart-panel">
    <div class="cart-header">
      <h2>
        {#if currentSaleId}
          🛒 Venta #{currentSaleId}
        {:else}
          🛒 Nueva Venta
        {/if}
      </h2>
    </div>

    {#if error}
      <div class="error-msg">
        <span>{error}</span>
        <button class="error-close" onclick={() => error = ''}>✕</button>
      </div>
    {/if}

    <div class="cart-lines">
      {#if saleLines.length === 0}
        <div class="cart-empty">
          <p>🍺</p>
          <p>Selecciona un producto para iniciar la venta</p>
        </div>
      {:else}
        {#each saleLines as line (line.id)}
          <div class="cart-line">
            <div class="line-info">
              <span class="line-name">{line.product_name}</span>
              <span class="line-unit-price">{formatMoney(line.unit_price_cents)} c/u</span>
            </div>
            <div class="line-qty">
              <button
                class="qty-btn"
                onclick={() => handleDecrement(line)}
                disabled={operationLoading}
              >−</button>
              <span class="qty-value">{line.qty}</span>
              <button
                class="qty-btn"
                onclick={() => handleIncrement(line)}
                disabled={operationLoading}
              >+</button>
            </div>
            <div class="line-total">
              {formatMoney(line.line_total_cents)}
            </div>
            <button
              class="line-remove"
              onclick={() => handleRemoveLine(line)}
              disabled={operationLoading}
              title="Eliminar"
            >✕</button>
          </div>
        {/each}
      {/if}
    </div>

    <div class="cart-footer">
      <div class="cart-total">
        <span>TOTAL</span>
        <span class="total-amount">{formatMoney(total)}</span>
      </div>

      <div class="cart-actions">
        {#if saleLines.length > 0}
          <button
            class="btn-cancel"
            onclick={handleCancel}
            disabled={operationLoading}
          >
            Cancelar
          </button>
        {/if}
        <button
          class="btn-finalize"
          onclick={handleFinalize}
          disabled={operationLoading || saleLines.length === 0}
        >
          {operationLoading ? 'Procesando...' : `💰 Cobrar ${formatMoney(total)}`}
        </button>
      </div>
    </div>
  </div>

  <!-- ═══ Overlay de venta completada ═══ -->
  {#if lastSaleTotal !== null}
    <div class="sale-success-overlay">
      <div class="sale-success">
        <span class="checkmark">✓</span>
        <h2>¡Venta completada!</h2>
        <p class="success-amount">{formatMoney(lastSaleTotal)}</p>
      </div>
    </div>
  {/if}
</div>

<style>
  .pos-layout {
    display: grid;
    grid-template-columns: 1fr 380px;
    height: 100%;
    gap: 0;
    position: relative;
  }

  /* ═══ Panel Productos ═══ */
  .products-panel {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-right: 1px solid rgba(255, 255, 255, 0.08);
  }

  .search-section {
    padding: 1rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .search-box {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    left: 1rem;
    font-size: 1rem;
    pointer-events: none;
  }

  .search-input {
    width: 100%;
    padding: 0.85rem 2.5rem 0.85rem 3rem;
    font-size: 1rem;
    background: rgba(255, 255, 255, 0.06);
    border: 2px solid rgba(255, 255, 255, 0.1);
    border-radius: 12px;
    color: #fff;
    outline: none;
    transition: border-color 0.2s;
  }

  .search-input:focus {
    border-color: #f59e0b;
  }

  .search-input::placeholder {
    color: rgba(255, 255, 255, 0.3);
  }

  .clear-search {
    position: absolute;
    right: 0.75rem;
    background: rgba(255, 255, 255, 0.1);
    border: none;
    color: rgba(255, 255, 255, 0.5);
    width: 24px;
    height: 24px;
    border-radius: 50%;
    cursor: pointer;
    font-size: 0.75rem;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .clear-search:hover {
    background: rgba(255, 255, 255, 0.2);
    color: #fff;
  }

  .products-grid {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 0.75rem;
    align-content: start;
  }

  .products-empty {
    grid-column: 1 / -1;
    text-align: center;
    color: rgba(255, 255, 255, 0.4);
    padding: 3rem 1rem;
    font-size: 1rem;
  }

  .products-empty p:first-child {
    font-size: 2rem;
    margin-bottom: 0.5rem;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid rgba(255, 255, 255, 0.1);
    border-top-color: #f59e0b;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin: 0 auto 1rem;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .product-card {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    padding: 1rem 0.75rem;
    cursor: pointer;
    text-align: center;
    transition: all 0.15s ease;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    min-height: 100px;
    justify-content: center;
  }

  .product-card:hover:not(:disabled) {
    background: rgba(245, 158, 11, 0.1);
    border-color: rgba(245, 158, 11, 0.4);
    transform: translateY(-2px);
  }

  .product-card:active:not(:disabled) {
    transform: translateY(0);
    background: rgba(245, 158, 11, 0.2);
  }

  .product-card.out-of-stock {
    opacity: 0.35;
  }

  .product-card:disabled {
    cursor: not-allowed;
  }

  .product-name {
    font-size: 0.85rem;
    font-weight: 600;
    color: #fff;
    line-height: 1.2;
  }

  .product-price {
    font-size: 1.1rem;
    font-weight: 700;
    color: #f59e0b;
  }

  .product-stock {
    font-size: 0.75rem;
    color: rgba(255, 255, 255, 0.4);
  }

  .product-stock.low {
    color: #fb923c;
  }

  /* ═══ Panel Carrito ═══ */
  .cart-panel {
    display: flex;
    flex-direction: column;
    background: rgba(255, 255, 255, 0.02);
    overflow: hidden;
  }

  .cart-header {
    padding: 1rem 1.25rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .cart-header h2 {
    font-size: 1.1rem;
    font-weight: 600;
    color: #fff;
    margin: 0;
  }

  .error-msg {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    margin: 0.75rem 1rem 0;
    background: rgba(239, 68, 68, 0.12);
    border: 1px solid rgba(239, 68, 68, 0.25);
    border-radius: 8px;
    color: #ef4444;
    font-size: 0.85rem;
  }

  .error-close {
    background: none;
    border: none;
    color: #ef4444;
    cursor: pointer;
    font-size: 0.85rem;
    padding: 0.25rem;
    opacity: 0.7;
  }

  .error-close:hover {
    opacity: 1;
  }

  .cart-lines {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem 0;
  }

  .cart-empty {
    text-align: center;
    color: rgba(255, 255, 255, 0.3);
    padding: 3rem 1rem;
  }

  .cart-empty p:first-child {
    font-size: 3rem;
    margin-bottom: 0.75rem;
  }

  .cart-empty p:last-child {
    font-size: 0.9rem;
  }

  .cart-line {
    display: grid;
    grid-template-columns: 1fr auto auto auto;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1.25rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
    transition: background 0.15s;
  }

  .cart-line:hover {
    background: rgba(255, 255, 255, 0.03);
  }

  .line-info {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    min-width: 0;
  }

  .line-name {
    font-size: 0.9rem;
    font-weight: 500;
    color: #fff;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .line-unit-price {
    font-size: 0.75rem;
    color: rgba(255, 255, 255, 0.4);
  }

  .line-qty {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .qty-btn {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.15);
    background: rgba(255, 255, 255, 0.06);
    color: #fff;
    font-size: 1rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s;
    padding: 0;
  }

  .qty-btn:hover:not(:disabled) {
    background: rgba(245, 158, 11, 0.2);
    border-color: #f59e0b;
  }

  .qty-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .qty-value {
    min-width: 28px;
    text-align: center;
    font-weight: 600;
    font-size: 0.95rem;
    color: #fff;
  }

  .line-total {
    font-weight: 600;
    font-size: 0.95rem;
    color: #f59e0b;
    min-width: 70px;
    text-align: right;
  }

  .line-remove {
    background: none;
    border: none;
    color: rgba(255, 255, 255, 0.25);
    cursor: pointer;
    font-size: 0.9rem;
    padding: 0.25rem;
    border-radius: 4px;
    transition: all 0.15s;
  }

  .line-remove:hover:not(:disabled) {
    color: #ef4444;
    background: rgba(239, 68, 68, 0.1);
  }

  .line-remove:disabled {
    cursor: not-allowed;
  }

  /* ═══ Footer del carrito ═══ */
  .cart-footer {
    border-top: 2px solid rgba(255, 255, 255, 0.1);
    padding: 1rem 1.25rem;
  }

  .cart-total {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }

  .cart-total span:first-child {
    font-size: 1rem;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.7);
    letter-spacing: 1px;
  }

  .total-amount {
    font-size: 1.75rem;
    font-weight: 800;
    color: #f59e0b;
  }

  .cart-actions {
    display: flex;
    gap: 0.75rem;
  }

  .btn-cancel {
    flex: 0 0 auto;
    padding: 0.85rem 1.25rem;
    font-size: 0.95rem;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.7);
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 10px;
    cursor: pointer;
    transition: all 0.15s;
  }

  .btn-cancel:hover:not(:disabled) {
    background: rgba(239, 68, 68, 0.15);
    border-color: rgba(239, 68, 68, 0.3);
    color: #ef4444;
  }

  .btn-cancel:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .btn-finalize {
    flex: 1;
    padding: 0.85rem 1.25rem;
    font-size: 1.05rem;
    font-weight: 700;
    color: #1a1a2e;
    background: linear-gradient(135deg, #f59e0b 0%, #d97706 100%);
    border: none;
    border-radius: 10px;
    cursor: pointer;
    transition: all 0.15s;
  }

  .btn-finalize:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 6px 16px rgba(245, 158, 11, 0.35);
  }

  .btn-finalize:disabled {
    opacity: 0.4;
    cursor: not-allowed;
    transform: none;
    box-shadow: none;
  }

  /* ═══ Overlay éxito ═══ */
  .sale-success-overlay {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    animation: fadeIn 0.2s ease;
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .sale-success {
    background: rgba(34, 197, 94, 0.15);
    border: 2px solid rgba(34, 197, 94, 0.3);
    border-radius: 24px;
    padding: 3rem 4rem;
    text-align: center;
    backdrop-filter: blur(10px);
  }

  .checkmark {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 64px;
    height: 64px;
    border-radius: 50%;
    background: #22c55e;
    color: #fff;
    font-size: 2rem;
    font-weight: 700;
    margin-bottom: 1rem;
  }

  .sale-success h2 {
    color: #22c55e;
    font-size: 1.5rem;
    margin: 0 0 0.5rem 0;
  }

  .success-amount {
    color: #fff;
    font-size: 2.5rem;
    font-weight: 800;
    margin: 0;
  }
</style>
