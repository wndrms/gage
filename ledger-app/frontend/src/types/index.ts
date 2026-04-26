export type User = {
  id: string;
  display_name: string;
  role: 'admin' | 'member';
};

export type Account = {
  id: string;
  name: string;
  type: 'bank' | 'cash' | 'investment' | 'credit_card_liability';
  institution?: string | null;
  currency: string;
  is_active: boolean;
};

export type Category = {
  id: string;
  name: string;
  type: 'expense' | 'income' | 'transfer';
  sort_order: number;
};

export type Card = {
  id: string;
  issuer: string;
  card_name: string;
  preset_id?: string | null;
  billing_day?: number | null;
  payment_day?: number | null;
  linked_account_id?: string | null;
  is_active: boolean;
};

export type Transaction = {
  id: string;
  transaction_at: string;
  type: 'expense' | 'income' | 'transfer' | 'card_payment';
  amount: number;
  merchant_name?: string | null;
  description?: string | null;
  category_id?: string | null;
  account_id?: string | null;
  card_id?: string | null;
  card_name?: string | null;
  memo?: string | null;
  scope?: 'personal' | 'kream';
  kream_kind?: 'purchase' | 'settlement' | 'side_cost' | null;
};

export type KreamSale = {
  id: string;
  sale_code: string;
  product_name: string;
  purchase_date: string;
  settlement_date?: string | null;
  purchase_price: number;
  settlement_price: number;
  side_cost: number;
  purchase_transaction_id?: string | null;
  settlement_transaction_id?: string | null;
  side_cost_transaction_id?: string | null;
  memo?: string | null;
};

export type KreamSalesResponse = {
  sales: KreamSale[];
  summary: {
    total_purchase_price: number;
    total_settlement_price: number;
    total_side_cost: number;
    total_profit: number;
  };
};

export type KreamLedgerTransaction = {
  id: string;
  transaction_at: string;
  type: 'expense' | 'income' | 'transfer' | 'card_payment';
  amount: number;
  merchant_name?: string | null;
  description?: string | null;
  memo?: string | null;
  sale_id?: string | null;
  sale_code?: string | null;
  product_name?: string | null;
  link_kind?: 'purchase' | 'settlement' | 'side_cost' | null;
};

export type KreamTransactionCandidate = KreamLedgerTransaction & {
  scope: 'personal' | 'kream';
};

export type KreamUploadResponse = {
  imported_count: number;
  duplicate_count: number;
  error_count: number;
  sales: KreamSale[];
};

export type KreamKeywordRule = {
  id: string;
  keyword: string;
  keyword_normalized: string;
  kream_kind: 'purchase' | 'settlement' | 'side_cost';
  is_active: boolean;
  created_at: string;
  updated_at: string;
};

export type KreamKeywordRuleResponse = {
  rule: KreamKeywordRule;
  applied_count: number;
};

export type DashboardMonthly = {
  month: string;
  total_income: number;
  total_expense: number;
  net_expense: number;
  comparison: {
    previous_month: string;
    previous_total_income: number;
    previous_total_expense: number;
    previous_net_expense: number;
    income_change_amount: number;
    expense_change_amount: number;
    net_expense_change_amount: number;
    expense_change_rate: number;
  };
  category_expense: Array<{ category_id: string | null; name: string; amount: number }>;
  card_expense: Array<{ name: string; amount: number }>;
  account_expense: Array<{ name: string; amount: number }>;
  recent_transactions: Transaction[];
};

export type DashboardDaily = {
  date: string;
  total_income: number;
  total_expense: number;
  transactions: Transaction[];
};

export type CalendarDayTotal = {
  date: string;
  total_expense: number;
};

export type CardSummary = {
  card_id: string;
  month: string;
  summary: {
    monthly_spending: number;
    eligible_spending: number;
    monthly_requirement: number;
    requirement_ratio: number;
    benefits: Array<{ name: string; used_amount: number; cap: number }>;
  };
};

export type CardTransactionDetail = {
  card_id: string;
  month: string;
  total_count: number;
  total_amount: number;
  transactions: Array<{
    id: string;
    transaction_at: string;
    amount: number;
    merchant_name?: string | null;
    description?: string | null;
    category_name?: string | null;
    account_name?: string | null;
    memo?: string | null;
  }>;
};

export type ImportRecord = {
  id: string;
  source_type: string;
  institution: string;
  original_filename?: string | null;
  status: string;
  parsed_count: number;
  imported_count: number;
  duplicate_count: number;
  error_message?: string | null;
  created_at: string;
};

export type ImportPreview = {
  import_id: string;
  status: string;
  총건수: number;
  신규건수: number;
  중복건수: number;
  오류건수: number;
};

export type AssetSnapshot = {
  id: string;
  snapshot_date: string;
  account_id: string;
  amount: number;
  memo?: string | null;
};

export type NetWorthPoint = {
  date: string;
  assets: number;
  liabilities: number;
  net_worth: number;
};

export type CategoryRule = {
  id: string;
  user_id: string;
  keyword: string;
  keyword_normalized: string;
  category_id: string;
  priority: number;
  source: 'user' | 'learned' | 'seed';
  created_at: string;
  updated_at: string;
};
