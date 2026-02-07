// Exports principales de la librería
export { authStore, type User, type AuthState } from './stores/auth.svelte';
export { loginWithPin, getUserById, type LoginResult } from './api/auth';
