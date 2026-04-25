import { Navigate } from 'react-router-dom';
import { useAuth } from '@/hooks/useAuth';

export function ProtectedRoute({ children }: { children: React.ReactNode }) {
  const auth = useAuth();

  if (auth.로딩중) {
    return (
      <div className="flex min-h-screen items-center justify-center text-slate-600">
        로그인 상태를 확인하는 중입니다...
      </div>
    );
  }

  if (!auth.user) {
    return <Navigate to="/login" replace />;
  }

  return <>{children}</>;
}
