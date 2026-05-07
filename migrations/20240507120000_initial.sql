CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE manufacturers (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL,
    slug        TEXT NOT NULL UNIQUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE parts (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    manufacturer_id  UUID NOT NULL REFERENCES manufacturers (id) ON DELETE RESTRICT,
    mpn              TEXT NOT NULL,
    description      TEXT,
    package_name     TEXT,
    attributes       JSONB NOT NULL DEFAULT '{}',
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (manufacturer_id, mpn)
);

CREATE INDEX idx_parts_mpn ON parts (mpn);
CREATE INDEX idx_parts_attrs_gin ON parts USING GIN (attributes);

CREATE TABLE part_offers (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    part_id       UUID NOT NULL REFERENCES parts (id) ON DELETE CASCADE,
    distributor   TEXT NOT NULL,
    sku           TEXT NOT NULL,
    product_url   TEXT,
    currency      TEXT NOT NULL DEFAULT 'USD',
    unit_price_cents BIGINT,
    stock_qty        BIGINT,
    fetched_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (part_id, distributor, sku)
);

CREATE INDEX idx_part_offers_part ON part_offers (part_id);

CREATE TABLE designs (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name         TEXT NOT NULL,
    description  TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE schematic_instances (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    design_id   UUID NOT NULL REFERENCES designs (id) ON DELETE CASCADE,
    part_id     UUID REFERENCES parts (id) ON DELETE SET NULL,
    ref_des     TEXT NOT NULL,
    pos_x       DOUBLE PRECISION,
    pos_y       DOUBLE PRECISION,
    rotation    DOUBLE PRECISION NOT NULL DEFAULT 0,
    meta        JSONB NOT NULL DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ref_des_unique UNIQUE (design_id, ref_des)
);

CREATE INDEX idx_schematic_instances_design ON schematic_instances (design_id);

CREATE TABLE schematic_nets (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    design_id   UUID NOT NULL REFERENCES designs (id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (design_id, name)
);

CREATE INDEX idx_schematic_nets_design ON schematic_nets (design_id);

CREATE TABLE schematic_pins (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    instance_id UUID NOT NULL REFERENCES schematic_instances (id) ON DELETE CASCADE,
    pin_name    TEXT NOT NULL,
    net_id      UUID NOT NULL REFERENCES schematic_nets (id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (instance_id, pin_name)
);

CREATE INDEX idx_schematic_pins_net ON schematic_pins (net_id);

CREATE TABLE bom_lines (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    design_id   UUID NOT NULL REFERENCES designs (id) ON DELETE CASCADE,
    part_id     UUID NOT NULL REFERENCES parts (id) ON DELETE RESTRICT,
    quantity    DOUBLE PRECISION NOT NULL CHECK (quantity > 0),
    sort_order  INT NOT NULL DEFAULT 0,
    notes       TEXT,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_bom_lines_design ON bom_lines (design_id);

CREATE OR REPLACE FUNCTION trg_touch_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER parts_touch_updated_at
    BEFORE UPDATE ON parts
    FOR EACH ROW EXECUTE PROCEDURE trg_touch_updated_at();

CREATE TRIGGER designs_touch_updated_at
    BEFORE UPDATE ON designs
    FOR EACH ROW EXECUTE PROCEDURE trg_touch_updated_at();

CREATE TRIGGER bom_lines_touch_updated_at
    BEFORE UPDATE ON bom_lines
    FOR EACH ROW EXECUTE PROCEDURE trg_touch_updated_at();
