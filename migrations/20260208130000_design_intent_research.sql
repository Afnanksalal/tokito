-- Design build intent + research artifacts (copilot grounding)

CREATE TABLE design_intents (
    design_id       UUID PRIMARY KEY REFERENCES designs (id) ON DELETE CASCADE,
    goal_text       TEXT NOT NULL DEFAULT '',
    constraints_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT design_intents_goal_len CHECK (char_length(goal_text) <= 100000)
);

CREATE TABLE design_research_artifacts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    design_id       UUID NOT NULL REFERENCES designs (id) ON DELETE CASCADE,
    kind            TEXT NOT NULL CHECK (kind IN ('firecrawl_scrape', 'manual_note')),
    title           TEXT,
    source_url      TEXT,
    content_text    TEXT NOT NULL,
    metadata_json   JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT design_research_content_len CHECK (char_length(content_text) <= 500000)
);

CREATE INDEX idx_design_research_design_created ON design_research_artifacts (design_id, created_at DESC);

-- Bump parent design row when intent or research changes (API lists by updated_at).
CREATE OR REPLACE FUNCTION trg_touch_design_from_child()
RETURNS TRIGGER AS $$
DECLARE
    did UUID;
BEGIN
    IF TG_OP = 'DELETE' THEN
        did := OLD.design_id;
    ELSE
        did := NEW.design_id;
    END IF;
    UPDATE designs SET updated_at = now() WHERE id = did;
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER design_intents_touch_design
    AFTER INSERT OR UPDATE ON design_intents
    FOR EACH ROW EXECUTE PROCEDURE trg_touch_design_from_child();

CREATE TRIGGER design_research_touch_design
    AFTER INSERT OR DELETE ON design_research_artifacts
    FOR EACH ROW EXECUTE PROCEDURE trg_touch_design_from_child();
