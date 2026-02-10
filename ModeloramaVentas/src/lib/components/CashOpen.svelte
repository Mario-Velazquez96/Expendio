<script lang="ts">
  import { openCashSession } from '$lib/api/cash';
  import { pesosToCents } from '$lib/helpers/money';
  import type { User, CashSession } from '$lib/types';

  let { user, onSessionOpened, onLogout }: {
    user: User;
    onSessionOpened: (session: CashSession) => void;
    onLogout: () => void;
  } = $props();

  let amount = $state('');
  let note = $state('');
  let loading = $state(false);
  let error = $state('');

  async function handleOpen(e: Event) {
    e.preventDefault();
    error = '';

    const amountNum = parseFloat(amount);
    if (isNaN(amountNum) || amountNum < 0) {
      error = 'Ingresa un monto válido (0 o más)';
      return;
    }

    loading = true;
    try {
      const session = await openCashSession(
        user.id,
        pesosToCents(amountNum),
        note.trim() || undefined
      );
      onSessionOpened(session);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }
</script>

<div class="cash-open-container">
  <div class="cash-open-card">
    <div class="icon">🏦</div>
    <h1>Apertura de Caja</h1>
    <p class="welcome">Bienvenido, <strong>{user.name}</strong></p>
    <p class="subtitle">Ingresa el fondo inicial para comenzar</p>

    <form onsubmit={handleOpen}>
      <div class="field">
        <label for="amount">Fondo Inicial ($)</label>
        <div class="input-wrapper">
          <span class="currency">$</span>
          <input
            id="amount"
            type="number"
            step="0.01"
            min="0"
            placeholder="0.00"
            bind:value={amount}
            disabled={loading}
            autofocus
          />
        </div>
      </div>

      <div class="field">
        <label for="note">Nota (opcional)</label>
        <input
          id="note"
          type="text"
          placeholder="Ej: Turno matutino"
          bind:value={note}
          disabled={loading}
        />
      </div>

      {#if error}
        <div class="error-msg">
          <span>⚠️ {error}</span>
        </div>
      {/if}

      <button type="submit" class="open-btn" disabled={loading || !amount}>
        {loading ? 'Abriendo...' : '🔓 Abrir Caja'}
      </button>
    </form>

    <button class="logout-link" onclick={onLogout}>
      ← Cerrar sesión
    </button>
  </div>
</div>

<style>
  .cash-open-container {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%);
    padding: 1rem;
  }

  .cash-open-card {
    background: rgba(255, 255, 255, 0.05);
    backdrop-filter: blur(10px);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 24px;
    padding: 3rem 2.5rem;
    text-align: center;
    max-width: 420px;
    width: 100%;
    box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);
  }

  .icon {
    font-size: 3.5rem;
    margin-bottom: 0.5rem;
  }

  h1 {
    color: #fff;
    font-size: 1.8rem;
    font-weight: 700;
    margin: 0 0 0.5rem 0;
  }

  .welcome {
    color: #f59e0b;
    font-size: 1rem;
    margin: 0 0 0.25rem 0;
  }

  .subtitle {
    color: rgba(255, 255, 255, 0.5);
    font-size: 0.9rem;
    margin: 0 0 2rem 0;
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .field {
    text-align: left;
  }

  .field label {
    display: block;
    color: rgba(255, 255, 255, 0.7);
    font-size: 0.85rem;
    font-weight: 500;
    margin-bottom: 0.5rem;
  }

  .input-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .currency {
    position: absolute;
    left: 1rem;
    color: #f59e0b;
    font-size: 1.2rem;
    font-weight: 600;
    pointer-events: none;
  }

  .input-wrapper input {
    padding-left: 2.5rem;
  }

  input {
    width: 100%;
    padding: 0.85rem 1rem;
    font-size: 1rem;
    background: rgba(255, 255, 255, 0.08);
    border: 2px solid rgba(255, 255, 255, 0.12);
    border-radius: 10px;
    color: #fff;
    outline: none;
    transition: all 0.2s ease;
  }

  input:focus {
    border-color: #f59e0b;
    background: rgba(255, 255, 255, 0.12);
  }

  input::placeholder {
    color: rgba(255, 255, 255, 0.3);
  }

  input:disabled {
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

  .error-msg {
    padding: 0.75rem 1rem;
    background: rgba(239, 68, 68, 0.12);
    border: 1px solid rgba(239, 68, 68, 0.25);
    border-radius: 8px;
    color: #ef4444;
    font-size: 0.9rem;
    text-align: left;
  }

  .open-btn {
    width: 100%;
    padding: 1rem;
    font-size: 1.1rem;
    font-weight: 600;
    color: #1a1a2e;
    background: linear-gradient(135deg, #f59e0b 0%, #d97706 100%);
    border: none;
    border-radius: 12px;
    cursor: pointer;
    transition: all 0.2s ease;
    margin-top: 0.5rem;
  }

  .open-btn:hover:not(:disabled) {
    transform: translateY(-2px);
    box-shadow: 0 10px 20px rgba(245, 158, 11, 0.3);
  }

  .open-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .logout-link {
    margin-top: 1.5rem;
    background: none;
    border: none;
    color: rgba(255, 255, 255, 0.4);
    font-size: 0.85rem;
    cursor: pointer;
    transition: color 0.2s;
  }

  .logout-link:hover {
    color: rgba(255, 255, 255, 0.7);
  }
</style>
