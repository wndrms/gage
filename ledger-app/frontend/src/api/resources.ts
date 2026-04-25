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
    api<{ message: string }>(`/api/category-rules/${id}`, { method: 'DELETE' })
};
