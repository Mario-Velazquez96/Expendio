// API functions para productos y ventas (POS)
import { invoke } from '@tauri-apps/api/core';
import type {
  Product,
  CreateSaleResult,
  SaleLineResult,
  SaleResult,
  SessionSalesSummary
} from '$lib/types';

/** Busca productos por nombre o código de barras */
export async function searchProducts(query: string): Promise<Product[]> {
  return await invoke<Product[]>('search_products', { query });
}

/** Obtiene un producto por código de barras exacto */
export async function getProductByBarcode(
  barcode: string
): Promise<Product | null> {
  return await invoke<Product | null>('get_product_by_barcode', { barcode });
}

/** Lista todos los productos activos */
export async function listProducts(): Promise<Product[]> {
  return await invoke<Product[]>('list_products');
}

/** Crea una venta vacía (DRAFT) */
export async function createSale(pin: string): Promise<CreateSaleResult> {
  return await invoke<CreateSaleResult>('create_sale', { pin });
}

/** Agrega (o incrementa) una línea a la venta */
export async function addSaleLine(
  pin: string,
  saleId: number,
  productId: number,
  qty: number
): Promise<SaleLineResult> {
  return await invoke<SaleLineResult>('add_sale_line', {
    pin,
    saleId,
    productId,
    qty
  });
}

/** Actualiza la cantidad de una línea existente */
export async function updateSaleLineQty(
  pin: string,
  lineId: number,
  newQty: number
): Promise<SaleLineResult> {
  return await invoke<SaleLineResult>('update_sale_line_qty', {
    pin,
    lineId,
    newQty
  });
}

/** Elimina una línea de la venta */
export async function removeSaleLine(
  pin: string,
  lineId: number
): Promise<void> {
  return await invoke<void>('remove_sale_line', { pin, lineId });
}

/** Finaliza la venta: valida stock, descuenta inventario, calcula totales */
export async function finalizeSale(
  pin: string,
  saleId: number
): Promise<SaleResult> {
  return await invoke<SaleResult>('finalize_sale', { pin, saleId });
}

/** Obtiene el detalle de una venta */
export async function getSaleDetail(saleId: number): Promise<SaleResult> {
  return await invoke<SaleResult>('get_sale_detail', { saleId });
}

/** Obtiene las ventas finalizadas de una sesión */
export async function getSessionSales(
  sessionId: number
): Promise<SessionSalesSummary> {
  return await invoke<SessionSalesSummary>('get_session_sales', { sessionId });
}
