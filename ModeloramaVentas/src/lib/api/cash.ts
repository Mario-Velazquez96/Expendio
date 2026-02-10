// API functions para sesiones de caja y movimientos
import { invoke } from '@tauri-apps/api/core';
import type { CashSession, CashMovement, CashSessionSummary } from '$lib/types';

/** Abre una nueva sesión de caja */
export async function openCashSession(
  userId: number,
  openingAmountCents: number,
  note?: string
): Promise<CashSession> {
  return await invoke<CashSession>('open_cash_session', {
    userId,
    openingAmountCents,
    note: note || null
  });
}

/** Obtiene la sesión de caja abierta (si existe) */
export async function getCurrentCashSession(): Promise<CashSession | null> {
  return await invoke<CashSession | null>('get_current_cash_session');
}

/** Registra un movimiento manual de caja (depósito o retiro) */
export async function addCashMovement(
  sessionId: number,
  movementType: string,
  amountCents: number,
  reason: string,
  userId: number
): Promise<CashMovement> {
  return await invoke<CashMovement>('add_cash_movement', {
    sessionId,
    movementType,
    amountCents,
    reason,
    userId
  });
}

/** Obtiene los movimientos de caja de una sesión */
export async function getCashMovements(
  sessionId: number
): Promise<CashMovement[]> {
  return await invoke<CashMovement[]>('get_cash_movements', { sessionId });
}

/** Calcula el resumen de la sesión (para cierre) */
export async function getCashSessionSummary(
  sessionId: number
): Promise<CashSessionSummary> {
  return await invoke<CashSessionSummary>('get_cash_session_summary', {
    sessionId
  });
}

/** Cierra la sesión de caja */
export async function closeCashSession(
  sessionId: number,
  userId: number,
  countedCashCents: number,
  notesClose?: string,
  ownerAuthUserId?: number
): Promise<CashSession> {
  return await invoke<CashSession>('close_cash_session', {
    sessionId,
    userId,
    countedCashCents,
    notesClose: notesClose || null,
    ownerAuthUserId: ownerAuthUserId || null
  });
}
