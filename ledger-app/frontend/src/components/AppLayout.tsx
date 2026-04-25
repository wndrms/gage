import { Link, Outlet, useLocation, useNavigate } from 'react-router-dom';
import clsx from 'clsx';
import { useAuth } from '@/hooks/useAuth';

const NAV = [
  { to: '/',             label: '홈',      icon: '🏠' },
  { to: '/transactions', label: '거래',    icon: '💳' },
  { to: '/calendar',     label: '캘린더',  icon: '📅' },
  { to: '/summary',      label: '월별',    icon: '📊' },
  { to: '/cards',        label: '카드',    icon: '🪪' },
  { to: '/imports',      label: '가져오기',icon: '📥' },
  { to: '/assets',       label: '자산',    icon: '💰' },
  { to: '/accounts',     label: '계좌',    icon: '🏦' },
  { to: '/categories',   label: '카테고리',icon: '🏷️' },
  { to: '/settings',     label: '설정',    icon: '⚙️' },
];

const BOTTOM_NAV = [
  { to: '/',             label: '홈',   icon: '🏠' },
  { to: '/transactions', label: '거래', icon: '💳' },
  { to: '/calendar',     label: '달력', icon: '📅' },
  { to: '/cards',        label: '카드', icon: '🪪' },
  { to: '/settings',     label: '설정', icon: '⚙️' },
];

function isActive(pathname: string, to: string) {
  if (to === '/') return pathname === '/';
  return pathname.startsWith(to);
}

export default function AppLayout() {
  const location = useLocation();
  const navigate = useNavigate();
  const auth = useAuth();

  return (
    <div className="min-h-screen bg-[#f6f7fb] pb-20 md:pb-0">
      {/* 상단 헤더 */}
      <header className="sticky top-0 z-30 border-b border-slate-200/70 bg-white/95 backdrop-blur-sm">
        <div className="mx-auto flex max-w-6xl items-center justify-between px-4 py-3">
          {/* 로고 */}
          <button
            onClick={() => navigate('/')}
            className="flex items-center gap-2 rounded-xl bg-gradient-to-r from-primary-700 to-primary-500 px-4 py-2 text-sm font-bold text-white shadow-soft transition-opacity hover:opacity-90"
          >
            <span>💰</span>
            <span>가계부</span>
          </button>

          {/* 데스크탑 네비 */}
          <nav className="hidden items-center gap-0.5 md:flex">
            {NAV.map((item) => (
              <Link
                key={item.to}
                to={item.to}
                className={clsx(
                  'rounded-lg px-3 py-1.5 text-sm font-medium transition-colors',
                  isActive(location.pathname, item.to)
                    ? 'bg-primary-50 text-primary-700 font-semibold'
                    : 'text-slate-500 hover:bg-slate-100 hover:text-slate-800'
                )}
              >
                {item.label}
              </Link>
            ))}
          </nav>

          {/* 사용자 */}
          <div className="flex items-center gap-2.5">
            <span className="hidden rounded-full bg-slate-100 px-3 py-1.5 text-xs font-medium text-slate-600 md:inline">
              {auth.user?.display_name}님
            </span>
            <button
              onClick={() => { void auth.로그아웃(); navigate('/login'); }}
              className="rounded-lg border border-slate-200 px-3 py-1.5 text-sm text-slate-600 transition-colors hover:border-slate-300 hover:bg-slate-50"
            >
              로그아웃
            </button>
          </div>
        </div>
      </header>

      {/* 콘텐츠 */}
      <main className="mx-auto max-w-6xl px-4 py-6">
        <Outlet />
      </main>

      {/* 모바일 하단 네비 */}
      <nav className="fixed bottom-0 left-0 right-0 z-30 border-t border-slate-200 bg-white/95 backdrop-blur-sm md:hidden">
        <div className="grid grid-cols-5">
          {BOTTOM_NAV.map((item) => {
            const active = isActive(location.pathname, item.to);
            return (
              <Link
                key={item.to}
                to={item.to}
                className={clsx(
                  'flex flex-col items-center gap-0.5 py-2.5 text-[10px] font-medium transition-colors',
                  active ? 'text-primary-700' : 'text-slate-400'
                )}
              >
                <span className="text-lg leading-none">{item.icon}</span>
                <span>{item.label}</span>
              </Link>
            );
          })}
        </div>
      </nav>
    </div>
  );
}
