import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { resourceApi } from '@/api/resources';
import { currentMonth, money } from '@/utils/format';

const DAYS = ['일', '월', '화', '수', '목', '금', '토'];

function ym(month: string) {
  const [y, m] = month.split('-').map(Number);
  return { y, m };
}
function toStr(y: number, m: number, d: number) {
  return `${y}-${String(m).padStart(2, '0')}-${String(d).padStart(2, '0')}`;
}
function addMonth(month: string, delta: number) {
  const { y, m } = ym(month);
  const d = new Date(y, m - 1 + delta, 1);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}`;
}

export default function CalendarPage() {
  const [month, setMonth] = useState(currentMonth);
  const [selected, setSelected] = useState('');

  const { y, m } = ym(month);
  const today = toStr(new Date().getFullYear(), new Date().getMonth() + 1, new Date().getDate());
  const daysInMonth = new Date(y, m, 0).getDate();
  const startDow = new Date(y, m - 1, 1).getDay();

  const calQ = useQuery({ queryKey: ['calendar', month], queryFn: () => resourceApi.dashboardCalendar(month) });
  const dailyQ = useQuery({
    queryKey: ['daily', selected],
    queryFn: () => resourceApi.dashboardDaily(selected),
    enabled: !!selected,
  });

  const expMap = useMemo(() => {
    const map = new Map<string, number>();
    for (const r of calQ.data ?? []) map.set(r.date, r.total_expense);
    return map;
  }, [calQ.data]);

  const maxExp = Math.max(...Array.from(expMap.values()), 1);
  const monthTotal = Array.from(expMap.values()).reduce((a, b) => a + b, 0);

  // 셀 배열 (null = 빈칸, number = 날짜)
  const cells: (number | null)[] = [
    ...Array<null>(startDow).fill(null),
    ...Array.from({ length: daysInMonth }, (_, i) => i + 1),
  ];
  while (cells.length % 7 !== 0) cells.push(null);

  const weeks: (number | null)[][] = [];
  for (let i = 0; i < cells.length; i += 7) weeks.push(cells.slice(i, i + 7));

  return (
    <div className="space-y-5">
      {/* 헤더 카드 */}
      <div className="flex items-center justify-between rounded-2xl bg-white px-5 py-4 shadow-card">
        <div>
          <h1 className="text-lg font-bold text-slate-800">{y}년 {m}월</h1>
          {monthTotal > 0 && (
            <p className="mt-0.5 text-sm text-slate-500">
              이번 달 총 지출
              <span className="ml-1.5 font-semibold text-rose-500">{money(monthTotal)}</span>
            </p>
          )}
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={() => { setMonth(addMonth(month, -1)); setSelected(''); }}
            className="flex h-9 w-9 items-center justify-center rounded-xl text-slate-500 hover:bg-slate-100 text-xl font-light"
          >‹</button>
          <span className="min-w-[90px] text-center text-sm font-semibold text-slate-700">
            {y}.{String(m).padStart(2, '0')}
          </span>
          <button
            onClick={() => { setMonth(addMonth(month, 1)); setSelected(''); }}
            className="flex h-9 w-9 items-center justify-center rounded-xl text-slate-500 hover:bg-slate-100 text-xl font-light"
          >›</button>
        </div>
      </div>

      {/* 캘린더 그리드 */}
      <div className="overflow-hidden rounded-2xl bg-white shadow-card">
        {/* 요일 헤더 */}
        <div className="grid grid-cols-7 border-b border-slate-100 bg-slate-50/80">
          {DAYS.map((d, i) => (
            <div
              key={d}
              className={`py-2.5 text-center text-xs font-semibold tracking-widest
                ${i === 0 ? 'text-rose-400' : i === 6 ? 'text-blue-400' : 'text-slate-400'}`}
            >
              {d}
            </div>
          ))}
        </div>

        {/* 주 행 */}
        {calQ.isLoading ? (
          <div className="flex h-72 items-center justify-center text-sm text-slate-300">
            <div className="flex flex-col items-center gap-2">
              <div className="h-6 w-6 animate-spin rounded-full border-2 border-primary-400 border-t-transparent" />
              불러오는 중
            </div>
          </div>
        ) : (
          weeks.map((week, wi) => (
            <div key={wi} className="grid grid-cols-7 border-b border-slate-100 last:border-b-0">
              {week.map((day, di) => {
                if (!day) {
                  return (
                    <div
                      key={`e-${wi}-${di}`}
                      className="min-h-[80px] bg-slate-50/40"
                    />
                  );
                }
                const dateStr = toStr(y, m, day);
                const exp = expMap.get(dateStr) ?? 0;
                const isToday = dateStr === today;
                const isSel = dateStr === selected;
                const dow = (startDow + day - 1) % 7;
                const isSun = dow === 0;
                const isSat = dow === 6;
                const barW = exp > 0 ? Math.max(15, Math.round((exp / maxExp) * 100)) : 0;

                return (
                  <button
                    key={dateStr}
                    onClick={() => setSelected(isSel ? '' : dateStr)}
                    className={`group relative min-h-[80px] border-r border-slate-100 p-2 text-left transition-all last:border-r-0
                      ${isSel
                        ? 'bg-primary-50 ring-1 ring-inset ring-primary-300'
                        : 'hover:bg-slate-50/80'
                      }`}
                  >
                    {/* 날짜 숫자 */}
                    <span
                      className={`inline-flex h-6 w-6 items-center justify-center rounded-full text-[13px] font-semibold leading-none
                        ${isToday
                          ? 'bg-primary-600 text-white shadow-soft'
                          : isSun ? 'text-rose-400' : isSat ? 'text-blue-400' : 'text-slate-700'
                        }`}
                    >
                      {day}
                    </span>

                    {/* 지출 */}
                    {exp > 0 && (
                      <div className="mt-1.5 space-y-1">
                        {/* 비율 바 */}
                        <div className="h-1 w-full rounded-full bg-slate-100">
                          <div
                            className={`h-1 rounded-full transition-all ${isSel ? 'bg-primary-400' : 'bg-rose-300 group-hover:bg-rose-400'}`}
                            style={{ width: `${barW}%` }}
                          />
                        </div>
                        {/* 금액 */}
                        <p className="text-[10px] font-semibold leading-none text-rose-500">
                          {exp >= 100000
                            ? `${Math.round(exp / 10000)}만`
                            : exp >= 10000
                              ? `${(exp / 10000).toFixed(1)}만`
                              : money(exp)}
                        </p>
                      </div>
                    )}
                  </button>
                );
              })}
            </div>
          ))
        )}
      </div>

      {/* 선택 날짜 상세 */}
      {selected && (
        <div className="overflow-hidden rounded-2xl bg-white shadow-card">
          {/* 상세 헤더 */}
          <div className="flex items-center justify-between border-b border-slate-100 px-5 py-4">
            <div>
              <h2 className="font-bold text-slate-800">
                {selected.replace(/-/g, '.')}
              </h2>
              <div className="mt-0.5 flex gap-4 text-sm">
                {(expMap.get(selected) ?? 0) > 0 && (
                  <span className="text-slate-500">
                    지출 <span className="font-semibold text-rose-500">{money(expMap.get(selected)!)}</span>
                  </span>
                )}
                {(dailyQ.data?.total_income ?? 0) > 0 && (
                  <span className="text-slate-500">
                    수입 <span className="font-semibold text-emerald-500">{money(dailyQ.data!.total_income)}</span>
                  </span>
                )}
              </div>
            </div>
            <button
              onClick={() => setSelected('')}
              className="flex h-8 w-8 items-center justify-center rounded-full text-slate-400 hover:bg-slate-100"
            >✕</button>
          </div>

          {/* 거래 목록 */}
          <div className="divide-y divide-slate-100">
            {dailyQ.isLoading ? (
              <div className="flex h-24 items-center justify-center text-sm text-slate-300">
                <div className="h-5 w-5 animate-spin rounded-full border-2 border-primary-400 border-t-transparent" />
              </div>
            ) : dailyQ.data?.transactions?.length ? (
              dailyQ.data.transactions.map((tx) => (
                <div key={tx.id} className="flex items-center justify-between gap-3 px-5 py-3.5">
                  <div className="min-w-0">
                    <p className="truncate text-sm font-medium text-slate-800">
                      {tx.merchant_name || tx.description || '이름 없음'}
                    </p>
                    <p className="mt-0.5 text-xs text-slate-400">
                      {new Date(tx.transaction_at).toLocaleTimeString('ko-KR', { hour: '2-digit', minute: '2-digit' })}
                      {tx.card_name && <span className="ml-1.5 rounded bg-slate-100 px-1.5 py-0.5 text-[10px]">{tx.card_name}</span>}
                    </p>
                  </div>
                  <span className={`shrink-0 text-sm font-bold ${tx.type === 'income' ? 'text-emerald-500' : 'text-rose-500'}`}>
                    {tx.type === 'income' ? '+' : '-'}{money(tx.amount)}
                  </span>
                </div>
              ))
            ) : (
              <p className="px-5 py-8 text-center text-sm text-slate-400">거래 내역이 없습니다.</p>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
