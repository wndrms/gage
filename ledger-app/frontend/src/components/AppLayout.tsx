import { Link, Outlet, useLocation, useNavigate } from 'react-router-dom';
import clsx from 'clsx';
import { useAuth } from '@/hooks/useAuth';

const NAV = [
  { to: '/', label: '홈' },
  { to: '/transactions', label: '거래' },
  { to: '/calendar', label: '캘린더' },
  { to: '/summary', label: '월결산' },
  { to: '/cards', label: '카드' },
  { to: '/imports', label: '가져오기' },
  { to: '/kream-sales', label: 'KREAM', adminOnly: true },
  { to: '/assets', label: '자산' },
  { to: '/accounts', label: '계좌' },
  { to: '/categories', label: '카테고리' },
  { to: '/settings', label: '설정' }
];

const BOTTOM_NAV = [
  { to: '/', label: '홈' },
  { to: '/transactions', label: '거래' },
  { to: '/calendar', label: '달력' },
  { to: '/cards', label: '카드' },
  { to: '/settings', label: '설정' }
];

function isActive(pathname: string, to: string) {
  if (to === '/') return pathname === '/';
  return pathname.startsWith(to);
}

export default function AppLayout() {
  const location = useLocation();
  const navigate = useNavigate();
  const auth = useAuth();
  const visibleNav = NAV.filter((item) => !item.adminOnly || auth.user?.role === 'admin');

  return (
    <div className="min-h-screen bg-[#f6f7fb] pb-20 md:pb-0">
      <header className="sticky top-0 z-30 border-b border-slate-200/70 bg-white/95 backdrop-blur-sm">
        <div className="mx-auto flex max-w-6xl items-center justify-between px-4 py-3">
          <button
            onClick={() => navigate('/')}
            className="flex items-center gap-2 rounded-xl bg-gradient-to-r from-primary-700 to-primary-500 px-4 py-2 text-sm font-bold text-white shadow-soft transition-opacity hover:opacity-90"
          >
            <span>가계부</span>
          </button>

          <nav className="hidden items-center gap-0.5 md:flex">
            {visibleNav.map((item) => (
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

          <div className="flex items-center gap-2.5">
            <span className="hidden rounded-full bg-slate-100 px-3 py-1.5 text-xs font-medium text-slate-600 md:inline">
              {auth.user?.display_name}님
            </span>
            <button
              onClick={() => {
                void auth.로그아웃();
                navigate('/login');
              }}
              className="rounded-lg border border-slate-200 px-3 py-1.5 text-sm text-slate-600 transition-colors hover:border-slate-300 hover:bg-slate-50"
            >
              로그아웃
            </button>
          </div>
        </div>
      </header>

      <main className="mx-auto max-w-6xl px-4 py-6">
        <Outlet />
      </main>

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
                <span>{item.label}</span>
              </Link>
            );
          })}
        </div>
      </nav>
    </div>
  );
}
