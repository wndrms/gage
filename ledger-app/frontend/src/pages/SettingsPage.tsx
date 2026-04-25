import { useState } from 'react';
import { useMutation } from '@tanstack/react-query';
import { api } from '@/api/client';
import CategoryRulesSection from '@/components/CategoryRulesSection';

function ChangePasswordSection() {
  const [form, setForm] = useState({ current_password: '', new_password: '', confirm_password: '' });
  const [error, setError] = useState('');
  const [success, setSuccess] = useState('');

  const mutation = useMutation({
    mutationFn: () =>
      api<{ message: string }>('/api/auth/change-password', {
        method: 'POST',
        body: JSON.stringify({
          current_password: form.current_password,
          new_password: form.new_password,
        }),
      }),
    onSuccess: (data) => {
      setSuccess(data.message);
      setError('');
      setForm({ current_password: '', new_password: '', confirm_password: '' });
    },
    onError: (err: Error) => {
      setError(err.message);
      setSuccess('');
    },
  });

  const handleSubmit = () => {
    setError('');
    setSuccess('');
    if (!form.current_password || !form.new_password) {
      setError('모든 항목을 입력해 주세요.');
      return;
    }
    if (form.new_password.length < 8) {
      setError('새 비밀번호는 8자 이상이어야 합니다.');
      return;
    }
    if (form.new_password !== form.confirm_password) {
      setError('새 비밀번호 확인이 일치하지 않습니다.');
      return;
    }
    void mutation.mutateAsync();
  };

  return (
    <section className="rounded-2xl bg-white p-4 shadow-soft">
      <h2 className="text-lg font-bold">비밀번호 변경</h2>
      <div className="mt-3 space-y-3">
        <label className="block text-sm">
          <span className="mb-1 block">현재 비밀번호</span>
          <input
            type="password"
            className="w-full rounded-lg border border-slate-300 px-3 py-2"
            value={form.current_password}
            onChange={(e) => setForm((prev) => ({ ...prev, current_password: e.target.value }))}
          />
        </label>
        <label className="block text-sm">
          <span className="mb-1 block">새 비밀번호</span>
          <input
            type="password"
            className="w-full rounded-lg border border-slate-300 px-3 py-2"
            value={form.new_password}
            onChange={(e) => setForm((prev) => ({ ...prev, new_password: e.target.value }))}
          />
        </label>
        <label className="block text-sm">
          <span className="mb-1 block">새 비밀번호 확인</span>
          <input
            type="password"
            className="w-full rounded-lg border border-slate-300 px-3 py-2"
            value={form.confirm_password}
            onChange={(e) => setForm((prev) => ({ ...prev, confirm_password: e.target.value }))}
          />
        </label>
      </div>
      {error ? <p className="mt-2 text-sm text-rose-600">{error}</p> : null}
      {success ? <p className="mt-2 text-sm text-emerald-600">{success}</p> : null}
      <button
        className="mt-3 rounded-lg bg-teal-700 px-4 py-2 font-semibold text-white disabled:opacity-50"
        disabled={mutation.isPending}
        onClick={handleSubmit}
      >
        {mutation.isPending ? '변경 중...' : '비밀번호 변경'}
      </button>
    </section>
  );
}

async function downloadFile(url: string, defaultFilename: string) {
  const res = await fetch(url, { credentials: 'include' });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ message: '다운로드에 실패했습니다.' }));
    throw new Error((err as { message?: string }).message ?? '다운로드에 실패했습니다.');
  }
  const disposition = res.headers.get('content-disposition') ?? '';
  const match = disposition.match(/filename="([^"]+)"/);
  const filename = match?.[1] ?? defaultFilename;
  const blob = await res.blob();
  const href = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = href;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(href);
}

function ExportSection() {
  const [error, setError] = useState('');
  const [loading, setLoading] = useState<'csv' | 'json' | null>(null);

  const handleDownload = async (type: 'csv' | 'json') => {
    setError('');
    setLoading(type);
    try {
      if (type === 'csv') {
        await downloadFile('/api/export/transactions.csv', 'transactions.csv');
      } else {
        await downloadFile('/api/export/backup.json', 'backup.json');
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : '다운로드에 실패했습니다.');
    } finally {
      setLoading(null);
    }
  };

  return (
    <section className="rounded-2xl bg-white p-4 shadow-soft">
      <h2 className="text-lg font-bold">데이터 내보내기</h2>
      <p className="mt-1 text-sm text-slate-500">거래 내역을 파일로 다운로드합니다.</p>
      {error ? <p className="mt-2 text-sm text-rose-600">{error}</p> : null}
      <div className="mt-3 flex flex-wrap gap-3">
        <button
          className="rounded-lg border border-teal-600 px-4 py-2 text-sm font-semibold text-teal-700 hover:bg-teal-50 disabled:opacity-50"
          disabled={loading !== null}
          onClick={() => void handleDownload('csv')}
        >
          {loading === 'csv' ? '준비 중...' : 'CSV 내보내기 (Excel)'}
        </button>
        <button
          className="rounded-lg border border-slate-400 px-4 py-2 text-sm font-semibold text-slate-700 hover:bg-slate-50 disabled:opacity-50"
          disabled={loading !== null}
          onClick={() => void handleDownload('json')}
        >
          {loading === 'json' ? '준비 중...' : 'JSON 백업 다운로드'}
        </button>
      </div>
    </section>
  );
}

function TelegramSection() {
  const [botToken, setBotToken] = useState('');
  const [webhookUrl, setWebhookUrl] = useState('');
  const [result, setResult] = useState<{ ok: boolean; description: string } | null>(null);

  const mutation = useMutation({
    mutationFn: () =>
      api<{ ok: boolean; description: string }>('/api/telegram/setup', {
        method: 'POST',
        body: JSON.stringify({ bot_token: botToken, webhook_url: webhookUrl }),
      }),
    onSuccess: (data) => setResult(data),
    onError: (err: Error) => setResult({ ok: false, description: err.message }),
  });

  return (
    <section className="rounded-2xl bg-white p-4 shadow-soft">
      <h2 className="text-lg font-bold">텔레그램 웹훅 설정</h2>
      <p className="mt-1 text-sm text-slate-500">
        봇 토큰과 웹훅 URL을 입력해 저장합니다. 서버에 <code className="rounded bg-slate-100 px-1 text-xs">TELEGRAM_BOT_TOKEN</code> 환경변수를 설정한 뒤 아래 버튼으로 Telegram에 웹훅을 등록하세요.
      </p>
      <div className="mt-3 space-y-3">
        <label className="block text-sm">
          <span className="mb-1 block">봇 토큰</span>
          <input
            className="w-full rounded-lg border border-slate-300 px-3 py-2 font-mono text-xs"
            placeholder="123456789:ABCdefGHI..."
            value={botToken}
            onChange={(e) => setBotToken(e.target.value)}
          />
        </label>
        <label className="block text-sm">
          <span className="mb-1 block">웹훅 URL</span>
          <input
            className="w-full rounded-lg border border-slate-300 px-3 py-2"
            placeholder="https://yourdomain.com/api/telegram/webhook"
            value={webhookUrl}
            onChange={(e) => setWebhookUrl(e.target.value)}
          />
        </label>
      </div>
      {result ? (
        <p className={`mt-2 text-sm ${result.ok ? 'text-emerald-600' : 'text-rose-600'}`}>
          {result.ok ? '웹훅 등록 성공: ' : '등록 실패: '}
          {result.description}
        </p>
      ) : null}
      <button
        className="mt-3 rounded-lg bg-teal-700 px-4 py-2 font-semibold text-white disabled:opacity-50"
        disabled={mutation.isPending}
        onClick={() => void mutation.mutateAsync()}
      >
        {mutation.isPending ? '등록 중...' : '웹훅 등록'}
      </button>
    </section>
  );
}

export default function SettingsPage() {
  return (
    <div className="space-y-4">
      <section className="rounded-2xl bg-white p-4 shadow-soft">
        <h1 className="text-xl font-bold">설정</h1>
      </section>
      <CategoryRulesSection />
      <ChangePasswordSection />
      <ExportSection />
      <TelegramSection />
    </div>
  );
}
