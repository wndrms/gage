import { useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Link } from 'react-router-dom';
import { resourceApi } from '@/api/resources';

export default function CardsPage() {
  const client = useQueryClient();
  const cards = useQuery({ queryKey: ['cards'], queryFn: resourceApi.cards });

  const 추가 = useMutation({
    mutationFn: (payload: { issuer: string; card_name: string }) => resourceApi.createCard(payload),
    onSuccess: async () => {
      await client.invalidateQueries({ queryKey: ['cards'] });
    }
  });

  const [issuer, setIssuer] = useState('신한카드');
  const [name, setName] = useState('');

  return (
    <div className="space-y-4">
      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <h1 className="text-xl font-bold">카드 목록</h1>
        <div className="mt-3 grid gap-2 md:grid-cols-3">
          <select
            value={issuer}
            onChange={(e) => setIssuer(e.target.value)}
            className="rounded-lg border border-slate-300 px-3 py-2"
          >
            <option>신한카드</option>
            <option>현대카드</option>
            <option>삼성카드</option>
            <option>BC카드</option>
          </select>
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="카드명"
            className="rounded-lg border border-slate-300 px-3 py-2"
          />
          <button
            className="rounded-lg bg-teal-700 px-3 py-2 font-semibold text-white"
            onClick={() => {
              if (!name.trim()) {
                window.alert('카드명을 입력해 주세요.');
                return;
              }
              void 추가.mutateAsync({ issuer, card_name: name });
              setName('');
            }}
          >
            카드 추가
          </button>
        </div>
      </section>

      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <h2 className="text-lg font-bold">카드 대시보드</h2>
        <div className="mt-3 space-y-2">
          {cards.data?.length ? (
            cards.data.map((card) => (
              <div key={card.id} className="rounded-lg border border-slate-200 p-3">
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <p className="font-semibold">{card.card_name}</p>
                    <p className="text-sm text-slate-500">{card.issuer}</p>
                  </div>
                  <div className="flex gap-2">
                    <Link
                      to={`/cards/${card.id}`}
                      className="rounded border border-teal-600 px-3 py-1 text-sm font-medium text-teal-700"
                    >
                      상세 보기
                    </Link>
                    <Link
                      to="/cards/presets"
                      className="rounded border border-slate-300 px-3 py-1 text-sm text-slate-700"
                    >
                      프리셋 관리
                    </Link>
                  </div>
                </div>
              </div>
            ))
          ) : (
            <p className="rounded-lg border border-dashed border-slate-300 p-4 text-sm text-slate-500">
              등록된 카드가 없습니다.
            </p>
          )}
        </div>
      </section>
    </div>
  );
}
