import { useMemo, useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { resourceApi } from '@/api/resources';
import { currentMonth, dateTime, money } from '@/utils/format';

export default function CardDetailPage() {
  const { id } = useParams();
  const client = useQueryClient();
  const [month, setMonth] = useState(currentMonth);

  const cards = useQuery({ queryKey: ['cards'], queryFn: resourceApi.cards });
  const card = useMemo(() => cards.data?.find((v) => v.id === id), [cards.data, id]);
  const presets = useQuery({ queryKey: ['card-presets'], queryFn: resourceApi.cardPresets });

  const summary = useQuery({
    queryKey: ['cards', id, 'summary', month],
    queryFn: () => resourceApi.cardSummary(id!, month),
    enabled: Boolean(id)
  });

  const transactions = useQuery({
    queryKey: ['cards', id, 'transactions', month],
    queryFn: () => resourceApi.cardTransactions(id!, month),
    enabled: Boolean(id)
  });

  const linkPreset = useMutation({
    mutationFn: (presetId: string | null) =>
      resourceApi.updateCard(id!, { preset_id: presetId }),
    onSuccess: async () => {
      await Promise.all([
        client.invalidateQueries({ queryKey: ['cards'] }),
        client.invalidateQueries({ queryKey: ['cards', id, 'summary', month] }),
      ]);
    },
    onError: (err: Error) => window.alert(err.message),
  });

  const applyPreset = useMutation({
    mutationFn: (presetId: string) =>
      resourceApi.applyCardPreset(presetId),
    onSuccess: (result) => {
      window.alert(
        `기존 거래 혜택 적용 완료\n적용: ${result.applied_count}건 / 스킵: ${result.skipped_count}건`
      );
    },
    onError: (err: Error) => window.alert(err.message),
  });

  if (!id) {
    return (
      <div className="rounded-2xl bg-white p-4 shadow-soft">
        <p>잘못된 카드 경로입니다.</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <div className="flex items-center justify-between gap-3">
          <div>
            <h1 className="text-xl font-bold">카드 상세</h1>
            <p className="text-sm text-slate-500">{card ? `${card.issuer} · ${card.card_name}` : '카드 정보를 불러오는 중입니다.'}</p>
          </div>
          <Link to="/cards" className="rounded border border-slate-300 px-3 py-2 text-sm">
            카드 목록으로
          </Link>
        </div>

        <div className="mt-3 max-w-xs">
          <label className="text-sm">
            <span className="mb-1 block text-slate-600">조회 월</span>
            <input
              type="month"
              className="w-full rounded-lg border border-slate-300 px-3 py-2"
              value={month}
              onChange={(e) => setMonth(e.target.value)}
            />
          </label>
        </div>
      </section>

      {/* 프리셋 연결 */}
      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h2 className="text-lg font-bold">혜택 프리셋</h2>
            {card?.preset_id ? (
              <p className="mt-0.5 text-sm text-teal-700">
                {presets.data?.find((p) => p.id === card.preset_id)?.card_name ?? '프리셋 연결됨'}
              </p>
            ) : (
              <p className="mt-0.5 text-sm text-slate-400">연결된 프리셋이 없습니다.</p>
            )}
          </div>
          {card?.preset_id && (
            <button
              className="rounded-lg border border-teal-300 bg-teal-50 px-3 py-1.5 text-xs font-semibold text-teal-700 hover:bg-teal-100 disabled:opacity-40"
              disabled={applyPreset.isPending}
              onClick={() => {
                if (window.confirm('이 프리셋을 기존 거래에 소급 적용하시겠습니까?\n이미 적용된 건은 건너뜁니다.'))
                  void applyPreset.mutateAsync(card.preset_id!);
              }}
            >
              {applyPreset.isPending ? '적용 중...' : '기존 거래에 적용'}
            </button>
          )}
        </div>

        <div className="mt-3 flex flex-wrap items-center gap-2">
          <select
            className="rounded-lg border border-slate-300 px-3 py-2 text-sm"
            value={card?.preset_id ?? ''}
            onChange={(e) => {
              const val = e.target.value;
              void linkPreset.mutateAsync(val || null);
            }}
            disabled={linkPreset.isPending}
          >
            <option value="">프리셋 없음</option>
            {presets.data?.map((p) => (
              <option key={p.id} value={p.id}>
                {p.issuer} · {p.card_name}
                {p.monthly_requirement != null ? ` (실적 ${(p.monthly_requirement / 10000).toFixed(0)}만원)` : ''}
              </option>
            ))}
          </select>
          {linkPreset.isPending && (
            <span className="text-xs text-slate-500">저장 중...</span>
          )}
          <Link
            to="/cards/presets"
            className="rounded-lg border border-slate-300 px-3 py-2 text-xs text-slate-600 hover:bg-slate-50"
          >
            프리셋 관리
          </Link>
        </div>
      </section>

      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <h2 className="text-lg font-bold">카드 실적</h2>
        <div className="mt-3 grid gap-2 sm:grid-cols-2">
          <div className="rounded-lg bg-slate-100 p-3">
            <p className="text-sm">이번 달 사용액</p>
            <p className="font-semibold">{money(summary.data?.summary.monthly_spending ?? 0)}</p>
          </div>
          <div className="rounded-lg bg-slate-100 p-3">
            <p className="text-sm">실적 인정금액</p>
            <p className="font-semibold">{money(summary.data?.summary.eligible_spending ?? 0)}</p>
          </div>
          <div className="rounded-lg bg-slate-100 p-3">
            <p className="text-sm">실적 기준</p>
            <p className="font-semibold">{money(summary.data?.summary.monthly_requirement ?? 0)}</p>
          </div>
          <div className="rounded-lg bg-slate-100 p-3">
            <p className="text-sm">실적 달성률</p>
            <p className="font-semibold">{(summary.data?.summary.requirement_ratio ?? 0).toFixed(1)}%</p>
          </div>
        </div>
      </section>

      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <h2 className="text-lg font-bold">혜택 사용률</h2>
        {summary.data?.summary.benefits.length ? (
          <div className="mt-3 space-y-2">
            {summary.data.summary.benefits.map((benefit) => {
              const ratio = benefit.cap <= 0 ? 0 : Math.min(100, (benefit.used_amount / benefit.cap) * 100);
              return (
                <div key={benefit.name} className="rounded-lg border border-slate-200 p-3">
                  <div className="flex items-center justify-between">
                    <p className="font-medium">{benefit.name}</p>
                    <p className="text-sm text-slate-600">{ratio.toFixed(1)}%</p>
                  </div>
                  <div className="mt-2 h-2 overflow-hidden rounded bg-slate-200">
                    <div className="h-2 bg-teal-600" style={{ width: `${ratio}%` }} />
                  </div>
                  <div className="mt-2 flex items-center justify-between text-sm text-slate-600">
                    <p>혜택별 사용 금액: {money(benefit.used_amount)}</p>
                    <p>혜택별 한도: {money(benefit.cap)}</p>
                  </div>
                </div>
              );
            })}
          </div>
        ) : (
          <p className="mt-3 text-sm text-slate-500">등록된 혜택 규칙이 없습니다.</p>
        )}
      </section>

      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <h2 className="text-lg font-bold">월간 이용 내역</h2>
        <p className="mt-1 text-sm text-slate-600">
          건수: {transactions.data?.total_count ?? 0}건 / 합계: {money(transactions.data?.total_amount ?? 0)}
        </p>

        <div className="mt-3 space-y-2">
          {transactions.data?.transactions.length ? (
            transactions.data.transactions.map((tx) => (
              <div key={tx.id} className="rounded-lg border border-slate-200 p-3">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="font-medium">{tx.merchant_name || tx.description || '내역 없음'}</p>
                    <p className="text-xs text-slate-500">{dateTime(tx.transaction_at)}</p>
                  </div>
                  <p className="font-semibold text-rose-700">{money(tx.amount)}</p>
                </div>
                <div className="mt-1 text-xs text-slate-500">
                  <span>{tx.category_name || '미분류'}</span>
                  <span className="mx-1">·</span>
                  <span>{tx.account_name || '계좌 없음'}</span>
                  {tx.memo ? (
                    <>
                      <span className="mx-1">·</span>
                      <span>{tx.memo}</span>
                    </>
                  ) : null}
                </div>
              </div>
            ))
          ) : (
            <p className="text-sm text-slate-500">해당 월의 카드 이용 내역이 없습니다.</p>
          )}
        </div>
      </section>
    </div>
  );
}
