// Exports principales de la librería
export { authStore } from './stores/auth.svelte';
export type { User, AuthState } from './stores/auth.svelte';
export { loginWithPin, getUserById } from './api/auth';
export type { LoginResult } from './api/auth';
