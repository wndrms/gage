import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { resourceApi } from '@/api/resources';
import { money, today } from '@/utils/format';

export default function AssetsPage() {
  const client = useQueryClient();
  const accounts = useQuery({ queryKey: ['accounts'], queryFn: resourceApi.accounts });
  const snapshots = useQuery({ queryKey: ['assets', 'snapshots'], queryFn: resourceApi.assetSnapshots });

  const todayDate = today();
  const [snapshotDate, setSnapshotDate] = useState(todayDate);
  const [accountId, setAccountId] = useState('');
  const [amount, setAmount] = useState(0);
  const [memo, setMemo] = useState('');

  const 생성 = useMutation({
    mutationFn: () =>
      resourceApi.createAssetSnapshot({
        snapshot_date: snapshotDate,
        account_id: accountId,
        amount: Number(amount),
        memo
      }),
    onSuccess: async () => {
      await client.invalidateQueries({ queryKey: ['assets'] });
      setMemo('');
    }
  });

  const netWorth = useQuery({
    queryKey: ['assets', 'net-worth', todayDate],
    queryFn: () => resourceApi.netWorth(`${todayDate.slice(0, 8)}01`, todayDate)
  });

  const latest = useMemo(() => {
    if (!netWorth.data?.length) return null;
    return netWorth.data[netWorth.data.length - 1];
  }, [netWorth.data]);

  return (
    <div className="space-y-4">
      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <h1 className="text-xl font-bold">자산</h1>
        <div className="mt-3 grid gap-2 sm:grid-cols-3">
          <div className="rounded-lg bg-slate-100 p-3">
            <p className="text-sm">총자산</p>
            <p className="font-semibold">{money(latest?.assets ?? 0)}</p>
          </div>
          <div className="rounded-lg bg-rose-50 p-3">
            <p className="text-sm">카드 미청구금</p>
            <p className="font-semibold text-rose-700">{money(latest?.liabilities ?? 0)}</p>
          </div>
          <div className="rounded-lg bg-emerald-50 p-3">
            <p className="text-sm">순자산</p>
            <p className="font-semibold text-emerald-700">{money(latest?.net_worth ?? 0)}</p>
          </div>
        </div>
      </section>

      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <h2 className="text-lg font-bold">자산 스냅샷</h2>
        <div className="mt-3 grid gap-2 md:grid-cols-4">
          <input
            type="date"
            className="rounded-lg border border-slate-300 px-3 py-2"
            value={snapshotDate}
            onChange={(e) => setSnapshotDate(e.target.value)}
          />
          <select
            className="rounded-lg border border-slate-300 px-3 py-2"
            value={accountId}
            onChange={(e) => setAccountId(e.target.value)}
          >
            <option value="">계좌 선택</option>
            {accounts.data?.map((account) => (
              <option key={account.id} value={account.id}>
                {account.name}
              </option>
            ))}
          </select>
          <input
            type="number"
            className="rounded-lg border border-slate-300 px-3 py-2"
            placeholder="금액"
            value={amount}
            onChange={(e) => setAmount(Number(e.target.value))}
          />
          <button
            className="rounded-lg bg-teal-700 px-3 py-2 font-semibold text-white"
            onClick={() => {
              if (!accountId) {
                window.alert('계좌를 선택해 주세요.');
                return;
              }
              void 생성.mutateAsync().catch((err) => window.alert(err.message));
            }}
          >
            스냅샷 추가
          </button>
        </div>
        <input
          className="mt-2 w-full rounded-lg border border-slate-300 px-3 py-2"
          placeholder="메모"
          value={memo}
          onChange={(e) => setMemo(e.target.value)}
        />

        <div className="mt-4 space-y-2">
          {snapshots.data?.map((snapshot) => (
            <div key={snapshot.id} className="rounded-lg border border-slate-200 p-3">
              <p className="font-medium">{snapshot.snapshot_date}</p>
              <p className="text-sm text-slate-600">{money(snapshot.amount)}</p>
              <p className="text-xs text-slate-500">{snapshot.memo || '메모 없음'}</p>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}
