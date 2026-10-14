# Brief: the commerce domain at a glance

Audience: an engineer joining the team who needs one place to see every entity, where it lives, and how entities connect; they will come back to it to look things up. Illustrative domain, dated 2026-10-14.

Opening summary: Five areas own twenty entities. An order is the spine: identity places it, catalog prices it, billing pays for it, fulfilment delivers it.

## Areas and their entities

### Identity — who the customer is
- **Customer** (aggregate) — person or company that places orders. Fields: id, name, email, type (person | company), created at. Rule: email is unique.
- **Account** (entity) — sign-in credentials for a customer. Fields: customer id, password hash, MFA enabled, last sign-in. Rule: one account per customer.
- **Address** (value) — postal address. Fields: lines, city, postcode, country. Used for shipping and invoices.
- **Consent record** (entity) — marketing and data-processing consents. Fields: customer id, purpose, granted at, version. Rule: append-only; latest version wins.

### Catalog — what can be sold
- **Product** (aggregate) — a sellable item family. Fields: id, name, description, status (draft | live | retired).
- **Variant / SKU** (entity) — a specific size or colour. Fields: sku, product id, attributes, weight.
- **Price** (value) — price of a SKU in a currency. Fields: sku, currency, amount, valid from. Rule: one active price per SKU and currency.
- **Inventory level** (entity) — stock per SKU per warehouse. Fields: sku, warehouse id, on hand, reserved.

### Ordering — carts become orders
- **Cart** (aggregate) — items before checkout. Fields: id, customer id (optional), lines, expires at. Rule: expires after 7 days idle.
- **Order** (aggregate) — the committed purchase. Fields: id, customer id, status, totals, placed at. States: placed → paid → fulfilled → closed; cancelled from placed or paid.
- **Order line** (entity) — one SKU in an order. Fields: sku, quantity, unit price at purchase. Rule: price is copied, never re-read.
- **Discount** (value) — a promotion applied to an order. Fields: code, amount or percent. Rule: at most one per order.

### Billing — money in and out
- **Payment** (aggregate) — an attempt to collect money. Fields: id, order id, method, amount, status (authorised | captured | failed | refunded).
- **Invoice** (entity) — legal document issued when an order is paid. Fields: number, order id, billing address, tax lines. Rule: numbers are sequential per country.
- **Refund** (entity) — money returned. Fields: payment id, amount, reason. Rule: total refunds ≤ captured amount.
- **Tax rate** (value) — rate by country and product class. Fields: country, class, percent.

### Fulfilment — getting it to the customer
- **Warehouse** (entity) — a stocking location. Fields: id, address, cut-off time.
- **Shipment** (aggregate) — goods leaving one warehouse for one order. Fields: id, order id, warehouse id, status (pending | packed | shipped | delivered).
- **Parcel** (entity) — a physical box in a shipment. Fields: tracking number, carrier, weight.
- **Return** (entity) — goods coming back. Fields: order line, reason, received at. Triggers a refund.

## Relationships

- Customer → Account (has one)
- Customer → Address (has many)
- Customer → Order (places)
- Cart → Order (checks out to)
- Order → Order line (contains)
- Order line → Variant / SKU (refers to)
- Variant / SKU → Product (belongs to)
- Variant / SKU → Price (priced by)
- Variant / SKU → Inventory level (stocked as)
- Order → Payment (paid by)
- Payment → Invoice (issues)
- Payment → Refund (reversed by)
- Order → Shipment (fulfilled by)
- Shipment → Warehouse (leaves from)
- Shipment → Parcel (contains)
- Return → Refund (triggers)

## Things people get wrong

- Order line prices are copied at purchase; changing a Price never changes an existing order.
- Inventory is reserved at checkout and released if payment fails within 15 minutes.
- A cancelled order that was already paid needs a Refund; cancelling does not refund by itself.

## Open questions

- Should Address become an entity with an id so invoices can reference it after edits?
- Do company customers need multiple accounts?
