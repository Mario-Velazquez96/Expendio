/**
 * Utilidades para formateo y conversión de dinero.
 * Internamente todo se almacena en centavos (INTEGER).
 */

/** Formatea centavos a string MXN: 1500 → "$15.00" */
export function formatMoney(cents: number): string {
  const pesos = cents / 100;
  return `$${pesos.toFixed(2)}`;
}

/** Convierte pesos (con decimales) a centavos enteros: 15.50 → 1550 */
export function pesosToCents(pesos: number): number {
  return Math.round(pesos * 100);
}

/** Convierte centavos a pesos: 1550 → 15.50 */
export function centsToPesos(cents: number): number {
  return cents / 100;
}

/** Formatea una hora ISO a formato legible: "2025-02-08 14:30:00" → "14:30" */
export function formatTime(isoString: string): string {
  try {
    const date = new Date(isoString + 'Z');
    return date.toLocaleTimeString('es-MX', {
      hour: '2-digit',
      minute: '2-digit'
    });
  } catch {
    return isoString;
  }
}

/** Formatea fecha+hora ISO a formato legible */
export function formatDateTime(isoString: string): string {
  try {
    const date = new Date(isoString + 'Z');
    return date.toLocaleString('es-MX', {
      day: '2-digit',
      month: '2-digit',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    });
  } catch {
    return isoString;
  }
}
