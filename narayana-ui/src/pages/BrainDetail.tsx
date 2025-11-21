import { useQuery } from '@tanstack/react-query'
import { useParams, useNavigate } from 'react-router-dom'
import { apiClient, Brain } from '../lib/api'
import { useWebSocket } from '../hooks/useWebSocket'
import { ArrowLeft, Brain as BrainIcon, Clock, MemoryStick, AlertTriangle, Activity, Eye } from 'lucide-react'
import { useState, useEffect } from 'react'

export default function BrainDetail() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const [activeTab, setActiveTab] = useState<'overview' | 'thoughts' | 'memories' | 'timeline' | 'conflicts' | 'accesses'>('overview')
  const [thoughtState, setThoughtState] = useState<string>('')

  const brainId = id || ''

  // WebSocket for real-time updates
  const { isConnected, subscribe } = useWebSocket({
    url: 'ws://localhost:8080/ws',
  })

  useEffect(() => {
    if (isConnected && brainId) {
      subscribe(`brains:${brainId}:thoughts`)
      subscribe(`brains:${brainId}:memories`)
    }
  }, [isConnected, brainId, subscribe])

  // Fetch brain details
  const { data: brains } = useQuery({
    queryKey: ['brains'],
    queryFn: apiClient.getBrains,
  })

  const brain: Brain | undefined = brains?.find((b) => b.brain_id === brainId)

  // Fetch thoughts
  const { data: thoughtsData, refetch: _refetchThoughts } = useQuery({
    queryKey: ['thoughts', brainId, thoughtState],
    queryFn: () => apiClient.getThoughts(brainId, thoughtState || undefined),
    enabled: !!brainId && (activeTab === 'thoughts' || activeTab === 'overview'),
    refetchInterval: 3000,
  })

  // Fetch memories
  const { data: memoriesData, refetch: _refetchMemories } = useQuery({
    queryKey: ['memories', brainId],
    queryFn: () => apiClient.getMemories(brainId),
    enabled: !!brainId && (activeTab === 'memories' || activeTab === 'overview'),
    refetchInterval: 5000,
  })

  // Fetch thought timeline
  const { data: timelineData, refetch: _refetchTimeline } = useQuery({
    queryKey: ['thoughtTimeline', brainId],
    queryFn: () => apiClient.getThoughtTimeline(brainId),
    enabled: !!brainId && activeTab === 'timeline',
    refetchInterval: 3000,
  })

  // Fetch conflicts
  const { data: conflictsData, refetch: _refetchConflicts } = useQuery({
    queryKey: ['conflicts', brainId],
    queryFn: () => apiClient.getConflicts(brainId),
    enabled: !!brainId && activeTab === 'conflicts',
    refetchInterval: 5000,
  })

  // Fetch memory accesses
  const { data: accessesData, refetch: _refetchAccesses } = useQuery({
    queryKey: ['memoryAccesses', brainId],
    queryFn: () => apiClient.getMemoryAccesses(brainId),
    enabled: !!brainId && activeTab === 'accesses',
    refetchInterval: 5000,
  })

  if (!brain) {
    return (
      <div className="space-y-6">
        <div className="flex items-center gap-4">
          <button
            onClick={() => navigate('/brains')}
            className="p-2 hover:bg-gray-100 rounded-lg transition-colors"
          >
            <ArrowLeft className="w-5 h-5" />
          </button>
          <div>
            <h1 className="text-3xl font-bold text-gray-900">Brain Not Found</h1>
          </div>
        </div>
        <div className="card text-center py-12">
          <p className="text-gray-500">Brain with ID "{brainId}" not found.</p>
        </div>
      </div>
    )
  }

  const thoughts = thoughtsData?.thoughts || []
  const memories = memoriesData?.memories || []
  const timeline = timelineData?.timeline || []
  const conflicts = conflictsData?.conflicts || []
  const accesses = accessesData?.accesses || []

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <button
            onClick={() => navigate('/brains')}
            className="p-2 hover:bg-gray-100 rounded-lg transition-colors"
          >
            <ArrowLeft className="w-5 h-5" />
          </button>
          <div>
            <h1 className="text-3xl font-bold text-gray-900 flex items-center gap-2">
              <BrainIcon className="w-8 h-8" />
              {brain.brain_id}
            </h1>
            <p className="text-gray-600 mt-1">
              {brain.memory_types.length > 0
                ? `Memory types: ${brain.memory_types.join(', ')}`
                : 'No memory types configured'}
            </p>
            {brain.created_at && (
              <p className="text-sm text-gray-500 mt-1">
                Created: {new Date(brain.created_at * 1000).toLocaleString()}
              </p>
            )}
          </div>
        </div>
      </div>

      {/* Tabs */}
      <div className="border-b border-gray-200">
        <nav className="flex space-x-8">
          {[
            { id: 'overview', label: 'Overview', icon: Eye },
            { id: 'thoughts', label: 'Thoughts', icon: BrainIcon },
            { id: 'memories', label: 'Memories', icon: MemoryStick },
            { id: 'timeline', label: 'Timeline', icon: Clock },
            { id: 'conflicts', label: 'Conflicts', icon: AlertTriangle },
            { id: 'accesses', label: 'Memory Accesses', icon: Activity },
          ].map((tab) => {
            const Icon = tab.icon
            return (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id as any)}
                className={`py-4 px-1 border-b-2 font-medium text-sm flex items-center gap-2 ${
                  activeTab === tab.id
                    ? 'border-primary-500 text-primary-600'
                    : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
                }`}
              >
                <Icon className="w-4 h-4" />
                {tab.label}
              </button>
            )
          })}
        </nav>
      </div>

      {/* Tab Content */}
      <div className="card">
        {activeTab === 'overview' && (
          <div className="space-y-6">
            <div>
              <h2 className="text-xl font-semibold text-gray-900 mb-4">Overview</h2>
              <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
                <div className="p-4 border rounded-lg">
                  <div className="flex items-center justify-between">
                    <div>
                      <p className="text-sm font-medium text-gray-600">Total Thoughts</p>
                      <p className="text-2xl font-bold text-gray-900 mt-2">{thoughts.length}</p>
                    </div>
                    <BrainIcon className="w-8 h-8 text-indigo-600" />
                  </div>
                </div>
                <div className="p-4 border rounded-lg">
                  <div className="flex items-center justify-between">
                    <div>
                      <p className="text-sm font-medium text-gray-600">Total Memories</p>
                      <p className="text-2xl font-bold text-gray-900 mt-2">{memories.length}</p>
                    </div>
                    <MemoryStick className="w-8 h-8 text-purple-600" />
                  </div>
                </div>
                <div className="p-4 border rounded-lg">
                  <div className="flex items-center justify-between">
                    <div>
                      <p className="text-sm font-medium text-gray-600">Active Conflicts</p>
                      <p className="text-2xl font-bold text-gray-900 mt-2">{conflicts.length}</p>
                    </div>
                    <AlertTriangle className="w-8 h-8 text-red-600" />
                  </div>
                </div>
              </div>
            </div>

            {/* Recent Thoughts */}
            <div>
              <h3 className="text-lg font-semibold text-gray-900 mb-3">Recent Thoughts</h3>
              {thoughts.length > 0 ? (
                <div className="space-y-2">
                  {thoughts.slice(0, 5).map((thought: any, idx: number) => (
                    <div key={idx} className="p-3 border rounded-lg">
                      <div className="flex items-center justify-between">
                        <div>
                          <p className="text-sm font-medium text-gray-900">
                            {thought.thought_id || `Thought ${idx + 1}`}
                          </p>
                          <p className="text-xs text-gray-500 mt-1">
                            State: {thought.state || 'unknown'}
                          </p>
                        </div>
                        {thought.timestamp && (
                          <span className="text-xs text-gray-500">
                            {new Date(thought.timestamp * 1000).toLocaleString()}
                          </span>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                <p className="text-gray-500 text-sm">No thoughts yet</p>
              )}
            </div>
          </div>
        )}

        {activeTab === 'thoughts' && (
          <div className="space-y-4">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-xl font-semibold text-gray-900">Thoughts</h2>
              <div className="flex items-center gap-2">
                <select
                  value={thoughtState}
                  onChange={(e) => setThoughtState(e.target.value)}
                  className="input text-sm"
                >
                  <option value="">All States</option>
                  <option value="pending">Pending</option>
                  <option value="active">Active</option>
                  <option value="completed">Completed</option>
                  <option value="conflicted">Conflicted</option>
                </select>
              </div>
            </div>
            {thoughts.length > 0 ? (
              <div className="space-y-3">
                {thoughts.map((thought: any, idx: number) => (
                  <div key={idx} className="p-4 border rounded-lg">
                    <div className="flex items-center justify-between mb-2">
                      <h3 className="font-semibold text-gray-900">
                        {thought.thought_id || `Thought ${idx + 1}`}
                      </h3>
                      <span
                        className={`px-2 py-1 text-xs rounded-full ${
                          thought.state === 'active'
                            ? 'bg-green-100 text-green-800'
                            : thought.state === 'completed'
                            ? 'bg-blue-100 text-blue-800'
                            : thought.state === 'conflicted'
                            ? 'bg-red-100 text-red-800'
                            : 'bg-gray-100 text-gray-800'
                        }`}
                      >
                        {thought.state || 'unknown'}
                      </span>
                    </div>
                    {thought.description && (
                      <p className="text-sm text-gray-700 mt-2">{thought.description}</p>
                    )}
                    {thought.timestamp && (
                      <p className="text-xs text-gray-500 mt-2">
                        {new Date(thought.timestamp * 1000).toLocaleString()}
                      </p>
                    )}
                  </div>
                ))}
              </div>
            ) : (
              <div className="text-center py-12">
                <BrainIcon className="w-16 h-16 text-gray-400 mx-auto mb-4" />
                <p className="text-gray-500">No thoughts found</p>
              </div>
            )}
          </div>
        )}

        {activeTab === 'memories' && (
          <div className="space-y-4">
            <h2 className="text-xl font-semibold text-gray-900 mb-4">Memories</h2>
            {memories.length > 0 ? (
              <div className="space-y-3">
                {memories.map((memory: any, idx: number) => (
                  <div key={idx} className="p-4 border rounded-lg">
                    <div className="flex items-center justify-between mb-2">
                      <h3 className="font-semibold text-gray-900">
                        Memory {idx + 1} ({memory.type || 'unknown'})
                      </h3>
                      {memory.importance && (
                        <span className="px-2 py-1 text-xs bg-indigo-100 text-indigo-800 rounded">
                          Importance: {memory.importance}
                        </span>
                      )}
                    </div>
                    {memory.content && (
                      <p className="text-sm text-gray-700 mt-2">{JSON.stringify(memory.content)}</p>
                    )}
                    {memory.timestamp && (
                      <p className="text-xs text-gray-500 mt-2">
                        {new Date(memory.timestamp * 1000).toLocaleString()}
                      </p>
                    )}
                  </div>
                ))}
              </div>
            ) : (
              <div className="text-center py-12">
                <MemoryStick className="w-16 h-16 text-gray-400 mx-auto mb-4" />
                <p className="text-gray-500">No memories found</p>
              </div>
            )}
          </div>
        )}

        {activeTab === 'timeline' && (
          <div className="space-y-4">
            <h2 className="text-xl font-semibold text-gray-900 mb-4">Thought Timeline</h2>
            {timeline.length > 0 ? (
              <div className="space-y-3">
                {timeline.map((event: any, idx: number) => (
                  <div key={idx} className="p-4 border rounded-lg">
                    <div className="flex items-center justify-between">
                      <div>
                        <p className="font-semibold text-gray-900">
                          {event.thought_id || `Event ${idx + 1}`}
                        </p>
                        <p className="text-sm text-gray-600 mt-1">{event.event_type || 'unknown'}</p>
                      </div>
                      {event.timestamp && (
                        <span className="text-xs text-gray-500">
                          {new Date(event.timestamp * 1000).toLocaleString()}
                        </span>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <div className="text-center py-12">
                <Clock className="w-16 h-16 text-gray-400 mx-auto mb-4" />
                <p className="text-gray-500">No timeline events found</p>
              </div>
            )}
          </div>
        )}

        {activeTab === 'conflicts' && (
          <div className="space-y-4">
            <h2 className="text-xl font-semibold text-gray-900 mb-4">Conflicts</h2>
            {conflicts.length > 0 ? (
              <div className="space-y-3">
                {conflicts.map((conflict: any, idx: number) => (
                  <div key={idx} className="p-4 border border-red-200 bg-red-50 rounded-lg">
                    <div className="flex items-center justify-between mb-2">
                      <h3 className="font-semibold text-red-900">Conflict {idx + 1}</h3>
                      <AlertTriangle className="w-5 h-5 text-red-600" />
                    </div>
                    {conflict.description && (
                      <p className="text-sm text-red-800 mt-2">{conflict.description}</p>
                    )}
                    {conflict.thoughts && (
                      <p className="text-xs text-red-700 mt-2">
                        Involved thoughts: {conflict.thoughts.join(', ')}
                      </p>
                    )}
                  </div>
                ))}
              </div>
            ) : (
              <div className="text-center py-12">
                <AlertTriangle className="w-16 h-16 text-gray-400 mx-auto mb-4" />
                <p className="text-gray-500">No conflicts found</p>
              </div>
            )}
          </div>
        )}

        {activeTab === 'accesses' && (
          <div className="space-y-4">
            <h2 className="text-xl font-semibold text-gray-900 mb-4">Memory Accesses</h2>
            {accesses.length > 0 ? (
              <div className="overflow-x-auto">
                <table className="w-full">
                  <thead>
                    <tr className="border-b border-gray-200">
                      <th className="text-left p-3 font-semibold text-gray-700">Memory ID</th>
                      <th className="text-left p-3 font-semibold text-gray-700">Type</th>
                      <th className="text-left p-3 font-semibold text-gray-700">Access Type</th>
                      <th className="text-left p-3 font-semibold text-gray-700">Timestamp</th>
                    </tr>
                  </thead>
                  <tbody>
                    {accesses.map((access: any, idx: number) => (
                      <tr key={idx} className="border-b border-gray-100">
                        <td className="p-3 text-sm text-gray-900">{access.memory_id || '-'}</td>
                        <td className="p-3 text-sm text-gray-900">{access.memory_type || '-'}</td>
                        <td className="p-3 text-sm text-gray-900">{access.access_type || '-'}</td>
                        <td className="p-3 text-sm text-gray-500">
                          {access.timestamp
                            ? new Date(access.timestamp * 1000).toLocaleString()
                            : '-'}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <div className="text-center py-12">
                <Activity className="w-16 h-16 text-gray-400 mx-auto mb-4" />
                <p className="text-gray-500">No memory accesses found</p>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  )
}
