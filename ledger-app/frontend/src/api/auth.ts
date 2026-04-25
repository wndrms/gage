import { api } from './client';
import type { User } from '@/types';

export const authApi = {
  me: () => api<User>('/api/auth/me'),
  login: (password: string) =>
    api<User>('/api/auth/login', {
      method: 'POST',
      body: JSON.stringify({ password })
    }),
  logout: () =>
    api<{ message: string }>('/api/auth/logout', {
      method: 'POST'
    })
};
