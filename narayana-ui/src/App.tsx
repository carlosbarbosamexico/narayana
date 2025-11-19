import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import Layout from './components/Layout'
import ProtectedRoute from './components/ProtectedRoute'
import SetupGuard from './components/SetupGuard'
import Login from './pages/Login'
import Signup from './pages/Signup'
import Dashboard from './pages/Dashboard'
import Tables from './pages/Tables'
import Brains from './pages/Brains'
import Workers from './pages/Workers'
import Webhooks from './pages/Webhooks'
import Query from './pages/Query'
import Performance from './pages/Performance'
import Settings from './pages/Settings'

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false,
      retry: 1,
      staleTime: 5000,
    },
  },
})

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <SetupGuard>
          <Routes>
            <Route path="/signup" element={<Signup />} />
            <Route path="/login" element={<Login />} />
            <Route
              path="/*"
              element={
                <ProtectedRoute>
                  <Layout>
                    <Routes>
                      <Route path="/" element={<Dashboard />} />
                      <Route path="/tables" element={<Tables />} />
                      <Route path="/brains" element={<Brains />} />
                      <Route path="/workers" element={<Workers />} />
                      <Route path="/webhooks" element={<Webhooks />} />
                      <Route path="/query" element={<Query />} />
                      <Route path="/performance" element={<Performance />} />
                      <Route path="/settings" element={<Settings />} />
                      <Route path="*" element={<Navigate to="/" replace />} />
                    </Routes>
                  </Layout>
                </ProtectedRoute>
              }
            />
          </Routes>
        </SetupGuard>
      </BrowserRouter>
    </QueryClientProvider>
  )
}

export default App

