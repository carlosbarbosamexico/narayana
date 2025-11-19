import { useEffect, useState, useCallback } from 'react'
import { useQuery } from '@tanstack/react-query'
import { apiClient } from '../lib/api'
import { useWebSocket } from '../hooks/useWebSocket'
import { Activity, Database, Zap, Brain as BrainIcon, Code, Wifi, WifiOff } from 'lucide-react'
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, BarChart, Bar } from 'recharts'

interface RealTimeMetric {
  time: string
  queries: number
  latency: number
  timestamp: number
}

interface SystemStats {
  total_queries: number
  avg_latency_ms: number
  total_rows_read: number
  total_rows_inserted: number
  tables: number
  brains: number
  workers: number
  active_connections: number
}

export default function Dashboard() {
  const [realTimeMetrics, setRealTimeMetrics] = useState<RealTimeMetric[]>([])
  const [lastUpdate, setLastUpdate] = useState<Date>(new Date())
  const [systemStats, setSystemStats] = useState<SystemStats | null>(null)
  const [tables, setTables] = useState<any[]>([])
  const [brains, setBrains] = useState<any[]>([])
  const [workers, setWorkers] = useState<any[]>([])

  // Initial data fetch (only once)
  const { data: initialStats } = useQuery({
    queryKey: ['stats'],
    queryFn: apiClient.getStats,
    refetchInterval: false, // No polling!
  })

  const { data: initialTables } = useQuery({
    queryKey: ['tables'],
    queryFn: apiClient.getTables,
    refetchInterval: false, // No polling!
  })

  const { data: initialBrains } = useQuery({
    queryKey: ['brains'],
    queryFn: apiClient.getBrains,
    refetchInterval: false, // No polling!
  })

  const { data: initialWorkers } = useQuery({
    queryKey: ['workers'],
    queryFn: apiClient.getWorkers,
    refetchInterval: false, // No polling!
  })

  // Initialize state from initial fetch
  useEffect(() => {
    if (initialStats) {
      setSystemStats({
        total_queries: initialStats.total_queries || 0,
        avg_latency_ms: initialStats.avg_duration_ms || 0,
        total_rows_read: initialStats.total_rows_read || 0,
        total_rows_inserted: initialStats.total_rows_inserted || 0,
        tables: initialTables?.length || 0,
        brains: initialBrains?.length || 0,
        workers: initialWorkers?.length || 0,
        active_connections: 0,
      })
    }
    if (initialTables) setTables(initialTables)
    if (initialBrains) setBrains(initialBrains)
    if (initialWorkers) setWorkers(initialWorkers)
  }, [initialStats, initialTables, initialBrains, initialWorkers])

  // Handle WebSocket messages
  const handleMessage = useCallback((message: any) => {
    if (message.type === 'event' && message.event) {
      const event = message.event
      
      // Handle query events
      if (event.type === 'query' || message.channel === 'system:queries') {
        const now = new Date()
        const timeStr = now.toLocaleTimeString()
        
        setRealTimeMetrics((prev) => {
          const updated = [
            ...prev,
            {
              time: timeStr,
              queries: 1,
              latency: event.duration_ms || event.latency || 0,
              timestamp: now.getTime(),
            },
          ]
          // Keep only last 50 data points
          return updated.slice(-50)
        })
        setLastUpdate(now)
      }
      
      // Handle stats updates
      if (event.type === 'stats_update' || message.channel === 'system:stats') {
        if (event.data) {
          setSystemStats((prev) => ({
            ...prev,
            ...event.data,
          }))
        } else {
          // Refetch stats if event doesn't contain data
          apiClient.getSystemStats().then((stats) => {
            if (stats) {
              setSystemStats({
                total_queries: stats.total_queries || 0,
                avg_latency_ms: stats.avg_latency_ms || 0,
                total_rows_read: stats.total_rows_read || 0,
                total_rows_inserted: stats.total_rows_inserted || 0,
                tables: stats.tables || 0,
                brains: stats.brains || 0,
                workers: stats.workers || 0,
                active_connections: stats.active_connections || 0,
              })
            }
          })
        }
        setLastUpdate(new Date())
      }
      
      // Handle tables updates
      if (message.channel === 'system:tables' || event.type === 'table_created' || event.type === 'table_deleted') {
        apiClient.getTables().then((tablesData) => {
          if (tablesData) setTables(tablesData)
        })
        setLastUpdate(new Date())
      }
      
      // Handle brains updates
      if (message.channel === 'system:brains' || event.type === 'brain_created' || event.type === 'brain_deleted') {
        apiClient.getBrains().then((brainsData) => {
          if (brainsData) setBrains(brainsData)
        })
        setLastUpdate(new Date())
      }
      
      // Handle workers updates
      if (message.channel === 'system:workers' || event.type === 'worker_created' || event.type === 'worker_deleted') {
        apiClient.getWorkers().then((workersData) => {
          if (workersData) setWorkers(workersData)
        })
        setLastUpdate(new Date())
      }
    }
  }, [])

  // WebSocket connection
  const { isConnected, subscribe } = useWebSocket({
    url: 'ws://localhost:8080/ws',
    onMessage: handleMessage,
  })

  // Subscribe to channels
  useEffect(() => {
    if (isConnected) {
      subscribe('system:stats')
      subscribe('system:queries')
      subscribe('system:tables')
      subscribe('system:brains')
      subscribe('system:workers')
    }
  }, [isConnected, subscribe])

  const cardStats = [
    {
      name: 'Total Queries',
      value: systemStats?.total_queries || initialStats?.total_queries || 0,
      icon: Zap,
      color: 'text-blue-600',
      bgColor: 'bg-blue-50',
      realtime: true,
    },
    {
      name: 'Avg Latency',
      value: `${(systemStats?.avg_latency_ms || initialStats?.avg_duration_ms || 0).toFixed(2)}ms`,
      icon: Activity,
      color: 'text-green-600',
      bgColor: 'bg-green-50',
      realtime: true,
    },
    {
      name: 'Tables',
      value: systemStats?.tables || tables.length || 0,
      icon: Database,
      color: 'text-purple-600',
      bgColor: 'bg-purple-50',
    },
    {
      name: 'Brains',
      value: systemStats?.brains || brains.length || 0,
      icon: BrainIcon,
      color: 'text-indigo-600',
      bgColor: 'bg-indigo-50',
    },
    {
      name: 'Workers',
      value: systemStats?.workers || workers.length || 0,
      icon: Code,
      color: 'text-orange-600',
      bgColor: 'bg-orange-50',
    },
    {
      name: 'Active Connections',
      value: systemStats?.active_connections || 0,
      icon: isConnected ? Wifi : WifiOff,
      color: isConnected ? 'text-green-600' : 'text-gray-600',
      bgColor: isConnected ? 'bg-green-50' : 'bg-gray-50',
      realtime: true,
    },
  ]

  // Prepare chart data from real-time metrics
  const chartData = realTimeMetrics.length > 0 
    ? realTimeMetrics 
    : [
        { time: '00:00', queries: 0, latency: 0 },
        { time: '01:00', queries: 0, latency: 0 },
      ]

  return (
    <div className="space-y-6">
      {/* Header with connection status */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-gray-900">Dashboard</h1>
          <p className="text-gray-600 mt-1">
            Overview of your NarayanaDB instance
            {isConnected && (
              <span className="ml-2 inline-flex items-center gap-1 text-green-600">
                <Wifi className="w-4 h-4" />
                <span className="text-sm">Real-time connected</span>
              </span>
            )}
          </p>
        </div>
        {lastUpdate && (
          <div className="text-sm text-gray-500">
            Last update: {lastUpdate.toLocaleTimeString()}
          </div>
        )}
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-4">
        {cardStats.map((stat) => {
          const Icon = stat.icon
          return (
            <div key={stat.name} className="card relative">
              {stat.realtime && isConnected && (
                <div className="absolute top-2 right-2">
                  <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
                </div>
              )}
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm font-medium text-gray-600">{stat.name}</p>
                  <p className="text-2xl font-bold text-gray-900 mt-2">
                    {typeof stat.value === 'string' ? stat.value : stat.value.toLocaleString()}
                  </p>
                </div>
                <div className={`${stat.bgColor} p-3 rounded-lg`}>
                  <Icon className={`w-6 h-6 ${stat.color}`} />
                </div>
              </div>
            </div>
          )
        })}
      </div>

      {/* Real-time Charts */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Query Performance */}
        <div className="card">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-gray-900">Query Performance</h2>
            {isConnected && (
              <span className="text-xs text-green-600 flex items-center gap-1">
                <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
                Live
              </span>
            )}
          </div>
          <ResponsiveContainer width="100%" height={300}>
            <LineChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis dataKey="time" />
              <YAxis />
              <Tooltip />
              <Line type="monotone" dataKey="queries" stroke="#0ea5e9" strokeWidth={2} name="Queries" />
            </LineChart>
          </ResponsiveContainer>
        </div>

        {/* Latency Trend */}
        <div className="card">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-gray-900">Latency Trend</h2>
            {isConnected && (
              <span className="text-xs text-green-600 flex items-center gap-1">
                <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
                Live
              </span>
            )}
          </div>
          <ResponsiveContainer width="100%" height={300}>
            <BarChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis dataKey="time" />
              <YAxis />
              <Tooltip />
              <Bar dataKey="latency" fill="#10b981" name="Latency (ms)" />
            </BarChart>
          </ResponsiveContainer>
        </div>
      </div>

      {/* System Overview - Brains, Workers, Tables */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Brains */}
        <div className="card">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-gray-900 flex items-center gap-2">
              <BrainIcon className="w-5 h-5" />
              Cognitive Brains
            </h2>
            <span className="text-sm text-gray-500">{brains.length}</span>
          </div>
          {brains.length > 0 ? (
            <div className="space-y-3">
              {brains.map((brain) => (
                <div key={brain.brain_id} className="p-3 bg-gray-50 rounded-lg">
                  <div className="font-medium text-gray-900">{brain.brain_id}</div>
                  <div className="text-sm text-gray-600 mt-1">
                    Memory types: {brain.memory_types.join(', ')}
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-gray-500 text-center py-4">No brains created yet</p>
          )}
        </div>

        {/* Workers */}
        <div className="card">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-gray-900 flex items-center gap-2">
              <Code className="w-5 h-5" />
              Workers
            </h2>
            <span className="text-sm text-gray-500">{workers.length}</span>
          </div>
          {workers.length > 0 ? (
            <div className="space-y-3">
              {workers.map((worker) => (
                <div key={worker.worker_id} className="p-3 bg-gray-50 rounded-lg">
                  <div className="flex items-center justify-between">
                    <div className="font-medium text-gray-900">{worker.name}</div>
                    <span
                      className={`text-xs px-2 py-1 rounded ${
                        worker.active
                          ? 'bg-green-100 text-green-800'
                          : 'bg-gray-100 text-gray-800'
                      }`}
                    >
                      {worker.active ? 'Active' : 'Inactive'}
                    </span>
                  </div>
                  <div className="text-sm text-gray-600 mt-1">Route: {worker.route}</div>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-gray-500 text-center py-4">No workers deployed yet</p>
          )}
        </div>

        {/* Tables */}
        <div className="card">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-gray-900 flex items-center gap-2">
              <Database className="w-5 h-5" />
              Tables
            </h2>
            <span className="text-sm text-gray-500">{tables.length}</span>
          </div>
          {tables.length > 0 ? (
            <div className="space-y-3">
              {tables.slice(0, 5).map((table) => (
                <div key={table.id} className="p-3 bg-gray-50 rounded-lg">
                  <div className="font-medium text-gray-900">{table.name || `Table ${table.id}`}</div>
                  <div className="text-sm text-gray-600 mt-1">
                    {table.row_count || 0} rows • {table.schema?.fields?.length || 0} columns
                  </div>
                </div>
              ))}
              {tables.length > 5 && (
                <p className="text-sm text-gray-500 text-center pt-2">
                  +{tables.length - 5} more tables
                </p>
              )}
            </div>
          ) : (
            <p className="text-gray-500 text-center py-4">No tables created yet</p>
          )}
        </div>
      </div>

      {/* Additional Stats */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="card">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">System Performance</h2>
          <div className="space-y-3">
            <div className="flex justify-between items-center">
              <span className="text-gray-600">Total Rows Read</span>
              <span className="font-semibold text-gray-900">
                {(systemStats?.total_rows_read || initialStats?.total_rows_read || 0).toLocaleString()}
              </span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-gray-600">Total Rows Inserted</span>
              <span className="font-semibold text-gray-900">
                {(systemStats?.total_rows_inserted || initialStats?.total_rows_inserted || 0).toLocaleString()}
              </span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-gray-600">Average Query Time</span>
              <span className="font-semibold text-gray-900">
                {(systemStats?.avg_latency_ms || initialStats?.avg_duration_ms || 0).toFixed(2)}ms
              </span>
            </div>
          </div>
        </div>

        <div className="card">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">Connection Status</h2>
          <div className="space-y-3">
            <div className="flex justify-between items-center">
              <span className="text-gray-600">WebSocket</span>
              <span
                className={`px-3 py-1 rounded-full text-sm font-medium ${
                  isConnected
                    ? 'bg-green-100 text-green-800'
                    : 'bg-red-100 text-red-800'
                }`}
              >
                {isConnected ? 'Connected' : 'Disconnected'}
              </span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-gray-600">Active Connections</span>
              <span className="font-semibold text-gray-900">
                {systemStats?.active_connections || 0}
              </span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-gray-600">Last Update</span>
              <span className="font-semibold text-gray-900">
                {lastUpdate.toLocaleTimeString()}
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
