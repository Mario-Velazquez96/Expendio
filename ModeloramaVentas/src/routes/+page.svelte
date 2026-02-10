<script lang="ts">
  import { authStore } from '$lib/stores/auth.svelte';
  import { getCurrentCashSession } from '$lib/api/cash';
  import type { CashSession } from '$lib/types';
  import Login from '$lib/components/Login.svelte';
  import CashOpen from '$lib/components/CashOpen.svelte';
  import POS from '$lib/components/POS.svelte';
  import CashPanel from '$lib/components/CashPanel.svelte';
  import Inventory from '$lib/components/Inventory.svelte';

  type Tab = 'pos' | 'cash' | 'inventory';

  // ── Estado principal ──
  let cashSession = $state<CashSession | null>(null);
  let checkingSession = $state(false);
  let activeTab = $state<Tab>('pos');

  // ── Handlers ──
  async function handleLogin() {
    checkingSession = true;
    try {
      cashSession = await getCurrentCashSession();
    } catch (e) {
      console.error('Error al verificar caja:', e);
    } finally {
      checkingSession = false;
    }
  }

  function handleSessionOpened(session: CashSession) {
    cashSession = session;
    activeTab = 'pos';
  }

  function handleSessionClosed() {
    cashSession = null;
    activeTab = 'pos';
  }

  function handleLogout() {
    authStore.logout();
    cashSession = null;
    activeTab = 'pos';
  }
</script>

{#if !authStore.isAuthenticated}
  <!-- ═══ LOGIN ═══ -->
  <Login onLogin={handleLogin} />

{:else if checkingSession}
  <!-- ═══ Verificando sesión de caja ═══ -->
  <div class="loading-screen">
    <div class="loading-card">
      <div class="spinner"></div>
      <p>Verificando caja...</p>
    </div>
  </div>

{:else if !cashSession}
  <!-- ═══ APERTURA DE CAJA ═══ -->
  <CashOpen
    user={authStore.user!}
    onSessionOpened={handleSessionOpened}
    onLogout={handleLogout}
  />

{:else}
  <!-- ═══ APLICACIÓN PRINCIPAL ═══ -->
  <div class="app-shell">
    <!-- Header -->
    <header class="app-header">
      <div class="header-left">
        <span class="brand">🍺 BeerPOS</span>
      </div>

      <nav class="header-tabs">
        <button
          class="tab-btn"
          class:active={activeTab === 'pos'}
          onclick={() => activeTab = 'pos'}
        >
          🛒 POS
        </button>
        <button
          class="tab-btn"
          class:active={activeTab === 'cash'}
          onclick={() => activeTab = 'cash'}
        >
          💰 Caja
        </button>
        <button
          class="tab-btn"
          class:active={activeTab === 'inventory'}
          onclick={() => activeTab = 'inventory'}
        >
          📦 Inventario
        </button>
      </nav>

      <div class="header-right">
        <div class="user-info">
          <span class="user-name">{authStore.user?.name}</span>
          <span class="user-role">{authStore.user?.role === 'OWNER' ? 'Dueño' : 'Empleado'}</span>
        </div>
        <button class="logout-btn" onclick={handleLogout} title="Cerrar sesión">
          🚪
        </button>
      </div>
    </header>

    <!-- Content -->
    <main class="app-content">
      {#if activeTab === 'pos'}
        <POS session={cashSession} />
      {:else if activeTab === 'cash'}
        <CashPanel
          session={cashSession}
          user={authStore.user!}
          onSessionClosed={handleSessionClosed}
        />
      {:else if activeTab === 'inventory'}
        <Inventory />
      {/if}
    </main>
  </div>
{/if}

<style>
  /* ═══ Loading Screen ═══ */
  .loading-screen {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, #1a1a2e 0%, #16213e 50%, #0f3460 100%);
  }

  .loading-card {
    text-align: center;
    color: rgba(255, 255, 255, 0.6);
  }

  .spinner {
    width: 40px;
    height: 40px;
    border: 3px solid rgba(255, 255, 255, 0.1);
    border-top-color: #f59e0b;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin: 0 auto 1rem;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* ═══ App Shell ═══ */
  .app-shell {
    height: 100vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  /* ═══ Header ═══ */
  .app-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 1.25rem;
    height: 56px;
    background: rgba(0, 0, 0, 0.3);
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    flex-shrink: 0;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .brand {
    font-size: 1.15rem;
    font-weight: 700;
    color: #fff;
    letter-spacing: -0.3px;
  }

  .header-tabs {
    display: flex;
    gap: 0.25rem;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 10px;
    padding: 0.25rem;
  }

  .tab-btn {
    padding: 0.45rem 1.25rem;
    border-radius: 8px;
    border: none;
    background: transparent;
    color: rgba(255, 255, 255, 0.5);
    font-size: 0.9rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
  }

  .tab-btn.active {
    background: rgba(245, 158, 11, 0.2);
    color: #f59e0b;
    font-weight: 600;
  }

  .tab-btn:hover:not(.active) {
    background: rgba(255, 255, 255, 0.08);
    color: rgba(255, 255, 255, 0.8);
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .user-info {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 0;
    line-height: 1.2;
  }

  .user-name {
    font-size: 0.9rem;
    font-weight: 600;
    color: #fff;
  }

  .user-role {
    font-size: 0.7rem;
    color: rgba(255, 255, 255, 0.4);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .logout-btn {
    width: 36px;
    height: 36px;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(255, 255, 255, 0.04);
    font-size: 1.1rem;
    cursor: pointer;
    transition: all 0.15s;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .logout-btn:hover {
    background: rgba(239, 68, 68, 0.15);
    border-color: rgba(239, 68, 68, 0.3);
  }

  /* ═══ Content ═══ */
  .app-content {
    flex: 1;
    overflow: hidden;
  }
</style>
