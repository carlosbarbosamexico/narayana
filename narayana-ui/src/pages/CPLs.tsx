import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../lib/api'
import { RefreshCw, Plus, Play, Square, Trash2, Brain, User } from 'lucide-react'
import { useState } from 'react'
import { useNavigate } from 'react-router-dom'

export default function CPLs() {
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [showCreateModal, setShowCreateModal] = useState(false)
  const [config, setConfig] = useState({
    loop_interval_ms: 100,
    enable_global_workspace: true,
    enable_background_daemon: true,
    enable_dreaming: true,
    working_memory_capacity: 7,
    enable_attention: true,
    enable_narrative: true,
    enable_memory_bridge: true,
    enable_persistence: true,
    persistence_dir: 'data/cpl',
    enable_genetics: true,
    genetic_mutation_rate: 0.01,
    evolution_frequency: 1000,
    trait_environmental_weight: 0.3,
    enable_talking_cricket: false,
    talking_cricket_llm_enabled: false,
    talking_cricket_veto_threshold: 0.3,
    talking_cricket_evolution_frequency: 1000,
    enable_avatar: false,
    avatar_config: {
      enabled: false,
      provider: 'BeyondPresence',
      expression_sensitivity: 0.7,
      animation_speed: 1.0,
      enable_lip_sync: true,
      enable_gestures: true,
      avatar_id: null,
      enable_vision: false,
      enable_audio_input: false,
      enable_tts: false,
    },
  })

  const { data: cpls, isLoading, refetch } = useQuery({
    queryKey: ['cpls'],
    queryFn: apiClient.getCPLs,
    refetchInterval: 3000,
  })

  const createMutation = useMutation({
    mutationFn: (cfg: any) => apiClient.createCPL(cfg),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['cpls'] })
      setShowCreateModal(false)
    },
  })

  const startMutation = useMutation({
    mutationFn: (cplId: string) => apiClient.startCPL(cplId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['cpls'] })
    },
    onError: (error: any) => {
      console.error('Failed to start CPL:', error)
      alert(`Failed to start CPL: ${error?.response?.data?.error || error?.message || 'Unknown error'}`)
    },
  })

  const stopMutation = useMutation({
    mutationFn: (cplId: string) => apiClient.stopCPL(cplId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['cpls'] })
    },
    onError: (error: any) => {
      console.error('Failed to stop CPL:', error)
      alert(`Failed to stop CPL: ${error?.response?.data?.error || error?.message || 'Unknown error'}`)
    },
  })

  const deleteMutation = useMutation({
    mutationFn: (cplId: string) => apiClient.deleteCPL(cplId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['cpls'] })
    },
  })

  const handleCreate = () => {
    // Prepare config for API - ensure avatar_config is properly structured
    const apiConfig = {
      ...config,
      avatar_config: config.enable_avatar ? {
        enabled: true,
        provider: config.avatar_config?.provider || 'BeyondPresence',
        expression_sensitivity: config.avatar_config?.expression_sensitivity || 0.7,
        animation_speed: config.avatar_config?.animation_speed || 1.0,
        enable_lip_sync: config.avatar_config?.enable_lip_sync !== false,
        enable_gestures: config.avatar_config?.enable_gestures !== false,
        enable_vision: config.avatar_config?.enable_vision === true,
        enable_audio_input: config.avatar_config?.enable_audio_input === true,
        enable_tts: config.avatar_config?.enable_tts === true,
        avatar_id: (() => {
          const avatarId: any = config.avatar_config?.avatar_id
          if (avatarId && typeof avatarId === 'string' && avatarId.length > 0) {
            return avatarId as string
          }
          return undefined
        })(),
      } : null,
    }
    createMutation.mutate({ config: apiConfig })
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-gray-900 flex items-center gap-2">
            <RefreshCw className="w-8 h-8 text-indigo-600" />
            Conscience Persistent Loops (CPL)
          </h1>
          <p className="text-gray-600 mt-1">
            Manage cognitive consciousness loops with brains, LLMs, and Talking Cricket
          </p>
        </div>
        <div className="flex items-center gap-3">
          <button
            onClick={() => refetch()}
            className="btn-secondary flex items-center gap-2"
          >
            <RefreshCw className="w-4 h-4" />
            Refresh
          </button>
          <button
            onClick={() => setShowCreateModal(true)}
            className="btn-primary flex items-center gap-2"
          >
            <Plus className="w-4 h-4" />
            Create CPL
          </button>
        </div>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Total CPLs</p>
              <p className="text-2xl font-bold text-gray-900 mt-2">{cpls?.count || 0}</p>
            </div>
            <RefreshCw className="w-8 h-8 text-indigo-600" />
          </div>
        </div>
        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Running</p>
              <p className="text-2xl font-bold text-gray-900 mt-2">
                {cpls?.cpls?.filter((c: any) => c.is_running).length || 0}
              </p>
            </div>
            <Play className="w-8 h-8 text-green-600" />
          </div>
        </div>
        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Stopped</p>
              <p className="text-2xl font-bold text-gray-900 mt-2">
                {cpls?.cpls?.filter((c: any) => !c.is_running).length || 0}
              </p>
            </div>
            <Square className="w-8 h-8 text-gray-600" />
          </div>
        </div>
        <div className="card">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">With Talking Cricket</p>
              <p className="text-2xl font-bold text-gray-900 mt-2">-</p>
            </div>
            <Brain className="w-8 h-8 text-purple-600" />
          </div>
        </div>
      </div>

      {/* CPLs List */}
      <div className="card">
        <h2 className="text-lg font-semibold text-gray-900 mb-4">All CPL Instances</h2>
        {isLoading ? (
          <div className="text-center py-12">
            <p className="text-gray-500">Loading CPLs...</p>
          </div>
        ) : cpls && cpls.cpls && cpls.cpls.length > 0 ? (
          <div className="space-y-4">
            {cpls.cpls.map((cpl: any) => (
              <div
                key={cpl.cpl_id}
                className="p-4 border rounded-lg hover:border-gray-300 transition-colors"
              >
                <div className="flex items-center justify-between">
                    <div 
                      className="flex-1 cursor-pointer"
                      onClick={() => navigate(`/cpls/${cpl.cpl_id}`)}
                    >
                    <div className="flex items-center gap-3 mb-2">
                      <h3 className="font-semibold text-gray-900">{cpl.cpl_id}</h3>
                      <span
                        className={`px-3 py-1 rounded-full text-sm font-medium ${
                          cpl.is_running
                            ? 'bg-green-100 text-green-800'
                            : 'bg-gray-100 text-gray-800'
                        }`}
                      >
                        {cpl.is_running ? 'Running' : 'Stopped'}
                      </span>
                      {cpl.config?.enable_avatar && (
                        <span className="px-3 py-1 rounded-full text-sm font-medium bg-blue-100 text-blue-800 flex items-center gap-1">
                          <User className="w-3 h-3" />
                          Avatar
                        </span>
                      )}
                    </div>
                    <p className="text-sm text-gray-500">CPL Instance</p>
                  </div>
                  <div 
                    className="flex items-center gap-2"
                    onClick={(e) => e.stopPropagation()}
                  >
                    {cpl.config?.enable_avatar && cpl.is_running && (
                      <button
                        onClick={() => window.open(`/cpls/${cpl.cpl_id}/avatar`, '_blank')}
                        className="btn-secondary flex items-center gap-2"
                        title="Open Avatar Window"
                      >
                        <User className="w-4 h-4" />
                        Avatar
                      </button>
                    )}
                    {cpl.is_running ? (
                      <button
                        onClick={() => stopMutation.mutate(cpl.cpl_id)}
                        disabled={stopMutation.isPending}
                        className="btn-secondary flex items-center gap-2"
                      >
                        <Square className="w-4 h-4" />
                        Stop
                      </button>
                    ) : (
                      <button
                        onClick={() => startMutation.mutate(cpl.cpl_id)}
                        disabled={startMutation.isPending}
                        className="btn-primary flex items-center gap-2"
                      >
                        <Play className="w-4 h-4" />
                        Start
                      </button>
                    )}
                    <button
                      onClick={() => deleteMutation.mutate(cpl.cpl_id)}
                      disabled={deleteMutation.isPending}
                      className="p-2 text-red-600 hover:bg-red-50 rounded-lg transition-colors"
                      title="Delete CPL"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-center py-12">
            <RefreshCw className="w-16 h-16 text-gray-400 mx-auto mb-4" />
            <p className="text-gray-500 text-lg">No CPLs created yet</p>
            <p className="text-gray-400 text-sm mt-2">Create a CPL to get started</p>
          </div>
        )}
      </div>

      {/* Create CPL Modal */}
      {showCreateModal && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 overflow-y-auto">
          <div className="bg-white rounded-lg p-6 w-full max-w-4xl max-h-[90vh] overflow-y-auto my-8">
            <h2 className="text-xl font-bold text-gray-900 mb-4">Create New CPL</h2>
            
            <div className="space-y-6">
              {/* Basic Settings */}
              <div>
                <h3 className="text-lg font-semibold text-gray-900 mb-3">Basic Settings</h3>
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-2">
                      Loop Interval (ms)
                    </label>
                    <input
                      type="number"
                      value={config.loop_interval_ms}
                      onChange={(e) => setConfig({ ...config, loop_interval_ms: Number(e.target.value) })}
                      className="input"
                      min="10"
                      max="10000"
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-2">
                      Working Memory Capacity
                    </label>
                    <input
                      type="number"
                      value={config.working_memory_capacity}
                      onChange={(e) => setConfig({ ...config, working_memory_capacity: Number(e.target.value) })}
                      className="input"
                      min="1"
                      max="20"
                    />
                  </div>
                </div>
              </div>

              {/* Cognitive Systems */}
              <div>
                <h3 className="text-lg font-semibold text-gray-900 mb-3">Cognitive Systems</h3>
                <div className="space-y-2">
                  {[
                    { key: 'enable_global_workspace', label: 'Global Workspace' },
                    { key: 'enable_background_daemon', label: 'Background Daemon' },
                    { key: 'enable_dreaming', label: 'Dreaming Loop' },
                    { key: 'enable_attention', label: 'Attention Router' },
                    { key: 'enable_narrative', label: 'Narrative Generator' },
                    { key: 'enable_memory_bridge', label: 'Memory Bridge' },
                  ].map(({ key, label }) => (
                    <label key={key} className="flex items-center gap-2">
                      <input
                        type="checkbox"
                        checked={config[key as keyof typeof config] as boolean}
                        onChange={(e) => setConfig({ ...config, [key]: e.target.checked })}
                        className="rounded"
                      />
                      <span className="text-sm text-gray-700">{label}</span>
                    </label>
                  ))}
                </div>
              </div>

              {/* Persistence */}
              <div>
                <h3 className="text-lg font-semibold text-gray-900 mb-3">Persistence</h3>
                <div className="space-y-2">
                  <label className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={config.enable_persistence}
                      onChange={(e) => setConfig({ ...config, enable_persistence: e.target.checked })}
                      className="rounded"
                    />
                    <span className="text-sm text-gray-700">Enable Persistence</span>
                  </label>
                  {config.enable_persistence && (
                    <div>
                      <label className="block text-sm font-medium text-gray-700 mb-2">
                        Persistence Directory
                      </label>
                      <input
                        type="text"
                        value={config.persistence_dir}
                        onChange={(e) => setConfig({ ...config, persistence_dir: e.target.value })}
                        className="input"
                      />
                    </div>
                  )}
                </div>
              </div>

              {/* Genetics */}
              <div>
                <h3 className="text-lg font-semibold text-gray-900 mb-3">Genetics System</h3>
                <div className="space-y-2">
                  <label className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={config.enable_genetics}
                      onChange={(e) => setConfig({ ...config, enable_genetics: e.target.checked })}
                      className="rounded"
                    />
                    <span className="text-sm text-gray-700">Enable Genetics</span>
                  </label>
                  {config.enable_genetics && (
                    <div className="grid grid-cols-3 gap-4 ml-6">
                      <div>
                        <label className="block text-sm font-medium text-gray-700 mb-2">
                          Mutation Rate
                        </label>
                        <input
                          type="number"
                          step="0.001"
                          value={config.genetic_mutation_rate}
                          onChange={(e) => setConfig({ ...config, genetic_mutation_rate: Number(e.target.value) })}
                          className="input"
                          min="0"
                          max="1"
                        />
                      </div>
                      <div>
                        <label className="block text-sm font-medium text-gray-700 mb-2">
                          Evolution Frequency
                        </label>
                        <input
                          type="number"
                          value={config.evolution_frequency}
                          onChange={(e) => setConfig({ ...config, evolution_frequency: Number(e.target.value) })}
                          className="input"
                        />
                      </div>
                      <div>
                        <label className="block text-sm font-medium text-gray-700 mb-2">
                          Environmental Weight
                        </label>
                        <input
                          type="number"
                          step="0.1"
                          value={config.trait_environmental_weight}
                          onChange={(e) => setConfig({ ...config, trait_environmental_weight: Number(e.target.value) })}
                          className="input"
                          min="0"
                          max="1"
                        />
                      </div>
                    </div>
                  )}
                </div>
              </div>

              {/* Avatar Configuration */}
              <div>
                <h3 className="text-lg font-semibold text-gray-900 mb-3 flex items-center gap-2">
                  <User className="w-5 h-5 text-blue-600" />
                  Avatar (Virtual 3D Interface)
                </h3>
                <div className="space-y-2">
                  <label className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={config.enable_avatar}
                      onChange={(e) => setConfig({ ...config, enable_avatar: e.target.checked })}
                      className="rounded"
                    />
                    <span className="text-sm text-gray-700">Enable Avatar</span>
                  </label>
                  {config.enable_avatar && (
                    <div className="ml-6 space-y-4 bg-gray-50 p-4 rounded-lg">
                      <div className="grid grid-cols-2 gap-4">
                        <div>
                          <label className="block text-sm font-medium text-gray-700 mb-2">
                            Provider
                          </label>
                          <select
                            value={config.avatar_config?.provider || 'BeyondPresence'}
                            onChange={(e) => setConfig({
                              ...config,
                              avatar_config: {
                                ...config.avatar_config,
                                provider: e.target.value,
                              }
                            })}
                            className="input"
                          >
                            <option value="BeyondPresence">Beyond Presence</option>
                            <option value="LiveAvatar">LiveAvatar</option>
                            <option value="ReadyPlayerMe">Ready Player Me</option>
                            <option value="AvatarSDK">Avatar SDK</option>
                            <option value="OpenAvatarChat">Open Avatar Chat</option>
                          </select>
                        </div>
                        <div>
                          <label className="block text-sm font-medium text-gray-700 mb-2">
                            Avatar ID (Optional)
                          </label>
                          <input
                            type="text"
                            value={config.avatar_config?.avatar_id || ''}
                            onChange={(e) => {
                              const trimmed = e.target.value.trim()
                              setConfig({
                                ...config,
                                avatar_config: {
                                  ...config.avatar_config,
                                  avatar_id: (trimmed.length > 0 ? trimmed : null) as any,
                                }
                              })
                            }}
                            className="input"
                            placeholder="default"
                            maxLength={256}
                          />
                        </div>
                      </div>
                      <div className="grid grid-cols-2 gap-4">
                        <div>
                          <label className="block text-sm font-medium text-gray-700 mb-2">
                            Expression Sensitivity (0.0-1.0)
                          </label>
                          <input
                            type="number"
                            step="0.1"
                            min="0"
                            max="1"
                            value={config.avatar_config?.expression_sensitivity || 0.7}
                            onChange={(e) => setConfig({
                              ...config,
                              avatar_config: {
                                ...config.avatar_config,
                                expression_sensitivity: Number(e.target.value),
                              }
                            })}
                            className="input"
                          />
                        </div>
                        <div>
                          <label className="block text-sm font-medium text-gray-700 mb-2">
                            Animation Speed (0.5-2.0)
                          </label>
                          <input
                            type="number"
                            step="0.1"
                            min="0.5"
                            max="2.0"
                            value={config.avatar_config?.animation_speed || 1.0}
                            onChange={(e) => setConfig({
                              ...config,
                              avatar_config: {
                                ...config.avatar_config,
                                animation_speed: Number(e.target.value),
                              }
                            })}
                            className="input"
                          />
                        </div>
                      </div>
                      <div className="space-y-2">
                        <label className="flex items-center gap-2">
                          <input
                            type="checkbox"
                            checked={config.avatar_config?.enable_lip_sync !== false}
                            onChange={(e) => setConfig({
                              ...config,
                              avatar_config: {
                                ...config.avatar_config,
                                enable_lip_sync: e.target.checked,
                              }
                            })}
                            className="rounded"
                          />
                          <span className="text-sm text-gray-700">Enable Lip Sync</span>
                        </label>
                        <label className="flex items-center gap-2">
                          <input
                            type="checkbox"
                            checked={config.avatar_config?.enable_gestures !== false}
                            onChange={(e) => setConfig({
                              ...config,
                              avatar_config: {
                                ...config.avatar_config,
                                enable_gestures: e.target.checked,
                              }
                            })}
                            className="rounded"
                          />
                          <span className="text-sm text-gray-700">Enable Gestures</span>
                        </label>
                      </div>
                      <div className="border-t pt-4 mt-4 space-y-2">
                        <h4 className="text-sm font-semibold text-gray-900 mb-2">Multimodal Capabilities</h4>
                        <label className="flex items-center gap-2">
                          <input
                            type="checkbox"
                            checked={config.avatar_config?.enable_vision === true}
                            onChange={(e) => setConfig({
                              ...config,
                              avatar_config: {
                                ...config.avatar_config,
                                enable_vision: e.target.checked,
                              }
                            })}
                            className="rounded"
                          />
                          <span className="text-sm text-gray-700">👁️ Enable Vision (Camera)</span>
                        </label>
                        <label className="flex items-center gap-2">
                          <input
                            type="checkbox"
                            checked={config.avatar_config?.enable_audio_input === true}
                            onChange={(e) => setConfig({
                              ...config,
                              avatar_config: {
                                ...config.avatar_config,
                                enable_audio_input: e.target.checked,
                              }
                            })}
                            className="rounded"
                          />
                          <span className="text-sm text-gray-700">🎤 Enable Audio Input (Hearing)</span>
                        </label>
                        <label className="flex items-center gap-2">
                          <input
                            type="checkbox"
                            checked={config.avatar_config?.enable_tts === true}
                            onChange={(e) => setConfig({
                              ...config,
                              avatar_config: {
                                ...config.avatar_config,
                                enable_tts: e.target.checked,
                              }
                            })}
                            className="rounded"
                          />
                          <span className="text-sm text-gray-700">🗣️ Enable Text-to-Speech (Voice)</span>
                        </label>
                      </div>
                    </div>
                  )}
                </div>
              </div>

              {/* Talking Cricket */}
              <div>
                <h3 className="text-lg font-semibold text-gray-900 mb-3 flex items-center gap-2">
                  <Brain className="w-5 h-5 text-purple-600" />
                  Talking Cricket (Moral Guide)
                </h3>
                <div className="space-y-2">
                  <label className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={config.enable_talking_cricket}
                      onChange={(e) => setConfig({ ...config, enable_talking_cricket: e.target.checked })}
                      className="rounded"
                    />
                    <span className="text-sm text-gray-700">Enable Talking Cricket</span>
                  </label>
                  {config.enable_talking_cricket && (
                    <div className="ml-6 space-y-4">
                      <label className="flex items-center gap-2">
                        <input
                          type="checkbox"
                          checked={config.talking_cricket_llm_enabled}
                          onChange={(e) => setConfig({ ...config, talking_cricket_llm_enabled: e.target.checked })}
                          className="rounded"
                        />
                        <span className="text-sm text-gray-700">Enable LLM for Principle Evolution</span>
                      </label>
                      <div className="grid grid-cols-2 gap-4">
                        <div>
                          <label className="block text-sm font-medium text-gray-700 mb-2">
                            Veto Threshold
                          </label>
                          <input
                            type="number"
                            step="0.1"
                            value={config.talking_cricket_veto_threshold}
                            onChange={(e) => setConfig({ ...config, talking_cricket_veto_threshold: Number(e.target.value) })}
                            className="input"
                            min="0"
                            max="1"
                          />
                        </div>
                        <div>
                          <label className="block text-sm font-medium text-gray-700 mb-2">
                            Evolution Frequency
                          </label>
                          <input
                            type="number"
                            value={config.talking_cricket_evolution_frequency}
                            onChange={(e) => setConfig({ ...config, talking_cricket_evolution_frequency: Number(e.target.value) })}
                            className="input"
                          />
                        </div>
                      </div>
                    </div>
                  )}
                </div>
              </div>

              <div className="flex gap-3 justify-end pt-4 border-t">
                <button
                  onClick={() => {
                    setShowCreateModal(false)
                    setConfig({
                      loop_interval_ms: 100,
                      enable_global_workspace: true,
                      enable_background_daemon: true,
                      enable_dreaming: true,
                      working_memory_capacity: 7,
                      enable_attention: true,
                      enable_narrative: true,
                      enable_memory_bridge: true,
                      enable_persistence: true,
                      persistence_dir: 'data/cpl',
                      enable_genetics: true,
                      genetic_mutation_rate: 0.01,
                      evolution_frequency: 1000,
                      trait_environmental_weight: 0.3,
                    enable_talking_cricket: false,
                    talking_cricket_llm_enabled: false,
                    talking_cricket_veto_threshold: 0.3,
                    talking_cricket_evolution_frequency: 1000,
                    enable_avatar: false,
                    avatar_config: {
                      enabled: false,
                      provider: 'BeyondPresence',
                      expression_sensitivity: 0.7,
                      animation_speed: 1.0,
                      enable_lip_sync: true,
                      enable_gestures: true,
                      avatar_id: null,
                      enable_vision: false,
                      enable_audio_input: false,
                      enable_tts: false,
                    },
                  })
                  }}
                  className="btn-secondary"
                >
                  Cancel
                </button>
                <button
                  onClick={handleCreate}
                  disabled={createMutation.isPending}
                  className="btn-primary"
                >
                  {createMutation.isPending ? 'Creating...' : 'Create CPL'}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}



