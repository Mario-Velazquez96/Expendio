// API functions para autenticación
import { invoke } from '@tauri-apps/api/core';
import type { User } from '../stores/auth';

export interface LoginResult {
  success: boolean;
  user: User | null;
  error: string | null;
}

/**
 * Intenta login con PIN de 6 dígitos
 */
export async function loginWithPin(pin: string): Promise<LoginResult> {
  return await invoke<LoginResult>('login_with_pin', { pin });
}

/**
 * Obtiene usuario por ID (para restaurar sesión)
 */
export async function getUserById(userId: number): Promise<User | null> {
  return await invoke<User | null>('get_user_by_id', { userId });
}
