import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useSearchParams } from 'react-router-dom';
import { resourceApi } from '@/api/resources';
import { dateTime, money, today } from '@/utils/format';
import type { Transaction } from '@/types';

const emptyForm = {
  transaction_at: `${today()}T09:00:00+09:00`,
  type: 'expense',
  amount: 0,
  merchant_name: '',
  description: '',
  category_id: '',
  account_id: '',
  card_id: '',
  memo: ''
};

export default function TransactionsPage() {
  const [params, setParams] = useSearchParams();
  const [keyword, setKeyword] = useState('');
  const [filterType, setFilterType] = useState('');
  const [startDate, setStartDate] = useState('');
  const [endDate, setEndDate] = useState('');
  const [편집대상, set편집대상] = useState<Transaction | null>(null);
  const [form, setForm] = useState(emptyForm);
  const client = useQueryClient();

  const transactions = useQuery({
    queryKey: ['transactions', keyword, filterType, startDate, endDate],
    queryFn: () => {
      const searchParams = new URLSearchParams();
      if (keyword) searchParams.set('keyword', keyword);
      if (filterType) searchParams.set('type', filterType);
      if (startDate) searchParams.set('start_date', startDate);
      if (endDate) searchParams.set('end_date', endDate);
      const qs = searchParams.toString();
      return resourceApi.transactions(qs ? `?${qs}` : undefined);
    }
  });
  const accounts = useQuery({ queryKey: ['accounts'], queryFn: resourceApi.accounts });
  const categories = useQuery({ queryKey: ['categories'], queryFn: resourceApi.categories });
  const cards = useQuery({ queryKey: ['cards'], queryFn: resourceApi.cards });

  const 생성 = useMutation({
    mutationFn: () =>
      resourceApi.createTransaction({
        ...form,
        amount: Number(form.amount),
        category_id: form.category_id || null,
        account_id: form.account_id || null,
        card_id: form.card_id || null,
        posted_at: null,
        source_type: 'manual'
      }),
    onSuccess: async () => {
      await client.invalidateQueries({ queryKey: ['transactions'] });
      setParams({});
      setForm(emptyForm);
    }
  });

  const 수정 = useMutation({
    mutationFn: () =>
      resourceApi.updateTransaction(편집대상!.id, {
        ...form,
        amount: Number(form.amount),
        category_id: form.category_id || null,
        account_id: form.account_id || null,
        card_id: form.card_id || null,
        posted_at: null,
        source_type: 'manual'
      }),
    onSuccess: async () => {
      await client.invalidateQueries({ queryKey: ['transactions'] });
      set편집대상(null);
      setParams({});
      setForm(emptyForm);
    }
  });

  const 삭제 = useMutation({
    mutationFn: (id: string) => resourceApi.deleteTransaction(id),
    onSuccess: async () => {
      await client.invalidateQueries({ queryKey: ['transactions'] });
    }
  });

  const 편집열기 = (tx: Transaction) => {
    set편집대상(tx);
    setParams({ new: '1' });
    setForm({
      transaction_at: new Date(tx.transaction_at).toISOString().slice(0, 19) + 'Z',
      type: tx.type,
      amount: tx.amount,
      merchant_name: tx.merchant_name || '',
      description: tx.description || '',
      category_id: tx.category_id || '',
      account_id: tx.account_id || '',
      card_id: tx.card_id || '',
      memo: tx.memo || ''
    });
  };

  const 폼열림 = useMemo(() => params.get('new') === '1', [params]);

  return (
    <div className="space-y-4">
      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h1 className="text-xl font-bold">거래 목록</h1>
          <button
            className="rounded-lg bg-teal-700 px-3 py-2 text-sm font-semibold text-white"
            onClick={() => {
              set편집대상(null);
              setForm(emptyForm);
              setParams({ new: '1' });
            }}
          >
            거래 추가
          </button>
        </div>
        <div className="mt-3 grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
          <input
            className="rounded-lg border border-slate-300 px-3 py-2 sm:col-span-2 lg:col-span-2"
            placeholder="가맹점, 내용, 메모 검색"
            value={keyword}
            onChange={(e) => setKeyword(e.target.value)}
          />
          <select
            className="rounded-lg border border-slate-300 px-3 py-2"
            value={filterType}
            onChange={(e) => setFilterType(e.target.value)}
          >
            <option value="">전체 유형</option>
            <option value="expense">지출</option>
            <option value="income">수입</option>
            <option value="transfer">이체</option>
            <option value="card_payment">카드결제</option>
          </select>
          <div className="flex gap-1">
            <input
              type="date"
              className="w-full rounded-lg border border-slate-300 px-2 py-2 text-sm"
              value={startDate}
              onChange={(e) => setStartDate(e.target.value)}
              placeholder="시작일"
            />
            <input
              type="date"
              className="w-full rounded-lg border border-slate-300 px-2 py-2 text-sm"
              value={endDate}
              onChange={(e) => setEndDate(e.target.value)}
              placeholder="종료일"
            />
          </div>
        </div>
      </section>

      {폼열림 ? (
        <section className="rounded-2xl bg-white p-4 shadow-soft">
          <h2 className="text-lg font-bold">거래 입력</h2>
          <div className="mt-3 grid gap-3 md:grid-cols-2">
            <label className="text-sm">
              <span className="mb-1 block">금액</span>
              <input
                type="number"
                className="w-full rounded-lg border border-slate-300 px-3 py-2"
                value={form.amount}
                onChange={(e) => setForm((prev) => ({ ...prev, amount: Number(e.target.value) }))}
              />
            </label>
            <label className="text-sm">
              <span className="mb-1 block">가맹점</span>
              <input
                className="w-full rounded-lg border border-slate-300 px-3 py-2"
                value={form.merchant_name}
                onChange={(e) => setForm((prev) => ({ ...prev, merchant_name: e.target.value }))}
              />
            </label>
            <label className="text-sm">
              <span className="mb-1 block">거래일시</span>
              <input
                type="datetime-local"
                className="w-full rounded-lg border border-slate-300 px-3 py-2"
                value={form.transaction_at.replace('Z', '').slice(0, 16)}
                onChange={(e) =>
                  setForm((prev) => ({ ...prev, transaction_at: `${e.target.value}:00+09:00` }))
                }
              />
            </label>
            <label className="text-sm">
              <span className="mb-1 block">유형</span>
              <select
                className="w-full rounded-lg border border-slate-300 px-3 py-2"
                value={form.type}
                onChange={(e) => setForm((prev) => ({ ...prev, type: e.target.value }))}
              >
                <option value="expense">지출</option>
                <option value="income">수입</option>
                <option value="transfer">이체</option>
                <option value="card_payment">카드결제</option>
              </select>
            </label>
            <label className="text-sm">
              <span className="mb-1 block">카테고리</span>
              <select
                className="w-full rounded-lg border border-slate-300 px-3 py-2"
                value={form.category_id}
                onChange={(e) => setForm((prev) => ({ ...prev, category_id: e.target.value }))}
              >
                <option value="">선택 안 함</option>
                {categories.data?.map((category) => (
                  <option key={category.id} value={category.id}>
                    {category.name}
                  </option>
                ))}
              </select>
            </label>
            <label className="text-sm">
              <span className="mb-1 block">계좌</span>
              <select
                className="w-full rounded-lg border border-slate-300 px-3 py-2"
                value={form.account_id}
                onChange={(e) => setForm((prev) => ({ ...prev, account_id: e.target.value }))}
              >
                <option value="">선택 안 함</option>
                {accounts.data?.map((account) => (
                  <option key={account.id} value={account.id}>
                    {account.name}
                  </option>
                ))}
              </select>
            </label>
            <label className="text-sm">
              <span className="mb-1 block">카드</span>
              <select
                className="w-full rounded-lg border border-slate-300 px-3 py-2"
                value={form.card_id}
                onChange={(e) => setForm((prev) => ({ ...prev, card_id: e.target.value }))}
              >
                <option value="">선택 안 함</option>
                {cards.data?.map((card) => (
                  <option key={card.id} value={card.id}>
                    {card.card_name}
                  </option>
                ))}
              </select>
            </label>
            <label className="text-sm">
              <span className="mb-1 block">내용</span>
              <input
                className="w-full rounded-lg border border-slate-300 px-3 py-2"
                value={form.description}
                onChange={(e) => setForm((prev) => ({ ...prev, description: e.target.value }))}
              />
            </label>
            <label className="text-sm md:col-span-2">
              <span className="mb-1 block">메모</span>
              <input
                className="w-full rounded-lg border border-slate-300 px-3 py-2"
                value={form.memo}
                onChange={(e) => setForm((prev) => ({ ...prev, memo: e.target.value }))}
              />
            </label>
          </div>

          <div className="mt-4 flex gap-2">
            <button
              className="rounded-lg bg-teal-700 px-4 py-2 font-semibold text-white"
              onClick={() => {
                if (편집대상) {
                  void 수정.mutateAsync();
                } else {
                  void 생성.mutateAsync();
                }
              }}
            >
              저장
            </button>
            <button
              className="rounded-lg border border-slate-300 px-4 py-2"
              onClick={() => {
                set편집대상(null);
                setParams({});
              }}
            >
              취소
            </button>
          </div>
        </section>
      ) : null}

      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <div className="space-y-2">
          {transactions.data?.length ? (
            transactions.data.map((tx) => (
              <div key={tx.id} className="rounded-lg border border-slate-100 p-3">
                <div className="flex items-start justify-between gap-4">
                  <div>
                    <p className="font-semibold text-slate-900">{tx.merchant_name || tx.description || '이름 없음'}</p>
                    <p className="text-xs text-slate-500">{dateTime(tx.transaction_at)}</p>
                  </div>
                  <p className={tx.type === 'income' ? 'font-semibold text-emerald-700' : 'font-semibold text-rose-700'}>
                    {tx.type === 'income' ? '+' : '-'} {money(tx.amount)}
                  </p>
                </div>
                <div className="mt-2 flex gap-2">
                  <button
                    className="rounded border border-slate-300 px-2 py-1 text-xs"
                    onClick={() => 편집열기(tx)}
                  >
                    수정
                  </button>
                  <button
                    className="rounded border border-rose-300 px-2 py-1 text-xs text-rose-700"
                    onClick={() => {
                      if (window.confirm('정말 삭제하시겠습니까?')) {
                        void 삭제.mutateAsync(tx.id);
                      }
                    }}
                  >
                    삭제
                  </button>
                </div>
              </div>
            ))
          ) : (
            <p className="rounded-lg border border-dashed border-slate-300 p-4 text-sm text-slate-500">
              아직 등록된 거래가 없습니다.
            </p>
          )}
        </div>
      </section>
    </div>
  );
}
