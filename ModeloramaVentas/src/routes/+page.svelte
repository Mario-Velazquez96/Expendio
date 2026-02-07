<script lang="ts">
  import { onMount } from "svelte";
  
  // Importar invoke dinámicamente solo en el cliente
  let invoke: any = null;
  let isTauri = false;
  
  onMount(async () => {
    try {
      // Solo importar en el cliente (no en SSR)
      if (typeof window !== 'undefined') {
        const tauriCore = await import("@tauri-apps/api/core");
        invoke = tauriCore.invoke;
        isTauri = tauriCore.isTauri();
        
        if (isTauri) {
          await loadDbPath();
        }
      }
    } catch (e) {
      console.warn("Tauri no está disponible:", e);
      isTauri = false;
    }
  });

  let name = $state("");
  let greetMsg = $state("");
  let dbPath = $state("");
  let testResults = $state<{
    connection?: any;
    tables?: any;
    query?: any;
  }>({});

  async function greet(event: Event) {
    event.preventDefault();
    if (!invoke || !isTauri) {
      greetMsg = "Error: Tauri no está disponible";
      return;
    }
    try {
      greetMsg = await invoke("greet", { name });
    } catch (error) {
      greetMsg = `Error: ${error}`;
    }
  }

  async function loadDbPath() {
    if (!invoke || !isTauri) {
      dbPath = "Error: Tauri no está disponible";
      return;
    }
    try {
      dbPath = await invoke("get_db_path");
    } catch (error) {
      dbPath = `Error: ${error}`;
    }
  }

  async function testConnection() {
    if (!invoke || !isTauri) {
      testResults.connection = { success: false, message: "Error: Tauri no está disponible" };
      return;
    }
    try {
      testResults.connection = await invoke("test_db_connection");
    } catch (error) {
      testResults.connection = { success: false, message: `Error: ${error}` };
    }
  }

  async function testTables() {
    if (!invoke || !isTauri) {
      testResults.tables = { success: false, message: "Error: Tauri no está disponible" };
      return;
    }
    try {
      testResults.tables = await invoke("test_db_tables");
    } catch (error) {
      testResults.tables = { success: false, message: `Error: ${error}` };
    }
  }

  async function testQuery() {
    if (!invoke || !isTauri) {
      testResults.query = { success: false, message: "Error: Tauri no está disponible" };
      return;
    }
    try {
      testResults.query = await invoke("test_db_query");
    } catch (error) {
      testResults.query = { success: false, message: `Error: ${error}` };
    }
  }

  async function runAllTests() {
    await loadDbPath();
    await testConnection();
    await testTables();
    await testQuery();
  }

</script>

<main class="container">
  <h1>🧪 Pruebas de Base de Datos</h1>

  <div class="db-section">
    <h2>📁 Ubicación de la Base de Datos</h2>
    <div class="db-path">
      <code>{dbPath || "Cargando..."}</code>
      <button onclick={loadDbPath}>🔄 Actualizar</button>
    </div>
  </div>

  <div class="db-section">
    <h2>🔍 Pruebas</h2>
    <div class="test-buttons">
      <button onclick={testConnection} class="test-btn">1. Probar Conexión</button>
      <button onclick={testTables} class="test-btn">2. Listar Tablas</button>
      <button onclick={testQuery} class="test-btn">3. Contar Registros</button>
      <button onclick={runAllTests} class="test-btn primary">▶️ Ejecutar Todas</button>
    </div>
  </div>

  <div class="results">
    {#if testResults.connection}
      <div class="result-card">
        <h3>🔌 Prueba de Conexión</h3>
        <p class={testResults.connection.success ? "success" : "error"}>
          {testResults.connection.message}
        </p>
      </div>
    {/if}

    {#if testResults.tables}
      <div class="result-card">
        <h3>📊 Tablas en la Base de Datos</h3>
        <p class={testResults.tables.success ? "success" : "error"}>
          {testResults.tables.message}
        </p>
        {#if testResults.tables.details}
          <div class="details">
            <p><strong>Total:</strong> {testResults.tables.details.count} tablas</p>
            <ul>
              {#each testResults.tables.details.tables as table}
                <li>{table}</li>
              {/each}
            </ul>
          </div>
        {/if}
      </div>
    {/if}

    {#if testResults.query}
      <div class="result-card">
        <h3>📈 Conteo de Registros</h3>
        <p class={testResults.query.success ? "success" : "error"}>
          {testResults.query.message}
        </p>
        {#if testResults.query.details}
          <div class="details">
            <table>
              <thead>
                <tr>
                  <th>Tabla</th>
                  <th>Registros</th>
                </tr>
              </thead>
              <tbody>
                {#each Object.entries(testResults.query.details) as [table, count]}
                  <tr>
                    <td>{table}</td>
                    <td>{count}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <div class="db-section">
    <h2>💬 Comando de Prueba Original</h2>
    <form class="row" onsubmit={greet}>
      <input id="greet-input" placeholder="Enter a name..." bind:value={name} />
      <button type="submit">Greet</button>
    </form>
    <p>{greetMsg}</p>
  </div>
</main>

<style>
.logo.vite:hover {
  filter: drop-shadow(0 0 2em #747bff);
}

.logo.svelte-kit:hover {
  filter: drop-shadow(0 0 2em #ff3e00);
}

:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: #0f0f0f;
  background-color: #f6f6f6;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

.container {
  margin: 0 auto;
  padding: 2rem;
  max-width: 900px;
}

.db-section {
  margin: 2rem 0;
  padding: 1.5rem;
  background: #ffffff;
  border-radius: 8px;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.db-section h2 {
  margin-top: 0;
  color: #396cd8;
}

.db-path {
  display: flex;
  gap: 1rem;
  align-items: center;
  flex-wrap: wrap;
}

.db-path code {
  flex: 1;
  padding: 0.5rem;
  background: #f5f5f5;
  border-radius: 4px;
  font-family: 'Courier New', monospace;
  word-break: break-all;
}

.test-buttons {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.test-btn {
  padding: 0.75rem 1.5rem;
  background-color: #646cff;
  color: white;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-weight: 500;
  transition: background-color 0.25s;
}

.test-btn:hover {
  background-color: #535bf2;
}

.test-btn.primary {
  background-color: #24c8db;
}

.test-btn.primary:hover {
  background-color: #1fa8b8;
}

.results {
  margin-top: 2rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.result-card {
  padding: 1.5rem;
  background: #ffffff;
  border-radius: 8px;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  border-left: 4px solid #646cff;
}

.result-card h3 {
  margin-top: 0;
  color: #396cd8;
}

.result-card .success {
  color: #22c55e;
  font-weight: 500;
}

.result-card .error {
  color: #ef4444;
  font-weight: 500;
}

.details {
  margin-top: 1rem;
  padding-top: 1rem;
  border-top: 1px solid #e5e5e5;
}

.details ul {
  list-style-type: none;
  padding: 0;
  margin: 0.5rem 0;
}

.details li {
  padding: 0.25rem 0;
  padding-left: 1rem;
  position: relative;
}

.details li::before {
  content: "▸";
  position: absolute;
  left: 0;
  color: #646cff;
}

.details table {
  width: 100%;
  border-collapse: collapse;
  margin-top: 0.5rem;
}

.details th,
.details td {
  padding: 0.5rem;
  text-align: left;
  border-bottom: 1px solid #e5e5e5;
}

.details th {
  background-color: #f5f5f5;
  font-weight: 600;
}

.logo {
  height: 6em;
  padding: 1.5em;
  will-change: filter;
  transition: 0.75s;
}

.logo.tauri:hover {
  filter: drop-shadow(0 0 2em #24c8db);
}

.row {
  display: flex;
  justify-content: center;
}

a {
  font-weight: 500;
  color: #646cff;
  text-decoration: inherit;
}

a:hover {
  color: #535bf2;
}

h1 {
  text-align: center;
}

input,
button {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  color: #0f0f0f;
  background-color: #ffffff;
  transition: border-color 0.25s;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
}

button {
  cursor: pointer;
}

button:hover {
  border-color: #396cd8;
}
button:active {
  border-color: #396cd8;
  background-color: #e8e8e8;
}

input,
button {
  outline: none;
}

#greet-input {
  margin-right: 5px;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #2f2f2f;
  }

  a:hover {
    color: #24c8db;
  }

  input,
  button {
    color: #ffffff;
    background-color: #0f0f0f98;
  }
  button:active {
    background-color: #0f0f0f69;
  }
}

</style>
