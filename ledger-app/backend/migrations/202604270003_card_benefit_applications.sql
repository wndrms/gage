CREATE TABLE IF NOT EXISTS card_benefit_applications (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    transaction_id UUID NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
    preset_id UUID NOT NULL REFERENCES card_presets(id) ON DELETE CASCADE,
    benefit_name TEXT NOT NULL,
    discount_amount BIGINT NOT NULL DEFAULT 0,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(transaction_id, benefit_name)
);

CREATE INDEX IF NOT EXISTS idx_card_benefit_applications_user
    ON card_benefit_applications(user_id, applied_at DESC);

CREATE INDEX IF NOT EXISTS idx_card_benefit_applications_tx
    ON card_benefit_applications(transaction_id);

ALTER TABLE card_presets
    ADD COLUMN IF NOT EXISTS parse_text TEXT NULL;
