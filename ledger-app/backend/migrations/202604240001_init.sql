CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY,
    email TEXT NULL,
    display_name TEXT NOT NULL,
    password_hash TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS accounts (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('bank', 'cash', 'investment', 'credit_card_liability')),
    institution TEXT NULL,
    currency TEXT NOT NULL DEFAULT 'KRW',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS cards (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    issuer TEXT NOT NULL,
    card_name TEXT NOT NULL,
    preset_id UUID NULL,
    billing_day INT NULL,
    payment_day INT NULL,
    linked_account_id UUID NULL REFERENCES accounts(id) ON DELETE SET NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS card_presets (
    id UUID PRIMARY KEY,
    issuer TEXT NOT NULL,
    card_name TEXT NOT NULL,
    aliases TEXT[] NOT NULL DEFAULT '{}',
    monthly_requirement BIGINT NULL,
    rules JSONB NOT NULL DEFAULT '{}'::jsonb,
    benefits JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE cards
    ADD CONSTRAINT fk_cards_preset
    FOREIGN KEY (preset_id) REFERENCES card_presets(id)
    ON DELETE SET NULL;

CREATE TABLE IF NOT EXISTS categories (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    parent_id UUID NULL REFERENCES categories(id) ON DELETE SET NULL,
    type TEXT NOT NULL CHECK (type IN ('expense', 'income', 'transfer')),
    sort_order INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS imports (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    source_type TEXT NOT NULL CHECK (source_type IN ('csv', 'xls', 'xlsx', 'pasted_text', 'telegram_file')),
    institution TEXT NOT NULL,
    original_filename TEXT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'parsed', 'imported', 'failed')),
    raw_text TEXT NULL,
    parsed_count INT NOT NULL DEFAULT 0,
    imported_count INT NOT NULL DEFAULT 0,
    duplicate_count INT NOT NULL DEFAULT 0,
    error_message TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS transactions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    transaction_at TIMESTAMPTZ NOT NULL,
    posted_at TIMESTAMPTZ NULL,
    type TEXT NOT NULL CHECK (type IN ('expense', 'income', 'transfer', 'card_payment')),
    amount BIGINT NOT NULL,
    merchant_name TEXT NULL,
    description TEXT NULL,
    category_id UUID NULL REFERENCES categories(id) ON DELETE SET NULL,
    account_id UUID NULL REFERENCES accounts(id) ON DELETE SET NULL,
    card_id UUID NULL REFERENCES cards(id) ON DELETE SET NULL,
    source_type TEXT NOT NULL CHECK (source_type IN ('manual', 'file', 'pasted_text', 'telegram', 'ocr')),
    source_institution TEXT NULL,
    source_file_id UUID NULL REFERENCES imports(id) ON DELETE SET NULL,
    balance_after BIGINT NULL,
    raw_data JSONB NOT NULL DEFAULT '{}'::jsonb,
    dedupe_key TEXT NOT NULL,
    memo TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, dedupe_key)
);

CREATE TABLE IF NOT EXISTS import_rows (
    id UUID PRIMARY KEY,
    import_id UUID NOT NULL REFERENCES imports(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    row_index INT NOT NULL,
    parsed_transaction JSONB NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('new', 'duplicate', 'error', 'ignored')),
    error_message TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS asset_snapshots (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    snapshot_date DATE NOT NULL,
    account_id UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    amount BIGINT NOT NULL,
    memo TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, snapshot_date, account_id)
);

CREATE TABLE IF NOT EXISTS telegram_connections (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    telegram_user_id TEXT NOT NULL,
    telegram_chat_id TEXT NOT NULL,
    connected_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    is_active BOOLEAN NOT NULL DEFAULT true
);

CREATE INDEX IF NOT EXISTS idx_transactions_user_transaction_at ON transactions(user_id, transaction_at DESC);
CREATE INDEX IF NOT EXISTS idx_transactions_user_type ON transactions(user_id, type);
CREATE INDEX IF NOT EXISTS idx_transactions_user_account ON transactions(user_id, account_id);
CREATE INDEX IF NOT EXISTS idx_transactions_user_card ON transactions(user_id, card_id);
CREATE INDEX IF NOT EXISTS idx_imports_user_created ON imports(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_import_rows_import ON import_rows(import_id, row_index);
CREATE INDEX IF NOT EXISTS idx_asset_snapshots_user_date ON asset_snapshots(user_id, snapshot_date);
