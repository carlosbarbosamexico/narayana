import { create } from 'zustand'

interface AuthState {
  isAuthenticated: boolean
  token: string | null
  login: (username: string, password: string) => Promise<boolean>
  logout: () => void
  initialize: () => void
}

const AUTH_STORAGE_KEY = 'narayana-auth'

export const useAuthStore = create<AuthState>((set) => ({
  isAuthenticated: false,
  token: null,
  initialize: () => {
    // Load auth state from localStorage on initialization
    if (typeof window !== 'undefined') {
      const stored = localStorage.getItem(AUTH_STORAGE_KEY)
      if (stored) {
        try {
          const auth = JSON.parse(stored)
          // SECURITY: Validate structure and prevent prototype pollution
          if (
            auth &&
            typeof auth === 'object' &&
            !Array.isArray(auth) &&
            !auth.hasOwnProperty('__proto__') &&
            !auth.hasOwnProperty('constructor') &&
            auth.isAuthenticated === true &&
            typeof auth.token === 'string' &&
            auth.token.length > 0
          ) {
            set({ isAuthenticated: true, token: auth.token })
          } else {
            // Invalid or malicious data, clear it
            localStorage.removeItem(AUTH_STORAGE_KEY)
          }
        } catch (e) {
          // Invalid stored data, clear it
          localStorage.removeItem(AUTH_STORAGE_KEY)
        }
      }
    }
  },
  login: async (username: string, password: string) => {
    try {
      const response = await fetch('/api/v1/auth/login', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ username, password }),
      })

      if (response.ok) {
        const data = await response.json()
        const authState = {
          isAuthenticated: true,
          token: data.token || 'authenticated',
        }
        set(authState)
        // Persist to localStorage
        if (typeof window !== 'undefined') {
          localStorage.setItem(AUTH_STORAGE_KEY, JSON.stringify(authState))
        }
        return true
      }
      return false
    } catch (error) {
      console.error('Login error:', error)
      return false
    }
  },
  logout: () => {
    set({
      isAuthenticated: false,
      token: null,
    })
    // Clear localStorage
    if (typeof window !== 'undefined') {
      localStorage.removeItem(AUTH_STORAGE_KEY)
    }
  },
}))

