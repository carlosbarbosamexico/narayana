import { useEffect, useState, useCallback } from 'react'
import { useQuery } from '@tanstack/react-query'
import { apiClient } from '../lib/api'
import { useWebSocket } from '../hooks/useWebSocket'
import { Activity, TrendingUp, Clock, Database, Wifi } from 'lucide-react'
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, AreaChart, Area } from 'recharts'

interface PerformanceMetric {
  time: string
  queries: number
  latency: number
  throughput: number
  timestamp: number
}

interface Stats {
  total_queries: number
  avg_duration_ms: number
  total_rows_read: number
  total_rows_inserted: number
}

export default function Performance() {
  const [realTimeData, setRealTimeData] = useState<PerformanceMetric[]>([])
  const [stats, setStats] = useState<Stats | null>(null)
  const [lastUpdate, setLastUpdate] = useState<Date>(new Date())

  // Initial data fetch (only once)
  const { data: initialStats } = useQuery({
    queryKey: ['stats'],
    queryFn: apiClient.getStats,
    refetchInterval: false, // No polling!
  })

  // Initialize state
  useEffect(() => {
    if (initialStats) {
      setStats({
        total_queries: initialStats.total_queries || 0,
        avg_duration_ms: initialStats.avg_duration_ms || 0,
        total_rows_read: initialStats.total_rows_read || 0,
        total_rows_inserted: initialStats.total_rows_inserted || 0,
      })
    }
  }, [initialStats])

  // Handle WebSocket messages
  const handleMessage = useCallback((message: any) => {
    if (message.type === 'event' && message.event) {
      const event = message.event
      
      // Handle query events
      if (event.type === 'query' || message.channel === 'system:queries') {
        const now = new Date()
        const timeStr = `${now.getSeconds()}s`
        
        setRealTimeData((prev) => {
          const updated = [
            ...prev,
            {
              time: timeStr,
              queries: 1,
              latency: event.duration_ms || event.latency || 0,
              throughput: event.rows_read || 0,
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
          setStats((prev) => ({
            ...prev,
            ...event.data,
          }))
        } else {
          // Refetch stats if event doesn't contain data
          apiClient.getStats().then((statsData) => {
            if (statsData) {
              setStats({
                total_queries: statsData.total_queries || 0,
                avg_duration_ms: statsData.avg_duration_ms || 0,
                total_rows_read: statsData.total_rows_read || 0,
                total_rows_inserted: statsData.total_rows_inserted || 0,
              })
            }
          })
        }
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
    }
  }, [isConnected, subscribe])

  // Initialize chart with empty data if no real-time data
  const chartData = realTimeData.length > 0 
    ? realTimeData 
    : Array.from({ length: 20 }, (_, i) => ({
        time: `${i * 5}s`,
        queries: 0,
        latency: 0,
        throughput: 0,
        timestamp: Date.now() - (20 - i) * 5000,
      }))

  const performanceMetrics = [
    {
      label: 'Queries/sec',
      value: stats?.total_queries || initialStats?.total_queries || 0,
      icon: Activity,
      color: 'text-blue-600',
    },
    {
      label: 'Avg Latency',
      value: `${(stats?.avg_duration_ms || initialStats?.avg_duration_ms || 0).toFixed(2)}ms`,
      icon: Clock,
      color: 'text-green-600',
    },
    {
      label: 'Throughput',
      value: `${((stats?.total_rows_read || initialStats?.total_rows_read || 0) / 1000).toFixed(1)}K rows/s`,
      icon: TrendingUp,
      color: 'text-purple-600',
    },
    {
      label: 'Total Rows',
      value: (stats?.total_rows_read || initialStats?.total_rows_read || 0).toLocaleString(),
      icon: Database,
      color: 'text-orange-600',
    },
  ]

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-gray-900">Performance Monitoring</h1>
          <p className="text-gray-600 mt-1">
            Real-time performance metrics and analytics
            {isConnected && (
              <span className="ml-2 inline-flex items-center gap-1 text-green-600">
                <Wifi className="w-4 h-4" />
                <span className="text-sm">Live</span>
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

      {/* Key Metrics */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        {performanceMetrics.map((metric) => {
          const Icon = metric.icon
          return (
            <div key={metric.label} className="card relative">
              {isConnected && (
                <div className="absolute top-2 right-2">
                  <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
                </div>
              )}
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm font-medium text-gray-600">{metric.label}</p>
                  <p className={`text-2xl font-bold mt-2 ${metric.color}`}>{metric.value}</p>
                </div>
                <Icon className={`w-8 h-8 ${metric.color} opacity-50`} />
              </div>
            </div>
          )
        })}
      </div>

      {/* Real-time Charts */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="card">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-gray-900">Query Rate (Real-time)</h2>
            {isConnected && (
              <span className="text-xs text-green-600 flex items-center gap-1">
                <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
                Live
              </span>
            )}
          </div>
          <ResponsiveContainer width="100%" height={300}>
            <AreaChart data={chartData}>
              <defs>
                <linearGradient id="colorQueries" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#0ea5e9" stopOpacity={0.8} />
                  <stop offset="95%" stopColor="#0ea5e9" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis dataKey="time" />
              <YAxis />
              <Tooltip />
              <Area
                type="monotone"
                dataKey="queries"
                stroke="#0ea5e9"
                fillOpacity={1}
                fill="url(#colorQueries)"
              />
            </AreaChart>
          </ResponsiveContainer>
        </div>

        <div className="card">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-gray-900">Latency Distribution</h2>
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
              <Line type="monotone" dataKey="latency" stroke="#10b981" strokeWidth={2} />
            </LineChart>
          </ResponsiveContainer>
        </div>
      </div>

      {/* Throughput Chart */}
      <div className="card">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-gray-900">Throughput Over Time</h2>
          {isConnected && (
            <span className="text-xs text-green-600 flex items-center gap-1">
              <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
              Live
            </span>
          )}
        </div>
        <ResponsiveContainer width="100%" height={400}>
          <AreaChart data={chartData}>
            <defs>
              <linearGradient id="colorThroughput" x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor="#8b5cf6" stopOpacity={0.8} />
                <stop offset="95%" stopColor="#8b5cf6" stopOpacity={0} />
              </linearGradient>
            </defs>
            <CartesianGrid strokeDasharray="3 3" />
            <XAxis dataKey="time" />
            <YAxis />
            <Tooltip />
            <Area
              type="monotone"
              dataKey="throughput"
              stroke="#8b5cf6"
              fillOpacity={1}
              fill="url(#colorThroughput)"
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>

      {/* Performance Stats Table */}
      <div className="card">
        <h2 className="text-lg font-semibold text-gray-900 mb-4">Performance Statistics</h2>
        <div className="overflow-x-auto">
          <table className="min-w-full divide-y divide-gray-200">
            <thead className="bg-gray-50">
              <tr>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                  Metric
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                  Value
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                  Status
                </th>
              </tr>
            </thead>
            <tbody className="bg-white divide-y divide-gray-200">
              <tr>
                <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">
                  Total Queries
                </td>
                <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                  {stats?.total_queries || initialStats?.total_queries || 0}
                </td>
                <td className="px-6 py-4 whitespace-nowrap">
                  <span className="px-2 py-1 text-xs font-medium bg-green-100 text-green-800 rounded-full">
                    Healthy
                  </span>
                </td>
              </tr>
              <tr>
                <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">
                  Average Latency
                </td>
                <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                  {(stats?.avg_duration_ms || initialStats?.avg_duration_ms || 0).toFixed(2)}ms
                </td>
                <td className="px-6 py-4 whitespace-nowrap">
                  <span className="px-2 py-1 text-xs font-medium bg-green-100 text-green-800 rounded-full">
                    Excellent
                  </span>
                </td>
              </tr>
              <tr>
                <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">
                  Rows Read
                </td>
                <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                  {(stats?.total_rows_read || initialStats?.total_rows_read || 0).toLocaleString()}
                </td>
                <td className="px-6 py-4 whitespace-nowrap">
                  <span className="px-2 py-1 text-xs font-medium bg-blue-100 text-blue-800 rounded-full">
                    Active
                  </span>
                </td>
              </tr>
              <tr>
                <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">
                  Rows Inserted
                </td>
                <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                  {(stats?.total_rows_inserted || initialStats?.total_rows_inserted || 0).toLocaleString()}
                </td>
                <td className="px-6 py-4 whitespace-nowrap">
                  <span className="px-2 py-1 text-xs font-medium bg-blue-100 text-blue-800 rounded-full">
                    Active
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  )
}
