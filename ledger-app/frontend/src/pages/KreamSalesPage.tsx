import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { resourceApi } from '@/api/resources';
import { dateTime, money, today } from '@/utils/format';
import type { KreamLedgerTransaction, KreamSale, KreamTransactionCandidate } from '@/types';

type MatchKind = 'purchase' | 'settlement' | 'side_cost';
type SaleMatchKind = 'purchase' | 'settlement';
type LedgerLinkDraft = { saleId: string; kind: MatchKind };

const KIND_LABEL: Record<MatchKind, string> = {
  purchase: '구매',
  settlement: '정산',
  side_cost: '부대비용'
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
  const unlinkedLedgerCount = ledgerRows.filter((tx) => !tx.link_kind).length;
  const commonSideCostCount = ledgerRows.filter((tx) => tx.link_kind === 'side_cost' && !tx.sale_id).length;
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

  const isWorking =
    matchTransaction.isPending ||
    unmatchTransaction.isPending ||
    markTransaction.isPending ||
    bulkSideCost.isPending;

  return (
    <div className="space-y-4">
      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h1 className="text-xl font-bold">KREAM 판매 장부</h1>
            <p className="mt-1 text-sm text-slate-500">
              판매건 {salesRows.length}건 · KREAM 거래 {ledgerRows.length}건 · 미연결 {unlinkedLedgerCount}건 · 공통 비용 {commonSideCostCount}건
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            <button
              className="rounded-lg border border-slate-300 px-3 py-2 text-sm font-semibold text-slate-700"
              onClick={() => setManualOpen((value) => !value)}
            >
              수동 등록
            </button>
            <label className="cursor-pointer rounded-lg bg-teal-700 px-3 py-2 text-sm font-semibold text-white">
              파일 업로드
              <input
                type="file"
                className="hidden"
                accept=".csv,.xls,.xlsx"
                onChange={(event) => {
                  const file = event.target.files?.[0];
                  event.target.value = '';
                  if (file) {
                    void upload.mutateAsync(file).catch((err) => window.alert(err.message));
                  }
                }}
              />
            </label>
          </div>
        </div>
      </section>

      {manualOpen ? (
        <section className="rounded-2xl bg-white p-4 shadow-soft">
          <h2 className="text-lg font-bold">판매 물품 수동 등록</h2>
          <div className="mt-3 grid gap-3 md:grid-cols-2">
            <label className="text-sm md:col-span-2">
              <span className="mb-1 block text-slate-600">상품명</span>
              <input
                className="w-full rounded-lg border border-slate-300 px-3 py-2"
                value={manualForm.product_name}
                onChange={(event) => setManualForm((prev) => ({ ...prev, product_name: event.target.value }))}
              />
            </label>
            <label className="text-sm">
              <span className="mb-1 block text-slate-600">구매일</span>
              <input
                type="date"
                className="w-full rounded-lg border border-slate-300 px-3 py-2"
                value={manualForm.purchase_date}
                onChange={(event) => setManualForm((prev) => ({ ...prev, purchase_date: event.target.value }))}
              />
            </label>
            <label className="text-sm">
              <span className="mb-1 block text-slate-600">정산일</span>
              <input
                type="date"
                className="w-full rounded-lg border border-slate-300 px-3 py-2"
                value={manualForm.settlement_date}
                onChange={(event) => setManualForm((prev) => ({ ...prev, settlement_date: event.target.value }))}
              />
            </label>
            <label className="text-sm">
              <span className="mb-1 block text-slate-600">구매가</span>
              <input
                type="number"
                min={0}
                className="w-full rounded-lg border border-slate-300 px-3 py-2"
                value={manualForm.purchase_price}
                onChange={(event) => setManualForm((prev) => ({ ...prev, purchase_price: Number(event.target.value) }))}
              />
            </label>
            <label className="text-sm">
              <span className="mb-1 block text-slate-600">정산가</span>
              <input
                type="number"
                min={0}
                className="w-full rounded-lg border border-slate-300 px-3 py-2"
                value={manualForm.settlement_price}
                onChange={(event) => setManualForm((prev) => ({ ...prev, settlement_price: Number(event.target.value) }))}
              />
            </label>
            <label className="text-sm md:col-span-2">
              <span className="mb-1 block text-slate-600">메모</span>
              <input
                className="w-full rounded-lg border border-slate-300 px-3 py-2"
                value={manualForm.memo}
                onChange={(event) => setManualForm((prev) => ({ ...prev, memo: event.target.value }))}
              />
            </label>
          </div>
          <div className="mt-4 flex gap-2">
            <button
              className="rounded-lg bg-teal-700 px-4 py-2 text-sm font-semibold text-white disabled:opacity-40"
              disabled={createSale.isPending}
              onClick={() => {
                if (!manualForm.product_name.trim()) {
                  window.alert('상품명을 입력해주세요.');
                  return;
                }
                void createSale.mutateAsync().catch((err) => window.alert(err.message));
              }}
            >
              등록
            </button>
            <button
              className="rounded-lg border border-slate-300 px-4 py-2 text-sm"
              onClick={() => {
                setManualForm(emptyManualForm);
                setManualOpen(false);
              }}
            >
              취소
            </button>
          </div>
        </section>
      ) : null}

      <section className="grid gap-3 sm:grid-cols-4">
        <SummaryCard label="구매 합계" value={summary?.total_purchase_price ?? 0} tone="rose" />
        <SummaryCard label="정산 합계" value={summary?.total_settlement_price ?? 0} tone="emerald" />
        <SummaryCard label="공통 부대비용" value={summary?.total_side_cost ?? 0} tone="amber" />
        <SummaryCard label="실현 손익" value={summary?.total_profit ?? 0} tone="slate" />
      </section>

      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h2 className="text-lg font-bold">KREAM 거래</h2>
            <p className="mt-1 text-sm text-slate-500">
              구매와 정산은 판매 상품에 연결하고, 부대비용은 상품 선택 없이 공통 비용으로 집계됩니다.
            </p>
          </div>
          <span className="rounded-full bg-slate-100 px-3 py-1 text-xs font-semibold text-slate-600">
            미연결 {unlinkedLedgerCount}건
          </span>
        </div>

        <div className="mt-3 space-y-2">
          {ledgerRows.length ? (
            ledgerRows.map((tx) => {
              const options = kindOptionsForTransaction(tx);
              const draft = ledgerDrafts[tx.id] ?? {
                saleId: '',
                kind: defaultKindForTransaction(tx)
              };
              const linkedToSale = Boolean(tx.sale_id && tx.link_kind !== 'side_cost');
              const commonSideCost = tx.link_kind === 'side_cost' && !tx.sale_id;
              const linked = linkedToSale || commonSideCost;
              const canApply = draft.kind === 'side_cost' || Boolean(draft.saleId);

              return (
                <div key={tx.id} className="rounded-lg border border-slate-100 p-3">
                  <div className="grid gap-3 lg:grid-cols-[1fr_2fr] lg:items-center">
                    <div>
                      <div className="flex flex-wrap items-center gap-2">
                        <p className="font-semibold text-slate-900">{transactionTitle(tx)}</p>
                        <span className={scopeBadgeClass(linked)}>
                          {commonSideCost ? '공통 부대비용' : linked ? `${KIND_LABEL[tx.link_kind as MatchKind]} 연결` : '미연결'}
                        </span>
                      </div>
                      <p className="mt-1 text-xs text-slate-500">
                        {dateTime(tx.transaction_at)} · {tx.description || tx.memo || tx.merchant_name || ''}
                      </p>
                      <p className={amountClass(tx.type)}>
                        {tx.type === 'income' ? '+' : '-'} {money(tx.amount)}
                      </p>
                    </div>

                    {linkedToSale ? (
                      <div className="flex flex-wrap items-center justify-between gap-2 rounded-lg bg-slate-50 px-3 py-2">
                        <div>
                          <p className="text-xs font-semibold text-slate-500">연결 상품</p>
                          <p className="font-medium text-slate-900">{tx.product_name}</p>
                          <p className="text-xs text-slate-500">{tx.sale_code}</p>
                        </div>
                        <button
                          className="rounded border border-slate-300 px-2 py-1 text-xs font-semibold text-slate-600 disabled:opacity-40"
                          disabled={isWorking}
                          onClick={() => {
                            if (tx.sale_id && tx.link_kind && tx.link_kind !== 'side_cost') {
                              void unmatchTransaction
                                .mutateAsync({ saleId: tx.sale_id, kind: tx.link_kind })
                                .catch((err) => window.alert(err.message));
                            }
                          }}
                        >
                          연결 해제
                        </button>
                      </div>
                    ) : commonSideCost ? (
                      <div className="flex flex-wrap items-center justify-between gap-2 rounded-lg bg-amber-50 px-3 py-2">
                        <div>
                          <p className="text-xs font-semibold text-amber-700">공통 비용</p>
                          <p className="font-medium text-amber-900">판매 상품과 연결하지 않고 전체 손익에서 차감됩니다.</p>
                        </div>
                        <button
                          className="rounded border border-amber-200 px-2 py-1 text-xs font-semibold text-amber-800 disabled:opacity-40"
                          disabled={isWorking}
                          onClick={() =>
                            void markTransaction
                              .mutateAsync({ transactionId: tx.id, scope: 'personal' })
                              .catch((err) => window.alert(err.message))
                          }
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
                          onChange={(event) =>
                            setLedgerDrafts((prev) => ({
                              ...prev,
                              [tx.id]: { ...draft, kind: event.target.value as MatchKind }
                            }))
                          }
                        >
                          {options.map((kind) => (
                            <option key={kind} value={kind}>
                              {KIND_LABEL[kind]}
                            </option>
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
                            onChange={(event) =>
                              setLedgerDrafts((prev) => ({
                                ...prev,
                                [tx.id]: { ...draft, saleId: event.target.value }
                              }))
                            }
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
                              void markTransaction
                                .mutateAsync({ transactionId: tx.id, scope: 'kream', kreamKind: 'side_cost' })
                                .catch((err) => window.alert(err.message));
                              return;
                            }
                            void matchTransaction
                              .mutateAsync({
                                saleId: draft.saleId,
                                transactionId: tx.id,
                                kind: draft.kind as SaleMatchKind
                              })
                              .catch((err) => window.alert(err.message));
                          }}
                        >
                          적용
                        </button>
                      </div>
                    )}
                  </div>
                </div>
              );
            })
          ) : (
            <p className="rounded-lg border border-dashed border-slate-300 p-4 text-sm text-slate-500">
              KREAM으로 분류된 거래가 없습니다.
            </p>
          )}
        </div>
      </section>

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
                      <td className={settled && profit >= 0 ? 'whitespace-nowrap px-2 py-3 text-right font-semibold text-teal-700' : settled ? 'whitespace-nowrap px-2 py-3 text-right font-semibold text-rose-700' : 'whitespace-nowrap px-2 py-3 text-right font-semibold text-slate-400'}>
                        {settled ? money(profit) : '정산 대기'}
                      </td>
                      <td className="min-w-80 px-2 py-3">
                        <div className="space-y-1.5">
                          <SaleLinkRow
                            saleId={sale.id}
                            kind="purchase"
                            transactionId={sale.purchase_transaction_id ?? null}
                            transaction={sale.purchase_transaction_id ? ledgerById.get(sale.purchase_transaction_id) : undefined}
                            onUnmatch={unmatchTransaction.mutateAsync}
                            disabled={isWorking}
                          />
                          <SaleLinkRow
                            saleId={sale.id}
                            kind="settlement"
                            transactionId={sale.settlement_transaction_id ?? null}
                            transaction={sale.settlement_transaction_id ? ledgerById.get(sale.settlement_transaction_id) : undefined}
                            onUnmatch={unmatchTransaction.mutateAsync}
                            disabled={isWorking}
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

      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <h2 className="text-lg font-bold">거래 후보</h2>
          <div className="flex flex-wrap items-center gap-2">
            <select
              className="rounded-lg border border-slate-300 px-3 py-2 text-sm"
              value={candidateKind}
              onChange={(event) => setCandidateKind(event.target.value as MatchKind)}
            >
              <option value="purchase">구매</option>
              <option value="settlement">정산</option>
              <option value="side_cost">부대비용</option>
            </select>
            <input
              className="rounded-lg border border-slate-300 px-3 py-2 text-sm"
              placeholder="가맹점, 내용, 메모"
              value={candidateKeyword}
              onChange={(event) => setCandidateKeyword(event.target.value)}
            />
            {candidateKind === 'side_cost' ? (
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
            ) : null}
          </div>
        </div>

        {candidateKind === 'side_cost' ? (
          <div className="mt-3 rounded-lg border border-amber-100 bg-amber-50 p-3">
            <div className="flex flex-wrap items-center gap-2">
              <input
                className="min-w-64 flex-1 rounded-lg border border-amber-200 px-3 py-2 text-sm"
                placeholder="예: 배송비, 택배, 포장"
                value={ruleKeyword}
                onChange={(event) => setRuleKeyword(event.target.value)}
              />
              <button
                className="rounded-lg bg-amber-700 px-3 py-2 text-sm font-semibold text-white disabled:opacity-40"
                disabled={!ruleKeyword.trim() || createRule.isPending}
                onClick={() => void createRule.mutateAsync().catch((err) => window.alert(err.message))}
              >
                자동분류 키워드 추가
              </button>
            </div>
            <div className="mt-2 flex flex-wrap gap-2">
              {ruleRows.length ? (
                ruleRows.map((rule) => (
                  <span key={rule.id} className="inline-flex items-center gap-2 rounded-full bg-white px-3 py-1 text-xs font-semibold text-amber-800">
                    {rule.keyword}
                    <button
                      className="text-amber-500"
                      disabled={deleteRule.isPending}
                      onClick={() => void deleteRule.mutateAsync(rule.id).catch((err) => window.alert(err.message))}
                    >
                      삭제
                    </button>
                  </span>
                ))
              ) : (
                <span className="text-xs text-amber-700">등록된 자동분류 키워드가 없습니다.</span>
              )}
            </div>
          </div>
        ) : null}

        <div className="mt-3 space-y-2">
          {candidateRows.length ? (
            candidateRows.map((tx) => (
              <div key={tx.id} className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-slate-100 p-3">
                <div>
                  <div className="flex flex-wrap items-center gap-2">
                    <p className="font-medium text-slate-900">{transactionTitle(tx)}</p>
                    <span className={scopeBadgeClass(tx.scope === 'kream')}>
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
                      void markTransaction
                        .mutateAsync({
                          transactionId: tx.id,
                          scope: nextScope,
                          kreamKind: nextScope === 'kream' && candidateKind === 'side_cost' ? 'side_cost' : null
                        })
                        .catch((err) => window.alert(err.message));
                    }}
                  >
                    {tx.scope === 'kream'
                      ? '개인거래로'
                      : candidateKind === 'side_cost'
                        ? '공통 비용으로'
                        : 'KREAM 장부로'}
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

function SummaryCard({
  label,
  value,
  tone
}: {
  label: string;
  value: number;
  tone: 'rose' | 'emerald' | 'amber' | 'slate';
}) {
  const toneClass = {
    rose: 'bg-rose-50 text-rose-700',
    emerald: 'bg-emerald-50 text-emerald-700',
    amber: 'bg-amber-50 text-amber-700',
    slate: 'bg-slate-100 text-slate-900'
  }[tone];

  return (
    <div className={`rounded-xl p-4 ${toneClass}`}>
      <p className="text-sm text-slate-600">{label}</p>
      <p className="mt-1 text-lg font-semibold">{money(value)}</p>
    </div>
  );
}

function SaleLinkRow({
  saleId,
  kind,
  transactionId,
  transaction,
  disabled,
  onUnmatch
}: {
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

function scopeBadgeClass(active: boolean) {
  return active
    ? 'rounded-full bg-teal-50 px-2 py-0.5 text-xs font-semibold text-teal-700'
    : 'rounded-full bg-slate-100 px-2 py-0.5 text-xs font-semibold text-slate-500';
}

function defaultKindForTransaction(tx: KreamLedgerTransaction): MatchKind {
  return tx.type === 'income' ? 'settlement' : 'purchase';
}

function kindOptionsForTransaction(tx: KreamLedgerTransaction): MatchKind[] {
  if (tx.type === 'income') return ['settlement'];
  if (tx.type === 'expense') return ['purchase', 'side_cost'];
  return [];
}
