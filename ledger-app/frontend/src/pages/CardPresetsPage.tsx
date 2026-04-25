import { useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { api } from '@/api/client';

type CardPreset = {
  id: string;
  issuer: string;
  card_name: string;
  monthly_requirement?: number | null;
  rules: unknown;
  benefits: unknown;
};

const emptyForm = {
  issuer: '신한카드',
  cardName: '',
  monthlyRequirement: '300000',
  rules: '{"excluded":[{"merchant_contains":"상품권"}]}',
  benefits: '[{"name":"커피 할인","match":{"merchant_keywords":["스타벅스","투썸","이디야"]},"discount":{"type":"percent","value":10,"monthly_cap":5000}}]',
};

export default function CardPresetsPage() {
  const client = useQueryClient();
  const list = useQuery({
    queryKey: ['card-presets'],
    queryFn: () => api<CardPreset[]>('/api/card-presets'),
  });

  const [form, setForm] = useState(emptyForm);
  const [editTarget, setEditTarget] = useState<CardPreset | null>(null);
  const [jsonError, setJsonError] = useState('');

  const buildPayload = () => {
    let parsedRules: unknown;
    let parsedBenefits: unknown;
    try {
      parsedRules = JSON.parse(form.rules);
    } catch {
      throw new Error('실적 제외 규칙의 JSON 형식이 올바르지 않습니다.');
    }
    try {
      parsedBenefits = JSON.parse(form.benefits);
    } catch {
      throw new Error('혜택 규칙의 JSON 형식이 올바르지 않습니다.');
    }
    return {
      issuer: form.issuer,
      card_name: form.cardName,
      monthly_requirement: Number(form.monthlyRequirement),
      rules: parsedRules,
      benefits: parsedBenefits,
    };
  };

  const create = useMutation({
    mutationFn: async () => {
      const payload = buildPayload();
      return api<CardPreset>('/api/card-presets', {
        method: 'POST',
        body: JSON.stringify(payload),
      });
    },
    onSuccess: async () => {
      await client.invalidateQueries({ queryKey: ['card-presets'] });
      setForm({ ...emptyForm, cardName: '' });
      setJsonError('');
    },
    onError: (err: Error) => setJsonError(err.message),
  });

  const update = useMutation({
    mutationFn: async () => {
      const payload = buildPayload();
      return api<CardPreset>(`/api/card-presets/${editTarget!.id}`, {
        method: 'PUT',
        body: JSON.stringify(payload),
      });
    },
    onSuccess: async () => {
      await client.invalidateQueries({ queryKey: ['card-presets'] });
      setEditTarget(null);
      setForm(emptyForm);
      setJsonError('');
    },
    onError: (err: Error) => setJsonError(err.message),
  });

  const remove = useMutation({
    mutationFn: (id: string) =>
      api<{ message: string }>(`/api/card-presets/${id}`, { method: 'DELETE' }),
    onSuccess: async () => client.invalidateQueries({ queryKey: ['card-presets'] }),
  });

  const openEdit = (preset: CardPreset) => {
    setEditTarget(preset);
    setForm({
      issuer: preset.issuer,
      cardName: preset.card_name,
      monthlyRequirement: String(preset.monthly_requirement ?? ''),
      rules: JSON.stringify(preset.rules, null, 2),
      benefits: JSON.stringify(preset.benefits, null, 2),
    });
    setJsonError('');
  };

  const handleSubmit = () => {
    if (!form.cardName.trim()) {
      setJsonError('카드명을 입력해 주세요.');
      return;
    }
    setJsonError('');
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
        <h1 className="text-xl font-bold">{editTarget ? '카드 프리셋 수정' : '카드 혜택 프리셋'}</h1>
        <div className="mt-3 grid gap-2 md:grid-cols-2">
          <label className="text-sm">
            <span className="mb-1 block">카드사</span>
            <input
              value={form.issuer}
              onChange={(e) => setForm((prev) => ({ ...prev, issuer: e.target.value }))}
              className="w-full rounded-lg border border-slate-300 px-3 py-2"
            />
          </label>
          <label className="text-sm">
            <span className="mb-1 block">카드명</span>
            <input
              value={form.cardName}
              onChange={(e) => setForm((prev) => ({ ...prev, cardName: e.target.value }))}
              className="w-full rounded-lg border border-slate-300 px-3 py-2"
            />
          </label>
          <label className="text-sm">
            <span className="mb-1 block">전월실적 기준</span>
            <input
              type="number"
              value={form.monthlyRequirement}
              onChange={(e) => setForm((prev) => ({ ...prev, monthlyRequirement: e.target.value }))}
              className="w-full rounded-lg border border-slate-300 px-3 py-2"
            />
          </label>
          <div className="rounded-lg border border-dashed border-slate-300 p-3 text-xs text-slate-600">
            실적 제외 규칙과 혜택 규칙은 구조화된 텍스트로 입력합니다.
          </div>
          <label className="text-sm md:col-span-2">
            <span className="mb-1 block">실적 제외 규칙</span>
            <textarea
              className="min-h-24 w-full rounded-lg border border-slate-300 px-3 py-2 font-mono"
              value={form.rules}
              onChange={(e) => setForm((prev) => ({ ...prev, rules: e.target.value }))}
            />
          </label>
          <label className="text-sm md:col-span-2">
            <span className="mb-1 block">혜택 규칙</span>
            <textarea
              className="min-h-24 w-full rounded-lg border border-slate-300 px-3 py-2 font-mono"
              value={form.benefits}
              onChange={(e) => setForm((prev) => ({ ...prev, benefits: e.target.value }))}
            />
          </label>
        </div>

        {jsonError ? <p className="mt-2 text-sm text-rose-600">{jsonError}</p> : null}

        <div className="mt-3 flex gap-2">
          <button
            className="rounded-lg bg-teal-700 px-4 py-2 font-semibold text-white disabled:opacity-50"
            disabled={isPending}
            onClick={handleSubmit}
          >
            {isPending ? '저장 중...' : editTarget ? '수정 저장' : '프리셋 추가'}
          </button>
          {editTarget ? (
            <button
              className="rounded-lg border border-slate-300 px-4 py-2 text-sm"
              onClick={() => { setEditTarget(null); setForm(emptyForm); setJsonError(''); }}
            >
              취소
            </button>
          ) : null}
        </div>
      </section>

      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <h2 className="text-lg font-bold">프리셋 목록</h2>
        <div className="mt-3 space-y-2">
          {list.data?.length ? (
            list.data.map((preset) => (
              <div key={preset.id} className="flex items-center justify-between rounded-lg border border-slate-200 p-3">
                <div>
                  <p className="font-medium">{preset.card_name}</p>
                  <p className="text-sm text-slate-500">
                    {preset.issuer}
                    {preset.monthly_requirement ? ` · 전월실적 ${preset.monthly_requirement.toLocaleString()}원` : ''}
                  </p>
                </div>
                <div className="flex gap-2">
                  <button
                    className="rounded border border-slate-300 px-2 py-1 text-xs"
                    onClick={() => openEdit(preset)}
                  >
                    수정
                  </button>
                  <button
                    className="rounded border border-rose-300 px-2 py-1 text-xs text-rose-700"
                    onClick={() => {
                      if (window.confirm('프리셋을 삭제하시겠습니까?')) {
                        void remove.mutateAsync(preset.id);
                      }
                    }}
                  >
                    삭제
                  </button>
                </div>
              </div>
            ))
          ) : (
            <p className="text-sm text-slate-500">등록된 프리셋이 없습니다.</p>
          )}
        </div>
      </section>
    </div>
  );
}
