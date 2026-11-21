import { Outlet, Route, Routes } from 'react-router';
import { api } from './lib/api';
import type { Me } from './lib/types';
import { useApi } from './lib/useApi';
import { Sidebar } from './components/Sidebar';
import { Home } from './pages/Home';
import { Insights } from './pages/Insights';
import { Persons } from './pages/Persons';
import { PersonDetail } from './pages/PersonDetail';

function Layout({ projectName }: { projectName?: string }) {
  return (
    <div className="shell">
      <Sidebar projectName={projectName} />
      <main className="main"><Outlet /></main>
    </div>
  );
}

export function App() {
  // Loading the session also bounces signed-out users to the login page.
  const me = useApi('me', () => api<Me>('/api/v2/me'));
  const name = me.data?.project_name;
  return (
    <Routes>
      <Route element={<Layout projectName={name} />}>
        <Route index element={<Home projectName={name} />} />
        <Route path="insights" element={<Insights />} />
        <Route path="insights/:kind" element={<Insights />} />
        <Route path="persons" element={<Persons />} />
        <Route path="persons/:id" element={<PersonDetail />} />
        <Route path="*" element={<div className="empty">Page not found.</div>} />
      </Route>
    </Routes>
  );
}
