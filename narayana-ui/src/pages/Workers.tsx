import { useQuery } from '@tanstack/react-query'
import { apiClient } from '../lib/api'
import { useWebSocket } from '../hooks/useWebSocket'
import { Code, Plus, Play, Pause, Trash2, Activity } from 'lucide-react'
import { useEffect } from 'react'

export default function Workers() {
  // WebSocket for real-time updates
  const { isConnected, subscribe } = useWebSocket({
    url: 'ws://localhost:8080/ws',
  })

  useEffect(() => {
    if (isConnected) {
      subscribe('system:workers')
      subscribe('workers:executions')
    }
  }, [isConnected, subscribe])

  const { data: workers } = useQuery({
    queryKey: ['workers'],
    queryFn: apiClient.getWorkers,
    refetchInterval: 3000,
  })

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-gray-900 flex items-center gap-2">
            <Code className="w-8 h-8" />
            Workers
          </h1>
          <p className="text-gray-600 mt-1">Manage and deploy edge workers</p>
        </div>
        <button className="btn-primary flex items-center gap-2">
          <Plus className="w-4 h-4" />
          Deploy Worker
        </button>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Total Workers</p>
              <p className="text-2xl font-bold text-gray-900 mt-2">{workers?.length || 0}</p>
            </div>
            <Code className="w-8 h-8 text-orange-600" />
          </div>
        </div>
        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Active</p>
              <p className="text-2xl font-bold text-gray-900 mt-2">
                {workers?.filter((w) => w.active).length || 0}
              </p>
            </div>
            <Activity className="w-8 h-8 text-green-600" />
          </div>
        </div>
        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Inactive</p>
              <p className="text-2xl font-bold text-gray-900 mt-2">
                {workers?.filter((w) => !w.active).length || 0}
              </p>
            </div>
            <Pause className="w-8 h-8 text-gray-600" />
          </div>
        </div>
        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Total Executions</p>
              <p className="text-2xl font-bold text-gray-900 mt-2">-</p>
            </div>
            <Play className="w-8 h-8 text-blue-600" />
          </div>
        </div>
      </div>

      {/* Workers List */}
      <div className="card">
        <h2 className="text-lg font-semibold text-gray-900 mb-4">All Workers</h2>
        {workers && workers.length > 0 ? (
          <div className="overflow-x-auto">
            <table className="min-w-full divide-y divide-gray-200">
              <thead className="bg-gray-50">
                <tr>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Name
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Route
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Status
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Created
                  </th>
                  <th className="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Actions
                  </th>
                </tr>
              </thead>
              <tbody className="bg-white divide-y divide-gray-200">
                {workers.map((worker) => (
                  <tr key={worker.worker_id} className="hover:bg-gray-50">
                    <td className="px-6 py-4 whitespace-nowrap">
                      <div className="font-medium text-gray-900">{worker.name}</div>
                      <div className="text-sm text-gray-500">{worker.worker_id}</div>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap">
                      <code className="text-sm bg-gray-100 px-2 py-1 rounded">{worker.route}</code>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap">
                      <span
                        className={`px-3 py-1 rounded-full text-sm font-medium ${
                          worker.active
                            ? 'bg-green-100 text-green-800'
                            : 'bg-gray-100 text-gray-800'
                        }`}
                      >
                        {worker.active ? 'Active' : 'Inactive'}
                      </span>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                      {worker.created_at
                        ? new Date(worker.created_at * 1000).toLocaleDateString()
                        : '-'}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                      <div className="flex items-center justify-end gap-2">
                        <button
                          className="text-blue-600 hover:text-blue-900"
                          title={worker.active ? 'Pause' : 'Resume'}
                        >
                          {worker.active ? (
                            <Pause className="w-4 h-4" />
                          ) : (
                            <Play className="w-4 h-4" />
                          )}
                        </button>
                        <button className="text-red-600 hover:text-red-900" title="Delete">
                          <Trash2 className="w-4 h-4" />
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="text-center py-12">
            <Code className="w-16 h-16 text-gray-400 mx-auto mb-4" />
            <p className="text-gray-500 text-lg">No workers deployed yet</p>
            <p className="text-gray-400 text-sm mt-2">Deploy a worker to get started</p>
          </div>
        )}
      </div>
    </div>
  )
}

