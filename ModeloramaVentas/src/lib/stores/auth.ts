// Re-exportación de tipos para compatibilidad con imports existentes
// El store real está en auth.svelte.ts

export interface User {
  id: number;
  name: string;
  role: 'OWNER' | 'EMPLOYEE';
}

export interface AuthState {
  user: User | null;
  isAuthenticated: boolean;
}
