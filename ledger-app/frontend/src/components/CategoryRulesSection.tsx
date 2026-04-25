import { useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { resourceApi } from '@/api/resources';
import type { CategoryRule } from '@/types';

const SOURCE_LABEL: Record<string, { label: string; cls: string }> = {
  seed: { label: '기본', cls: 'bg-slate-100 text-slate-600' },
  learned: { label: '학습', cls: 'bg-blue-100 text-blue-700' },
  user: { label: '수동', cls: 'bg-teal-100 text-teal-700' },
};

export default function CategoryRulesSection() {
  const qc = useQueryClient();
  const [keyword, setKeyword] = useState('');
  const [categoryId, setCategoryId] = useState('');
  const [error, setError] = useState('');

  const rulesQuery = useQuery({
    queryKey: ['category-rules'],
    queryFn: resourceApi.categoryRules,
  });

  const categoriesQuery = useQuery({
    queryKey: ['categories'],
    queryFn: resourceApi.categories,
  });

  const createMutation = useMutation({
    mutationFn: resourceApi.createCategoryRule,
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['category-rules'] });
      setKeyword('');
      setCategoryId('');
      setError('');
    },
    onError: (e: Error) => setError(e.message),
  });

  const deleteMutation = useMutation({
    mutationFn: resourceApi.deleteCategoryRule,
    onSuccess: () => qc.invalidateQueries({ queryKey: ['category-rules'] }),
  });

  const expenseCategories = categoriesQuery.data?.filter((c) => c.type === 'expense') ?? [];

  function handleAdd() {
    if (!keyword.trim()) { setError('키워드를 입력해 주세요'); return; }
    if (!categoryId) { setError('카테고리를 선택해 주세요'); return; }
    setError('');
    createMutation.mutate({ keyword: keyword.trim(), category_id: categoryId });
  }

  const categoryName = (id: string) =>
    categoriesQuery.data?.find((c) => c.id === id)?.name ?? id;

  // seed/learned/user로 그룹 정렬 (user > learned > seed)
  const sortedRules: CategoryRule[] = [...(rulesQuery.data ?? [])].sort((a, b) => {
    const order = { user: 0, learned: 1, seed: 2 };
    return (order[a.source] ?? 3) - (order[b.source] ?? 3);
  });

  return (
    <section className="rounded-2xl bg-white p-5 shadow-soft">
      <h2 className="text-base font-bold text-slate-800">자동 분류 규칙</h2>
      <p className="mt-0.5 text-xs text-slate-400">
        가맹점명에 키워드가 포함되면 해당 카테고리로 자동 분류됩니다.
        거래 카테고리를 직접 수정하면 해당 가맹점 규칙이 자동으로 학습됩니다.
      </p>

      {/* 규칙 추가 폼 */}
      <div className="mt-4 flex flex-wrap gap-2">
        <input
          className="flex-1 min-w-[140px] rounded-lg border border-slate-300 px-3 py-2 text-sm focus:border-teal-500 focus:outline-none"
          placeholder="키워드 (예: 스타벅스)"
          value={keyword}
          onChange={(e) => setKeyword(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && handleAdd()}
        />
        <select
          className="flex-1 min-w-[120px] rounded-lg border border-slate-300 px-3 py-2 text-sm focus:border-teal-500 focus:outline-none"
          value={categoryId}
          onChange={(e) => setCategoryId(e.target.value)}
        >
          <option value="">카테고리 선택</option>
          {expenseCategories.map((c) => (
            <option key={c.id} value={c.id}>{c.name}</option>
          ))}
        </select>
        <button
          onClick={handleAdd}
          disabled={createMutation.isPending}
          className="rounded-lg bg-teal-600 px-4 py-2 text-sm font-medium text-white hover:bg-teal-700 disabled:opacity-50"
        >
          추가
        </button>
      </div>
      {error && <p className="mt-1.5 text-xs text-rose-600">{error}</p>}

      {/* 규칙 목록 */}
      <div className="mt-4 space-y-1.5">
        {rulesQuery.isLoading ? (
          <p className="text-sm text-slate-400">불러오는 중...</p>
        ) : sortedRules.length === 0 ? (
          <p className="text-sm text-slate-400">등록된 규칙이 없습니다.</p>
        ) : (
          sortedRules.map((rule) => {
            const src = SOURCE_LABEL[rule.source] ?? SOURCE_LABEL.user;
            return (
              <div
                key={rule.id}
                className="flex items-center justify-between gap-2 rounded-lg border border-slate-100 bg-slate-50 px-3 py-2"
              >
                <div className="flex min-w-0 items-center gap-2">
                  <span className={`shrink-0 rounded-full px-2 py-0.5 text-[10px] font-semibold ${src.cls}`}>
                    {src.label}
                  </span>
                  <span className="truncate text-sm font-medium text-slate-800">
                    {rule.keyword}
                  </span>
                  <span className="text-xs text-slate-400">→</span>
                  <span className="truncate text-sm text-teal-700">
                    {categoryName(rule.category_id)}
                  </span>
                </div>
                <button
                  onClick={() => deleteMutation.mutate(rule.id)}
                  disabled={deleteMutation.isPending}
                  className="shrink-0 rounded px-2 py-1 text-xs text-rose-500 hover:bg-rose-50 disabled:opacity-50"
                >
                  삭제
                </button>
              </div>
            );
          })
        )}
      </div>
    </section>
  );
}
