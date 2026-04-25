import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { resourceApi } from '@/api/resources';
import { currentMonth, money } from '@/utils/format';

const BAR_COLORS = [
  '#0f766e', '#14b8a6', '#2dd4bf', '#5eead4',
  '#0284c7', '#7c3aed', '#db2777', '#ea580c',
];

export default function MonthlySummaryPage() {
  const [month, setMonth] = useState(currentMonth);
  const [selectedCat, setSelectedCat] = useState<{ id: string | null; name: string } | null>(null);

  const monthly = useQuery({
    queryKey: ['dashboard', 'monthly', month, 'summary'],
    queryFn: () => resourceApi.dashboardMonthly(month),
  });

  // 선택한 카테고리의 거래 목록
  const { start, end } = useMemo(() => {
    const [y, m] = month.split('-').map(Number);
    const s = `${month}-01`;
    const e = new Date(y, m, 0).toISOString().slice(0, 10);
    return { start: s, end: e };
  }, [month]);

  const catTxQuery = useQuery({
    queryKey: ['transactions', 'category', selectedCat?.id ?? 'null', month],
    queryFn: () => {
      const qs = selectedCat?.id
        ? `?category_id=${selectedCat.id}&start_date=${start}&end_date=${end}`
        : `?start_date=${start}&end_date=${end}`;
      // 미분류는 category_id 없이 불러온 뒤 필터
      return resourceApi.transactions(qs);
    },
    enabled: selectedCat !== null,
    select: (data) => {
      if (selectedCat?.id) return data;
      // 미분류: category_id가 null인 expense만
      return data.filter((t) => !t.category_id && t.type === 'expense');
    },
  });

  const categoryBars = useMemo(() => {
    const list = monthly.data?.category_expense ?? [];
    const max = list.length ? Math.max(...list.map((v) => v.amount)) : 0;
    return list.map((item) => ({ ...item, ratio: max <= 0 ? 0 : (item.amount / max) * 100 }));
  }, [monthly.data?.category_expense]);

  const cardDonut = useMemo(() => {
    const list = (monthly.data?.card_expense ?? []).slice(0, 5);
    const total = list.reduce((a, b) => a + b.amount, 0);
    if (total <= 0) return { background: '#e2e8f0', list, total };
    let deg = 0;
    const segs = list.map((item, i) => {
      const a = (item.amount / total) * 360;
      const s = `${BAR_COLORS[i % BAR_COLORS.length]} ${deg}deg ${deg + a}deg`;
      deg += a;
      return s;
    });
    return { background: `conic-gradient(${segs.join(', ')})`, list, total };
  }, [monthly.data?.card_expense]);

  function handleCatClick(id: string | null, name: string) {
    if (selectedCat?.name === name) {
      setSelectedCat(null);
    } else {
      setSelectedCat({ id, name });
    }
  }

  return (
    <div className="space-y-4">
      {/* 헤더 */}
      <section className="rounded-2xl bg-white p-5 shadow-card">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <h1 className="text-xl font-bold text-slate-800">월별 정산</h1>
          <input
            type="month"
            className="rounded-lg border border-slate-200 px-3 py-2 text-sm focus:border-primary-500 focus:outline-none"
            value={month}
            onChange={(e) => { setMonth(e.target.value); setSelectedCat(null); }}
          />
        </div>
        <div className="mt-4 grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          {[
            { label: '총수입', value: monthly.data?.total_income ?? 0, color: 'text-emerald-600', bg: 'bg-emerald-50' },
            { label: '총지출', value: monthly.data?.total_expense ?? 0, color: 'text-rose-600', bg: 'bg-rose-50' },
            { label: '순지출', value: monthly.data?.net_expense ?? 0, color: 'text-slate-700', bg: 'bg-slate-100' },
            { label: '전월 대비', value: monthly.data?.comparison.net_expense_change_amount ?? 0, color: 'text-slate-700', bg: 'bg-slate-100' },
          ].map(({ label, value, color, bg }) => (
            <div key={label} className={`rounded-xl ${bg} p-4`}>
              <p className="text-xs font-medium text-slate-500">{label}</p>
              <p className={`mt-1 text-lg font-bold ${color}`}>{money(value)}</p>
            </div>
          ))}
        </div>
      </section>

      {/* 카테고리별 지출 */}
      <section className="rounded-2xl bg-white p-5 shadow-card">
        <h2 className="text-base font-bold text-slate-800">카테고리별 지출</h2>
        <p className="mt-0.5 text-xs text-slate-400">항목을 누르면 해당 카테고리 거래를 볼 수 있습니다.</p>

        <div className="mt-4 space-y-2">
          {categoryBars.length ? categoryBars.map((item, idx) => {
            const isOpen = selectedCat?.name === item.name;
            const color = BAR_COLORS[idx % BAR_COLORS.length];
            return (
              <div key={item.name}>
                <button
                  onClick={() => handleCatClick(item.category_id, item.name)}
                  className={`w-full rounded-xl border p-3.5 text-left transition-all
                    ${isOpen
                      ? 'border-primary-300 bg-primary-50 shadow-soft'
                      : 'border-slate-100 bg-slate-50/60 hover:border-slate-200 hover:bg-slate-50'
                    }`}
                >
                  <div className="flex items-center justify-between text-sm">
                    <div className="flex items-center gap-2">
                      <span
                        className="h-2.5 w-2.5 rounded-full shrink-0"
                        style={{ backgroundColor: color }}
                      />
                      <span className={`font-medium ${isOpen ? 'text-primary-700' : 'text-slate-700'}`}>
                        {item.name}
                      </span>
                    </div>
                    <div className="flex items-center gap-2">
                      <span className="font-bold text-rose-500">{money(item.amount)}</span>
                      <span className={`text-xs transition-transform ${isOpen ? 'rotate-90' : ''} text-slate-400`}>▶</span>
                    </div>
                  </div>
                  <div className="mt-2.5 h-1.5 w-full overflow-hidden rounded-full bg-slate-200">
                    <div
                      className="h-1.5 rounded-full transition-all duration-500"
                      style={{ width: `${item.ratio}%`, backgroundColor: color }}
                    />
                  </div>
                </button>

                {/* 인라인 거래 목록 */}
                {isOpen && (
                  <div className="mx-1 overflow-hidden rounded-b-xl border border-t-0 border-primary-200 bg-white">
                    {catTxQuery.isLoading ? (
                      <div className="flex h-20 items-center justify-center">
                        <div className="h-5 w-5 animate-spin rounded-full border-2 border-primary-400 border-t-transparent" />
                      </div>
                    ) : catTxQuery.data?.length ? (
                      <ul className="divide-y divide-slate-100">
                        {catTxQuery.data.map((tx) => (
                          <li key={tx.id} className="flex items-center justify-between gap-3 px-4 py-3">
                            <div className="min-w-0">
                              <p className="truncate text-sm font-medium text-slate-800">
                                {tx.merchant_name || tx.description || '이름 없음'}
                              </p>
                              <p className="text-xs text-slate-400">
                                {new Date(tx.transaction_at).toLocaleDateString('ko-KR', { month: 'short', day: 'numeric' })}
                                {tx.card_name && (
                                  <span className="ml-1.5 rounded bg-slate-100 px-1.5 py-0.5 text-[10px]">
                                    {tx.card_name}
                                  </span>
                                )}
                              </p>
                            </div>
                            <span className="shrink-0 text-sm font-bold text-rose-500">
                              -{money(tx.amount)}
                            </span>
                          </li>
                        ))}
                      </ul>
                    ) : (
                      <p className="px-4 py-6 text-center text-sm text-slate-400">거래 내역이 없습니다.</p>
                    )}
                  </div>
                )}
              </div>
            );
          }) : (
            <p className="text-sm text-slate-400">카테고리별 지출 데이터가 없습니다.</p>
          )}
        </div>
      </section>

      {/* 카드별 지출 */}
      <section className="rounded-2xl bg-white p-5 shadow-card">
        <h2 className="text-base font-bold text-slate-800">카드별 지출 비중</h2>
        {cardDonut.total > 0 ? (
          <div className="mt-4 grid gap-4 md:grid-cols-[200px_1fr] md:items-center">
            <div
              className="mx-auto grid h-40 w-40 place-items-center rounded-full"
              style={{ background: cardDonut.background }}
            >
              <div className="grid h-24 w-24 place-items-center rounded-full bg-white text-center">
                <span className="text-xs text-slate-500">합계<br /><span className="font-bold text-slate-800">{money(cardDonut.total)}</span></span>
              </div>
            </div>
            <div className="space-y-2">
              {cardDonut.list.map((item, idx) => (
                <div key={item.name} className="flex items-center justify-between rounded-xl border border-slate-100 bg-slate-50 p-3">
                  <div className="flex items-center gap-2">
                    <span className="h-3 w-3 rounded" style={{ backgroundColor: BAR_COLORS[idx % BAR_COLORS.length] }} />
                    <span className="text-sm text-slate-700">{item.name}</span>
                  </div>
                  <span className="text-sm font-bold text-rose-500">{money(item.amount)}</span>
                </div>
              ))}
            </div>
          </div>
        ) : (
          <p className="mt-3 text-sm text-slate-400">카드별 지출 데이터가 없습니다.</p>
        )}
      </section>

      {/* 계좌별 지출 */}
      <section className="rounded-2xl bg-white p-5 shadow-card">
        <h2 className="text-base font-bold text-slate-800">계좌별 지출</h2>
        <div className="mt-4 space-y-2">
          {monthly.data?.account_expense.length ? monthly.data.account_expense.map((item) => (
            <div key={item.name} className="flex items-center justify-between rounded-xl border border-slate-100 bg-slate-50 p-3">
              <p className="text-sm text-slate-700">{item.name}</p>
              <p className="text-sm font-bold text-rose-500">{money(item.amount)}</p>
            </div>
          )) : (
            <p className="text-sm text-slate-400">계좌별 지출 데이터가 없습니다.</p>
          )}
        </div>
      </section>
    </div>
  );
}
