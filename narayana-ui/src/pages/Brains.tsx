import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from 'react-router-dom'
import { apiClient } from '../lib/api'
import { useWebSocket } from '../hooks/useWebSocket'
import { Brain as BrainIcon, Plus, MemoryStick, Clock, Eye } from 'lucide-react'
import { useEffect, useState } from 'react'

export default function Brains() {
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [selectedBrain] = useState<string | null>(null)
  const [showCreateModal, setShowCreateModal] = useState(false)
  const [newBrainId, setNewBrainId] = useState('')
  const [newMemoryTypes, setNewMemoryTypes] = useState<string[]>([])

  // WebSocket for real-time updates
  const { isConnected, subscribe } = useWebSocket({
    url: 'ws://localhost:8080/ws',
  })

  useEffect(() => {
    if (isConnected) {
      subscribe('system:brains')
      subscribe('brains:thoughts')
      subscribe('brains:memories')
    }
  }, [isConnected, subscribe])

  const { data: brains } = useQuery({
    queryKey: ['brains'],
    queryFn: apiClient.getBrains,
    refetchInterval: 3000,
  })

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-gray-900 flex items-center gap-2">
            <BrainIcon className="w-8 h-8" />
            Cognitive Brains
          </h1>
          <p className="text-gray-600 mt-1">Manage and monitor cognitive brains</p>
        </div>
        <button
          onClick={() => setShowCreateModal(true)}
          className="btn-primary flex items-center gap-2"
        >
          <Plus className="w-4 h-4" />
          Create Brain
        </button>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Total Brains</p>
              <p className="text-2xl font-bold text-gray-900 mt-2">{brains?.length || 0}</p>
            </div>
            <BrainIcon className="w-8 h-8 text-indigo-600" />
          </div>
        </div>
        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Active Thoughts</p>
              <p className="text-2xl font-bold text-gray-900 mt-2">-</p>
            </div>
            <MemoryStick className="w-8 h-8 text-purple-600" />
          </div>
        </div>
        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Total Memories</p>
              <p className="text-2xl font-bold text-gray-900 mt-2">-</p>
            </div>
            <Clock className="w-8 h-8 text-blue-600" />
          </div>
        </div>
      </div>

      {/* Brains List */}
      <div className="card">
        <h2 className="text-lg font-semibold text-gray-900 mb-4">All Brains</h2>
        {brains && brains.length > 0 ? (
          <div className="space-y-4">
            {brains.map((brain) => (
              <div
                key={brain.brain_id}
                className={`p-4 border rounded-lg transition-colors ${
                  selectedBrain === brain.brain_id
                    ? 'border-primary-500 bg-primary-50'
                    : 'border-gray-200 hover:border-gray-300'
                }`}
              >
                <div className="flex items-center justify-between">
                  <div
                    className="flex-1 cursor-pointer"
                    onClick={() => navigate(`/brains/${brain.brain_id}`)}
                  >
                    <h3 className="font-semibold text-gray-900">{brain.brain_id}</h3>
                    <div className="flex flex-wrap gap-2 mt-2">
                      {brain.memory_types.map((type) => (
                        <span
                          key={type}
                          className="px-2 py-1 text-xs bg-indigo-100 text-indigo-800 rounded"
                        >
                          {type}
                        </span>
                      ))}
                    </div>
                    {brain.created_at && (
                      <p className="text-sm text-gray-500 mt-2">
                        Created: {new Date(brain.created_at * 1000).toLocaleString()}
                      </p>
                    )}
                  </div>
                  <div className="flex items-center gap-2" onClick={(e) => e.stopPropagation()}>
                    <button
                      onClick={() => navigate(`/brains/${brain.brain_id}`)}
                      className="p-2 text-blue-600 hover:bg-blue-50 rounded-lg transition-colors"
                      title="View details"
                    >
                      <Eye className="w-4 h-4" />
                    </button>
                    <div className="text-right">
                      <span className="text-sm text-gray-500">Status</span>
                      <div className="mt-1">
                        <span className="px-3 py-1 bg-green-100 text-green-800 rounded-full text-sm font-medium">
                          Active
                        </span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-center py-12">
            <BrainIcon className="w-16 h-16 text-gray-400 mx-auto mb-4" />
            <p className="text-gray-500 text-lg">No brains created yet</p>
            <p className="text-gray-400 text-sm mt-2">Create a brain to get started</p>
          </div>
        )}
      </div>

      {/* Create Brain Modal */}
      {showCreateModal && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-full max-w-md">
            <h2 className="text-xl font-bold text-gray-900 mb-4">Create New Brain</h2>
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Brain ID
                </label>
                <input
                  type="text"
                  value={newBrainId}
                  onChange={(e) => setNewBrainId(e.target.value)}
                  className="input"
                  placeholder="Enter brain ID"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Memory Types (comma-separated)
                </label>
                <input
                  type="text"
                  value={newMemoryTypes.join(', ')}
                  onChange={(e) => setNewMemoryTypes(e.target.value.split(',').map(s => s.trim()).filter(Boolean))}
                  className="input"
                  placeholder="episodic, semantic, procedural"
                />
              </div>
              <div className="flex gap-3 justify-end pt-4">
                <button
                  onClick={() => {
                    setShowCreateModal(false)
                    setNewBrainId('')
                    setNewMemoryTypes([])
                  }}
                  className="btn-secondary"
                >
                  Cancel
                </button>
                <button
                  onClick={async () => {
                    try {
                      await apiClient.createBrain(newBrainId, newMemoryTypes.length > 0 ? newMemoryTypes : undefined)
                      setShowCreateModal(false)
                      setNewBrainId('')
                      setNewMemoryTypes([])
                      // Invalidate queries to refresh the list
                      queryClient.invalidateQueries({ queryKey: ['brains'] })
                    } catch (error) {
                      console.error('Failed to create brain:', error)
                      alert('Failed to create brain')
                    }
                  }}
                  disabled={!newBrainId}
                  className="btn-primary"
                >
                  Create
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

