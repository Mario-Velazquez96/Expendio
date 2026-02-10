// API functions para gestión de productos e inventario
import { invoke } from '@tauri-apps/api/core';
import type { ProductDetail, StockResult, AdjustResult } from '$lib/types';

/** Lista todos los productos (activos e inactivos) — solo OWNER */
export async function listAllProductsAdmin(pin: string): Promise<ProductDetail[]> {
  return await invoke<ProductDetail[]>('list_all_products_admin', { pin });
}

/** Crea un nuevo producto con stock inicial opcional — solo OWNER */
export async function createProduct(
  pin: string,
  name: string,
  category: string,
  barcode: string | null,
  unitsPerCase: number,
  costCents: number,
  unitPriceCents: number,
  stockMin: number,
  initialStockQty: number
): Promise<ProductDetail> {
  return await invoke<ProductDetail>('create_product', {
    pin,
    name,
    category,
    barcode: barcode || null,
    unitsPerCase,
    costCents,
    unitPriceCents,
    stockMin,
    initialStockQty
  });
}

/** Agrega stock a un producto (compra) — solo OWNER */
export async function addStock(
  pin: string,
  productId: number,
  qty: number,
  note?: string
): Promise<StockResult> {
  return await invoke<StockResult>('add_stock', {
    pin,
    productId,
    qty,
    note: note || null
  });
}

/** Actualiza información de un producto — solo OWNER */
export async function updateProduct(
  pin: string,
  productId: number,
  name: string,
  category: string,
  barcode: string | null,
  unitsPerCase: number,
  costCents: number,
  unitPriceCents: number,
  stockMin: number
): Promise<ProductDetail> {
  return await invoke<ProductDetail>('update_product', {
    pin,
    productId,
    name,
    category,
    barcode: barcode || null,
    unitsPerCase,
    costCents,
    unitPriceCents,
    stockMin
  });
}

/** Activa/desactiva un producto — solo OWNER */
export async function toggleProductActive(
  pin: string,
  productId: number
): Promise<ProductDetail> {
  return await invoke<ProductDetail>('toggle_product_active', {
    pin,
    productId
  });
}

/** Ajusta (resta) stock por pérdida/merma — solo OWNER */
export async function adjustStock(
  pin: string,
  productId: number,
  qty: number,
  reason: string
): Promise<AdjustResult> {
  return await invoke<AdjustResult>('adjust_stock', {
    pin,
    productId,
    qty,
    reason
  });
}
