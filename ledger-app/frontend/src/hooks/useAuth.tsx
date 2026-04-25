import { createContext, useContext, useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { authApi } from '@/api/auth';
import type { User } from '@/types';

type AuthContextValue = {
  user: User | null;
  로딩중: boolean;
  로그인: (password: string) => Promise<void>;
  로그아웃: () => Promise<void>;
};

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const client = useQueryClient();
  const [로딩중, set로딩중] = useState(false);

  const meQuery = useQuery({
    queryKey: ['auth', 'me'],
    queryFn: authApi.me,
    retry: false
  });

  const value = useMemo<AuthContextValue>(() => {
    return {
      user: meQuery.data ?? null,
      로딩중: meQuery.isLoading || 로딩중,
      로그인: async (password: string) => {
        set로딩중(true);
        try {
          await authApi.login(password);
          await client.invalidateQueries({ queryKey: ['auth', 'me'] });
        } finally {
          set로딩중(false);
        }
      },
      로그아웃: async () => {
        set로딩중(true);
        try {
          await authApi.logout();
          await client.invalidateQueries({ queryKey: ['auth', 'me'] });
          client.clear();
        } finally {
          set로딩중(false);
        }
      }
    };
  }, [client, meQuery.data, meQuery.isLoading, 로딩중]);

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('인증 컨텍스트가 설정되지 않았습니다');
  }
  return context;
}
