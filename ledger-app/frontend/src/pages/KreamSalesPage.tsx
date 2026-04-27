import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { resourceApi } from '@/api/resources';
import { dateTime, money, today } from '@/utils/format';
import type { KreamLedgerTransaction, KreamSale, KreamTransactionCandidate } from '@/types';

type MatchKind = 'purchase' | 'settlement' | 'side_cost';
type SaleMatchKind = 'purchase' | 'settlement';
type LedgerLinkDraft = { saleId: string; kind: MatchKind };
type LedgerTab = 'unlinked' | 'purchase' | 'settlement' | 'side_cost';

const KIND_LABEL: Record<MatchKind, string> = {
  purchase: '구매',
  settlement: '정산',
  side_cost: '부대비용'
};

const TAB_META: Record<LedgerTab, { label: string; tone: 'rose' | 'emerald' | 'amber' | 'slate' }> = {
  unlinked:   { label: '미연결',        tone: 'slate' },
  purchase:   { label: '구매 연결됨',   tone: 'rose' },
  settlement: { label: '정산 연결됨',   tone: 'emerald' },
  side_cost:  { label: '공통 부대비용', tone: 'amber' },
};

const EMPTY_MSG: Record<LedgerTab, string> = {
  unlinked:   '모든 거래가 분류되었습니다.',
  purchase:   '구매로 연결된 거래가 없습니다.',
  settlement: '정산으로 연결된 거래가 없습니다.',
  side_cost:  '공통 부대비용으로 분류된 거래가 없습니다.',
};

const emptyManualForm = {
  product_name: '',
  purchase_date: today(),
  settlement_date: '',
  purchase_price: 0,
  settlement_price: 0,
  memo: ''
};

export default function KreamSalesPage() {
  const client = useQueryClient();
  const [ledgerTab, setLedgerTab] = useState<LedgerTab>('unlinked');
  const [candidateKind, setCandidateKind] = useState<MatchKind>('purchase');
  const [candidateKeyword, setCandidateKeyword] = useState('');
  const [ledgerDrafts, setLedgerDrafts] = useState<Record<string, LedgerLinkDraft>>({});
  const [manualOpen, setManualOpen] = useState(false);
  const [manualForm, setManualForm] = useState(emptyManualForm);
  const [ruleKeyword, setRuleKeyword] = useState('');

  const sales = useQuery({ queryKey: ['kream-sales'], queryFn: resourceApi.kreamSales });
  const ledger = useQuery({ queryKey: ['kream-ledger'], queryFn: resourceApi.kreamLedger });
  const rules = useQuery({ queryKey: ['kream-keyword-rules'], queryFn: resourceApi.kreamKeywordRules });
  const candidates = useQuery({
    queryKey: ['kream-candidates', candidateKind, candidateKeyword],
    queryFn: () => resourceApi.kreamCandidates(candidateKind, candidateKeyword)
  });

  const salesRows = sales.data?.sales ?? [];
  const ledgerRows = ledger.data ?? [];
  const candidateRows = candidates.data ?? [];
  const ruleRows = rules.data ?? [];
  const summary = sales.data?.summary;

  const ledgerById = useMemo(() => new Map(ledgerRows.map((tx) => [tx.id, tx])), [ledgerRows]);

  const groupedLedger = useMemo(() => {
    const groups: Record<LedgerTab, KreamLedgerTransaction[]> = {
      unlinked: [], purchase: [], settlement: [], side_cost: []
    };
    for (const tx of ledgerRows) {
      if (tx.link_kind === 'side_cost' && !tx.sale_id) groups.side_cost.push(tx);
      else if (tx.sale_id && tx.link_kind === 'purchase') groups.purchase.push(tx);
      else if (tx.sale_id && tx.link_kind === 'settlement') groups.settlement.push(tx);
      else groups.unlinked.push(tx);
    }
    return groups;
  }, [ledgerRows]);

  const sideCostCandidateIds = candidateRows
    .filter((tx) => tx.type === 'expense' && !(tx.scope === 'kream' && tx.link_kind === 'side_cost'))
    .map((tx) => tx.id);

  const refreshKream = async () => {
    await Promise.all([
      client.invalidateQueries({ queryKey: ['kream-sales'] }),
      client.invalidateQueries({ queryKey: ['kream-ledger'] }),
      client.invalidateQueries({ queryKey: ['kream-candidates'] }),
      client.invalidateQueries({ queryKey: ['kream-keyword-rules'] }),
      client.invalidateQueries({ queryKey: ['transactions'] }),
      client.invalidateQueries({ queryKey: ['dashboard'] })
    ]);
  };

  const createSale = useMutation({
    mutationFn: () =>
      resourceApi.createKreamSale({
        product_name: manualForm.product_name,
        purchase_date: manualForm.purchase_date,
        settlement_date: manualForm.settlement_date || null,
        purchase_price: Number(manualForm.purchase_price),
        settlement_price: Number(manualForm.settlement_price) || 0,
        memo: manualForm.memo.trim() || null
      }),
    onSuccess: async () => {
      setManualForm(emptyManualForm);
      setManualOpen(false);
      await refreshKream();
    }
  });

  const upload = useMutation({
    mutationFn: (file: File) => resourceApi.uploadKreamSales(file),
    onSuccess: async (result) => {
      await refreshKream();
      window.alert(
        `KREAM 가져오기 완료\n신규 ${result.imported_count}건 / 중복 ${result.duplicate_count}건 / 오류 ${result.error_count}건`
      );
    }
  });

  const matchTransaction = useMutation({
    mutationFn: ({
      saleId,
      transactionId,
      kind
    }: {
      saleId: string;
      transactionId: string;
      kind: SaleMatchKind;
    }) => resourceApi.matchKreamTransaction(saleId, { transaction_id: transactionId, kind }),
    onSuccess: async (_, variables) => {
      setLedgerDrafts((prev) => {
        const next = { ...prev };
        delete next[variables.transactionId];
        return next;
      });
      await refreshKream();
    }
  });

  const unmatchTransaction = useMutation({
    mutationFn: ({ saleId, kind }: { saleId: string; kind: SaleMatchKind }) =>
      resourceApi.unmatchKreamTransaction(saleId, { kind }),
    onSuccess: refreshKream
  });

  const markTransaction = useMutation({
    mutationFn: ({
      transactionId,
      scope,
      kreamKind
    }: {
      transactionId: string;
      scope: 'personal' | 'kream';
      kreamKind?: MatchKind | null;
    }) =>
      resourceApi.markKreamTransaction({
        transaction_id: transactionId,
        scope,
        kream_kind: scope === 'kream' ? kreamKind ?? null : null
      }),
    onSuccess: refreshKream
  });

  const bulkSideCost = useMutation({
    mutationFn: (transactionIds: string[]) =>
      resourceApi.bulkMarkKreamTransactions({
        transaction_ids: transactionIds,
        kream_kind: 'side_cost'
      }),
    onSuccess: async (result) => {
      await refreshKream();
      window.alert(`${result.updated_count}건을 공통 부대비용으로 분류했습니다.`);
    }
  });

  const createRule = useMutation({
    mutationFn: () => resourceApi.createKreamKeywordRule({ keyword: ruleKeyword, kream_kind: 'side_cost' }),
    onSuccess: async (result) => {
      setRuleKeyword('');
      await refreshKream();
      window.alert(`자동분류 키워드를 저장했습니다. 기존 거래 ${result.applied_count}건에 적용했습니다.`);
    }
  });

  const deleteRule = useMutation({
    mutationFn: (id: string) => resourceApi.deleteKreamKeywordRule(id),
    onSuccess: refreshKream
  });

  const applyRules = useMutation({
    mutationFn: () => resourceApi.applyKreamKeywordRules(),
    onSuccess: async (result) => {
      await refreshKream();
      window.alert(`규칙 ${result.rule_count}개를 전체 거래에 적용했습니다. ${result.applied_count}건이 분류되었습니다.`);
    }
  });

  const isWorking =
    matchTransaction.isPending ||
    unmatchTransaction.isPending ||
    markTransaction.isPending ||
    bulkSideCost.isPending;

  const activeRows = groupedLedger[ledgerTab];

  return (
    <div className="space-y-4">
      {/* 헤더 */}
      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h1 className="text-xl font-bold">KREAM 판매 장부</h1>
            <p className="mt-1 text-sm text-slate-500">
              판매건 {salesRows.length}건 · KREAM 거래 {ledgerRows.length}건
              {groupedLedger.unlinked.length > 0 && (
                <span className="ml-2 rounded-full bg-red-100 px-2 py-0.5 text-xs font-semibold text-red-700">
                  미연결 {groupedLedger.unlinked.length}건
                </span>
              )}
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            <button
              className="rounded-lg border border-slate-300 px-3 py-2 text-sm font-semibold text-slate-700"
              onClick={() => setManualOpen((v) => !v)}
            >
              수동 등록
            </button>
            <label className="cursor-pointer rounded-lg bg-teal-700 px-3 py-2 text-sm font-semibold text-white">
              파일 업로드
              <input
                type="file"
                className="hidden"
                accept=".csv,.xls,.xlsx"
                onChange={(e) => {
                  const file = e.target.files?.[0];
                  e.target.value = '';
                  if (file) void upload.mutateAsync(file).catch((err) => window.alert(err.message));
                }}
              />
            </label>
          </div>
        </div>
      </section>

      {/* 수동 등록 폼 */}
      {manualOpen && (
        <section className="rounded-2xl bg-white p-4 shadow-soft">
          <h2 className="text-lg font-bold">판매 물품 수동 등록</h2>
          <div className="mt-3 grid gap-3 md:grid-cols-2">
            <label className="text-sm md:col-span-2">
              <span className="mb-1 block text-slate-600">상품명</span>
              <input
                className="w-full rounded-lg border border-slate-300 px-3 py-2"
                value={manualForm.product_name}
                onChange={(e) => setManualForm((p) => ({ ...p, product_name: e.target.value }))}
              />
            </label>
            <label className="text-sm">
              <span className="mb-1 block text-slate-600">구매일</span>
              <input type="date" className="w-full rounded-lg border border-slate-300 px-3 py-2"
                value={manualForm.purchase_date}
                onChange={(e) => setManualForm((p) => ({ ...p, purchase_date: e.target.value }))}
              />
            </label>
            <label className="text-sm">
              <span className="mb-1 block text-slate-600">정산일</span>
              <input type="date" className="w-full rounded-lg border border-slate-300 px-3 py-2"
                value={manualForm.settlement_date}
                onChange={(e) => setManualForm((p) => ({ ...p, settlement_date: e.target.value }))}
              />
            </label>
            <label className="text-sm">
              <span className="mb-1 block text-slate-600">구매가</span>
              <input type="number" min={0} className="w-full rounded-lg border border-slate-300 px-3 py-2"
                value={manualForm.purchase_price}
                onChange={(e) => setManualForm((p) => ({ ...p, purchase_price: Number(e.target.value) }))}
              />
            </label>
            <label className="text-sm">
              <span className="mb-1 block text-slate-600">정산가</span>
              <input type="number" min={0} className="w-full rounded-lg border border-slate-300 px-3 py-2"
                value={manualForm.settlement_price}
                onChange={(e) => setManualForm((p) => ({ ...p, settlement_price: Number(e.target.value) }))}
              />
            </label>
            <label className="text-sm md:col-span-2">
              <span className="mb-1 block text-slate-600">메모</span>
              <input className="w-full rounded-lg border border-slate-300 px-3 py-2"
                value={manualForm.memo}
                onChange={(e) => setManualForm((p) => ({ ...p, memo: e.target.value }))}
              />
            </label>
          </div>
          <div className="mt-4 flex gap-2">
            <button
              className="rounded-lg bg-teal-700 px-4 py-2 text-sm font-semibold text-white disabled:opacity-40"
              disabled={createSale.isPending}
              onClick={() => {
                if (!manualForm.product_name.trim()) { window.alert('상품명을 입력해주세요.'); return; }
                void createSale.mutateAsync().catch((err) => window.alert(err.message));
              }}
            >등록</button>
            <button
              className="rounded-lg border border-slate-300 px-4 py-2 text-sm"
              onClick={() => { setManualForm(emptyManualForm); setManualOpen(false); }}
            >취소</button>
          </div>
        </section>
      )}

      {/* 요약 카드 */}
      <section className="grid gap-3 sm:grid-cols-4">
        <SummaryCard label="구매 합계"   value={summary?.total_purchase_price ?? 0}   tone="rose" />
        <SummaryCard label="정산 합계"   value={summary?.total_settlement_price ?? 0} tone="emerald" />
        <SummaryCard label="공통 부대비용" value={summary?.total_side_cost ?? 0}       tone="amber" />
        <SummaryCard label="실현 손익"   value={summary?.total_profit ?? 0}            tone="slate" />
      </section>

      {/* KREAM 거래 — 탭 분류 */}
      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h2 className="text-lg font-bold">KREAM 거래</h2>
            <p className="mt-0.5 text-xs text-slate-400">
              구매·정산은 판매 상품에 연결하고, 부대비용은 공통 비용으로 집계됩니다.
            </p>
          </div>
        </div>

        {/* 탭바 */}
        <div className="mt-3 flex flex-wrap gap-2">
          {(Object.entries(TAB_META) as [LedgerTab, typeof TAB_META[LedgerTab]][]).map(([tab, meta]) => {
            const count = groupedLedger[tab].length;
            const isActive = ledgerTab === tab;
            const isUnlinkedAlert = tab === 'unlinked' && count > 0;

            const activeClass = isUnlinkedAlert
              ? 'bg-red-600 text-white'
              : { rose: 'bg-rose-600 text-white', emerald: 'bg-emerald-600 text-white', amber: 'bg-amber-600 text-white', slate: 'bg-slate-700 text-white' }[meta.tone];
            const idleClass = isUnlinkedAlert
              ? 'bg-red-50 text-red-700 hover:bg-red-100 border border-red-200'
              : { rose: 'bg-rose-50 text-rose-700 hover:bg-rose-100', emerald: 'bg-emerald-50 text-emerald-700 hover:bg-emerald-100', amber: 'bg-amber-50 text-amber-700 hover:bg-amber-100', slate: 'bg-slate-100 text-slate-600 hover:bg-slate-200' }[meta.tone];

            return (
              <button
                key={tab}
                type="button"
                aria-pressed={isActive}
                onClick={() => setLedgerTab(tab)}
                className={`flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-sm font-semibold transition-colors ${isActive ? activeClass : idleClass}`}
              >
                {meta.label}
                <span className={`rounded-full px-1.5 py-0.5 text-xs font-bold ${isActive ? 'bg-white/25 text-inherit' : 'bg-white/60'}`}>
                  {count}
                </span>
              </button>
            );
          })}
        </div>

        {/* 현재 탭 콘텐츠 */}
        <div className="mt-3 space-y-2">
          {activeRows.length === 0 ? (
            <div className={`rounded-lg border border-dashed p-5 text-center text-sm ${ledgerTab === 'unlinked' ? 'border-emerald-300 bg-emerald-50 text-emerald-700' : 'border-slate-300 text-slate-500'}`}>
              {EMPTY_MSG[ledgerTab]}
            </div>
          ) : (
            activeRows.map((tx) => (
              <LedgerCard
                key={tx.id}
                tx={tx}
                tab={ledgerTab}
                salesRows={salesRows}
                draft={ledgerDrafts[tx.id] ?? { saleId: '', kind: defaultKindForTransaction(tx) }}
                onDraftChange={(txId, patch) =>
                  setLedgerDrafts((prev) => ({ ...prev, [txId]: { ...prev[txId] ?? { saleId: '', kind: defaultKindForTransaction(tx) }, ...patch } }))
                }
                onMatch={(saleId, transactionId, kind) =>
                  void matchTransaction.mutateAsync({ saleId, transactionId, kind }).catch((err) => window.alert(err.message))
                }
                onMark={(transactionId, scope, kreamKind) =>
                  void markTransaction.mutateAsync({ transactionId, scope, kreamKind }).catch((err) => window.alert(err.message))
                }
                onUnmatch={(saleId, kind) =>
                  void unmatchTransaction.mutateAsync({ saleId, kind }).catch((err) => window.alert(err.message))
                }
                isWorking={isWorking}
              />
            ))
          )}
        </div>
      </section>

      {/* 판매건 테이블 */}
      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <h2 className="text-lg font-bold">판매건</h2>
        <div className="mt-3 overflow-x-auto">
          <table className="min-w-full text-sm">
            <thead>
              <tr className="border-b border-slate-200 text-left text-xs text-slate-500">
                <th className="whitespace-nowrap px-2 py-2">판매코드</th>
                <th className="whitespace-nowrap px-2 py-2">상품명</th>
                <th className="whitespace-nowrap px-2 py-2">구매일</th>
                <th className="whitespace-nowrap px-2 py-2">정산일</th>
                <th className="whitespace-nowrap px-2 py-2 text-right">구매가</th>
                <th className="whitespace-nowrap px-2 py-2 text-right">정산가</th>
                <th className="whitespace-nowrap px-2 py-2 text-right">상품 손익</th>
                <th className="whitespace-nowrap px-2 py-2">거래 연결</th>
              </tr>
            </thead>
            <tbody>
              {salesRows.length ? (
                salesRows.map((sale) => {
                  const settled = Boolean(sale.settlement_date);
                  const profit = sale.settlement_price - sale.purchase_price;
                  return (
                    <tr key={sale.id} className="border-b border-slate-100 align-top">
                      <td className="whitespace-nowrap px-2 py-3 font-mono text-xs text-slate-500">{sale.sale_code}</td>
                      <td className="min-w-56 px-2 py-3 font-medium text-slate-900">{sale.product_name}</td>
                      <td className="whitespace-nowrap px-2 py-3">{sale.purchase_date}</td>
                      <td className="whitespace-nowrap px-2 py-3">{sale.settlement_date ?? '미정'}</td>
                      <td className="whitespace-nowrap px-2 py-3 text-right text-rose-700">{money(sale.purchase_price)}</td>
                      <td className="whitespace-nowrap px-2 py-3 text-right text-emerald-700">
                        {settled ? money(sale.settlement_price) : '미정'}
                      </td>
                      <td className={`whitespace-nowrap px-2 py-3 text-right font-semibold ${!settled ? 'text-slate-400' : profit >= 0 ? 'text-teal-700' : 'text-rose-700'}`}>
                        {settled ? money(profit) : '정산 대기'}
                      </td>
                      <td className="min-w-80 px-2 py-3">
                        <div className="space-y-1.5">
                          <SaleLinkRow saleId={sale.id} kind="purchase"
                            transactionId={sale.purchase_transaction_id ?? null}
                            transaction={sale.purchase_transaction_id ? ledgerById.get(sale.purchase_transaction_id) : undefined}
                            onUnmatch={unmatchTransaction.mutateAsync} disabled={isWorking}
                          />
                          <SaleLinkRow saleId={sale.id} kind="settlement"
                            transactionId={sale.settlement_transaction_id ?? null}
                            transaction={sale.settlement_transaction_id ? ledgerById.get(sale.settlement_transaction_id) : undefined}
                            onUnmatch={unmatchTransaction.mutateAsync} disabled={isWorking}
                          />
                        </div>
                      </td>
                    </tr>
                  );
                })
              ) : (
                <tr>
                  <td colSpan={8} className="px-2 py-8 text-center text-sm text-slate-500">
                    등록된 KREAM 판매건이 없습니다.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </section>

      {/* 거래 후보 */}
      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <h2 className="text-lg font-bold">거래 후보</h2>
          <div className="flex flex-wrap items-center gap-2">
            <select
              className="rounded-lg border border-slate-300 px-3 py-2 text-sm"
              value={candidateKind}
              onChange={(e) => setCandidateKind(e.target.value as MatchKind)}
            >
              <option value="purchase">구매</option>
              <option value="settlement">정산</option>
              <option value="side_cost">부대비용</option>
            </select>
            <input
              className="rounded-lg border border-slate-300 px-3 py-2 text-sm"
              placeholder="가맹점, 내용, 메모"
              value={candidateKeyword}
              onChange={(e) => setCandidateKeyword(e.target.value)}
            />
            {candidateKind === 'side_cost' && (
              <button
                className="rounded-lg bg-amber-600 px-3 py-2 text-sm font-semibold text-white disabled:opacity-40"
                disabled={!sideCostCandidateIds.length || bulkSideCost.isPending}
                onClick={() => {
                  if (window.confirm(`현재 후보 ${sideCostCandidateIds.length}건을 공통 부대비용으로 분류할까요?`)) {
                    void bulkSideCost.mutateAsync(sideCostCandidateIds).catch((err) => window.alert(err.message));
                  }
                }}
              >
                현재 후보 전체 비용 처리
              </button>
            )}
          </div>
        </div>

        {candidateKind === 'side_cost' && (
          <div className="mt-3 rounded-lg border border-amber-100 bg-amber-50 p-3">
            <div className="flex flex-wrap items-center gap-2">
              <input
                className="min-w-64 flex-1 rounded-lg border border-amber-200 px-3 py-2 text-sm"
                placeholder="예: 배송비, 택배, 포장"
                value={ruleKeyword}
                onChange={(e) => setRuleKeyword(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && ruleKeyword.trim())
                    void createRule.mutateAsync().catch((err) => window.alert(err.message));
                }}
              />
              <button
                className="rounded-lg bg-amber-700 px-3 py-2 text-sm font-semibold text-white disabled:opacity-40"
                disabled={!ruleKeyword.trim() || createRule.isPending}
                onClick={() => void createRule.mutateAsync().catch((err) => window.alert(err.message))}
              >
                키워드 추가
              </button>
              {ruleRows.length > 0 && (
                <button
                  className="rounded-lg border border-amber-400 bg-white px-3 py-2 text-sm font-semibold text-amber-700 hover:bg-amber-50 disabled:opacity-40"
                  disabled={applyRules.isPending}
                  onClick={() => {
                    if (window.confirm(`등록된 키워드 ${ruleRows.length}개를 전체 거래에 일괄 적용할까요?\n이미 KREAM으로 분류된 거래는 건너뜁니다.`))
                      void applyRules.mutateAsync().catch((err) => window.alert(err.message));
                  }}
                >
                  {applyRules.isPending ? '적용 중...' : '전체 재적용'}
                </button>
              )}
            </div>
            <div className="mt-2 flex flex-wrap gap-2">
              {ruleRows.length ? (
                ruleRows.map((rule) => (
                  <span key={rule.id} className="inline-flex items-center gap-2 rounded-full bg-white px-3 py-1 text-xs font-semibold text-amber-800">
                    {rule.keyword}
                    <button
                      className="text-amber-500 hover:text-red-600"
                      disabled={deleteRule.isPending}
                      onClick={() => void deleteRule.mutateAsync(rule.id).catch((err) => window.alert(err.message))}
                    >✕</button>
                  </span>
                ))
              ) : (
                <span className="text-xs text-amber-700">등록된 자동분류 키워드가 없습니다.</span>
              )}
            </div>
          </div>
        )}

        <div className="mt-3 space-y-2">
          {candidateRows.length ? (
            candidateRows.map((tx) => (
              <div key={tx.id} className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-slate-100 p-3">
                <div>
                  <div className="flex flex-wrap items-center gap-2">
                    <p className="font-medium text-slate-900">{transactionTitle(tx)}</p>
                    <span className={tx.scope === 'kream'
                      ? 'rounded-full bg-teal-50 px-2 py-0.5 text-xs font-semibold text-teal-700'
                      : 'rounded-full bg-slate-100 px-2 py-0.5 text-xs font-semibold text-slate-500'}>
                      {tx.scope === 'kream' ? 'KREAM' : '개인'}
                    </span>
                  </div>
                  <p className="mt-1 text-xs text-slate-500">
                    {dateTime(tx.transaction_at)} · {tx.description || tx.memo || ''}
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  <p className={amountClass(tx.type)}>
                    {tx.type === 'income' ? '+' : '-'} {money(tx.amount)}
                  </p>
                  <button
                    className="rounded border border-slate-300 px-2 py-1 text-xs font-semibold text-slate-600 disabled:opacity-40"
                    disabled={markTransaction.isPending}
                    onClick={() => {
                      const nextScope = tx.scope === 'kream' ? 'personal' : 'kream';
                      void markTransaction.mutateAsync({
                        transactionId: tx.id,
                        scope: nextScope,
                        kreamKind: nextScope === 'kream' && candidateKind === 'side_cost' ? 'side_cost' : null
                      }).catch((err) => window.alert(err.message));
                    }}
                  >
                    {tx.scope === 'kream' ? '개인거래로' : candidateKind === 'side_cost' ? '공통 비용으로' : 'KREAM 장부로'}
                  </button>
                </div>
              </div>
            ))
          ) : (
            <p className="rounded-lg border border-dashed border-slate-300 p-4 text-sm text-slate-500">
              조건에 맞는 거래 후보가 없습니다.
            </p>
          )}
        </div>
      </section>
    </div>
  );
}

// ─── 서브 컴포넌트 ────────────────────────────────────────────────────────────

const TAB_CARD_STYLE: Record<LedgerTab, { border: string; bg: string; accent: string }> = {
  purchase:   { border: 'border-rose-200',    bg: 'bg-rose-50/40',    accent: 'border-l-4 border-l-rose-400' },
  settlement: { border: 'border-emerald-200', bg: 'bg-emerald-50/40', accent: 'border-l-4 border-l-emerald-400' },
  side_cost:  { border: 'border-amber-200',   bg: 'bg-amber-50/40',   accent: 'border-l-4 border-l-amber-400' },
  unlinked:   { border: 'border-slate-200',   bg: 'bg-white',         accent: 'border-l-4 border-l-red-400' },
};

function LedgerCard({
  tx, tab, salesRows, draft, onDraftChange, onMatch, onMark, onUnmatch, isWorking
}: {
  tx: KreamLedgerTransaction;
  tab: LedgerTab;
  salesRows: KreamSale[];
  draft: LedgerLinkDraft;
  onDraftChange: (txId: string, patch: Partial<LedgerLinkDraft>) => void;
  onMatch: (saleId: string, transactionId: string, kind: SaleMatchKind) => void;
  onMark: (transactionId: string, scope: 'personal' | 'kream', kreamKind?: MatchKind | null) => void;
  onUnmatch: (saleId: string, kind: SaleMatchKind) => void;
  isWorking: boolean;
}) {
  const style = TAB_CARD_STYLE[tab];
  const linkedToSale = Boolean(tx.sale_id && tx.link_kind !== 'side_cost');
  const commonSideCost = tx.link_kind === 'side_cost' && !tx.sale_id;
  const canApply = draft.kind === 'side_cost' || Boolean(draft.saleId);
  const options = kindOptionsForTransaction(tx);

  return (
    <div className={`rounded-lg border ${style.border} ${style.bg} ${style.accent} p-3`}>
      <div className="grid gap-3 lg:grid-cols-[1fr_2fr] lg:items-center">
        {/* 왼쪽: 거래 정보 */}
        <div>
          <div className="flex flex-wrap items-center gap-2">
            <p className="font-semibold text-slate-900">{transactionTitle(tx)}</p>
            <StatusBadge tab={tab} linkedToSale={linkedToSale} commonSideCost={commonSideCost} linkKind={tx.link_kind} />
          </div>
          <p className="mt-1 text-xs text-slate-500">
            {dateTime(tx.transaction_at)}
            {(tx.description || tx.memo) && <> · {tx.description || tx.memo}</>}
          </p>
          <p className={amountClass(tx.type)}>
            {tx.type === 'income' ? '+' : '-'} {money(tx.amount)}
          </p>
        </div>

        {/* 오른쪽: 연결 상태 또는 연결 폼 */}
        {linkedToSale ? (
          <div className="flex flex-wrap items-center justify-between gap-2 rounded-lg bg-white/70 px-3 py-2 ring-1 ring-slate-200">
            <div>
              <p className="text-xs font-semibold text-slate-500">연결 상품</p>
              <p className="font-medium text-slate-900">{tx.product_name}</p>
              <p className="text-xs text-slate-400">{tx.sale_code}</p>
            </div>
            <button
              className="rounded border border-slate-300 px-2 py-1 text-xs font-semibold text-slate-600 hover:bg-slate-100 disabled:opacity-40"
              disabled={isWorking}
              onClick={() => {
                if (tx.sale_id && tx.link_kind && tx.link_kind !== 'side_cost')
                  onUnmatch(tx.sale_id, tx.link_kind as SaleMatchKind);
              }}
            >
              연결 해제
            </button>
          </div>
        ) : commonSideCost ? (
          <div className="flex flex-wrap items-center justify-between gap-2 rounded-lg bg-amber-50 px-3 py-2 ring-1 ring-amber-200">
            <div>
              <p className="text-xs font-semibold text-amber-700">공통 비용</p>
              <p className="text-sm text-amber-800">전체 손익에서 차감됩니다.</p>
            </div>
            <button
              className="rounded border border-amber-200 px-2 py-1 text-xs font-semibold text-amber-800 hover:bg-amber-100 disabled:opacity-40"
              disabled={isWorking}
              onClick={() => onMark(tx.id, 'personal')}
            >
              개인거래로
            </button>
          </div>
        ) : (
          <div className="grid gap-2 sm:grid-cols-[120px_1fr_auto]">
            <select
              className="rounded-lg border border-slate-300 px-2 py-2 text-sm"
              value={draft.kind}
              disabled={!options.length}
              onChange={(e) => onDraftChange(tx.id, { kind: e.target.value as MatchKind })}
            >
              {options.map((k) => (
                <option key={k} value={k}>{KIND_LABEL[k]}</option>
              ))}
            </select>
            {draft.kind === 'side_cost' ? (
              <div className="rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-900">
                공통 부대비용으로 등록
              </div>
            ) : (
              <select
                className="rounded-lg border border-slate-300 px-2 py-2 text-sm"
                value={draft.saleId}
                onChange={(e) => onDraftChange(tx.id, { saleId: e.target.value })}
              >
                <option value="">판매 상품 선택</option>
                {salesRows.map((sale) => (
                  <option key={sale.id} value={sale.id}>
                    {sale.product_name} · {sale.purchase_date} · {money(sale.purchase_price)}
                  </option>
                ))}
              </select>
            )}
            <button
              className="rounded-lg bg-teal-700 px-3 py-2 text-sm font-semibold text-white disabled:opacity-40"
              disabled={!canApply || isWorking}
              onClick={() => {
                if (draft.kind === 'side_cost') {
                  onMark(tx.id, 'kream', 'side_cost');
                } else {
                  onMatch(draft.saleId, tx.id, draft.kind as SaleMatchKind);
                }
              }}
            >
              적용
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

function StatusBadge({
  tab, linkedToSale, commonSideCost, linkKind
}: {
  tab: LedgerTab;
  linkedToSale: boolean;
  commonSideCost: boolean;
  linkKind?: string | null;
}) {
  if (commonSideCost) return (
    <span className="rounded-full bg-amber-100 px-2 py-0.5 text-xs font-semibold text-amber-800">
      공통 부대비용
    </span>
  );
  if (linkedToSale) return (
    <span className={`rounded-full px-2 py-0.5 text-xs font-semibold ${
      tab === 'purchase' ? 'bg-rose-100 text-rose-700' : 'bg-emerald-100 text-emerald-700'
    }`}>
      {KIND_LABEL[(linkKind as MatchKind) ?? 'purchase']} 연결
    </span>
  );
  return (
    <span className="rounded-full bg-red-100 px-2 py-0.5 text-xs font-semibold text-red-700">
      미연결
    </span>
  );
}

function SummaryCard({ label, value, tone }: {
  label: string; value: number; tone: 'rose' | 'emerald' | 'amber' | 'slate';
}) {
  const cls = { rose: 'bg-rose-50 text-rose-700', emerald: 'bg-emerald-50 text-emerald-700', amber: 'bg-amber-50 text-amber-700', slate: 'bg-slate-100 text-slate-900' }[tone];
  return (
    <div className={`rounded-xl p-4 ${cls}`}>
      <p className="text-sm text-slate-600">{label}</p>
      <p className="mt-1 text-lg font-semibold">{money(value)}</p>
    </div>
  );
}

function SaleLinkRow({ saleId, kind, transactionId, transaction, disabled, onUnmatch }: {
  saleId: string;
  kind: SaleMatchKind;
  transactionId: string | null;
  transaction?: KreamLedgerTransaction;
  disabled: boolean;
  onUnmatch: (payload: { saleId: string; kind: SaleMatchKind }) => Promise<KreamSale>;
}) {
  if (!transactionId) {
    return (
      <div className="flex items-center justify-between gap-2 rounded-lg bg-slate-50 px-2 py-1.5 text-xs text-slate-500">
        <span className="font-semibold">{KIND_LABEL[kind]}</span>
        <span>미연결</span>
      </div>
    );
  }
  return (
    <div className="flex items-center justify-between gap-2 rounded-lg bg-teal-50 px-2 py-1.5 text-xs text-teal-800">
      <div className="min-w-0">
        <span className="font-semibold">{KIND_LABEL[kind]}</span>
        <span className="ml-2 text-teal-700">
          {transaction ? `${dateTime(transaction.transaction_at)} · ${money(transaction.amount)}` : '연결됨'}
        </span>
      </div>
      <button
        className="shrink-0 rounded border border-teal-200 px-2 py-0.5 font-semibold disabled:opacity-40"
        disabled={disabled}
        onClick={() => void onUnmatch({ saleId, kind }).catch((err) => window.alert(err.message))}
      >
        해제
      </button>
    </div>
  );
}

function transactionTitle(tx: Pick<KreamTransactionCandidate, 'merchant_name' | 'description'>) {
  return tx.merchant_name || tx.description || '거래';
}

function amountClass(type: string) {
  return type === 'income'
    ? 'whitespace-nowrap font-semibold text-emerald-700'
    : 'whitespace-nowrap font-semibold text-rose-700';
}

function defaultKindForTransaction(tx: KreamLedgerTransaction): MatchKind {
  return tx.type === 'income' ? 'settlement' : 'purchase';
}

function kindOptionsForTransaction(tx: KreamLedgerTransaction): MatchKind[] {
  if (tx.type === 'income') return ['settlement'];
  if (tx.type === 'expense') return ['purchase', 'side_cost'];
  return [];
}
