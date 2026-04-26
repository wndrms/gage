ALTER TABLE kream_sales
    ALTER COLUMN settlement_date DROP NOT NULL;

CREATE TABLE IF NOT EXISTS kream_keyword_rules (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    keyword TEXT NOT NULL,
    keyword_normalized TEXT NOT NULL,
    kream_kind TEXT NOT NULL DEFAULT 'side_cost',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, keyword_normalized, kream_kind),
    CONSTRAINT kream_keyword_rules_kind_check CHECK (kream_kind IN ('purchase', 'settlement', 'side_cost'))
);

CREATE INDEX IF NOT EXISTS idx_kream_keyword_rules_user_active
    ON kream_keyword_rules(user_id, is_active, kream_kind);
