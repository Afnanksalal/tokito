# Tokito HTTP API (MVP)

Base URL: `http://localhost:8080` (or your deployment). JSON bodies use `Content-Type: application/json`.

## Health

### `GET /health`

Returns:

```json
{ "status": "ok", "service": "tokito" }
```

---

## Manufacturers

### `POST /v1/manufacturers`

```json
{ "name": "STMicroelectronics", "slug": null }
```

`slug` is optional; if omitted, a slug is generated from `name`.

### `GET /v1/manufacturers?limit=100`

---

## Parts

### `POST /v1/parts`

```json
{
  "manufacturer_id": "<uuid>",
  "mpn": "STM32F103C8T6",
  "description": "MCU",
  "package_name": "LQFP48",
  "attributes": { "voltage_v": 3.3 }
}
```

### `GET /v1/parts?q=stm&limit=50`

`q` optional; if empty, returns up to `limit` parts ordered by MPN.

### `GET /v1/parts/:id`

---

## Designs

### `POST /v1/designs`

```json
{ "name": "Sensor board", "description": "Rev A" }
```

### `GET /v1/designs/:id`

### `PATCH /v1/designs/:id`

```json
{ "name": "Sensor board rev B", "description": null }
```

### `GET /v1/designs/:id/export?format=json`

Default snapshot JSON (`design`, `bom`, `schematic`, `intent`, `research_artifacts`).

### `GET /v1/designs/:id/export?format=csv`

BOM-only CSV download.

### `GET /v1/designs/:id/export?format=netlist`

Plain-text connectivity listing (`NET  REFDES.PIN`), derived from stored schematic pins.

---

## BOM

### `GET /v1/designs/:id/bom`

### `PUT /v1/designs/:id/bom`

Replaces the entire BOM for the design.

```json
{
  "lines": [
    { "part_id": "<uuid>", "quantity": 2, "sort_order": 0, "notes": "bulk cap" },
    { "part_id": "<uuid>", "quantity": 1, "sort_order": 1, "notes": null }
  ]
}
```

`quantity` must be \> 0. All `part_id` values must exist.

---

## Schematic

### `GET /v1/designs/:id/schematic`

Returns:

```json
{
  "instances": [
    {
      "id": "...",
      "design_id": "...",
      "part_id": null,
      "ref_des": "U1",
      "pos_x": 10,
      "pos_y": 20,
      "rotation": 0,
      "meta": {},
      "created_at": "..."
    }
  ],
  "nets": [{ "id": "...", "design_id": "...", "name": "VCC", "created_at": "..." }],
  "pins": [
    { "id": "...", "instance_id": "...", "pin_name": "1", "net_id": "...", "created_at": "..." }
  ]
}
```

### `PUT /v1/designs/:id/schematic`

Replaces the schematic for the design in one transaction.

```json
{
  "instances": [
    {
      "part_id": "<uuid optional>",
      "ref_des": "U1",
      "position": { "x": 0, "y": 0 },
      "rotation": 0,
      "meta": {}
    }
  ],
  "nets": [{ "name": "GND" }, { "name": "VCC" }],
  "pins": [
    { "instance_ref": "U1", "pin_name": "VDD", "net_name": "VCC" },
    { "instance_ref": "U1", "pin_name": "GND", "net_name": "GND" }
  ]
}
```

- `instance_ref` must match a `ref_des` in the `instances` array within the same payload.
- `net_name` must match a net in `nets`.
- `ref_des` values must be unique within the payload.

Success:

```json
{ "ok": true, "erc_warnings": [{ "code": "...", "severity": "warning", "message": "...", "detail": null }] }
```

`erc_warnings` are advisory (floating stubs, unused nets, etc.). Topology violations still yield HTTP **400**.

### `POST /v1/designs/:id/schematic/suggest`

Body `{ "prompt": "..." }`.

This endpoint runs the **full copilot pipeline** (same behavior as **Generate** in the native app), not a prompt-only schematic draft:

1. Persists **intent** (`goal_text` = prompt).
2. **xAI** returns planned Firecrawl queries + candidate MPNs.
3. **Firecrawl web search** runs per query; hits are stored as **`design_research_artifacts`** (markdown excerpts).
4. **xAI** resolves concrete parts + quantities from excerpts + candidates.
5. **Postgres** upserts manufacturers/parts and **replaces the design BOM** with validated `part_id`s.
6. **xAI** drafts `ReplaceSchematic` **grounded on that BOM** (instances must use BOM `part_id`s — no null catalog IDs when BOM lines exist).

**Requirements:** `TOKITO_XAI_API_KEY` and `TOKITO_FIRECRAWL_API_KEY` must be configured on the server. If Firecrawl returns no ingestible pages or the model cannot resolve parts, the handler returns **400** with an explanatory message.

Response wraps the draft plus ERC advisories:

```json
{
  "schematic": { "instances": [], "nets": [], "pins": [] },
  "erc_warnings": []
}
```

### `POST /v1/designs/:id/schematic/validate`

Same schematic JSON body as `PUT`; nothing is persisted. Response:

```json
{
  "topology_ok": true,
  "topology_error": null,
  "erc_warnings": []
}
```

When `topology_ok` is `false`, `topology_error` holds the blocking validation message.

---

## Intent & research (copilot grounding)

### `GET /v1/designs/:id/intent`

### `PUT /v1/designs/:id/intent`

```json
{ "goal_text": "5 V buck from 12 V in…", "constraints": { "iout_a": 2 } }
```

### `GET /v1/designs/:id/research`

### `POST /v1/designs/:id/research/scrape`

Body: `{ "urls": ["https://example.com/datasheet.pdf"] }` — Firecrawl **scrape** per URL; artifacts appended.

### `POST /v1/designs/:id/research/search`

Body: `{ "query": "LM2596 5V buck datasheet", "limit": 5 }` — Firecrawl **web search** ([docs](https://docs.firecrawl.dev/features/search)); optional `scrapeOptions` are applied server-side (markdown per result). Each saved result counts toward scrape quota like URL scrapes.

Response: `{ "artifact_ids": [...], "count": N }`.

### Firecrawl proxies (authenticated)

- `POST /v1/integrations/firecrawl/scrape` — passthrough scrape body (`url`, `formats`, …). One scrape quota unit per request.
- `POST /v1/integrations/firecrawl/search` — passthrough search body (`query`, `limit`, `scrapeOptions`, …). One scrape quota unit per request.

## Errors

Errors return JSON `{ "error": "message" }` with appropriate HTTP status (400 / 404 / 409 / 500).
