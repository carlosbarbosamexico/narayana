import axios from 'axios'

const api = axios.create({
  baseURL: '/api/v1',
  headers: {
    'Content-Type': 'application/json',
  },
})

// Add authentication interceptor to include JWT token in all requests
api.interceptors.request.use(
  (config) => {
    // Get token from localStorage
    if (typeof window !== 'undefined') {
      const authStorage = localStorage.getItem('narayana-auth')
      if (authStorage) {
        try {
          const auth = JSON.parse(authStorage)
          if (auth?.token && typeof auth.token === 'string') {
            // Add Bearer token to Authorization header
            config.headers.Authorization = `Bearer ${auth.token}`
          }
        } catch (e) {
          // Invalid auth data, ignore
        }
      }
    }
    return config
  },
  (error) => {
    return Promise.reject(error)
  }
)

// Handle 401 errors (unauthorized) - redirect to login
api.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error.response?.status === 401) {
      // Clear auth and redirect to login
      if (typeof window !== 'undefined') {
        localStorage.removeItem('narayana-auth')
        // Only redirect if not already on login page
        if (!window.location.pathname.includes('/login')) {
          window.location.href = '/login'
        }
      }
    }
    return Promise.reject(error)
  }
)

export interface Table {
  id: number
  name: string
  schema: any
  row_count?: number
}

export interface Metric {
  name: string
  value: number
  timestamp: number
}

export interface QueryStats {
  total_queries: number
  avg_duration_ms: number
  total_rows_read: number
  total_rows_inserted: number
}

export interface Brain {
  brain_id: string
  memory_types: string[]
  created_at?: number
}

export interface Worker {
  worker_id: string
  name: string
  route: string
  active: boolean
  created_at?: number
}

export interface SystemStats {
  tables: number
  brains: number
  workers: number
  active_connections: number
  total_queries: number
  avg_latency_ms: number
  total_rows_read?: number
  total_rows_inserted?: number
}

export const apiClient = {
  // Health check
  health: async () => {
    const response = await api.get('/health')
    return response.data
  },

  // Tables
  getTables: async (): Promise<Table[]> => {
    const response = await api.get('/tables')
    return response.data.tables || []
  },

  createTable: async (name: string, schema: any) => {
    const response = await api.post('/tables', { table_name: name, schema })
    return response.data
  },

  deleteTable: async (tableId: number) => {
    const response = await api.delete(`/tables/${tableId}`)
    return response.data
  },

  // Data operations
  insertData: async (tableId: number, columns: any[]) => {
    const response = await api.post(`/tables/${tableId}/insert`, { columns })
    return response.data
  },

  queryData: async (tableId: number, params: { columns?: string; limit?: number }) => {
    const response = await api.get(`/tables/${tableId}/query`, { params })
    return response.data
  },

  // Metrics
  getMetrics: async (): Promise<string> => {
    const response = await axios.get('/metrics')
    return response.data
  },

  getStats: async (): Promise<QueryStats> => {
    const response = await api.get('/stats')
    return response.data
  },

  // Brains
  getBrains: async (): Promise<Brain[]> => {
    const response = await api.get('/brains')
    return response.data.brains || []
  },

  createBrain: async (brainId: string, memoryTypes?: string[]) => {
    const response = await api.post('/brains', {
      brain_id: brainId,
      memory_types: memoryTypes || [],
    })
    return response.data
  },

  // Brain details
  getThoughts: async (brainId: string, state?: string) => {
    const params = state ? { state } : {}
    const response = await api.get(`/brains/${brainId}/thoughts/list`, { params })
    return response.data
  },

  getMemories: async (brainId: string, type: string = 'episodic', limit: number = 100) => {
    const response = await api.get(`/brains/${brainId}/memories`, {
      params: { type, limit },
    })
    return response.data
  },

  getThoughtTimeline: async (brainId: string) => {
    const response = await api.get(`/brains/${brainId}/thought-timeline`)
    return response.data
  },

  getConflicts: async (brainId: string) => {
    const response = await api.get(`/brains/${brainId}/conflicts`)
    return response.data
  },

  getMemoryAccesses: async (brainId: string) => {
    const response = await api.get(`/brains/${brainId}/memory-accesses`)
    return response.data
  },

  // CPL Management
  getCPLs: async () => {
    const response = await api.get('/cpls')
    // API returns { cpls: [], count: 0 }, so return the full response data
    return response.data || { cpls: [], count: 0 }
  },

  createCPL: async (config: any) => {
    const response = await api.post('/cpls', config)
    return response.data
  },

  getCPL: async (cplId: string) => {
    const response = await api.get(`/cpls/${cplId}`)
    return response.data
  },

  startCPL: async (cplId: string) => {
    const response = await api.post(`/cpls/${cplId}/start`)
    return response.data
  },

  stopCPL: async (cplId: string) => {
    const response = await api.post(`/cpls/${cplId}/stop`)
    return response.data
  },

  deleteCPL: async (cplId: string) => {
    const response = await api.delete(`/cpls/${cplId}`)
    return response.data
  },

  // Workers
  getWorkers: async (): Promise<Worker[]> => {
    const response = await api.get('/workers')
    return response.data.workers || []
  },

  // System stats
  getSystemStats: async (): Promise<SystemStats> => {
    const response = await api.get('/system/stats')
    return response.data
  },

  // Webhooks
  getWebhooks: async (): Promise<Webhook[]> => {
    const response = await api.get('/webhooks')
    return response.data.webhooks || []
  },

  getWebhook: async (id: string): Promise<Webhook> => {
    const response = await api.get(`/webhooks/${id}`)
    return response.data
  },

  createWebhook: async (webhook: CreateWebhookRequest) => {
    const response = await api.post('/webhooks', webhook)
    return response.data
  },

  deleteWebhook: async (id: string) => {
    const response = await api.delete(`/webhooks/${id}`)
    return response.data
  },

  enableWebhook: async (id: string) => {
    const response = await api.post(`/webhooks/${id}/enable`)
    return response.data
  },

  disableWebhook: async (id: string) => {
    const response = await api.post(`/webhooks/${id}/disable`)
    return response.data
  },

  getWebhookDeliveries: async (id: string, limit?: number): Promise<WebhookDelivery[]> => {
    const response = await api.get(`/webhooks/${id}/deliveries`, {
      params: { limit: limit || 50 },
    })
    return response.data.deliveries || []
  },

  // Setup
  checkSetup: async (): Promise<{ setup_required: boolean; message: string }> => {
    const response = await api.get('/auth/setup/check')
    return response.data
  },

  setup: async (name: string, username: string, password: string) => {
    const response = await api.post('/auth/setup', { name, username, password })
    return response.data
  },
}

export interface Webhook {
  id: string
  name: string
  url: string
  enabled: boolean
  events: string[]
  scope: string
  retry_count: number
  created_at: number
  updated_at: number
  total_deliveries: number
  successful_deliveries: number
  failed_deliveries: number
}

export interface CreateWebhookRequest {
  name: string
  url: string
  events: string[]
  scope: string
  secret?: string
  retry_count?: number
}

export interface WebhookDelivery {
  id: string
  webhook_id: string
  status: 'pending' | 'processing' | 'success' | 'failed'
  attempt: number
  max_attempts: number
  created_at: number
  completed_at?: number
  error?: string
  response_status?: number
  duration_ms?: number
}

export default api
