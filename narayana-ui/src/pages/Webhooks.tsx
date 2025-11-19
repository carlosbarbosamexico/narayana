import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../lib/api'
import { useWebSocket } from '../hooks/useWebSocket'
import { useEffect, useState } from 'react'
import {
  Webhook as WebhookIcon,
  Plus,
  Play,
  Pause,
  Trash2,
  Clock,
  CheckCircle,
  XCircle,
  AlertCircle,
  RefreshCw,
  Eye,
} from 'lucide-react'

export default function Webhooks() {
  const [selectedWebhook, setSelectedWebhook] = useState<string | null>(null)
  const [showDeliveries, setShowDeliveries] = useState<string | null>(null)
  const queryClient = useQueryClient()

  // WebSocket for real-time updates
  const { isConnected, subscribe } = useWebSocket({
    url: 'ws://localhost:8080/ws',
  })

  useEffect(() => {
    if (isConnected) {
      subscribe('system:webhooks')
      subscribe('webhooks:deliveries')
    }
  }, [isConnected, subscribe])

  const { data: webhooks } = useQuery({
    queryKey: ['webhooks'],
    queryFn: apiClient.getWebhooks,
    refetchInterval: 3000,
  })

  const { data: deliveries } = useQuery({
    queryKey: ['webhook-deliveries', showDeliveries],
    queryFn: () => apiClient.getWebhookDeliveries(showDeliveries!),
    enabled: !!showDeliveries,
    refetchInterval: 2000,
  })

  const enableMutation = useMutation({
    mutationFn: apiClient.enableWebhook,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['webhooks'] })
    },
  })

  const disableMutation = useMutation({
    mutationFn: apiClient.disableWebhook,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['webhooks'] })
    },
  })

  const deleteMutation = useMutation({
    mutationFn: apiClient.deleteWebhook,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['webhooks'] })
    },
  })

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'success':
        return 'bg-green-100 text-green-800'
      case 'failed':
        return 'bg-red-100 text-red-800'
      case 'processing':
        return 'bg-blue-100 text-blue-800'
      case 'pending':
        return 'bg-yellow-100 text-yellow-800'
      default:
        return 'bg-gray-100 text-gray-800'
    }
  }

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'success':
        return <CheckCircle className="w-4 h-4" />
      case 'failed':
        return <XCircle className="w-4 h-4" />
      case 'processing':
        return <RefreshCw className="w-4 h-4 animate-spin" />
      case 'pending':
        return <Clock className="w-4 h-4" />
      default:
        return <AlertCircle className="w-4 h-4" />
    }
  }

  const totalDeliveries = webhooks?.reduce((sum, w) => sum + w.total_deliveries, 0) || 0
  const successfulDeliveries = webhooks?.reduce((sum, w) => sum + w.successful_deliveries, 0) || 0
  const failedDeliveries = webhooks?.reduce((sum, w) => sum + w.failed_deliveries, 0) || 0
  const successRate =
    totalDeliveries > 0 ? ((successfulDeliveries / totalDeliveries) * 100).toFixed(1) : '0.0'

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-gray-900 flex items-center gap-2">
            <WebhookIcon className="w-8 h-8" />
            Webhooks
          </h1>
          <p className="text-gray-600 mt-1">Manage webhook deliveries and retries</p>
        </div>
        <button className="btn-primary flex items-center gap-2">
          <Plus className="w-4 h-4" />
          Create Webhook
        </button>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Total Webhooks</p>
              <p className="text-2xl font-bold text-gray-900 mt-2">{webhooks?.length || 0}</p>
            </div>
            <WebhookIcon className="w-8 h-8 text-indigo-600" />
          </div>
        </div>
        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Total Deliveries</p>
              <p className="text-2xl font-bold text-gray-900 mt-2">{totalDeliveries.toLocaleString()}</p>
            </div>
            <CheckCircle className="w-8 h-8 text-green-600" />
          </div>
        </div>
        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Success Rate</p>
              <p className="text-2xl font-bold text-gray-900 mt-2">{successRate}%</p>
            </div>
            <AlertCircle className="w-8 h-8 text-blue-600" />
          </div>
        </div>
        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Failed</p>
              <p className="text-2xl font-bold text-gray-900 mt-2">{failedDeliveries.toLocaleString()}</p>
            </div>
            <XCircle className="w-8 h-8 text-red-600" />
          </div>
        </div>
      </div>

      {/* Webhooks List */}
      <div className="card">
        <h2 className="text-lg font-semibold text-gray-900 mb-4">All Webhooks</h2>
        {webhooks && webhooks.length > 0 ? (
          <div className="overflow-x-auto">
            <table className="min-w-full divide-y divide-gray-200">
              <thead className="bg-gray-50">
                <tr>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Name
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    URL
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Events
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Status
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Deliveries
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Success Rate
                  </th>
                  <th className="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Actions
                  </th>
                </tr>
              </thead>
              <tbody className="bg-white divide-y divide-gray-200">
                {webhooks.map((webhook) => {
                  const webhookSuccessRate =
                    webhook.total_deliveries > 0
                      ? ((webhook.successful_deliveries / webhook.total_deliveries) * 100).toFixed(1)
                      : '0.0'
                  return (
                    <>
                      <tr
                        key={webhook.id}
                        className={`hover:bg-gray-50 cursor-pointer ${
                          selectedWebhook === webhook.id ? 'bg-blue-50' : ''
                        }`}
                        onClick={() => setSelectedWebhook(webhook.id)}
                      >
                        <td className="px-6 py-4 whitespace-nowrap">
                          <div className="font-medium text-gray-900">{webhook.name}</div>
                          <div className="text-sm text-gray-500">{webhook.id}</div>
                        </td>
                        <td className="px-6 py-4">
                          <code className="text-sm bg-gray-100 px-2 py-1 rounded break-all">
                            {webhook.url}
                          </code>
                        </td>
                        <td className="px-6 py-4">
                          <div className="flex flex-wrap gap-1">
                            {webhook.events.slice(0, 3).map((event) => (
                              <span
                                key={event}
                                className="px-2 py-1 text-xs bg-indigo-100 text-indigo-800 rounded"
                              >
                                {event}
                              </span>
                            ))}
                            {webhook.events.length > 3 && (
                              <span className="px-2 py-1 text-xs bg-gray-100 text-gray-600 rounded">
                                +{webhook.events.length - 3}
                              </span>
                            )}
                          </div>
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap">
                          <span
                            className={`px-3 py-1 rounded-full text-sm font-medium ${
                              webhook.enabled
                                ? 'bg-green-100 text-green-800'
                                : 'bg-gray-100 text-gray-800'
                            }`}
                          >
                            {webhook.enabled ? 'Enabled' : 'Disabled'}
                          </span>
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
                          <div>
                            <div>Total: {webhook.total_deliveries}</div>
                            <div className="text-xs text-gray-500">
                              ✓ {webhook.successful_deliveries} / ✗ {webhook.failed_deliveries}
                            </div>
                          </div>
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap">
                          <div className="flex items-center gap-2">
                            <div className="flex-1 bg-gray-200 rounded-full h-2">
                              <div
                                className="bg-green-500 h-2 rounded-full"
                                style={{ width: `${webhookSuccessRate}%` }}
                              ></div>
                            </div>
                            <span className="text-sm font-medium text-gray-900">{webhookSuccessRate}%</span>
                          </div>
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                          <div className="flex items-center justify-end gap-2">
                            <button
                              onClick={(e) => {
                                e.stopPropagation()
                                setShowDeliveries(showDeliveries === webhook.id ? null : webhook.id)
                              }}
                              className="text-blue-600 hover:text-blue-900"
                              title="View Deliveries"
                            >
                              <Eye className="w-4 h-4" />
                            </button>
                            <button
                              onClick={(e) => {
                                e.stopPropagation()
                                webhook.enabled
                                  ? disableMutation.mutate(webhook.id)
                                  : enableMutation.mutate(webhook.id)
                              }}
                              className="text-blue-600 hover:text-blue-900"
                              title={webhook.enabled ? 'Disable' : 'Enable'}
                            >
                              {webhook.enabled ? <Pause className="w-4 h-4" /> : <Play className="w-4 h-4" />}
                            </button>
                            <button
                              onClick={(e) => {
                                e.stopPropagation()
                                if (confirm(`Delete webhook "${webhook.name}"?`)) {
                                  deleteMutation.mutate(webhook.id)
                                }
                              }}
                              className="text-red-600 hover:text-red-900"
                              title="Delete"
                            >
                              <Trash2 className="w-4 h-4" />
                            </button>
                          </div>
                        </td>
                      </tr>
                      {showDeliveries === webhook.id && (
                        <tr>
                          <td colSpan={7} className="px-6 py-4 bg-gray-50">
                            <div className="space-y-4">
                              <div className="flex items-center justify-between">
                                <h3 className="font-semibold text-gray-900">Delivery History</h3>
                                <button
                                  onClick={() => setShowDeliveries(null)}
                                  className="text-sm text-gray-600 hover:text-gray-900"
                                >
                                  Close
                                </button>
                              </div>
                              {deliveries && deliveries.length > 0 ? (
                                <div className="overflow-x-auto">
                                  <table className="min-w-full divide-y divide-gray-200">
                                    <thead className="bg-white">
                                      <tr>
                                        <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">
                                          Status
                                        </th>
                                        <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">
                                          Attempt
                                        </th>
                                        <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">
                                          Created
                                        </th>
                                        <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">
                                          Duration
                                        </th>
                                        <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">
                                          Error
                                        </th>
                                      </tr>
                                    </thead>
                                    <tbody className="bg-white divide-y divide-gray-200">
                                      {deliveries.map((delivery) => (
                                        <tr key={delivery.id}>
                                          <td className="px-4 py-2 whitespace-nowrap">
                                            <span
                                              className={`px-2 py-1 rounded-full text-xs font-medium flex items-center gap-1 w-fit ${getStatusColor(
                                                delivery.status
                                              )}`}
                                            >
                                              {getStatusIcon(delivery.status)}
                                              {delivery.status}
                                            </span>
                                          </td>
                                          <td className="px-4 py-2 whitespace-nowrap text-sm text-gray-900">
                                            {delivery.attempt} / {delivery.max_attempts}
                                          </td>
                                          <td className="px-4 py-2 whitespace-nowrap text-sm text-gray-500">
                                            {new Date(delivery.created_at * 1000).toLocaleString()}
                                          </td>
                                          <td className="px-4 py-2 whitespace-nowrap text-sm text-gray-500">
                                            {delivery.duration_ms ? `${delivery.duration_ms}ms` : '-'}
                                          </td>
                                          <td className="px-4 py-2 text-sm text-red-600">
                                            {delivery.error || '-'}
                                          </td>
                                        </tr>
                                      ))}
                                    </tbody>
                                  </table>
                                </div>
                              ) : (
                                <p className="text-gray-500 text-center py-4">No deliveries yet</p>
                              )}
                            </div>
                          </td>
                        </tr>
                      )}
                    </>
                  )
                })}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="text-center py-12">
            <WebhookIcon className="w-16 h-16 text-gray-400 mx-auto mb-4" />
            <p className="text-gray-500 text-lg">No webhooks configured yet</p>
            <p className="text-gray-400 text-sm mt-2">Create a webhook to get started</p>
          </div>
        )}
      </div>
    </div>
  )
}

