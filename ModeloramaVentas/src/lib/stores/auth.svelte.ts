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
  
  setUser(user: User | null) {
    currentUser = user;
    // Persistir en localStorage para restaurar sesión
    if (user) {
      localStorage.setItem('beerpos_user_id', String(user.id));
    } else {
      localStorage.removeItem('beerpos_user_id');
    }
  },
  
  logout() {
    currentUser = null;
    localStorage.removeItem('beerpos_user_id');
  },
  
  // Obtener ID guardado para restaurar sesión
  getSavedUserId(): number | null {
    const saved = localStorage.getItem('beerpos_user_id');
    return saved ? parseInt(saved, 10) : null;
  }
};
