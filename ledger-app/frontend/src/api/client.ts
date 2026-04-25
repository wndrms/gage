export class ApiError extends Error {
  status: number;

  constructor(message: string, status: number) {
    super(message);
    this.status = status;
  }
}

export async function api<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    credentials: 'include',
    headers: {
      'Content-Type': 'application/json',
      ...(init?.headers || {})
    },
    ...init
  });

  if (response.status === 204) {
    return null as T;
  }

  const contentType = response.headers.get('content-type') || '';
  const body = contentType.includes('application/json') ? await response.json() : null;

  if (!response.ok) {
    const message = body?.message || '요청 처리 중 오류가 발생했습니다';
    throw new ApiError(message, response.status);
  }

  return body as T;
}
