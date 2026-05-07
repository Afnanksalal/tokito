-- Web search artifacts use kind = 'firecrawl_search' (research_pipeline::search_web_into_design).
-- Original constraint only listed firecrawl_scrape + manual_note.
-- IF EXISTS: safe if the DB was hot-fixed manually or the constraint was renamed.

ALTER TABLE design_research_artifacts DROP CONSTRAINT IF EXISTS design_research_artifacts_kind_check;

ALTER TABLE design_research_artifacts ADD CONSTRAINT design_research_artifacts_kind_check
    CHECK (kind IN ('firecrawl_scrape', 'firecrawl_search', 'manual_note'));
