export const money = (value: number) =>
  new Intl.NumberFormat('ko-KR', {
    style: 'currency',
    currency: 'KRW',
    maximumFractionDigits: 0
  }).format(value);

export const dateTime = (value: string) =>
  new Date(value).toLocaleString('ko-KR', {
    timeZone: 'Asia/Seoul',
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit'
  });

export const dateOnly = (value: string) =>
  new Date(value).toLocaleDateString('ko-KR', {
    timeZone: 'Asia/Seoul',
    year: 'numeric',
    month: '2-digit',
    day: '2-digit'
  });

export const currentMonthStr = (): string => {
  const now = new Date();
  return `${now.getFullYear()}-${`${now.getMonth() + 1}`.padStart(2, '0')}`;
};

export const currentMonth = currentMonthStr();

export const today = () => {
  const now = new Date();
  return `${now.getFullYear()}-${`${now.getMonth() + 1}`.padStart(2, '0')}-${`${now.getDate()}`.padStart(2, '0')}`;
};
