import { invoke } from '@tauri-apps/api/core';
import type { DashboardSummary } from '$lib/types';

/** Obtiene métricas del dashboard para los últimos N días */
export async function getDashboardSummary(days: number): Promise<DashboardSummary> {
  return await invoke<DashboardSummary>('get_dashboard_summary', { days });
}
