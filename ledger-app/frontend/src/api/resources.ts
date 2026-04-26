import type {
  Account,
  AssetSnapshot,
  Card,
  CalendarDayTotal,
  CardTransactionDetail,
  CardSummary,
  Category,
  CategoryRule,
  DashboardDaily,
  DashboardMonthly,
  ImportPreview,
  ImportRecord,
  KreamKeywordRule,
  KreamKeywordRuleResponse,
  KreamLedgerTransaction,
  KreamSalesResponse,
  KreamTransactionCandidate,
  KreamUploadResponse,
  NetWorthPoint,
  Transaction
} from '@/types';
import { api } from './client';

export const resourceApi = {
  dashboardMonthly: (month: string) => api<DashboardMonthly>(`/api/dashboard/monthly?month=${month}`),
  dashboardDaily: (date: string) => api<DashboardDaily>(`/api/dashboard/daily?date=${date}`),
  dashboardCalendar: (month: string) => api<CalendarDayTotal[]>(`/api/dashboard/calendar?month=${month}`),
  transactions: (qs?: string) => {
    return api<Transaction[]>(`/api/transactions${qs ?? ''}`);
  },
  createTransaction: (payload: Record<string, unknown>) =>
    api<Transaction>('/api/transactions', {
      method: 'POST',
      body: JSON.stringify(payload)
    }),
  updateTransaction: (id: string, payload: Record<string, unknown>) =>
    api<Transaction>(`/api/transactions/${id}`, {
      method: 'PUT',
      body: JSON.stringify(payload)
    }),
  deleteTransaction: (id: string) =>
    api<{ message: string }>(`/api/transactions/${id}`, { method: 'DELETE' }),
  accounts: () => api<Account[]>('/api/accounts'),
  categories: () => api<Category[]>('/api/categories'),
  cards: () => api<Card[]>('/api/cards'),
  createCard: (payload: Record<string, unknown>) =>
    api<Card>('/api/cards', {
      method: 'POST',
      body: JSON.stringify(payload)
    }),
  cardSummary: (id: string, month: string) =>
    api<CardSummary>(`/api/cards/${id}/summary?month=${month}`),
  cardTransactions: (id: string, month: string) =>
    api<CardTransactionDetail>(`/api/cards/${id}/transactions?month=${month}`),
  imports: () => api<ImportRecord[]>('/api/imports'),
  importPastedText: (text: string, institution?: string) =>
    api<ImportPreview>('/api/imports/pasted-text', {
      method: 'POST',
      body: JSON.stringify({ text, institution })
    }),
  confirmImport: (id: string) =>
    api<{ message: string }>(`/api/imports/${id}/confirm`, { method: 'POST' }),
  assetSnapshots: () => api<AssetSnapshot[]>('/api/asset-snapshots'),
  createAssetSnapshot: (payload: Record<string, unknown>) =>
    api<AssetSnapshot>('/api/asset-snapshots', {
      method: 'POST',
      body: JSON.stringify(payload)
    }),
  netWorth: (from: string, to: string) =>
    api<NetWorthPoint[]>(`/api/assets/net-worth?from=${from}&to=${to}`),
  categoryRules: () => api<CategoryRule[]>('/api/category-rules'),
  createCategoryRule: (payload: { keyword: string; category_id: string; priority?: number }) =>
    api<CategoryRule>('/api/category-rules', {
      method: 'POST',
      body: JSON.stringify(payload)
    }),
  deleteCategoryRule: (id: string) =>
    api<{ message: string }>(`/api/category-rules/${id}`, { method: 'DELETE' }),
  kreamSales: () => api<KreamSalesResponse>('/api/admin/kream-sales'),
  createKreamSale: (payload: {
    product_name: string;
    purchase_date: string;
    settlement_date?: string | null;
    purchase_price: number;
    settlement_price?: number | null;
    memo?: string | null;
  }) =>
    api<KreamSalesResponse['sales'][number]>('/api/admin/kream-sales', {
      method: 'POST',
      body: JSON.stringify(payload)
    }),
  kreamLedger: () => api<KreamLedgerTransaction[]>('/api/admin/kream-sales/ledger'),
  kreamCandidates: (kind: 'purchase' | 'settlement' | 'side_cost' = 'purchase', keyword?: string) => {
    const params = new URLSearchParams({ kind });
    if (keyword?.trim()) params.set('keyword', keyword.trim());
    return api<KreamTransactionCandidate[]>(`/api/admin/kream-sales/candidates?${params.toString()}`);
  },
  matchKreamTransaction: (
    saleId: string,
    payload: { transaction_id: string; kind: 'purchase' | 'settlement' }
  ) =>
    api<KreamSalesResponse['sales'][number]>(`/api/admin/kream-sales/${saleId}/match`, {
      method: 'POST',
      body: JSON.stringify(payload)
    }),
  unmatchKreamTransaction: (
    saleId: string,
    payload: { kind: 'purchase' | 'settlement' }
  ) =>
    api<KreamSalesResponse['sales'][number]>(`/api/admin/kream-sales/${saleId}/unmatch`, {
      method: 'POST',
      body: JSON.stringify(payload)
    }),
  markKreamTransaction: (payload: {
    transaction_id: string;
    scope: 'personal' | 'kream';
    kream_kind?: 'purchase' | 'settlement' | 'side_cost' | null;
  }) =>
    api<KreamTransactionCandidate>('/api/admin/kream-transactions/mark', {
      method: 'POST',
      body: JSON.stringify(payload)
    }),
  bulkMarkKreamTransactions: (payload: {
    transaction_ids: string[];
    kream_kind: 'side_cost';
  }) =>
    api<{ updated_count: number }>('/api/admin/kream-transactions/bulk-mark', {
      method: 'POST',
      body: JSON.stringify(payload)
    }),
  kreamKeywordRules: () => api<KreamKeywordRule[]>('/api/admin/kream-keyword-rules'),
  createKreamKeywordRule: (payload: { keyword: string; kream_kind?: 'side_cost' }) =>
    api<KreamKeywordRuleResponse>('/api/admin/kream-keyword-rules', {
      method: 'POST',
      body: JSON.stringify(payload)
    }),
  deleteKreamKeywordRule: (id: string) =>
    api<{ message: string }>(`/api/admin/kream-keyword-rules/${id}`, {
      method: 'DELETE'
    }),
  uploadKreamSales: async (file: File) => {
    const form = new FormData();
    form.append('file', file);

    const response = await fetch('/api/admin/kream-sales/upload', {
      method: 'POST',
      body: form,
      credentials: 'include'
    });

    const body = await response.json();
    if (!response.ok) {
      throw new Error(body?.message || 'KREAM file upload failed.');
    }

    return body as KreamUploadResponse;
  }
};
