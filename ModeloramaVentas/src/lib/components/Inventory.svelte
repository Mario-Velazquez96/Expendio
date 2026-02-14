<script lang="ts">
  import { onMount } from 'svelte';
  import { authStore } from '$lib/stores/auth.svelte';
  import {
    listAllProductsAdmin,
    createProduct,
    addStock,
    adjustStock,
    updateProduct,
    toggleProductActive,
    listProductPriceRulesAdmin,
    createPriceRuleAdmin,
    updatePriceRuleAdmin,
    togglePriceRuleAdmin
  } from '$lib/api/inventory';
  import { formatMoney, pesosToCents, centsToPesos } from '$lib/helpers/money';
  import type { ProductDetail, PriceRule } from '$lib/types';

  // ── Estado principal ──
  let products = $state<ProductDetail[]>([]);
  let loading = $state(true);
  let error = $state('');
  let successMsg = $state('');
  let searchQuery = $state('');
  let filterActive = $state<'all' | 'active' | 'inactive'>('all');

  // ── Modal ──
  type ModalMode = 'none' | 'create' | 'edit' | 'stock' | 'adjust' | 'promos';
  let modalMode = $state<ModalMode>('none');
  let modalLoading = $state(false);
  let modalError = $state('');
  let selectedProduct = $state<ProductDetail | null>(null);

  // ── Form fields (create/edit) ──
  let fName = $state('');
  let fCategory = $state<'BEER' | 'PRODUCT'>('BEER');
  let fBarcode = $state('');
  let fUnitsPerCase = $state(12);
  let fCostPesos = $state('');
  let fPricePesos = $state('');
  let fStockMin = $state('0');
  let fInitialQty = $state('0');

  // ── Form fields (add stock) ──
  let fStockQty = $state('');
  let fStockNote = $state('');

  // ── Form fields (adjust stock) ──
  let fAdjustQty = $state('');
  let fAdjustReason = $state('');

  // ── Form fields (promos) ──
  let promoRules = $state<PriceRule[]>([]);
  let promoLoading = $state(false);
  let editingPromoId = $state<number | null>(null);
  let fPromoName = $state('');
  let fPromoRequiredQty = $state('');
  let fPromoBundlePrice = $state('');
  let fPromoPriority = $state('0');
  let fPromoEnabled = $state(true);

  // ── Computed ──
  let filteredProducts = $derived.by(() => {
    let result = products;

    // Filter by active status
    if (filterActive === 'active') {
      result = result.filter(p => p.active === 1);
    } else if (filterActive === 'inactive') {
      result = result.filter(p => p.active === 0);
    }

    // Filter by search
    const q = searchQuery.trim().toLowerCase();
    if (q) {
      result = result.filter(p =>
        p.name.toLowerCase().includes(q) ||
        (p.barcode && p.barcode.toLowerCase().includes(q))
      );
    }

    return result;
  });

  let activeCount = $derived(products.filter(p => p.active === 1).length);
  let totalCount = $derived(products.length);

  // ── Lifecycle ──
  onMount(() => {
    loadProducts();
  });

  // ── Functions ──
  async function loadProducts() {
    loading = true;
    error = '';
    try {
      products = await listAllProductsAdmin(authStore.pin);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function showSuccess(msg: string) {
    successMsg = msg;
    setTimeout(() => { successMsg = ''; }, 3000);
  }

  // ── Create ──
  function openCreateModal() {
    modalMode = 'create';
    modalError = '';
    fName = '';
    fCategory = 'BEER';
    fBarcode = '';
    fUnitsPerCase = 12;
    fCostPesos = '';
    fPricePesos = '';
    fStockMin = '0';
    fInitialQty = '0';
  }

  async function handleCreate(e: Event) {
    e.preventDefault();
    modalError = '';

    const cost = parseFloat(fCostPesos);
    const price = parseFloat(fPricePesos);
    const stockMin = parseInt(fStockMin) || 0;
    const initialQty = parseInt(fInitialQty) || 0;

    if (!fName.trim()) { modalError = 'Ingresa el nombre del producto'; return; }
    if (isNaN(cost) || cost < 0) { modalError = 'Ingresa un costo válido'; return; }
    if (isNaN(price) || price <= 0) { modalError = 'Ingresa un precio de venta válido'; return; }

    modalLoading = true;
    try {
      await createProduct(
        authStore.pin,
        fName.trim(),
        fCategory,
        fBarcode.trim() || null,
        fCategory === 'BEER' ? fUnitsPerCase : 0,
        pesosToCents(cost),
        pesosToCents(price),
        stockMin,
        initialQty
      );
      modalMode = 'none';
      showSuccess(`Producto "${fName.trim()}" creado exitosamente`);
      await loadProducts();
    } catch (e) {
      modalError = String(e);
    } finally {
      modalLoading = false;
    }
  }

  // ── Edit ──
  function openEditModal(product: ProductDetail) {
    modalMode = 'edit';
    modalError = '';
    selectedProduct = product;
    fName = product.name;
    fCategory = product.category as 'BEER' | 'PRODUCT';
    fBarcode = product.barcode || '';
    fUnitsPerCase = product.units_per_case || 12;
    fCostPesos = centsToPesos(product.cost_cents).toString();
    fPricePesos = centsToPesos(product.unit_price_cents).toString();
    fStockMin = product.stock_min.toString();
  }

  async function handleEdit(e: Event) {
    e.preventDefault();
    if (!selectedProduct) return;
    modalError = '';

    const cost = parseFloat(fCostPesos);
    const price = parseFloat(fPricePesos);
    const stockMin = parseInt(fStockMin) || 0;

    if (!fName.trim()) { modalError = 'Ingresa el nombre'; return; }
    if (isNaN(price) || price <= 0) { modalError = 'Precio inválido'; return; }

    modalLoading = true;
    try {
      await updateProduct(
        authStore.pin,
        selectedProduct.id,
        fName.trim(),
        fCategory,
        fBarcode.trim() || null,
        fCategory === 'BEER' ? fUnitsPerCase : 0,
        pesosToCents(cost),
        pesosToCents(price),
        stockMin
      );
      modalMode = 'none';
      showSuccess(`Producto "${fName.trim()}" actualizado`);
      await loadProducts();
    } catch (e) {
      modalError = String(e);
    } finally {
      modalLoading = false;
    }
  }

  // ── Add Stock ──
  function openStockModal(product: ProductDetail) {
    modalMode = 'stock';
    modalError = '';
    selectedProduct = product;
    fStockQty = '';
    fStockNote = '';
  }

  async function handleAddStock(e: Event) {
    e.preventDefault();
    if (!selectedProduct) return;
    modalError = '';

    const qty = parseInt(fStockQty);
    if (isNaN(qty) || qty <= 0) {
      modalError = 'Ingresa una cantidad válida mayor a 0';
      return;
    }

    modalLoading = true;
    try {
      const result = await addStock(
        authStore.pin,
        selectedProduct.id,
        qty,
        fStockNote.trim() || undefined
      );
      modalMode = 'none';
      const label = selectedProduct.units_per_case > 0
        ? `${result.cases_added} cajas (${result.units_added} pzas)`
        : `${result.units_added} unidades`;
      showSuccess(`Stock agregado: ${label} de "${result.product_name}". Total: ${result.new_on_hand} pzas`);
      await loadProducts();
    } catch (e) {
      modalError = String(e);
    } finally {
      modalLoading = false;
    }
  }

  // ── Toggle Active ──
  async function handleToggleActive(product: ProductDetail) {
    error = '';
    try {
      const updated = await toggleProductActive(authStore.pin, product.id);
      const idx = products.findIndex(p => p.id === product.id);
      if (idx >= 0) products[idx] = updated;
      showSuccess(`"${updated.name}" ${updated.active ? 'activado' : 'desactivado'}`);
    } catch (e) {
      error = String(e);
    }
  }

  // ── Adjust Stock (subtract) ──
  function openAdjustModal(product: ProductDetail) {
    modalMode = 'adjust';
    modalError = '';
    selectedProduct = product;
    fAdjustQty = '';
    fAdjustReason = '';
  }

  async function handleAdjustStock(e: Event) {
    e.preventDefault();
    if (!selectedProduct) return;
    modalError = '';

    const qty = parseInt(fAdjustQty);
    if (isNaN(qty) || qty <= 0) {
      modalError = 'Ingresa una cantidad válida mayor a 0';
      return;
    }
    if (!fAdjustReason.trim()) {
      modalError = 'Debes indicar el motivo del ajuste';
      return;
    }

    modalLoading = true;
    try {
      const result = await adjustStock(
        authStore.pin,
        selectedProduct.id,
        qty,
        fAdjustReason.trim()
      );
      modalMode = 'none';
      showSuccess(`Ajuste realizado: -${result.units_removed} pzas de "${result.product_name}". Nuevo stock: ${result.new_on_hand} pzas`);
      await loadProducts();
    } catch (e) {
      modalError = String(e);
    } finally {
      modalLoading = false;
    }
  }

  // ── Promos ──
  async function openPromosModal(product: ProductDetail) {
    modalMode = 'promos';
    modalError = '';
    selectedProduct = product;
    promoLoading = true;
    editingPromoId = null;
    resetPromoForm();

    try {
      promoRules = await listProductPriceRulesAdmin(authStore.pin, product.id);
    } catch (e) {
      modalError = String(e);
    } finally {
      promoLoading = false;
    }
  }

  function resetPromoForm() {
    fPromoName = '';
    fPromoRequiredQty = '';
    fPromoBundlePrice = '';
    fPromoPriority = '0';
    fPromoEnabled = true;
  }

  function startEditPromo(rule: PriceRule) {
    editingPromoId = rule.id;
    fPromoName = rule.name;
    fPromoRequiredQty = rule.required_qty.toString();
    fPromoBundlePrice = centsToPesos(rule.bundle_price_cents).toString();
    fPromoPriority = rule.priority.toString();
    fPromoEnabled = rule.enabled === 1;
    modalError = '';
  }

  function cancelEditPromo() {
    editingPromoId = null;
    resetPromoForm();
    modalError = '';
  }

  async function handleSavePromo(e: Event) {
    e.preventDefault();
    if (!selectedProduct) return;
    modalError = '';

    const requiredQty = parseInt(fPromoRequiredQty);
    const bundlePrice = parseFloat(fPromoBundlePrice);
    const priority = parseInt(fPromoPriority) || 0;
    const enabled = fPromoEnabled ? 1 : 0;

    if (!fPromoName.trim()) {
      modalError = 'Ingresa nombre de la promoción';
      return;
    }
    if (isNaN(requiredQty) || requiredQty <= 1) {
      modalError = 'La cantidad requerida debe ser mayor a 1';
      return;
    }
    if (isNaN(bundlePrice) || bundlePrice < 0) {
      modalError = 'Ingresa un precio de promo válido';
      return;
    }

    modalLoading = true;
    try {
      if (editingPromoId) {
        await updatePriceRuleAdmin(
          authStore.pin,
          editingPromoId,
          fPromoName.trim(),
          requiredQty,
          pesosToCents(bundlePrice),
          priority,
          enabled
        );
        showSuccess('Promoción actualizada');
      } else {
        await createPriceRuleAdmin(
          authStore.pin,
          selectedProduct.id,
          fPromoName.trim(),
          requiredQty,
          pesosToCents(bundlePrice),
          priority,
          enabled
        );
        showSuccess('Promoción creada');
      }

      promoRules = await listProductPriceRulesAdmin(authStore.pin, selectedProduct.id);
      cancelEditPromo();
    } catch (e) {
      modalError = String(e);
    } finally {
      modalLoading = false;
    }
  }

  async function handleTogglePromo(rule: PriceRule) {
    if (!selectedProduct) return;
    modalError = '';
    try {
      await togglePriceRuleAdmin(authStore.pin, rule.id);
      promoRules = await listProductPriceRulesAdmin(authStore.pin, selectedProduct.id);
    } catch (e) {
      modalError = String(e);
    }
  }

  function closeModal() {
    modalMode = 'none';
    selectedProduct = null;
    promoRules = [];
    editingPromoId = null;
    resetPromoForm();
  }
</script>

<div class="inventory-layout">
  <!-- ═══ Header ═══ -->
  <div class="inv-header">
    <div class="inv-header-left">
      <h2>📦 Inventario</h2>
      <span class="product-count">{activeCount} activos / {totalCount} total</span>
    </div>
    <div class="inv-header-right">
      <div class="search-box">
        <span class="search-icon">🔍</span>
        <input
          type="text"
          placeholder="Buscar producto..."
          bind:value={searchQuery}
          class="search-input"
        />
      </div>
      <div class="filter-pills">
        <button class="pill" class:active={filterActive === 'all'} onclick={() => filterActive = 'all'}>Todos</button>
        <button class="pill" class:active={filterActive === 'active'} onclick={() => filterActive = 'active'}>Activos</button>
        <button class="pill" class:active={filterActive === 'inactive'} onclick={() => filterActive = 'inactive'}>Inactivos</button>
      </div>
      {#if authStore.isOwner}
        <button class="btn-new" onclick={openCreateModal}>
          ＋ Nuevo Producto
        </button>
      {/if}
    </div>
  </div>

  <!-- ═══ Mensajes ═══ -->
  {#if successMsg}
    <div class="success-msg">✅ {successMsg}</div>
  {/if}
  {#if error}
    <div class="error-banner">⚠️ {error}</div>
  {/if}

  <!-- ═══ Tabla de productos ═══ -->
  <div class="table-container">
    {#if loading}
      <div class="table-empty">
        <div class="spinner"></div>
        <p>Cargando productos...</p>
      </div>
    {:else if filteredProducts.length === 0}
      <div class="table-empty">
        <p style="font-size:2rem">📦</p>
        <p>{products.length === 0 ? 'No hay productos registrados. Crea el primero.' : 'No se encontraron productos con ese filtro.'}</p>
      </div>
    {:else}
      <table>
        <thead>
          <tr>
            <th>Producto</th>
            <th>Categoría</th>
            <th class="right">Precio</th>
            <th class="right">Costo</th>
            <th class="right">Stock</th>
            <th class="right">Mín</th>
            <th>Estado</th>
            <th>Acciones</th>
          </tr>
        </thead>
        <tbody>
          {#each filteredProducts as product (product.id)}
            <tr class:inactive={product.active === 0}>
              <td>
                <div class="product-cell">
                  <span class="product-name-cell">{product.name}</span>
                  {#if product.barcode}
                    <span class="product-barcode">{product.barcode}</span>
                  {/if}
                </div>
              </td>
              <td>
                <span class="category-badge" class:beer={product.category === 'BEER'} class:prod={product.category === 'PRODUCT'}>
                  {product.category === 'BEER' ? '🍺 Cerveza' : '📦 Producto'}
                </span>
              </td>
              <td class="right mono">{formatMoney(product.unit_price_cents)}</td>
              <td class="right mono dim">{formatMoney(product.cost_cents)}</td>
              <td class="right">
                <span
                  class="stock-value"
                  class:ok={product.on_hand > product.stock_min}
                  class:low={product.on_hand > 0 && product.on_hand <= product.stock_min}
                  class:zero={product.on_hand <= 0}
                >
                  {product.on_hand}
                </span>
                {#if product.units_per_case > 0}
                  <span class="stock-cases">({Math.floor(product.on_hand / product.units_per_case)}c)</span>
                {/if}
              </td>
              <td class="right dim">{product.stock_min}</td>
              <td>
                {#if authStore.isOwner}
                  <button
                    class="status-toggle"
                    class:is-active={product.active === 1}
                    class:is-inactive={product.active === 0}
                    onclick={() => handleToggleActive(product)}
                    title={product.active ? 'Desactivar' : 'Activar'}
                  >
                    {product.active ? '✓ Activo' : '✗ Inactivo'}
                  </button>
                {:else}
                  <span
                    class="status-label"
                    class:is-active={product.active === 1}
                    class:is-inactive={product.active === 0}
                  >
                    {product.active ? '✓ Activo' : '✗ Inactivo'}
                  </span>
                {/if}
              </td>
              <td>
                <div class="action-btns">
                  {#if authStore.isOwner}
                    <button class="action-btn" onclick={() => openEditModal(product)} title="Editar">
                      ✏️
                    </button>
                    <button class="action-btn promo-btn" onclick={() => openPromosModal(product)} title="Promociones">
                      🎯
                    </button>
                  {/if}
                  {#if product.active === 1}
                    <button class="action-btn stock-btn" onclick={() => openStockModal(product)} title="Agregar stock">
                      📥
                    </button>
                    {#if authStore.isOwner && product.on_hand > 0}
                      <button class="action-btn adjust-btn" onclick={() => openAdjustModal(product)} title="Ajustar stock (restar)">
                        📤
                      </button>
                    {/if}
                  {/if}
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>

<!-- ═══ MODAL: Crear / Editar Producto ═══ -->
{#if modalMode === 'create' || modalMode === 'edit'}
  <div class="modal-overlay">
    <div class="modal-card">
      <h3>{modalMode === 'create' ? '➕ Nuevo Producto' : `✏️ Editar: ${selectedProduct?.name}`}</h3>

      <form onsubmit={modalMode === 'create' ? handleCreate : handleEdit}>
        <div class="form-field">
          <label for="f-name">Nombre *</label>
          <input id="f-name" type="text" bind:value={fName} placeholder="Ej: Corona Extra 355ml" disabled={modalLoading} />
        </div>

        <div class="form-row-2">
          <div class="form-field">
            <span class="field-label">Categoría *</span>
            <div class="toggle-group">
              <button type="button" class="toggle-opt" class:active={fCategory === 'BEER'} onclick={() => fCategory = 'BEER'}>🍺 Cerveza</button>
              <button type="button" class="toggle-opt" class:active={fCategory === 'PRODUCT'} onclick={() => fCategory = 'PRODUCT'}>📦 Producto</button>
            </div>
          </div>
          {#if fCategory === 'BEER'}
            <div class="form-field">
              <span class="field-label">Envase (pzas/caja)</span>
              <div class="toggle-group">
                <button type="button" class="toggle-opt" class:active={fUnitsPerCase === 12} onclick={() => fUnitsPerCase = 12}>Botella (12)</button>
                <button type="button" class="toggle-opt" class:active={fUnitsPerCase === 24} onclick={() => fUnitsPerCase = 24}>Lata (24)</button>
              </div>
            </div>
          {/if}
        </div>

        <div class="form-field">
          <label for="f-barcode">Código de barras (opcional)</label>
          <input id="f-barcode" type="text" bind:value={fBarcode} placeholder="Escanea o escribe el código" disabled={modalLoading} />
        </div>

        <div class="form-row-2">
          <div class="form-field">
            <label for="f-cost">Costo por pieza ($) *</label>
            <input id="f-cost" type="number" step="0.01" min="0" bind:value={fCostPesos} placeholder="0.00" disabled={modalLoading} />
          </div>
          <div class="form-field">
            <label for="f-price">Precio de venta ($) *</label>
            <input id="f-price" type="number" step="0.01" min="0.01" bind:value={fPricePesos} placeholder="0.00" disabled={modalLoading} />
          </div>
        </div>

        <div class="form-row-2">
          <div class="form-field">
            <label for="f-stockmin">Stock mínimo</label>
            <input id="f-stockmin" type="number" min="0" bind:value={fStockMin} disabled={modalLoading} />
          </div>
          {#if modalMode === 'create'}
            <div class="form-field">
              <label for="f-initqty">
                Stock inicial ({fCategory === 'BEER' ? 'cajas' : 'unidades'})
              </label>
              <input id="f-initqty" type="number" min="0" bind:value={fInitialQty} placeholder="0" disabled={modalLoading} />
              {#if fCategory === 'BEER' && parseInt(fInitialQty) > 0}
                <span class="field-hint">= {parseInt(fInitialQty) * fUnitsPerCase} piezas</span>
              {/if}
            </div>
          {/if}
        </div>

        {#if modalError}
          <div class="modal-error">⚠️ {modalError}</div>
        {/if}

        <div class="modal-actions">
          <button type="button" class="btn-modal-cancel" onclick={closeModal} disabled={modalLoading}>
            Cancelar
          </button>
          <button type="submit" class="btn-modal-ok" disabled={modalLoading}>
            {modalLoading ? 'Guardando...' : (modalMode === 'create' ? 'Crear Producto' : 'Guardar Cambios')}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- ═══ MODAL: Agregar Stock ═══ -->
{#if modalMode === 'stock' && selectedProduct}
  <div class="modal-overlay">
    <div class="modal-card small">
      <h3>📥 Agregar Stock</h3>
      <p class="stock-product-name">{selectedProduct.name}</p>
      <p class="stock-current">Stock actual: <strong>{selectedProduct.on_hand}</strong> piezas</p>

      <form onsubmit={handleAddStock}>
        <div class="form-field">
          <label for="f-stock-qty">
            Cantidad ({selectedProduct.units_per_case > 0 ? 'cajas' : 'unidades'})
          </label>
          <input
            id="f-stock-qty"
            type="number"
            min="1"
            bind:value={fStockQty}
            placeholder={selectedProduct.units_per_case > 0 ? 'Ej: 5 cajas' : 'Ej: 20'}
            disabled={modalLoading}
          />
          {#if selectedProduct.units_per_case > 0 && parseInt(fStockQty) > 0}
            <span class="field-hint">= {parseInt(fStockQty) * selectedProduct.units_per_case} piezas</span>
          {/if}
        </div>

        <div class="form-field">
          <label for="f-stock-note">Nota (opcional)</label>
          <input
            id="f-stock-note"
            type="text"
            bind:value={fStockNote}
            placeholder="Ej: Compra proveedor X"
            disabled={modalLoading}
          />
        </div>

        {#if modalError}
          <div class="modal-error">⚠️ {modalError}</div>
        {/if}

        <div class="modal-actions">
          <button type="button" class="btn-modal-cancel" onclick={closeModal} disabled={modalLoading}>
            Cancelar
          </button>
          <button type="submit" class="btn-modal-ok" disabled={modalLoading || !fStockQty}>
            {modalLoading ? 'Registrando...' : 'Agregar Stock'}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- ═══ MODAL: Ajustar Stock (restar) ═══ -->
{#if modalMode === 'adjust' && selectedProduct}
  <div class="modal-overlay">
    <div class="modal-card small">
      <h3>📤 Ajustar Stock</h3>
      <p class="stock-product-name">{selectedProduct.name}</p>
      <p class="stock-current">Stock actual: <strong>{selectedProduct.on_hand}</strong> piezas</p>

      <form onsubmit={handleAdjustStock}>
        <div class="form-field">
          <label for="f-adjust-qty">
            Piezas a restar
          </label>
          <input
            id="f-adjust-qty"
            type="number"
            min="1"
            max={selectedProduct.on_hand}
            bind:value={fAdjustQty}
            placeholder="Ej: 5 piezas"
            disabled={modalLoading}
          />
          {#if parseInt(fAdjustQty) > 0}
            <span class="field-hint adjust-hint">
              Stock resultante: {selectedProduct.on_hand - parseInt(fAdjustQty)} piezas
              {#if selectedProduct.units_per_case > 0}
                ({Math.floor((selectedProduct.on_hand - parseInt(fAdjustQty)) / selectedProduct.units_per_case)} cajas)
              {/if}
            </span>
          {/if}
        </div>

        <div class="form-field">
          <label for="f-adjust-reason">Motivo del ajuste *</label>
          <input
            id="f-adjust-reason"
            type="text"
            bind:value={fAdjustReason}
            placeholder="Ej: Producto dañado, merma, robo..."
            disabled={modalLoading}
          />
        </div>

        {#if modalError}
          <div class="modal-error">⚠️ {modalError}</div>
        {/if}

        <div class="modal-actions">
          <button type="button" class="btn-modal-cancel" onclick={closeModal} disabled={modalLoading}>
            Cancelar
          </button>
          <button type="submit" class="btn-modal-danger" disabled={modalLoading || !fAdjustQty || !fAdjustReason.trim()}>
            {modalLoading ? 'Procesando...' : 'Confirmar Ajuste'}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- ═══ MODAL: Promociones ═══ -->
{#if modalMode === 'promos' && selectedProduct}
  <div class="modal-overlay">
    <div class="modal-card">
      <h3>🎯 Promociones: {selectedProduct.name}</h3>

      <div class="promo-section">
        <h4>{editingPromoId ? 'Editar promoción' : 'Nueva promoción'}</h4>
        <form onsubmit={handleSavePromo}>
          <div class="form-row-2">
            <div class="form-field">
              <label for="f-promo-name">Nombre *</label>
              <input id="f-promo-name" type="text" bind:value={fPromoName} placeholder="Ej: Promo 6 cervezas" disabled={modalLoading} />
            </div>
            <div class="form-field">
              <label for="f-promo-qty">Cantidad requerida *</label>
              <input id="f-promo-qty" type="number" min="2" bind:value={fPromoRequiredQty} placeholder="Ej: 6" disabled={modalLoading} />
            </div>
          </div>

          <div class="form-row-2">
            <div class="form-field">
              <label for="f-promo-price">Precio promo ($) *</label>
              <input id="f-promo-price" type="number" step="0.01" min="0" bind:value={fPromoBundlePrice} placeholder="Ej: 110.00" disabled={modalLoading} />
            </div>
            <div class="form-field">
              <label for="f-promo-priority">Prioridad</label>
              <input id="f-promo-priority" type="number" bind:value={fPromoPriority} disabled={modalLoading} />
            </div>
          </div>

          <div class="promo-enabled">
            <input id="f-promo-enabled" type="checkbox" bind:checked={fPromoEnabled} disabled={modalLoading} />
            <label for="f-promo-enabled">Promoción habilitada</label>
          </div>

          <div class="modal-actions">
            {#if editingPromoId}
              <button type="button" class="btn-modal-cancel" onclick={cancelEditPromo} disabled={modalLoading}>
                Cancelar edición
              </button>
            {/if}
            <button type="submit" class="btn-modal-ok" disabled={modalLoading}>
              {modalLoading ? 'Guardando...' : (editingPromoId ? 'Guardar promoción' : 'Crear promoción')}
            </button>
          </div>
        </form>
      </div>

      <div class="promo-section">
        <h4>Promociones registradas</h4>
        {#if promoLoading}
          <p class="promo-empty">Cargando promociones...</p>
        {:else if promoRules.length === 0}
          <p class="promo-empty">No hay promociones registradas para este producto.</p>
        {:else}
          <div class="promo-list">
            {#each promoRules as rule (rule.id)}
              <div class="promo-row" class:disabled={rule.enabled === 0}>
                <div class="promo-info">
                  <span class="promo-title">{rule.name}</span>
                  <span class="promo-meta">
                    {rule.required_qty} pzas por {formatMoney(rule.bundle_price_cents)} · prioridad {rule.priority}
                  </span>
                </div>
                <div class="promo-actions">
                  <button class="action-btn" onclick={() => startEditPromo(rule)} title="Editar">✏️</button>
                  <button class="action-btn" onclick={() => handleTogglePromo(rule)} title={rule.enabled ? 'Desactivar' : 'Activar'}>
                    {rule.enabled ? '🚫' : '✅'}
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      {#if modalError}
        <div class="modal-error">⚠️ {modalError}</div>
      {/if}

      <div class="modal-actions">
        <button type="button" class="btn-modal-cancel" onclick={closeModal}>Cerrar</button>
      </div>
    </div>
  </div>
{/if}

<style>
  /* ═══ Layout ═══ */
  .inventory-layout {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .inv-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 1.5rem;
    border-bottom: 1px solid rgba(255,255,255,0.08);
    gap: 1rem;
    flex-wrap: wrap;
  }

  .inv-header-left {
    display: flex;
    align-items: baseline;
    gap: 1rem;
  }

  .inv-header-left h2 {
    font-size: 1.2rem;
    font-weight: 700;
    color: #fff;
    margin: 0;
  }

  .product-count {
    font-size: 0.8rem;
    color: rgba(255,255,255,0.4);
  }

  .inv-header-right {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .search-box {
    position: relative;
  }
  .search-icon {
    position: absolute;
    left: 0.7rem;
    top: 50%;
    transform: translateY(-50%);
    font-size: 0.85rem;
    pointer-events: none;
  }
  .search-input {
    padding: 0.55rem 0.85rem 0.55rem 2.2rem;
    font-size: 0.9rem;
    background: rgba(255,255,255,0.06);
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 8px;
    color: #fff;
    outline: none;
    width: 200px;
    transition: border-color 0.15s;
  }
  .search-input:focus { border-color: #f59e0b; }
  .search-input::placeholder { color: rgba(255,255,255,0.3); }

  .filter-pills {
    display: flex;
    gap: 0.25rem;
    background: rgba(255,255,255,0.04);
    border-radius: 8px;
    padding: 0.2rem;
  }
  .pill {
    padding: 0.35rem 0.7rem;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: rgba(255,255,255,0.45);
    font-size: 0.8rem;
    cursor: pointer;
    transition: all 0.15s;
  }
  .pill.active {
    background: rgba(245,158,11,0.2);
    color: #f59e0b;
    font-weight: 600;
  }
  .pill:hover:not(.active) {
    background: rgba(255,255,255,0.06);
    color: rgba(255,255,255,0.7);
  }

  .btn-new {
    padding: 0.55rem 1.25rem;
    font-size: 0.9rem;
    font-weight: 600;
    color: #1a1a2e;
    background: linear-gradient(135deg, #f59e0b 0%, #d97706 100%);
    border: none;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.15s;
  }
  .btn-new:hover {
    box-shadow: 0 4px 12px rgba(245,158,11,0.3);
    transform: translateY(-1px);
  }

  /* ═══ Messages ═══ */
  .success-msg {
    margin: 0.75rem 1.5rem 0;
    padding: 0.7rem 1rem;
    background: rgba(34,197,94,0.12);
    border: 1px solid rgba(34,197,94,0.25);
    border-radius: 8px;
    color: #22c55e;
    font-size: 0.9rem;
  }
  .error-banner {
    margin: 0.75rem 1.5rem 0;
    padding: 0.7rem 1rem;
    background: rgba(239,68,68,0.12);
    border: 1px solid rgba(239,68,68,0.25);
    border-radius: 8px;
    color: #ef4444;
    font-size: 0.9rem;
  }

  /* ═══ Table ═══ */
  .table-container {
    flex: 1;
    overflow-y: auto;
    padding: 0 1.5rem 1.5rem;
  }

  .table-empty {
    text-align: center;
    color: rgba(255,255,255,0.4);
    padding: 4rem 1rem;
  }
  .spinner {
    width: 32px; height: 32px;
    border: 3px solid rgba(255,255,255,0.1);
    border-top-color: #f59e0b;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin: 0 auto 1rem;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  table {
    width: 100%;
    border-collapse: collapse;
    margin-top: 1rem;
  }

  th {
    text-align: left;
    padding: 0.65rem 0.75rem;
    font-size: 0.75rem;
    font-weight: 600;
    color: rgba(255,255,255,0.4);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    border-bottom: 1px solid rgba(255,255,255,0.08);
  }
  th.right { text-align: right; }

  td {
    padding: 0.7rem 0.75rem;
    font-size: 0.9rem;
    border-bottom: 1px solid rgba(255,255,255,0.04);
    vertical-align: middle;
  }
  td.right { text-align: right; }
  td.mono { font-family: 'SF Mono', 'Cascadia Code', monospace; font-size: 0.85rem; }
  td.dim { color: rgba(255,255,255,0.4); }

  tr:hover { background: rgba(255,255,255,0.02); }
  tr.inactive { opacity: 0.45; }

  .product-cell {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .product-name-cell {
    font-weight: 600;
    color: #fff;
  }
  .product-barcode {
    font-size: 0.75rem;
    color: rgba(255,255,255,0.35);
    font-family: monospace;
  }

  .category-badge {
    display: inline-block;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 500;
  }
  .category-badge.beer { background: rgba(245,158,11,0.12); color: #f59e0b; }
  .category-badge.prod { background: rgba(99,102,241,0.12); color: #818cf8; }

  .stock-value {
    font-weight: 700;
    font-size: 0.95rem;
  }
  .stock-value.ok { color: #22c55e; }
  .stock-value.low { color: #fb923c; }
  .stock-value.zero { color: #ef4444; }
  .stock-cases {
    font-size: 0.7rem;
    color: rgba(255,255,255,0.3);
    margin-left: 0.25rem;
  }

  .status-toggle {
    padding: 0.2rem 0.6rem;
    border-radius: 12px;
    border: 1px solid;
    font-size: 0.75rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
    background: transparent;
  }
  .status-toggle.is-active {
    border-color: rgba(34,197,94,0.3);
    color: #22c55e;
  }
  .status-toggle.is-active:hover {
    background: rgba(239,68,68,0.1);
    border-color: rgba(239,68,68,0.3);
    color: #ef4444;
  }
  .status-toggle.is-inactive {
    border-color: rgba(255,255,255,0.15);
    color: rgba(255,255,255,0.4);
  }
  .status-toggle.is-inactive:hover {
    background: rgba(34,197,94,0.1);
    border-color: rgba(34,197,94,0.3);
    color: #22c55e;
  }

  .status-label {
    display: inline-block;
    padding: 0.2rem 0.6rem;
    border-radius: 12px;
    border: 1px solid;
    font-size: 0.75rem;
    font-weight: 600;
  }
  .status-label.is-active {
    border-color: rgba(34,197,94,0.3);
    color: #22c55e;
  }
  .status-label.is-inactive {
    border-color: rgba(255,255,255,0.15);
    color: rgba(255,255,255,0.4);
  }

  .action-btns {
    display: flex;
    gap: 0.35rem;
  }
  .action-btn {
    width: 32px; height: 32px;
    border-radius: 6px;
    border: 1px solid rgba(255,255,255,0.08);
    background: rgba(255,255,255,0.04);
    font-size: 0.9rem;
    cursor: pointer;
    transition: all 0.15s;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }
  .action-btn:hover {
    background: rgba(255,255,255,0.1);
    border-color: rgba(255,255,255,0.2);
  }
  .action-btn.stock-btn:hover {
    background: rgba(34,197,94,0.15);
    border-color: rgba(34,197,94,0.3);
  }
  .action-btn.adjust-btn:hover {
    background: rgba(239,68,68,0.15);
    border-color: rgba(239,68,68,0.3);
  }
  .action-btn.promo-btn:hover {
    background: rgba(59,130,246,0.18);
    border-color: rgba(59,130,246,0.35);
  }

  /* ═══ Modal ═══ */
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.65);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 200;
    padding: 1rem;
    animation: fadeIn 0.15s ease;
  }
  @keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }

  .modal-card {
    background: #1e2140;
    border: 1px solid rgba(255,255,255,0.12);
    border-radius: 20px;
    padding: 2rem;
    max-width: 560px;
    width: 100%;
    max-height: 90vh;
    overflow-y: auto;
    box-shadow: 0 25px 50px rgba(0,0,0,0.4);
  }
  .modal-card.small { max-width: 420px; }

  .modal-card h3 {
    font-size: 1.3rem;
    font-weight: 700;
    color: #fff;
    margin: 0 0 1.5rem 0;
  }

  .stock-product-name {
    font-size: 1.1rem;
    font-weight: 600;
    color: #f59e0b;
    margin: -0.75rem 0 0.25rem 0;
  }
  .stock-current {
    font-size: 0.9rem;
    color: rgba(255,255,255,0.5);
    margin: 0 0 1.25rem 0;
  }

  .form-field {
    margin-bottom: 1rem;
  }
  .form-field label {
    display: block;
    font-size: 0.8rem;
    color: rgba(255,255,255,0.55);
    font-weight: 500;
    margin-bottom: 0.4rem;
  }
  .form-field input {
    width: 100%;
    padding: 0.7rem 0.85rem;
    font-size: 0.95rem;
    background: rgba(255,255,255,0.06);
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 8px;
    color: #fff;
    outline: none;
    transition: border-color 0.15s;
    font-family: inherit;
  }
  .form-field input:focus { border-color: #f59e0b; }
  .form-field input::placeholder { color: rgba(255,255,255,0.25); }
  .form-field input:disabled { opacity: 0.5; }

  /* hide number spinners */
  .form-field input[type="number"]::-webkit-inner-spin-button,
  .form-field input[type="number"]::-webkit-outer-spin-button { -webkit-appearance: none; margin: 0; }
  .form-field input[type="number"] { -moz-appearance: textfield; appearance: textfield; }

  .field-hint {
    display: block;
    font-size: 0.75rem;
    color: rgba(245,158,11,0.7);
    margin-top: 0.3rem;
  }

  .field-label {
    display: block;
    font-size: 0.8rem;
    color: rgba(255,255,255,0.55);
    font-weight: 500;
    margin-bottom: 0.4rem;
  }

  .form-row-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }

  .toggle-group {
    display: flex;
    gap: 0.4rem;
  }
  .toggle-opt {
    flex: 1;
    padding: 0.55rem 0.5rem;
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 8px;
    background: rgba(255,255,255,0.04);
    color: rgba(255,255,255,0.45);
    font-size: 0.85rem;
    cursor: pointer;
    transition: all 0.15s;
  }
  .toggle-opt.active {
    background: rgba(245,158,11,0.15);
    border-color: rgba(245,158,11,0.4);
    color: #f59e0b;
    font-weight: 600;
  }
  .toggle-opt:hover:not(.active) {
    background: rgba(255,255,255,0.08);
    color: rgba(255,255,255,0.7);
  }

  .modal-error {
    padding: 0.7rem 1rem;
    background: rgba(239,68,68,0.12);
    border: 1px solid rgba(239,68,68,0.25);
    border-radius: 8px;
    color: #ef4444;
    font-size: 0.85rem;
    margin-bottom: 1rem;
  }

  .modal-actions {
    display: flex;
    gap: 0.75rem;
    margin-top: 0.5rem;
  }
  .btn-modal-cancel {
    flex: 0 0 auto;
    padding: 0.75rem 1.25rem;
    font-size: 0.95rem;
    color: rgba(255,255,255,0.6);
    background: rgba(255,255,255,0.06);
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 10px;
    cursor: pointer;
    transition: all 0.15s;
  }
  .btn-modal-cancel:hover:not(:disabled) {
    background: rgba(255,255,255,0.1);
    color: #fff;
  }
  .btn-modal-cancel:disabled { opacity: 0.4; cursor: not-allowed; }

  .btn-modal-ok {
    flex: 1;
    padding: 0.75rem 1.25rem;
    font-size: 1rem;
    font-weight: 700;
    color: #1a1a2e;
    background: linear-gradient(135deg, #f59e0b 0%, #d97706 100%);
    border: none;
    border-radius: 10px;
    cursor: pointer;
    transition: all 0.15s;
  }
  .btn-modal-ok:hover:not(:disabled) {
    box-shadow: 0 6px 16px rgba(245,158,11,0.35);
  }
  .btn-modal-ok:disabled { opacity: 0.4; cursor: not-allowed; }

  .btn-modal-danger {
    flex: 1;
    padding: 0.75rem 1.25rem;
    font-size: 1rem;
    font-weight: 700;
    color: #fff;
    background: linear-gradient(135deg, #ef4444 0%, #dc2626 100%);
    border: none;
    border-radius: 10px;
    cursor: pointer;
    transition: all 0.15s;
  }
  .btn-modal-danger:hover:not(:disabled) {
    box-shadow: 0 6px 16px rgba(239,68,68,0.35);
  }
  .btn-modal-danger:disabled { opacity: 0.4; cursor: not-allowed; }

  .adjust-hint {
    color: rgba(239,68,68,0.8) !important;
  }

  .promo-section {
    border: 1px solid rgba(255,255,255,0.08);
    border-radius: 12px;
    padding: 0.9rem;
    margin-bottom: 1rem;
  }

  .promo-section h4 {
    margin: 0 0 0.8rem;
    color: #fff;
    font-size: 0.95rem;
  }

  .promo-enabled {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.6rem;
    color: rgba(255,255,255,0.7);
    font-size: 0.85rem;
  }

  .promo-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-height: 220px;
    overflow-y: auto;
  }

  .promo-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.75rem;
    padding: 0.55rem 0.65rem;
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 8px;
    background: rgba(255,255,255,0.03);
  }

  .promo-row.disabled {
    opacity: 0.45;
  }

  .promo-info {
    display: flex;
    flex-direction: column;
    min-width: 0;
    gap: 0.15rem;
  }

  .promo-title {
    color: #fff;
    font-size: 0.86rem;
    font-weight: 600;
  }

  .promo-meta {
    color: rgba(255,255,255,0.5);
    font-size: 0.78rem;
  }

  .promo-actions {
    display: flex;
    gap: 0.3rem;
  }

  .promo-empty {
    color: rgba(255,255,255,0.45);
    font-size: 0.85rem;
    margin: 0;
  }
</style>
