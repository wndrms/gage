import { useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { api } from '@/api/client';
import type { Account } from '@/types';

const ACCOUNT_TYPES = [
  { value: 'bank', label: '은행' },
  { value: 'cash', label: '현금' },
  { value: 'investment', label: '투자' },
  { value: 'credit_card_liability', label: '신용카드 부채' },
];

const emptyForm = { name: '', type: 'bank', institution: '', currency: 'KRW', is_active: true };

export default function AccountsPage() {
  const client = useQueryClient();
  const [form, setForm] = useState(emptyForm);
  const [editTarget, setEditTarget] = useState<Account | null>(null);
  const [error, setError] = useState('');

  const accounts = useQuery({ queryKey: ['accounts'], queryFn: () => api<Account[]>('/api/accounts') });

  const create = useMutation({
    mutationFn: () =>
      api<Account>('/api/accounts', {
        method: 'POST',
        body: JSON.stringify({ ...form, institution: form.institution || null }),
      }),
    onSuccess: async () => {
      await client.invalidateQueries({ queryKey: ['accounts'] });
      setForm(emptyForm);
      setError('');
    },
    onError: (e: Error) => setError(e.message),
  });

  const update = useMutation({
    mutationFn: () =>
      api<Account>(`/api/accounts/${editTarget!.id}`, {
        method: 'PUT',
        body: JSON.stringify({ ...form, institution: form.institution || null }),
      }),
    onSuccess: async () => {
      await client.invalidateQueries({ queryKey: ['accounts'] });
      setEditTarget(null);
      setForm(emptyForm);
      setError('');
    },
    onError: (e: Error) => setError(e.message),
  });

  const remove = useMutation({
    mutationFn: (id: string) => api<{ message: string }>(`/api/accounts/${id}`, { method: 'DELETE' }),
    onSuccess: async () => client.invalidateQueries({ queryKey: ['accounts'] }),
  });

  const openEdit = (account: Account) => {
    setEditTarget(account);
    setForm({
      name: account.name,
      type: account.type,
      institution: account.institution ?? '',
      currency: account.currency,
      is_active: account.is_active,
    });
    setError('');
  };

  const handleSubmit = () => {
    setError('');
    if (!form.name.trim()) {
      setError('계좌 이름을 입력해 주세요.');
      return;
    }
    if (editTarget) {
      void update.mutateAsync();
    } else {
      void create.mutateAsync();
    }
  };

  const isPending = create.isPending || update.isPending;

  return (
    <div className="space-y-4">
      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <h1 className="text-xl font-bold">{editTarget ? '계좌 수정' : '계좌 추가'}</h1>
        <div className="mt-3 grid gap-3 md:grid-cols-2">
          <label className="text-sm">
            <span className="mb-1 block">계좌 이름</span>
            <input
              className="w-full rounded-lg border border-slate-300 px-3 py-2"
              value={form.name}
              onChange={(e) => setForm((prev) => ({ ...prev, name: e.target.value }))}
            />
          </label>
          <label className="text-sm">
            <span className="mb-1 block">유형</span>
            <select
              className="w-full rounded-lg border border-slate-300 px-3 py-2"
              value={form.type}
              onChange={(e) => setForm((prev) => ({ ...prev, type: e.target.value }))}
            >
              {ACCOUNT_TYPES.map((t) => (
                <option key={t.value} value={t.value}>{t.label}</option>
              ))}
            </select>
          </label>
          <label className="text-sm">
            <span className="mb-1 block">금융기관</span>
            <input
              className="w-full rounded-lg border border-slate-300 px-3 py-2"
              placeholder="예: 신한은행"
              value={form.institution}
              onChange={(e) => setForm((prev) => ({ ...prev, institution: e.target.value }))}
            />
          </label>
          <label className="text-sm">
            <span className="mb-1 block">통화</span>
            <input
              className="w-full rounded-lg border border-slate-300 px-3 py-2"
              value={form.currency}
              onChange={(e) => setForm((prev) => ({ ...prev, currency: e.target.value }))}
            />
          </label>
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={form.is_active}
              onChange={(e) => setForm((prev) => ({ ...prev, is_active: e.target.checked }))}
            />
            <span>활성화</span>
          </label>
        </div>
        {error ? <p className="mt-2 text-sm text-rose-600">{error}</p> : null}
        <div className="mt-3 flex gap-2">
          <button
            className="rounded-lg bg-teal-700 px-4 py-2 font-semibold text-white disabled:opacity-50"
            disabled={isPending}
            onClick={handleSubmit}
          >
            {isPending ? '저장 중...' : editTarget ? '수정 저장' : '계좌 추가'}
          </button>
          {editTarget ? (
            <button
              className="rounded-lg border border-slate-300 px-4 py-2 text-sm"
              onClick={() => { setEditTarget(null); setForm(emptyForm); setError(''); }}
            >
              취소
            </button>
          ) : null}
        </div>
      </section>

      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <h2 className="text-lg font-bold">계좌 목록</h2>
        <div className="mt-3 space-y-2">
          {accounts.data?.length ? (
            accounts.data.map((account) => (
              <div key={account.id} className="flex items-center justify-between rounded-lg border border-slate-200 p-3">
                <div>
                  <p className="font-medium">{account.name}</p>
                  <p className="text-xs text-slate-500">
                    {ACCOUNT_TYPES.find((t) => t.value === account.type)?.label ?? account.type}
                    {account.institution ? ` · ${account.institution}` : ''}
                    {!account.is_active ? ' · 비활성' : ''}
                  </p>
                </div>
                <div className="flex gap-2">
                  <button
                    className="rounded border border-slate-300 px-2 py-1 text-xs"
                    onClick={() => openEdit(account)}
                  >
                    수정
                  </button>
                  <button
                    className="rounded border border-rose-300 px-2 py-1 text-xs text-rose-700"
                    onClick={() => {
                      if (window.confirm('계좌를 삭제하시겠습니까?')) {
                        void remove.mutateAsync(account.id);
                      }
                    }}
                  >
                    삭제
                  </button>
                </div>
              </div>
            ))
          ) : (
            <p className="text-sm text-slate-500">등록된 계좌가 없습니다.</p>
          )}
        </div>
      </section>
    </div>
  );
}
