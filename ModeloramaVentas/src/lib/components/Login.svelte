<script lang="ts">
  import { onMount } from 'svelte';
  import { authStore } from '$lib/stores/auth.svelte';
  import { loginWithPin, getUserById } from '$lib/api/auth';

  let pin = $state('');
  let error = $state('');
  let loading = $state(false);
  let initialized = $state(false);

  onMount(async () => {
    // Intentar restaurar sesión
    const savedUserId = authStore.getSavedUserId();
    if (savedUserId) {
      try {
        const user = await getUserById(savedUserId);
        if (user) {
          authStore.setUser(user);
        }
      } catch (e) {
        console.warn('No se pudo restaurar la sesión:', e);
      }
    }
    initialized = true;
  });

  async function handleLogin(e: Event) {
    e.preventDefault();
    error = '';
    
    if (pin.length !== 6) {
      error = 'El PIN debe ser de 6 dígitos';
      return;
    }

    loading = true;
    try {
      const result = await loginWithPin(pin);
      if (result.success && result.user) {
        authStore.setUser(result.user);
        pin = '';
      } else {
        error = result.error || 'PIN incorrecto';
      }
    } catch (e) {
      error = `Error: ${e}`;
    } finally {
      loading = false;
    }
  }

  function handlePinInput(e: Event) {
    const input = e.target as HTMLInputElement;
    // Solo permitir dígitos
    input.value = input.value.replace(/\D/g, '').slice(0, 6);
    pin = input.value;
  }
</script>

{#if !initialized}
  <div class="login-container">
    <div class="login-card">
      <div class="loading-spinner"></div>
      <p>Cargando...</p>
    </div>
  </div>
{:else}
  <div class="login-container">
    <div class="login-card">
      <div class="logo">🍺</div>
      <h1>BeerPOS</h1>
      <p class="subtitle">Ingresa tu PIN para continuar</p>
      
      <form onsubmit={handleLogin}>
        <div class="pin-input-container">
          <input
            type="password"
            inputmode="numeric"
            pattern="[0-9]*"
            maxlength="6"
            placeholder="••••••"
            value={pin}
            oninput={handlePinInput}
            disabled={loading}
            class="pin-input"
            autofocus
          />
          <div class="pin-dots">
            {#each Array(6) as _, i}
              <span class="dot" class:filled={pin.length > i}></span>
            {/each}
          </div>
        </div>

        {#if error}
          <p class="error">{error}</p>
        {/if}

        <button type="submit" disabled={loading || pin.length !== 6} class="login-btn">
          {loading ? 'Verificando...' : 'Ingresar'}
        </button>
      </form>

      <p class="hint">PIN de 6 dígitos</p>
    </div>
  </div>
{/if}

<style>
  .login-container {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%);
    padding: 1rem;
  }

  .login-card {
    background: rgba(255, 255, 255, 0.05);
    backdrop-filter: blur(10px);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 24px;
    padding: 3rem 2.5rem;
    text-align: center;
    max-width: 380px;
    width: 100%;
    box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);
  }

  .logo {
    font-size: 4rem;
    margin-bottom: 0.5rem;
  }

  h1 {
    color: #fff;
    font-size: 2rem;
    font-weight: 700;
    margin: 0 0 0.5rem 0;
    letter-spacing: -0.5px;
  }

  .subtitle {
    color: rgba(255, 255, 255, 0.6);
    margin: 0 0 2rem 0;
    font-size: 0.95rem;
  }

  .pin-input-container {
    position: relative;
    margin-bottom: 1.5rem;
  }

  .pin-input {
    width: 100%;
    padding: 1rem;
    font-size: 2rem;
    text-align: center;
    letter-spacing: 0.5rem;
    background: rgba(255, 255, 255, 0.08);
    border: 2px solid rgba(255, 255, 255, 0.15);
    border-radius: 12px;
    color: transparent;
    caret-color: #f59e0b;
    outline: none;
    transition: all 0.2s ease;
  }

  .pin-input:focus {
    border-color: #f59e0b;
    background: rgba(255, 255, 255, 0.12);
  }

  .pin-input::placeholder {
    color: rgba(255, 255, 255, 0.3);
    letter-spacing: 0.8rem;
  }

  .pin-dots {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    display: flex;
    gap: 0.75rem;
    pointer-events: none;
  }

  .dot {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.2);
    transition: all 0.15s ease;
  }

  .dot.filled {
    background: #f59e0b;
    box-shadow: 0 0 10px rgba(245, 158, 11, 0.5);
  }

  .error {
    color: #ef4444;
    font-size: 0.9rem;
    margin: 0 0 1rem 0;
    padding: 0.75rem;
    background: rgba(239, 68, 68, 0.1);
    border-radius: 8px;
    border: 1px solid rgba(239, 68, 68, 0.2);
  }

  .login-btn {
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
  }

  .login-btn:hover:not(:disabled) {
    transform: translateY(-2px);
    box-shadow: 0 10px 20px rgba(245, 158, 11, 0.3);
  }

  .login-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .hint {
    color: rgba(255, 255, 255, 0.4);
    font-size: 0.8rem;
    margin: 1.5rem 0 0 0;
  }

  .loading-spinner {
    width: 40px;
    height: 40px;
    border: 3px solid rgba(255, 255, 255, 0.1);
    border-top-color: #f59e0b;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin: 0 auto 1rem;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
