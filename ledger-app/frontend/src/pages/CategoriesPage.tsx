import { useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { api } from '@/api/client';
import type { Category } from '@/types';

const CATEGORY_TYPES = [
  { value: 'expense', label: '지출' },
  { value: 'income', label: '수입' },
  { value: 'transfer', label: '이체' },
];

const emptyForm = { name: '', type: 'expense', sort_order: 0 };

export default function CategoriesPage() {
  const client = useQueryClient();
  const [form, setForm] = useState(emptyForm);
  const [editTarget, setEditTarget] = useState<Category | null>(null);
  const [error, setError] = useState('');

  const categories = useQuery({
    queryKey: ['categories'],
    queryFn: () => api<Category[]>('/api/categories'),
  });

  const create = useMutation({
    mutationFn: () =>
      api<Category>('/api/categories', {
        method: 'POST',
        body: JSON.stringify(form),
      }),
    onSuccess: async () => {
      await client.invalidateQueries({ queryKey: ['categories'] });
      setForm(emptyForm);
      setError('');
    },
    onError: (e: Error) => setError(e.message),
  });

  const update = useMutation({
    mutationFn: () =>
      api<Category>(`/api/categories/${editTarget!.id}`, {
        method: 'PUT',
        body: JSON.stringify(form),
      }),
    onSuccess: async () => {
      await client.invalidateQueries({ queryKey: ['categories'] });
      setEditTarget(null);
      setForm(emptyForm);
      setError('');
    },
    onError: (e: Error) => setError(e.message),
  });

  const remove = useMutation({
    mutationFn: (id: string) => api<{ message: string }>(`/api/categories/${id}`, { method: 'DELETE' }),
    onSuccess: async () => client.invalidateQueries({ queryKey: ['categories'] }),
  });

  const openEdit = (cat: Category) => {
    setEditTarget(cat);
    setForm({ name: cat.name, type: cat.type, sort_order: cat.sort_order });
    setError('');
  };

  const handleSubmit = () => {
    setError('');
    if (!form.name.trim()) {
      setError('카테고리 이름을 입력해 주세요.');
      return;
    }
    if (editTarget) {
      void update.mutateAsync();
    } else {
      void create.mutateAsync();
    }
  };

  const isPending = create.isPending || update.isPending;

  const groupedByType = CATEGORY_TYPES.map((t) => ({
    ...t,
    items: categories.data?.filter((c) => c.type === t.value) ?? [],
  }));

  return (
    <div className="space-y-4">
      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <h1 className="text-xl font-bold">{editTarget ? '카테고리 수정' : '카테고리 추가'}</h1>
        <div className="mt-3 grid gap-3 md:grid-cols-3">
          <label className="text-sm md:col-span-1">
            <span className="mb-1 block">카테고리명</span>
            <input
              className="w-full rounded-lg border border-slate-300 px-3 py-2"
              value={form.name}
              onChange={(e) => setForm((prev) => ({ ...prev, name: e.target.value }))}
            />
          </label>
          <label className="text-sm">
            <span className="mb-1 block">유형</span>
            <select
              className="w-full rounded-lg border border-slate-300 px-3 py-2"
              value={form.type}
              onChange={(e) => setForm((prev) => ({ ...prev, type: e.target.value }))}
            >
              {CATEGORY_TYPES.map((t) => (
                <option key={t.value} value={t.value}>{t.label}</option>
              ))}
            </select>
          </label>
          <label className="text-sm">
            <span className="mb-1 block">정렬 순서</span>
            <input
              type="number"
              className="w-full rounded-lg border border-slate-300 px-3 py-2"
              value={form.sort_order}
              onChange={(e) => setForm((prev) => ({ ...prev, sort_order: Number(e.target.value) }))}
            />
          </label>
        </div>
        {error ? <p className="mt-2 text-sm text-rose-600">{error}</p> : null}
        <div className="mt-3 flex gap-2">
          <button
            className="rounded-lg bg-teal-700 px-4 py-2 font-semibold text-white disabled:opacity-50"
            disabled={isPending}
            onClick={handleSubmit}
          >
            {isPending ? '저장 중...' : editTarget ? '수정 저장' : '카테고리 추가'}
          </button>
          {editTarget ? (
            <button
              className="rounded-lg border border-slate-300 px-4 py-2 text-sm"
              onClick={() => { setEditTarget(null); setForm(emptyForm); setError(''); }}
            >
              취소
            </button>
          ) : null}
        </div>
      </section>

      {groupedByType.map((group) => (
        group.items.length > 0 ? (
          <section key={group.value} className="rounded-2xl bg-white p-4 shadow-soft">
            <h2 className="text-base font-bold text-slate-700">{group.label} 카테고리</h2>
            <div className="mt-3 space-y-2">
              {group.items.map((cat) => (
                <div
                  key={cat.id}
                  className="flex items-center justify-between rounded-lg border border-slate-200 p-3"
                >
                  <div>
                    <p className="font-medium">{cat.name}</p>
                    <p className="text-xs text-slate-400">순서: {cat.sort_order}</p>
                  </div>
                  <div className="flex gap-2">
                    <button
                      className="rounded border border-slate-300 px-2 py-1 text-xs"
                      onClick={() => openEdit(cat)}
                    >
                      수정
                    </button>
                    <button
                      className="rounded border border-rose-300 px-2 py-1 text-xs text-rose-700"
                      onClick={() => {
                        if (window.confirm('카테고리를 삭제하시겠습니까?')) {
                          void remove.mutateAsync(cat.id);
                        }
                      }}
                    >
                      삭제
                    </button>
                  </div>
                </div>
              ))}
            </div>
          </section>
        ) : null
      ))}

      {!categories.data?.length && !categories.isLoading ? (
        <section className="rounded-2xl bg-white p-4 shadow-soft">
          <p className="text-sm text-slate-500">등록된 카테고리가 없습니다.</p>
        </section>
      ) : null}
    </div>
  );
}
