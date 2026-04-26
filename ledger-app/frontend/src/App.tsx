import { Navigate, Route, Routes } from 'react-router-dom';
import { AuthProvider } from '@/hooks/useAuth';
import { ProtectedRoute } from '@/components/ProtectedRoute';
import AppLayout from '@/components/AppLayout';
import LoginPage from '@/pages/LoginPage';
import DashboardPage from '@/pages/DashboardPage';
import TransactionsPage from '@/pages/TransactionsPage';
import CalendarPage from '@/pages/CalendarPage';
import MonthlySummaryPage from '@/pages/MonthlySummaryPage';
import CardsPage from '@/pages/CardsPage';
import CardDetailPage from '@/pages/CardDetailPage';
import CardPresetsPage from '@/pages/CardPresetsPage';
import ImportsPage from '@/pages/ImportsPage';
import KreamSalesPage from '@/pages/KreamSalesPage';
import AssetsPage from '@/pages/AssetsPage';
import SettingsPage from '@/pages/SettingsPage';
import AccountsPage from '@/pages/AccountsPage';
import CategoriesPage from '@/pages/CategoriesPage';

export default function App() {
  return (
    <AuthProvider>
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route
          path="/"
          element={
            <ProtectedRoute>
              <AppLayout />
            </ProtectedRoute>
          }
        >
          <Route index element={<DashboardPage />} />
          <Route path="transactions" element={<TransactionsPage />} />
          <Route path="calendar" element={<CalendarPage />} />
          <Route path="summary" element={<MonthlySummaryPage />} />
          <Route path="cards" element={<CardsPage />} />
          <Route path="cards/presets" element={<CardPresetsPage />} />
          <Route path="cards/:id" element={<CardDetailPage />} />
          <Route path="imports" element={<ImportsPage />} />
          <Route path="kream-sales" element={<KreamSalesPage />} />
          <Route path="assets" element={<AssetsPage />} />
          <Route path="settings" element={<SettingsPage />} />
          <Route path="accounts" element={<AccountsPage />} />
          <Route path="categories" element={<CategoriesPage />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Route>
      </Routes>
    </AuthProvider>
  );
}
