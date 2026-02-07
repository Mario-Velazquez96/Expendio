# BeerPOS – Contexto del Proyecto

## 1. Objetivo del sistema

BeerPOS es una aplicación **local (desktop)** para el control integral de una tienda de cerveza con alto volumen de ventas (~500 ventas/día), enfocada en:

- Control estricto de **inventario por unidad** (botellas y latas).
- Control de **caja en efectivo** con apertura y cierre diario.
- Registro de **ventas de mostrador** con promociones manuales.
- Eliminación de errores de captura propios de Excel.

El sistema está diseñado como **software interno crítico**, no como producto comercial.

---

## 2. Stack tecnológico

- **Frontend**: Tauri + Svelte + TypeScript  
- **Backend**: Rust (Tauri commands)  
- **Base de datos**: SQLite (archivo único `.db`)  
- **ORM / DB access**: sqlx  
- **Modo de operación**: Offline, una sola computadora  

---

## 3. Reglas de negocio clave (INMUTABLES para el MVP)

### Operación general

- No se pueden registrar ventas si **no hay caja abierta**.
- Todo el dinero es **efectivo**.
- Dos usuarios: **dueño** y **empleado**.
- Autenticación mediante **PIN numérico**.

### Inventario

- Unidad base de inventario: **pieza**.
- No existen barriles.
- Las compras se realizan por **caja**:
  - Botellas: 12 piezas por caja.
  - Latas: 24 piezas por caja.
- El sistema **desglosa automáticamente** las cajas a piezas.
- **Bloqueo estricto**: no se puede vender si no hay stock suficiente.
- Conteos físicos cada 3 días (los ajustes se registran como movimientos).

### Ventas

- Solo ventas de mostrador.
- Una venta puede incluir múltiples productos.
- No se emiten tickets fiscales (solo comprobante interno opcional).

### Promociones

- Las promociones se **aplican manualmente** mediante botón “Aplicar promo”.
- Las promociones son **por producto exacto**, no se mezclan marcas.
- Ejemplos:
  - 3 cervezas
  - 6 cervezas
  - 11 cervezas
  - 12 cervezas
- Al aplicar una promoción:
  - Se valida stock.
  - Se recalcula precio.
  - Se divide la línea si hay sobrantes.
- Las promociones **no se aplican automáticamente**.

### Caja

- El fondo inicial se captura **manualmente todos los días**.
- Se permiten **retiros parciales** durante el día.
- El empleado puede cerrar la caja.
- Si hay diferencia en el cierre:
  - Nota obligatoria.
  - (Opcional) autorización con PIN de dueño.

---

## 4. Flujo operativo diario

1. Login con PIN.
2. Apertura de caja (captura de fondo inicial).
3. Ventas durante el día.
4. Movimientos de caja (retiros / depósitos).
5. Cierre de caja:
   - El sistema calcula el efectivo esperado.
   - El usuario ingresa el efectivo contado.
   - Se registra la diferencia.

---

## 5. Módulos del sistema (orden de implementación)

### MVP obligatorio

1. Base de datos y esquema (DDL).
2. Inicialización SQLite (`db.rs`).
3. Caja:
   - Apertura
   - Movimientos
   - Cierre
4. POS básico (sin promociones):
   - Agregar producto
   - Cantidad
   - Total
5. Promociones manuales.
6. Reportes básicos:
   - Ventas por día
   - Ganancia diaria
   - Diferencia de caja

### Fuera de alcance del MVP

- Multi-PC
- Multi-sucursal
- Pagos electrónicos
- Integraciones externas
- Impresora fiscal

---

## 6. Modelo de datos (alto nivel)

### Principales entidades

- `users`
- `products`
- `inventory_balances`
- `inventory_movements`
- `price_rules`
- `sales`
- `sale_lines`
- `cash_sessions`
- `cash_movements`

### Principios del modelo

- Todo movimiento de inventario se registra (auditoría completa).
- El costo se congela al momento de la venta (`cost_at_sale`).
- Los montos se almacenan en **centavos (INTEGER)**.
- Las sesiones de caja agrupan las ventas.

---

## 7. Convenciones técnicas

- SQLite con `WAL` habilitado.
- `foreign_keys = ON`.
- Transacciones obligatorias en:
  - `finalize_sale`
  - `close_cash_session`
- No se edita historia:
  - No modificar ventas cerradas.
  - Correcciones únicamente vía ajustes.

---

## 8. Filosofía del proyecto

- Prioridad absoluta: **evitar errores humanos**.
- UX funcional > UI estética.
- Reglas claras y explícitas.
- El sistema debe indicar **cuánto dinero debería haber**, no estimarlo.
- Cualquier ambigüedad se resuelve a favor del control, no de la flexibilidad.



