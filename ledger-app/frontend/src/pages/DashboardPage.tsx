import { useQuery } from '@tanstack/react-query';
import { resourceApi } from '@/api/resources';
import { currentMonth, money, today } from '@/utils/format';

export default function DashboardPage() {
  const month = currentMonth;
  const monthly = useQuery({
    queryKey: ['dashboard', 'monthly', month],
    queryFn: () => resourceApi.dashboardMonthly(month)
  });

  return (
    <div className="space-y-5">
      <section className="rounded-2xl bg-white p-5 shadow-soft">
        <h2 className="text-xl font-bold">이번 달 요약</h2>
        <p className="mt-1 text-sm text-slate-500">기준 월: {month}</p>

        <div className="mt-4 grid gap-3 sm:grid-cols-3">
          <div className="rounded-xl bg-emerald-50 p-4">
            <p className="text-sm text-slate-600">총수입</p>
            <p className="mt-1 text-lg font-semibold text-emerald-700">
              {money(monthly.data?.total_income ?? 0)}
            </p>
          </div>
          <div className="rounded-xl bg-rose-50 p-4">
            <p className="text-sm text-slate-600">이번 달 지출</p>
            <p className="mt-1 text-lg font-semibold text-rose-700">
              {money(monthly.data?.total_expense ?? 0)}
            </p>
          </div>
          <div className="rounded-xl bg-slate-100 p-4">
            <p className="text-sm text-slate-600">순지출</p>
            <p className="mt-1 text-lg font-semibold text-slate-900">
              {money(monthly.data?.net_expense ?? 0)}
            </p>
          </div>
        </div>

        <div className="mt-4 rounded-lg border border-teal-100 bg-teal-50 p-3">
          <p className="text-sm text-slate-600">전월 대비 지출 증감</p>
          <p className="font-semibold text-teal-800">
            {money(monthly.data?.comparison.expense_change_amount ?? 0)}
            <span className="ml-1 text-sm">({(monthly.data?.comparison.expense_change_rate ?? 0).toFixed(1)}%)</span>
          </p>
        </div>
      </section>

      <section className="rounded-2xl bg-white p-5 shadow-soft">
        <h3 className="text-lg font-bold">카테고리별 지출 상위</h3>
        <div className="mt-3 space-y-2">
          {monthly.data?.category_expense.length ? (
            monthly.data.category_expense.slice(0, 5).map((item) => (
              <div key={item.name} className="flex items-center justify-between rounded-lg border border-slate-100 p-3">
                <p className="font-medium">{item.name}</p>
                <p className="font-semibold text-rose-700">{money(item.amount)}</p>
              </div>
            ))
          ) : (
            <p className="rounded-lg border border-dashed border-slate-300 p-4 text-sm text-slate-500">
              아직 등록된 지출이 없습니다.
            </p>
          )}
        </div>
      </section>

      <section className="rounded-2xl bg-white p-5 shadow-soft">
        <h3 className="text-lg font-bold">최근 거래</h3>
        <p className="text-sm text-slate-500">오늘 날짜: {today()}</p>
        <div className="mt-3 space-y-2">
          {monthly.data?.recent_transactions.length ? (
            monthly.data.recent_transactions.map((tx) => (
              <div key={tx.id} className="flex items-center justify-between rounded-lg border border-slate-100 p-3">
                <div>
                  <p className="font-medium">{tx.merchant_name || tx.description || '이름 없음'}</p>
                  <p className="text-xs text-slate-500">{new Date(tx.transaction_at).toLocaleString('ko-KR')}</p>
                </div>
                <p className={tx.type === 'income' ? 'font-semibold text-emerald-700' : 'font-semibold text-rose-700'}>
                  {tx.type === 'income' ? '+' : '-'} {money(tx.amount)}
                </p>
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
