import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { apiClient } from '../lib/api'
import Signup from '../pages/Signup'

interface SetupGuardProps {
  children: React.ReactNode
}

export default function SetupGuard({ children }: SetupGuardProps) {
  const [checking, setChecking] = useState(true)
  const [setupRequired, setSetupRequired] = useState(false)
  const navigate = useNavigate()

  useEffect(() => {
    let cancelled = false
    
    const checkSetup = async () => {
      try {
        const result = await apiClient.checkSetup()
        if (cancelled) return
        
        setSetupRequired(result.setup_required)
        if (result.setup_required) {
          // If we're not already on signup, redirect to signup
          // This ensures signup form is shown even when on /login
          const currentPath = window.location.pathname
          if (currentPath !== '/signup') {
            navigate('/signup', { replace: true })
          }
        }
      } catch (error) {
        if (cancelled) return
        console.error('Failed to check setup status:', error)
        // On error, check if we're on signup/login - if so, assume setup might be required
        // This handles cases where the server isn't running yet
        const currentPath = window.location.pathname
        if (currentPath === '/signup' || currentPath === '/login') {
          // If we're on signup/login and API fails, show signup form as fallback
          setSetupRequired(true)
          if (currentPath === '/login') {
            navigate('/signup', { replace: true })
          }
        } else {
          // Otherwise, assume setup is not required to allow normal flow
          setSetupRequired(false)
        }
      } finally {
        if (!cancelled) {
          setChecking(false)
        }
      }
    }

    checkSetup()
    
    return () => {
      cancelled = true
    }
  }, [navigate])

  if (checking) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600 mx-auto"></div>
          <p className="mt-4 text-gray-600">Checking setup status...</p>
        </div>
      </div>
    )
  }

  if (setupRequired) {
    return <Signup />
  }

  return <>{children}</>
}

