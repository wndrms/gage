ALTER TABLE transactions
    ADD COLUMN IF NOT EXISTS kream_kind TEXT NULL;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'transactions_kream_kind_check'
    ) THEN
        ALTER TABLE transactions
            ADD CONSTRAINT transactions_kream_kind_check
            CHECK (kream_kind IS NULL OR kream_kind IN ('purchase', 'settlement', 'side_cost'));
    END IF;
END $$;

UPDATE transactions t
SET kream_kind = 'purchase'
FROM kream_sales ks
WHERE ks.purchase_transaction_id = t.id
  AND t.kream_kind IS NULL;

UPDATE transactions t
SET kream_kind = 'settlement'
FROM kream_sales ks
WHERE ks.settlement_transaction_id = t.id
  AND t.kream_kind IS NULL;

UPDATE transactions t
SET kream_kind = 'side_cost'
FROM kream_sales ks
WHERE ks.side_cost_transaction_id = t.id;

UPDATE kream_sales
SET side_cost = 0,
    side_cost_transaction_id = NULL,
    updated_at = now()
WHERE side_cost_transaction_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_transactions_user_kream_kind_date
    ON transactions(user_id, kream_kind, transaction_at DESC)
    WHERE scope = 'kream';
