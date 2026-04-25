import { useState } from 'react';
import { Navigate } from 'react-router-dom';
import { useAuth } from '@/hooks/useAuth';

export default function LoginPage() {
  const auth = useAuth();
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');

  if (auth.user) {
    return <Navigate to="/" replace />;
  }

  return (
    <div className="flex min-h-screen items-center justify-center px-4">
      <div className="w-full max-w-md rounded-2xl bg-white p-6 shadow-soft">
        <h1 className="text-2xl font-bold text-slate-900">가계부 로그인</h1>
        <p className="mt-2 text-sm text-slate-600">관리자 비밀번호를 입력해 주세요.</p>

        <form
          className="mt-6 space-y-4"
          onSubmit={async (event) => {
            event.preventDefault();
            setError('');
            if (!password.trim()) {
              setError('비밀번호를 입력해 주세요.');
              return;
            }
            try {
              await auth.로그인(password);
            } catch (e) {
              setError(e instanceof Error ? e.message : '로그인에 실패했습니다.');
            }
          }}
        >
          <div>
            <label className="mb-1 block text-sm font-medium text-slate-700">관리자 비밀번호</label>
            <input
              type="password"
              className="w-full rounded-lg border border-slate-300 px-3 py-2"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="비밀번호 입력"
            />
          </div>

          {error ? <p className="text-sm text-rose-600">{error}</p> : null}

          <button
            type="submit"
            className="w-full rounded-lg bg-teal-700 px-3 py-2 font-semibold text-white"
            disabled={auth.로딩중}
          >
            로그인
          </button>
        </form>
      </div>
    </div>
  );
}
