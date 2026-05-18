-- Allow annotation notes linked to Firecrawl artifacts (Epic J).

ALTER TABLE design_research_artifacts
    DROP CONSTRAINT IF EXISTS design_research_artifacts_kind_check;

ALTER TABLE design_research_artifacts
    ADD CONSTRAINT design_research_artifacts_kind_check
    CHECK (kind IN ('firecrawl_scrape', 'firecrawl_search', 'manual_note', 'annotation'));

CREATE INDEX IF NOT EXISTS idx_design_research_parent
    ON design_research_artifacts ((metadata_json->>'parent_artifact_id'));
