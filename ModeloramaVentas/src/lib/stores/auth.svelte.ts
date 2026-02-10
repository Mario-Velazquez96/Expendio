// Store para el estado de autenticación del usuario actual

export interface User {
  id: number;
  name: string;
  role: 'OWNER' | 'EMPLOYEE';
}

export interface AuthState {
  user: User | null;
  isAuthenticated: boolean;
}

// Estado reactivo usando Svelte 5 runes
let currentUser = $state<User | null>(null);
let currentPin = $state<string>('');

export const authStore = {
  get user() {
    return currentUser;
  },
  get isAuthenticated() {
    return currentUser !== null;
  },
  get isOwner() {
    return currentUser?.role === 'OWNER';
  },
  get isEmployee() {
    return currentUser?.role === 'EMPLOYEE';
  },
  /** PIN en memoria para operaciones de venta (nunca se persiste) */
  get pin() {
    return currentPin;
  },

  /** Establece el usuario y guarda el PIN en memoria */
  setUser(user: User, pin: string) {
    currentUser = user;
    currentPin = pin;
  },

  /** Cierra sesión y limpia todo */
  logout() {
    currentUser = null;
    currentPin = '';
  }
};
