<script lang="ts">
  import { onMount } from 'svelte';
  import {
    addCashMovement,
    getCashSessionSummary,
    closeCashSession
  } from '$lib/api/cash';
  import { getSessionSales } from '$lib/api/sales';
  import { formatMoney, pesosToCents, formatTime, formatDateTime } from '$lib/helpers/money';
  import type { User, CashSession, CashSessionSummary } from '$lib/types';

  let { session, user, onSessionClosed }: {
    session: CashSession;
    user: User;
    onSessionClosed: () => void;
  } = $props();

  // ── Estado del panel ──
  let summary = $state<CashSessionSummary | null>(null);
  let summaryLoading = $state(true);
  let error = $state('');

  // ── Formulario de movimiento ──
  let movType = $state<'DEPOSIT' | 'WITHDRAWAL'>('WITHDRAWAL');
  let movAmount = $state('');
  let movReason = $state('');
  let movLoading = $state(false);

  // ── Flujo de cierre ──
  let showClose = $state(false);
  let countedAmount = $state('');
  let closeNote = $state('');
  let closeLoading = $state(false);
  let closeDifference = $state<number | null>(null);

  // ── Ventas de la sesión ──
  let salesCount = $state(0);
  let salesTotalCents = $state(0);

  onMount(async () => {
    await loadSummary();
  });

  async function loadSummary() {
    summaryLoading = true;
    error = '';
    try {
      summary = await getCashSessionSummary(session.id);
      // Cargar resumen de ventas
      const salesData = await getSessionSales(session.id);
      salesCount = salesData.count;
      salesTotalCents = salesData.total_cents;
    } catch (e) {
      error = `Error al cargar resumen: ${e}`;
    } finally {
      summaryLoading = false;
    }
  }

  async function handleAddMovement(e: Event) {
    e.preventDefault();
    error = '';

    const amount = parseFloat(movAmount);
    if (isNaN(amount) || amount <= 0) {
      error = 'El monto debe ser mayor a 0';
      return;
    }
    if (!movReason.trim()) {
      error = 'Debes indicar una razón para el movimiento';
      return;
    }

    movLoading = true;
    try {
      await addCashMovement(
        session.id,
        movType,
        pesosToCents(amount),
        movReason.trim(),
        user.id
      );
      // Limpiar formulario y recargar
      movAmount = '';
      movReason = '';
      await loadSummary();
    } catch (e) {
      error = String(e);
    } finally {
      movLoading = false;
    }
  }

  function handleStartClose() {
    showClose = true;
    countedAmount = '';
    closeNote = '';
    closeDifference = null;
  }

  function handleCancelClose() {
    showClose = false;
    closeDifference = null;
  }

  function calculateDifference() {
    if (!summary) return;
    const counted = parseFloat(countedAmount);
    if (isNaN(counted)) {
      closeDifference = null;
      return;
    }
    closeDifference = pesosToCents(counted) - summary.expected_cash_cents;
  }

  async function handleClose() {
    if (!summary) return;
    error = '';

    const counted = parseFloat(countedAmount);
    if (isNaN(counted) || counted < 0) {
      error = 'Ingresa el efectivo contado';
      return;
    }

    const countedCents = pesosToCents(counted);
    const diff = countedCents - summary.expected_cash_cents;

    // Si hay diferencia, la nota es obligatoria
    if (diff !== 0 && !closeNote.trim()) {
      error = `Hay una diferencia de ${formatMoney(diff)}. Debes agregar una nota explicativa.`;
      return;
    }

    closeLoading = true;
    try {
      await closeCashSession(
        session.id,
        user.id,
        countedCents,
        closeNote.trim() || undefined
      );
      onSessionClosed();
    } catch (e) {
      error = String(e);
    } finally {
      closeLoading = false;
    }
  }
</script>

<div class="cash-layout">
  {#if summaryLoading}
    <div class="loading-center">
      <div class="spinner"></div>
      <p>Cargando información de caja...</p>
    </div>
  {:else if showClose}
    <!-- ═══ Flujo de cierre ═══ -->
    <div class="close-flow">
      <div class="close-card">
        <h2>🔒 Cierre de Caja</h2>
        <p class="close-subtitle">Sesión #{session.id} · Abierta {formatDateTime(session.opened_at)}</p>

        {#if summary}
          <div class="close-summary">
            <div class="summary-row">
              <span>Fondo inicial</span>
              <span>{formatMoney(summary.session.opening_float_cents)}</span>
            </div>
            <div class="summary-row">
              <span>Ventas en efectivo ({summary.total_sales_count})</span>
              <span class="positive">+{formatMoney(summary.total_sales_cash_cents)}</span>
            </div>
            <div class="summary-row">
              <span>Depósitos</span>
              <span class="positive">+{formatMoney(summary.total_deposits_cents)}</span>
            </div>
            <div class="summary-row">
              <span>Retiros</span>
              <span class="negative">-{formatMoney(summary.total_withdrawals_cents)}</span>
            </div>
            <div class="summary-row expected">
              <span>Efectivo esperado</span>
              <span>{formatMoney(summary.expected_cash_cents)}</span>
            </div>
          </div>
        {/if}

        <div class="close-form">
          <div class="field">
            <label for="counted">Efectivo contado ($)</label>
            <div class="input-wrapper">
              <span class="currency">$</span>
              <input
                id="counted"
                type="number"
                step="0.01"
                min="0"
                placeholder="0.00"
                bind:value={countedAmount}
                oninput={calculateDifference}
                disabled={closeLoading}
                autofocus
              />
            </div>
          </div>

          {#if closeDifference !== null}
            <div class="difference" class:ok={closeDifference === 0} class:bad={closeDifference !== 0}>
              <span>Diferencia:</span>
              <span class="diff-amount">{formatMoney(closeDifference)}</span>
              {#if closeDifference === 0}
                <span class="diff-icon">✓</span>
              {:else}
                <span class="diff-icon">⚠️</span>
              {/if}
            </div>
          {/if}

          {#if closeDifference !== null && closeDifference !== 0}
            <div class="field">
              <label for="close-note">Nota explicativa (obligatoria)</label>
              <textarea
                id="close-note"
                rows="3"
                placeholder="Explica la diferencia..."
                bind:value={closeNote}
                disabled={closeLoading}
              ></textarea>
            </div>
          {/if}

          {#if error}
            <div class="error-msg">{error}</div>
          {/if}

          <div class="close-actions">
            <button class="btn-secondary" onclick={handleCancelClose} disabled={closeLoading}>
              ← Volver
            </button>
            <button class="btn-primary" onclick={handleClose} disabled={closeLoading || !countedAmount}>
              {closeLoading ? 'Cerrando...' : '🔒 Confirmar Cierre'}
            </button>
          </div>
        </div>
      </div>
    </div>
  {:else}
    <!-- ═══ Vista normal ═══ -->
    <div class="cash-content">
      <!-- Información de la sesión -->
      <div class="session-info-card">
        <div class="session-header">
          <h2>💰 Sesión de Caja #{session.id}</h2>
          <span class="status-badge open">ABIERTA</span>
        </div>
        <div class="session-details">
          <div class="detail-item">
            <span class="label">Abierta</span>
            <span class="value">{formatDateTime(session.opened_at)}</span>
          </div>
          <div class="detail-item">
            <span class="label">Fondo inicial</span>
            <span class="value">{formatMoney(session.opening_float_cents)}</span>
          </div>
          <div class="detail-item">
            <span class="label">Ventas</span>
            <span class="value">{salesCount} · {formatMoney(salesTotalCents)}</span>
          </div>
          {#if summary}
            <div class="detail-item highlight">
              <span class="label">Efectivo esperado</span>
              <span class="value">{formatMoney(summary.expected_cash_cents)}</span>
            </div>
          {/if}
        </div>
      </div>

      <!-- Agregar movimiento -->
      <div class="section-card">
        <h3>📝 Registrar Movimiento</h3>
        <form onsubmit={handleAddMovement}>
          <div class="mov-type-toggle">
            <button
              type="button"
              class="toggle-btn"
              class:active={movType === 'WITHDRAWAL'}
              onclick={() => movType = 'WITHDRAWAL'}
            >
              📤 Retiro
            </button>
            <button
              type="button"
              class="toggle-btn"
              class:active={movType === 'DEPOSIT'}
              onclick={() => movType = 'DEPOSIT'}
            >
              📥 Depósito
            </button>
          </div>

          <div class="form-row">
            <div class="field compact">
              <label for="mov-amount">Monto ($)</label>
              <input
                id="mov-amount"
                type="number"
                step="0.01"
                min="0.01"
                placeholder="0.00"
                bind:value={movAmount}
                disabled={movLoading}
              />
            </div>
            <div class="field compact flex-1">
              <label for="mov-reason">Razón</label>
              <input
                id="mov-reason"
                type="text"
                placeholder="Ej: Cambio para tortillería"
                bind:value={movReason}
                disabled={movLoading}
              />
            </div>
          </div>

          <button type="submit" class="btn-action" disabled={movLoading || !movAmount || !movReason.trim()}>
            {movLoading ? 'Registrando...' : 'Registrar'}
          </button>
        </form>
      </div>

      <!-- Lista de movimientos -->
      {#if summary && summary.movements.length > 0}
        <div class="section-card">
          <h3>📋 Movimientos del Día</h3>
          <div class="movements-list">
            {#each summary.movements as mov (mov.id)}
              <div class="movement-item">
                <span class="mov-time">{formatTime(mov.created_at)}</span>
                <span class="mov-type-badge" class:deposit={mov.type === 'DEPOSIT'} class:withdrawal={mov.type === 'WITHDRAWAL'}>
                  {mov.type === 'DEPOSIT' ? '📥 Depósito' : '📤 Retiro'}
                </span>
                <span class="mov-reason">{mov.reason}</span>
                <span class="mov-amount" class:deposit={mov.type === 'DEPOSIT'} class:withdrawal={mov.type === 'WITHDRAWAL'}>
                  {mov.type === 'DEPOSIT' ? '+' : '-'}{formatMoney(mov.amount_cents)}
                </span>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      {#if error}
        <div class="error-msg">{error}</div>
      {/if}

      <!-- Botón de cierre -->
      <button class="btn-close-session" onclick={handleStartClose}>
        🔒 Cerrar Caja
      </button>
    </div>
  {/if}
</div>

<style>
  .cash-layout {
    height: 100%;
    overflow-y: auto;
  }

  .loading-center {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: rgba(255, 255, 255, 0.5);
    gap: 1rem;
  }

  .spinner {
    width: 36px;
    height: 36px;
    border: 3px solid rgba(255, 255, 255, 0.1);
    border-top-color: #f59e0b;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* ═══ Vista Normal ═══ */
  .cash-content {
    max-width: 720px;
    margin: 0 auto;
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .session-info-card {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 16px;
    padding: 1.5rem;
  }

  .session-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }

  .session-header h2 {
    font-size: 1.15rem;
    font-weight: 600;
    color: #fff;
    margin: 0;
  }

  .status-badge {
    padding: 0.3rem 0.75rem;
    border-radius: 20px;
    font-size: 0.75rem;
    font-weight: 700;
    letter-spacing: 0.5px;
  }

  .status-badge.open {
    background: rgba(34, 197, 94, 0.15);
    color: #22c55e;
    border: 1px solid rgba(34, 197, 94, 0.3);
  }

  .session-details {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.75rem;
  }

  .detail-item {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .detail-item .label {
    font-size: 0.8rem;
    color: rgba(255, 255, 255, 0.45);
  }

  .detail-item .value {
    font-size: 1rem;
    font-weight: 600;
    color: #fff;
  }

  .detail-item.highlight .value {
    color: #f59e0b;
    font-size: 1.15rem;
  }

  /* ═══ Section Card ═══ */
  .section-card {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 16px;
    padding: 1.5rem;
  }

  .section-card h3 {
    font-size: 1rem;
    font-weight: 600;
    color: #fff;
    margin: 0 0 1rem 0;
  }

  .mov-type-toggle {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }

  .toggle-btn {
    flex: 1;
    padding: 0.65rem;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: rgba(255, 255, 255, 0.04);
    color: rgba(255, 255, 255, 0.5);
    font-size: 0.9rem;
    cursor: pointer;
    transition: all 0.15s;
  }

  .toggle-btn.active {
    background: rgba(245, 158, 11, 0.15);
    border-color: rgba(245, 158, 11, 0.4);
    color: #f59e0b;
    font-weight: 600;
  }

  .toggle-btn:hover:not(.active) {
    background: rgba(255, 255, 255, 0.08);
    color: rgba(255, 255, 255, 0.7);
  }

  .form-row {
    display: flex;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .field.compact {
    min-width: 0;
  }

  .field.flex-1 {
    flex: 1;
  }

  .field label {
    font-size: 0.8rem;
    color: rgba(255, 255, 255, 0.5);
    font-weight: 500;
  }

  input, textarea {
    padding: 0.7rem 0.85rem;
    font-size: 0.95rem;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    color: #fff;
    outline: none;
    transition: border-color 0.15s;
    font-family: inherit;
  }

  input:focus, textarea:focus {
    border-color: #f59e0b;
  }

  input::placeholder, textarea::placeholder {
    color: rgba(255, 255, 255, 0.25);
  }

  input:disabled, textarea:disabled {
    opacity: 0.5;
  }

  /* Ocultar flechas del input number */
  input[type="number"]::-webkit-inner-spin-button,
  input[type="number"]::-webkit-outer-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }
  input[type="number"] {
    -moz-appearance: textfield;
  }

  textarea {
    resize: vertical;
    min-height: 60px;
  }

  .btn-action {
    width: 100%;
    padding: 0.75rem;
    font-size: 0.95rem;
    font-weight: 600;
    color: #1a1a2e;
    background: linear-gradient(135deg, #f59e0b 0%, #d97706 100%);
    border: none;
    border-radius: 10px;
    cursor: pointer;
    transition: all 0.15s;
  }

  .btn-action:hover:not(:disabled) {
    box-shadow: 0 4px 12px rgba(245, 158, 11, 0.3);
  }

  .btn-action:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  /* ═══ Movimientos ═══ */
  .movements-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .movement-item {
    display: grid;
    grid-template-columns: auto auto 1fr auto;
    align-items: center;
    gap: 0.75rem;
    padding: 0.65rem 0.75rem;
    background: rgba(255, 255, 255, 0.03);
    border-radius: 8px;
  }

  .mov-time {
    font-size: 0.8rem;
    color: rgba(255, 255, 255, 0.4);
    font-family: monospace;
  }

  .mov-type-badge {
    font-size: 0.75rem;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    font-weight: 500;
  }

  .mov-type-badge.deposit {
    background: rgba(34, 197, 94, 0.1);
    color: #22c55e;
  }

  .mov-type-badge.withdrawal {
    background: rgba(239, 68, 68, 0.1);
    color: #ef4444;
  }

  .mov-reason {
    font-size: 0.85rem;
    color: rgba(255, 255, 255, 0.6);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .mov-amount {
    font-weight: 600;
    font-size: 0.9rem;
    text-align: right;
  }

  .mov-amount.deposit {
    color: #22c55e;
  }

  .mov-amount.withdrawal {
    color: #ef4444;
  }

  .error-msg {
    padding: 0.75rem 1rem;
    background: rgba(239, 68, 68, 0.12);
    border: 1px solid rgba(239, 68, 68, 0.25);
    border-radius: 8px;
    color: #ef4444;
    font-size: 0.9rem;
  }

  .btn-close-session {
    width: 100%;
    padding: 1rem;
    font-size: 1.05rem;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.7);
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.25);
    border-radius: 12px;
    cursor: pointer;
    transition: all 0.15s;
    margin-top: 0.5rem;
  }

  .btn-close-session:hover {
    background: rgba(239, 68, 68, 0.2);
    border-color: rgba(239, 68, 68, 0.4);
    color: #ef4444;
  }

  /* ═══ Flujo de Cierre ═══ */
  .close-flow {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100%;
    padding: 2rem;
  }

  .close-card {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 20px;
    padding: 2rem;
    max-width: 520px;
    width: 100%;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.3);
  }

  .close-card h2 {
    font-size: 1.5rem;
    font-weight: 700;
    color: #fff;
    margin: 0 0 0.5rem 0;
    text-align: center;
  }

  .close-subtitle {
    text-align: center;
    color: rgba(255, 255, 255, 0.4);
    font-size: 0.85rem;
    margin: 0 0 1.5rem 0;
  }

  .close-summary {
    background: rgba(0, 0, 0, 0.2);
    border-radius: 12px;
    padding: 1rem 1.25rem;
    margin-bottom: 1.5rem;
  }

  .summary-row {
    display: flex;
    justify-content: space-between;
    padding: 0.5rem 0;
    font-size: 0.95rem;
    color: rgba(255, 255, 255, 0.7);
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
  }

  .summary-row:last-child {
    border-bottom: none;
  }

  .summary-row.expected {
    padding-top: 0.75rem;
    margin-top: 0.25rem;
    border-top: 2px solid rgba(245, 158, 11, 0.3);
    border-bottom: none;
    font-weight: 700;
    color: #f59e0b;
    font-size: 1.1rem;
  }

  .positive {
    color: #22c55e;
  }

  .negative {
    color: #ef4444;
  }

  .close-form {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .input-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .currency {
    position: absolute;
    left: 0.85rem;
    color: #f59e0b;
    font-size: 1.1rem;
    font-weight: 600;
    pointer-events: none;
  }

  .input-wrapper input {
    padding-left: 2.2rem;
    font-size: 1.2rem;
    font-weight: 600;
  }

  .difference {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.85rem 1rem;
    border-radius: 10px;
    font-weight: 600;
  }

  .difference.ok {
    background: rgba(34, 197, 94, 0.1);
    border: 1px solid rgba(34, 197, 94, 0.25);
    color: #22c55e;
  }

  .difference.bad {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.25);
    color: #ef4444;
  }

  .diff-amount {
    flex: 1;
    text-align: right;
    font-size: 1.1rem;
  }

  .diff-icon {
    font-size: 1.2rem;
  }

  .close-actions {
    display: flex;
    gap: 0.75rem;
    margin-top: 0.5rem;
  }

  .btn-secondary {
    flex: 0 0 auto;
    padding: 0.85rem 1.25rem;
    font-size: 0.95rem;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.6);
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 10px;
    cursor: pointer;
    transition: all 0.15s;
  }

  .btn-secondary:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
    color: #fff;
  }

  .btn-secondary:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .btn-primary {
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

  .btn-primary:hover:not(:disabled) {
    box-shadow: 0 6px 16px rgba(245, 158, 11, 0.35);
  }

  .btn-primary:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
