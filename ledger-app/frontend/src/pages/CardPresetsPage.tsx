import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { api } from '@/api/client';
import { money } from '@/utils/format';

type CardPreset = {
  id: string;
  issuer: string;
  card_name: string;
  aliases: string[];
  monthly_requirement?: number | null;
  rules: unknown;
  benefits: BenefitConfig[];
  parse_text?: string | null;
  created_at: string;
};

type BenefitConfig = {
  name: string;
  match?: { merchant_keywords?: string[] };
  discount: { type: string; value: number; monthly_cap?: number; per_tx_cap?: number };
  min_amount?: number;
};

type ParsedBenefitGroup = {
  group_name: string;
  discount_rate: number;
  monthly_cap?: number | null;
  monthly_usage_limit?: number | null;
  merchants: string[];
  benefit_name: string;
};

type ParsedPresetResponse = {
  monthly_requirement?: number | null;
  rules: unknown;
  benefits: BenefitConfig[];
  benefit_groups: ParsedBenefitGroup[];
};

type ApplyBenefitsResponse = {
  applied_count: number;
  skipped_count: number;
  preset_id: string;
};

const ISSUERS = ['신한카드', '삼성카드', '현대카드', 'KB국민카드', 'NH농협카드', '롯데카드', '우리카드', 'BC카드'];

const emptyManualForm = {
  issuer: '신한카드',
  cardName: '',
  monthlyRequirement: '300000',
  rules: '{"excluded":[{"merchant_contains":"상품권"},{"merchant_contains":"선불카드"}]}',
  benefits: '[]',
};

export default function CardPresetsPage() {
  const client = useQueryClient();
  const [mode, setMode] = useState<'parse' | 'manual'>('parse');

  // 파싱 모드 상태
  const [parseText, setParseText] = useState('');
  const [issuer, setIssuer] = useState('신한카드');
  const [cardNameForParse, setCardNameForParse] = useState('');
  const [parsed, setParsed] = useState<ParsedPresetResponse | null>(null);
  const [parseEditTarget, setParseEditTarget] = useState<CardPreset | null>(null);

  // 수동 모드 상태
  const [manualForm, setManualForm] = useState(emptyManualForm);
  const [editTarget, setEditTarget] = useState<CardPreset | null>(null);
  const [jsonError, setJsonError] = useState('');

  const list = useQuery({
    queryKey: ['card-presets'],
    queryFn: () => api<CardPreset[]>('/api/card-presets'),
  });

  const parsePreset = useMutation({
    mutationFn: (text: string) =>
      api<ParsedPresetResponse>('/api/card-presets/parse', {
        method: 'POST',
        body: JSON.stringify({ text }),
      }),
    onSuccess: (data) => setParsed(data),
    onError: (err: Error) => window.alert(err.message),
  });

  const saveFromParsed = useMutation({
    mutationFn: (payload: {
      issuer: string;
      card_name: string;
      monthly_requirement?: number | null;
      rules: unknown;
      benefits: unknown;
      parse_text: string;
    }) => {
      if (parseEditTarget) {
        return api<CardPreset>(`/api/card-presets/${parseEditTarget.id}`, {
          method: 'PUT',
          body: JSON.stringify(payload),
        });
      }
      return api<CardPreset>('/api/card-presets', {
        method: 'POST',
        body: JSON.stringify(payload),
      });
    },
    onSuccess: async () => {
      await client.invalidateQueries({ queryKey: ['card-presets'] });
      setParsed(null);
      setParseText('');
      setCardNameForParse('');
      setParseEditTarget(null);
      window.alert('프리셋이 저장되었습니다.');
    },
    onError: (err: Error) => window.alert(err.message),
  });

  const createManual = useMutation({
    mutationFn: () => {
      const payload = buildManualPayload();
      return api<CardPreset>('/api/card-presets', {
        method: 'POST',
        body: JSON.stringify(payload),
      });
    },
    onSuccess: async () => {
      await client.invalidateQueries({ queryKey: ['card-presets'] });
      setManualForm({ ...emptyManualForm, cardName: '' });
      setJsonError('');
    },
    onError: (err: Error) => setJsonError(err.message),
  });

  const updateManual = useMutation({
    mutationFn: () => {
      const payload = buildManualPayload();
      return api<CardPreset>(`/api/card-presets/${editTarget!.id}`, {
        method: 'PUT',
        body: JSON.stringify(payload),
      });
    },
    onSuccess: async () => {
      await client.invalidateQueries({ queryKey: ['card-presets'] });
      setEditTarget(null);
      setManualForm(emptyManualForm);
      setJsonError('');
    },
    onError: (err: Error) => setJsonError(err.message),
  });

  const removePreset = useMutation({
    mutationFn: (id: string) =>
      api<{ message: string }>(`/api/card-presets/${id}`, { method: 'DELETE' }),
    onSuccess: async () => client.invalidateQueries({ queryKey: ['card-presets'] }),
  });

  const applyPreset = useMutation({
    mutationFn: (id: string) =>
      api<ApplyBenefitsResponse>(`/api/card-presets/${id}/apply`, { method: 'POST' }),
    onSuccess: (result) => {
      void client.invalidateQueries({ queryKey: ['card-presets'] });
      window.alert(
        `혜택 적용 완료\n적용: ${result.applied_count}건 / 이미 적용된 건(스킵): ${result.skipped_count}건`
      );
    },
    onError: (err: Error) => window.alert(err.message),
  });

  function buildManualPayload() {
    let parsedRules: unknown;
    let parsedBenefits: unknown;
    try {
      parsedRules = JSON.parse(manualForm.rules);
    } catch {
      throw new Error('실적 제외 규칙의 JSON 형식이 올바르지 않습니다.');
    }
    try {
      parsedBenefits = JSON.parse(manualForm.benefits);
    } catch {
      throw new Error('혜택 규칙의 JSON 형식이 올바르지 않습니다.');
    }
    return {
      issuer: manualForm.issuer,
      card_name: manualForm.cardName,
      monthly_requirement: Number(manualForm.monthlyRequirement) || null,
      rules: parsedRules,
      benefits: parsedBenefits,
    };
  }

  function openEdit(preset: CardPreset) {
    if (preset.parse_text) {
      // 원본 텍스트가 있으면 파싱 모드로 열기
      setMode('parse');
      setParseEditTarget(preset);
      setIssuer(preset.issuer);
      setCardNameForParse(preset.card_name);
      setParseText(preset.parse_text);
      setParsed(null);
    } else {
      setEditTarget(preset);
      setMode('manual');
      setManualForm({
        issuer: preset.issuer,
        cardName: preset.card_name,
        monthlyRequirement: String(preset.monthly_requirement ?? ''),
        rules: JSON.stringify(preset.rules, null, 2),
        benefits: JSON.stringify(preset.benefits, null, 2),
      });
      setJsonError('');
    }
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }

  const isPending = createManual.isPending || updateManual.isPending;

  return (
    <div className="space-y-4">
      {/* 헤더 + 모드 전환 */}
      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h1 className="text-xl font-bold">카드 혜택 프리셋</h1>
            <p className="mt-0.5 text-sm text-slate-500">
              카드사 혜택 텍스트를 붙여넣으면 자동으로 프리셋을 생성합니다.
            </p>
          </div>
          <div className="flex gap-2">
            <button
              className={`rounded-lg px-3 py-2 text-sm font-semibold ${mode === 'parse' ? 'bg-teal-700 text-white' : 'border border-slate-300 text-slate-700'}`}
              onClick={() => setMode('parse')}
            >
              텍스트 파싱
            </button>
            <button
              className={`rounded-lg px-3 py-2 text-sm font-semibold ${mode === 'manual' ? 'bg-teal-700 text-white' : 'border border-slate-300 text-slate-700'}`}
              onClick={() => { setMode('manual'); setEditTarget(null); setManualForm(emptyManualForm); }}
            >
              직접 입력
            </button>
          </div>
        </div>
      </section>

      {/* 텍스트 파싱 모드 */}
      {mode === 'parse' && (
        <section className="rounded-2xl bg-white p-4 shadow-soft">
          <h2 className="font-bold">카드 혜택 텍스트 붙여넣기</h2>
          <p className="mt-1 text-xs text-slate-500">
            카드사 홈페이지에서 혜택 안내 텍스트를 복사해서 붙여넣으세요.
          </p>

          <div className="mt-3 grid gap-3 sm:grid-cols-2">
            <label className="text-sm">
              <span className="mb-1 block text-slate-600">카드사</span>
              <select
                className="w-full rounded-lg border border-slate-300 px-3 py-2"
                value={issuer}
                onChange={(e) => setIssuer(e.target.value)}
              >
                {ISSUERS.map((s) => <option key={s}>{s}</option>)}
              </select>
            </label>
            <label className="text-sm">
              <span className="mb-1 block text-slate-600">카드명</span>
              <input
                className="w-full rounded-lg border border-slate-300 px-3 py-2"
                placeholder="예: Deep On Platinum+"
                value={cardNameForParse}
                onChange={(e) => setCardNameForParse(e.target.value)}
              />
            </label>
          </div>

          <label className="mt-3 block text-sm">
            <span className="mb-1 block text-slate-600">혜택 텍스트</span>
            <textarea
              className="min-h-48 w-full rounded-lg border border-slate-300 px-3 py-2 text-sm leading-relaxed"
              placeholder="카드사 혜택 안내 텍스트를 여기에 붙여넣으세요..."
              value={parseText}
              onChange={(e) => { setParseText(e.target.value); setParsed(null); }}
            />
          </label>

          <div className="mt-3 flex gap-2">
            <button
              className="rounded-lg bg-teal-700 px-4 py-2 text-sm font-semibold text-white disabled:opacity-40"
              disabled={!parseText.trim() || parsePreset.isPending}
              onClick={() => void parsePreset.mutateAsync(parseText)}
            >
              {parsePreset.isPending ? '분석 중...' : '혜택 분석'}
            </button>
            {parseText && (
              <button
                className="rounded-lg border border-slate-300 px-4 py-2 text-sm"
                onClick={() => {
                  setParseText('');
                  setParsed(null);
                  setParseEditTarget(null);
                }}
              >
                초기화
              </button>
            )}
            {parseEditTarget && (
              <span className="rounded-lg bg-amber-50 px-3 py-2 text-sm font-semibold text-amber-700">
                수정 중: {parseEditTarget.card_name}
              </span>
            )}
          </div>

          {/* 파싱 결과 미리보기 */}
          {parsed && (
            <ParsedPreview
              parsed={parsed}
              issuer={issuer}
              cardName={cardNameForParse}
              parseText={parseText}
              isEdit={Boolean(parseEditTarget)}
              onSave={(cardName) => {
                void saveFromParsed.mutateAsync({
                  issuer,
                  card_name: cardName,
                  monthly_requirement: parsed.monthly_requirement,
                  rules: parsed.rules,
                  benefits: parsed.benefits,
                  parse_text: parseText,
                });
              }}
              isSaving={saveFromParsed.isPending}
            />
          )}
        </section>
      )}

      {/* 직접 입력 모드 */}
      {mode === 'manual' && (
        <section className="rounded-2xl bg-white p-4 shadow-soft">
          <h2 className="font-bold">{editTarget ? '프리셋 수정' : '직접 입력'}</h2>
          <div className="mt-3 grid gap-2 md:grid-cols-2">
            <label className="text-sm">
              <span className="mb-1 block">카드사</span>
              <select
                value={manualForm.issuer}
                onChange={(e) => setManualForm((prev) => ({ ...prev, issuer: e.target.value }))}
                className="w-full rounded-lg border border-slate-300 px-3 py-2"
              >
                {ISSUERS.map((s) => <option key={s}>{s}</option>)}
              </select>
            </label>
            <label className="text-sm">
              <span className="mb-1 block">카드명</span>
              <input
                value={manualForm.cardName}
                onChange={(e) => setManualForm((prev) => ({ ...prev, cardName: e.target.value }))}
                className="w-full rounded-lg border border-slate-300 px-3 py-2"
              />
            </label>
            <label className="text-sm">
              <span className="mb-1 block">전월실적 기준 (원)</span>
              <input
                type="number"
                value={manualForm.monthlyRequirement}
                onChange={(e) => setManualForm((prev) => ({ ...prev, monthlyRequirement: e.target.value }))}
                className="w-full rounded-lg border border-slate-300 px-3 py-2"
              />
            </label>
            <div className="rounded-lg border border-dashed border-slate-300 p-3 text-xs text-slate-500">
              혜택 규칙 JSON 배열: name, match.merchant_keywords, discount.type/value/monthly_cap
            </div>
            <label className="text-sm md:col-span-2">
              <span className="mb-1 block">실적 제외 규칙 (JSON)</span>
              <textarea
                className="min-h-20 w-full rounded-lg border border-slate-300 px-3 py-2 font-mono text-xs"
                value={manualForm.rules}
                onChange={(e) => setManualForm((prev) => ({ ...prev, rules: e.target.value }))}
              />
            </label>
            <label className="text-sm md:col-span-2">
              <span className="mb-1 block">혜택 규칙 (JSON)</span>
              <textarea
                className="min-h-32 w-full rounded-lg border border-slate-300 px-3 py-2 font-mono text-xs"
                value={manualForm.benefits}
                onChange={(e) => setManualForm((prev) => ({ ...prev, benefits: e.target.value }))}
              />
            </label>
          </div>
          {jsonError && <p className="mt-2 text-sm text-rose-600">{jsonError}</p>}
          <div className="mt-3 flex gap-2">
            <button
              className="rounded-lg bg-teal-700 px-4 py-2 font-semibold text-white disabled:opacity-50"
              disabled={isPending}
              onClick={() => {
                if (!manualForm.cardName.trim()) { setJsonError('카드명을 입력해 주세요.'); return; }
                setJsonError('');
                if (editTarget) void updateManual.mutateAsync();
                else void createManual.mutateAsync();
              }}
            >
              {isPending ? '저장 중...' : editTarget ? '수정 저장' : '프리셋 추가'}
            </button>
            {editTarget && (
              <button
                className="rounded-lg border border-slate-300 px-4 py-2 text-sm"
                onClick={() => { setEditTarget(null); setManualForm(emptyManualForm); setJsonError(''); }}
              >
                취소
              </button>
            )}
          </div>
        </section>
      )}

      {/* 프리셋 목록 */}
      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <h2 className="text-lg font-bold">프리셋 목록</h2>
        <div className="mt-3 space-y-3">
          {list.data?.length ? (
            list.data.map((preset) => (
              <PresetCard
                key={preset.id}
                preset={preset}
                onEdit={openEdit}
                onDelete={(id) => {
                  if (window.confirm('프리셋을 삭제하시겠습니까?')) void removePreset.mutateAsync(id);
                }}
                onApply={(id) => {
                  if (window.confirm('이 프리셋을 연결된 카드의 기존 거래에 소급 적용하시겠습니까?\n이미 적용된 건은 건너뜁니다.'))
                    void applyPreset.mutateAsync(id);
                }}
                isApplying={applyPreset.isPending}
              />
            ))
          ) : (
            <p className="rounded-lg border border-dashed border-slate-300 p-4 text-sm text-slate-500">
              등록된 프리셋이 없습니다.
            </p>
          )}
        </div>
      </section>
    </div>
  );
}

// ─── 파싱 결과 미리보기 컴포넌트 ──────────────────────────────────────────────

function ParsedPreview({
  parsed,
  issuer,
  cardName: initialCardName,
  parseText,
  isEdit,
  onSave,
  isSaving,
}: {
  parsed: ParsedPresetResponse;
  issuer: string;
  cardName: string;
  parseText: string;
  isEdit?: boolean;
  onSave: (cardName: string) => void;
  isSaving: boolean;
}) {
  const [cardName, setCardName] = useState(initialCardName);

  const totalMaxCap = useMemo(() => {
    return parsed.benefit_groups.reduce((sum, g) => sum + (g.monthly_cap ?? 0), 0);
  }, [parsed.benefit_groups]);

  return (
    <div className="mt-4 rounded-lg border border-teal-200 bg-teal-50 p-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <p className="font-semibold text-teal-900">분석 결과</p>
          <p className="mt-0.5 text-xs text-teal-700">
            혜택 그룹 {parsed.benefit_groups.length}개 · 최대 월 할인 {money(totalMaxCap)}
            {parsed.monthly_requirement != null && (
              <> · 전월 실적 {money(parsed.monthly_requirement)} 이상</>
            )}
          </p>
        </div>
      </div>

      {/* 혜택 그룹 목록 */}
      <div className="mt-3 space-y-2">
        {parsed.benefit_groups.map((g, idx) => (
          <div key={idx} className="rounded-lg border border-teal-100 bg-white p-3">
            <div className="flex flex-wrap items-center gap-2">
              <span className="rounded-full bg-teal-100 px-2 py-0.5 text-xs font-bold text-teal-800">
                {g.discount_rate}% 할인
              </span>
              <span className="font-medium text-slate-900">{g.group_name || g.benefit_name}</span>
              {g.monthly_cap != null && (
                <span className="text-xs text-slate-500">월 최대 {money(g.monthly_cap)}</span>
              )}
              {g.monthly_usage_limit != null && (
                <span className="text-xs text-slate-500">월 {g.monthly_usage_limit}회</span>
              )}
            </div>
            {g.merchants.length > 0 && (
              <div className="mt-1.5 flex flex-wrap gap-1">
                {g.merchants.map((m) => (
                  <span key={m} className="rounded bg-slate-100 px-1.5 py-0.5 text-xs text-slate-700">
                    {m}
                  </span>
                ))}
              </div>
            )}
          </div>
        ))}
      </div>

      {/* 저장 폼 */}
      <div className="mt-4 flex flex-wrap items-end gap-3">
        <label className="flex-1 text-sm">
          <span className="mb-1 block text-teal-800">카드명 확인</span>
          <input
            className="w-full rounded-lg border border-teal-300 bg-white px-3 py-2"
            value={cardName}
            onChange={(e) => setCardName(e.target.value)}
            placeholder="카드명을 입력하세요"
          />
        </label>
        <button
          className="rounded-lg bg-teal-700 px-4 py-2 text-sm font-semibold text-white disabled:opacity-40"
          disabled={!cardName.trim() || isSaving}
          onClick={() => onSave(cardName.trim())}
        >
          {isSaving ? '저장 중...' : isEdit ? '프리셋 수정 저장' : `${issuer} 프리셋 저장`}
        </button>
      </div>

      {/* 생성될 JSON 미리보기 (접기 가능) */}
      <details className="mt-3">
        <summary className="cursor-pointer text-xs text-teal-700 hover:text-teal-900">
          혜택 JSON 보기
        </summary>
        <pre className="mt-2 overflow-x-auto rounded-lg bg-white p-3 text-xs text-slate-700">
          {JSON.stringify(parsed.benefits, null, 2)}
        </pre>
      </details>
    </div>
  );
}

// ─── 프리셋 카드 컴포넌트 ─────────────────────────────────────────────────────

function PresetCard({
  preset,
  onEdit,
  onDelete,
  onApply,
  isApplying,
}: {
  preset: CardPreset;
  onEdit: (preset: CardPreset) => void;
  onDelete: (id: string) => void;
  onApply: (id: string) => void;
  isApplying: boolean;
}) {
  const benefits = Array.isArray(preset.benefits) ? preset.benefits : [];

  return (
    <div className="rounded-xl border border-slate-200 p-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="flex flex-wrap items-center gap-2">
            <p className="font-semibold text-slate-900">{preset.card_name}</p>
            <span className="rounded-full bg-slate-100 px-2 py-0.5 text-xs text-slate-600">
              {preset.issuer}
            </span>
            {preset.monthly_requirement != null && (
              <span className="rounded-full bg-teal-50 px-2 py-0.5 text-xs text-teal-700">
                전월실적 {money(preset.monthly_requirement)} 이상
              </span>
            )}
          </div>
          {benefits.length > 0 && (
            <div className="mt-2 flex flex-wrap gap-1.5">
              {benefits.map((b, i) => (
                <BenefitChip key={i} benefit={b} />
              ))}
            </div>
          )}
        </div>
        <div className="flex flex-wrap gap-2">
          <button
            className="rounded-lg border border-teal-300 bg-teal-50 px-3 py-1.5 text-xs font-semibold text-teal-700 hover:bg-teal-100 disabled:opacity-40"
            disabled={isApplying}
            onClick={() => onApply(preset.id)}
          >
            기존 거래 적용
          </button>
          <button
            className="rounded border border-slate-300 px-2 py-1 text-xs"
            onClick={() => onEdit(preset)}
          >
            수정
          </button>
          <button
            className="rounded border border-rose-300 px-2 py-1 text-xs text-rose-700"
            onClick={() => onDelete(preset.id)}
          >
            삭제
          </button>
        </div>
      </div>
    </div>
  );
}

function BenefitChip({ benefit }: { benefit: BenefitConfig }) {
  const merchants = benefit.match?.merchant_keywords ?? [];
  const discountLabel =
    benefit.discount.type === 'percent'
      ? `${benefit.discount.value}%`
      : money(benefit.discount.value);

  return (
    <div className="flex items-center gap-1 rounded-lg bg-slate-50 px-2 py-1 text-xs">
      <span className="font-semibold text-teal-700">{discountLabel}</span>
      <span className="text-slate-600">{benefit.name}</span>
      {benefit.discount.monthly_cap != null && (
        <span className="text-slate-400">최대 {money(benefit.discount.monthly_cap)}</span>
      )}
      {merchants.length > 0 && (
        <span className="text-slate-400">({merchants.slice(0, 3).join(', ')}{merchants.length > 3 ? ` 외 ${merchants.length - 3}개` : ''})</span>
      )}
    </div>
  );
}
