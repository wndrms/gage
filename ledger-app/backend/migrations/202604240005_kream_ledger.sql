ALTER TABLE users
    ADD COLUMN IF NOT EXISTS role TEXT NOT NULL DEFAULT 'member';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'users_role_check'
    ) THEN
        ALTER TABLE users
            ADD CONSTRAINT users_role_check CHECK (role IN ('admin', 'member'));
    END IF;
END $$;

UPDATE users
SET role = 'admin'
WHERE id = (
    SELECT id
    FROM users
    ORDER BY created_at ASC
    LIMIT 1
);

ALTER TABLE transactions
    ADD COLUMN IF NOT EXISTS scope TEXT NOT NULL DEFAULT 'personal';

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'transactions_scope_check'
    ) THEN
        ALTER TABLE transactions
            ADD CONSTRAINT transactions_scope_check CHECK (scope IN ('personal', 'kream'));
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS kream_sales (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    sale_code TEXT NOT NULL,
    product_name TEXT NOT NULL,
    purchase_date DATE NOT NULL,
    settlement_date DATE NOT NULL,
    purchase_price BIGINT NOT NULL,
    settlement_price BIGINT NOT NULL,
    side_cost BIGINT NOT NULL DEFAULT 0,
    purchase_transaction_id UUID NULL REFERENCES transactions(id) ON DELETE SET NULL,
    settlement_transaction_id UUID NULL REFERENCES transactions(id) ON DELETE SET NULL,
    side_cost_transaction_id UUID NULL REFERENCES transactions(id) ON DELETE SET NULL,
    dedupe_key TEXT NOT NULL,
    source_filename TEXT NULL,
    source_row_index INT NULL,
    raw_data JSONB NOT NULL DEFAULT '{}'::jsonb,
    memo TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, sale_code),
    UNIQUE(user_id, dedupe_key)
);

CREATE INDEX IF NOT EXISTS idx_kream_sales_user_purchase_date
    ON kream_sales(user_id, purchase_date DESC);

CREATE INDEX IF NOT EXISTS idx_kream_sales_user_settlement_date
    ON kream_sales(user_id, settlement_date DESC);

CREATE INDEX IF NOT EXISTS idx_transactions_user_scope_date
    ON transactions(user_id, scope, transaction_at DESC);
