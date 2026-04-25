import { useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { resourceApi } from '@/api/resources';

export default function ImportsPage() {
  const client = useQueryClient();
  const [pasted, setPasted] = useState('');
  const [institution, setInstitution] = useState('shinhan_bank');

  const list = useQuery({ queryKey: ['imports'], queryFn: resourceApi.imports });

  const 붙여넣기 = useMutation({
    mutationFn: () => resourceApi.importPastedText(pasted, institution),
    onSuccess: async () => {
      setPasted('');
      await client.invalidateQueries({ queryKey: ['imports'] });
    }
  });

  const 저장확정 = useMutation({
    mutationFn: (id: string) => resourceApi.confirmImport(id),
    onSuccess: async () => {
      await client.invalidateQueries({ queryKey: ['imports'] });
      window.alert('가져오기 저장이 완료되었습니다.');
    }
  });

  const 파일업로드 = async (file: File) => {
    const form = new FormData();
    form.append('file', file);
    form.append('institution', institution);

    const response = await fetch('/api/imports/upload', {
      method: 'POST',
      body: form,
      credentials: 'include'
    });

    const body = await response.json();
    if (!response.ok) {
      throw new Error(body?.message || '파일 가져오기에 실패했습니다.');
    }

    await client.invalidateQueries({ queryKey: ['imports'] });
  };

  return (
    <div className="space-y-4">
      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <h1 className="text-xl font-bold">파일 가져오기</h1>
        <p className="mt-1 text-sm text-slate-500">거래 내역 파일을 올리면 미리보기가 생성됩니다.</p>

        <div className="mt-3 flex flex-wrap items-center gap-2">
          <select
            value={institution}
            onChange={(e) => setInstitution(e.target.value)}
            className="rounded-lg border border-slate-300 px-3 py-2"
          >
            <option value="shinhan_bank">신한은행</option>
            <option value="shinhan_card">신한카드</option>
            <option value="hyundai_card">현대카드</option>
            <option value="samsung_card">삼성카드</option>
            <option value="bc_card">BC카드</option>
          </select>
          <label className="cursor-pointer rounded-lg bg-teal-700 px-3 py-2 text-sm font-semibold text-white">
            파일 선택
            <input
              type="file"
              className="hidden"
              accept=".csv,.xls,.xlsx"
              onChange={(e) => {
                const file = e.target.files?.[0];
                if (file) {
                  void 파일업로드(file).catch((err) => window.alert(err.message));
                }
              }}
            />
          </label>
        </div>
      </section>

      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <h2 className="text-lg font-bold">텍스트 붙여넣기</h2>
        <textarea
          className="mt-3 min-h-40 w-full rounded-lg border border-slate-300 px-3 py-2"
          placeholder="날짜,시간,가맹점,금액 형식의 텍스트를 붙여넣어 주세요"
          value={pasted}
          onChange={(e) => setPasted(e.target.value)}
        />
        <button
          className="mt-3 rounded-lg bg-teal-700 px-4 py-2 font-semibold text-white"
          onClick={() => {
            if (!pasted.trim()) {
              window.alert('붙여넣을 텍스트를 입력해 주세요.');
              return;
            }
            void 붙여넣기.mutateAsync().catch((err) => window.alert(err.message));
          }}
        >
          미리보기 생성
        </button>
      </section>

      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <h2 className="text-lg font-bold">가져오기 내역</h2>
        <div className="mt-3 space-y-2">
          {list.data?.length ? (
            list.data.map((item) => (
              <div key={item.id} className="rounded-lg border border-slate-200 p-3">
                <p className="font-medium">{item.original_filename || item.institution}</p>
                <p className="text-sm text-slate-600">
                  상태: {상태표시(item.status)} / 총 {item.parsed_count}건 / 중복 {item.duplicate_count}건 / 저장 {item.imported_count}건
                </p>
                <div className="mt-2 flex gap-2">
                  <button
                    className="rounded border border-teal-600 px-2 py-1 text-xs text-teal-700"
                    onClick={() => void 저장확정.mutateAsync(item.id)}
                  >
                    저장하기
                  </button>
                </div>
              </div>
            ))
          ) : (
            <p className="rounded-lg border border-dashed border-slate-300 p-4 text-sm text-slate-500">
              아직 가져오기 내역이 없습니다.
            </p>
          )}
        </div>
      </section>
    </div>
  );
}

function 상태표시(status: string) {
  switch (status) {
    case 'pending':
      return '대기';
    case 'parsed':
      return '미리보기 완료';
    case 'imported':
      return '저장 완료';
    case 'failed':
      return '실패';
    default:
      return '알 수 없음';
  }
}
