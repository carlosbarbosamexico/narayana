// Thought-Oriented Debugger UI
// Shows active thoughts, memory accesses, spawned threads, merging paths, conflicts, timeline graphs

import { useState, useEffect } from 'react';
import axios from 'axios';

interface Thought {
  id: string;
  thread_id: string;
  content: any;
  state: string;
  created_at: number;
  updated_at: number;
  priority: number;
  associations: string[];
}

interface MemoryAccess {
  memory_id: string;
  access_type: string;
  timestamp: number;
}

interface ThoughtEvent {
  type: string;
  thought_id: string;
  timestamp: number;
  data?: any;
}

export default function ThoughtDebugger() {
  const [activeThoughts, setActiveThoughts] = useState<Thought[]>([]);
  const [memoryAccesses, setMemoryAccesses] = useState<MemoryAccess[]>([]);
  const [thoughtTimeline, setThoughtTimeline] = useState<ThoughtEvent[]>([]);
  const [selectedThought, setSelectedThought] = useState<Thought | null>(null);
  const [conflicts, setConflicts] = useState<any[]>([]);
  const [autoRefresh, setAutoRefresh] = useState(true);

  useEffect(() => {
    if (autoRefresh) {
      const interval = setInterval(() => {
        fetchActiveThoughts();
        fetchMemoryAccesses();
        fetchTimeline();
        fetchConflicts();
      }, 1000);
      return () => clearInterval(interval);
    }
  }, [autoRefresh]);

  const fetchActiveThoughts = async () => {
    try {
      const response = await axios.get('/api/v1/brains/default/thoughts?state=active');
      setActiveThoughts(response.data.thoughts || []);
    } catch (error) {
      console.error('Failed to fetch active thoughts:', error);
    }
  };

  const fetchMemoryAccesses = async () => {
    try {
      const response = await axios.get('/api/v1/brains/default/memory-accesses');
      setMemoryAccesses(response.data.accesses || []);
    } catch (error) {
      console.error('Failed to fetch memory accesses:', error);
    }
  };

  const fetchTimeline = async () => {
    try {
      const response = await axios.get('/api/v1/brains/default/thought-timeline');
      setThoughtTimeline(response.data.timeline || []);
    } catch (error) {
      console.error('Failed to fetch timeline:', error);
    }
  };

  const fetchConflicts = async () => {
    try {
      const response = await axios.get('/api/v1/brains/default/conflicts');
      setConflicts(response.data.conflicts || []);
    } catch (error) {
      console.error('Failed to fetch conflicts:', error);
    }
  };

  const cancelThought = async (thoughtId: string) => {
    try {
      await axios.post(`/api/v1/brains/default/thoughts/${thoughtId}/cancel`);
      fetchActiveThoughts();
    } catch (error) {
      console.error('Failed to cancel thought:', error);
    }
  };

  const getStateColor = (state: string) => {
    switch (state) {
      case 'Active': return 'bg-green-500';
      case 'Paused': return 'bg-yellow-500';
      case 'Completed': return 'bg-blue-500';
      case 'Merged': return 'bg-purple-500';
      case 'Discarded': return 'bg-red-500';
      default: return 'bg-gray-500';
    }
  };

  return (
    <div className="p-6 space-y-6">
      <div className="flex justify-between items-center">
        <h1 className="text-3xl font-bold">Thought Debugger</h1>
        <label className="flex items-center space-x-2">
          <input
            type="checkbox"
            checked={autoRefresh}
            onChange={(e) => setAutoRefresh(e.target.checked)}
            className="rounded"
          />
          <span>Auto-refresh</span>
        </label>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Active Thoughts */}
        <div className="bg-white rounded-lg shadow p-4">
          <h2 className="text-xl font-semibold mb-4">Active Thoughts ({activeThoughts.length})</h2>
          <div className="space-y-2 max-h-96 overflow-y-auto">
            {activeThoughts.map((thought) => (
              <div
                key={thought.id}
                className={`p-3 rounded border-l-4 ${getStateColor(thought.state)} cursor-pointer hover:bg-gray-50`}
                onClick={() => setSelectedThought(thought)}
              >
                <div className="flex justify-between items-start">
                  <div>
                    <div className="font-medium">{thought.id.substring(0, 8)}...</div>
                    <div className="text-sm text-gray-600">Priority: {thought.priority.toFixed(2)}</div>
                    <div className="text-sm text-gray-600">State: {thought.state}</div>
                  </div>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      cancelThought(thought.id);
                    }}
                    className="text-red-600 hover:text-red-800 text-sm"
                  >
                    Cancel
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Memory Accesses */}
        <div className="bg-white rounded-lg shadow p-4">
          <h2 className="text-xl font-semibold mb-4">Memory Accesses ({memoryAccesses.length})</h2>
          <div className="space-y-2 max-h-96 overflow-y-auto">
            {memoryAccesses.slice(0, 20).map((access, idx) => (
              <div key={idx} className="p-2 border-b">
                <div className="flex justify-between">
                  <span className="font-mono text-sm">{access.memory_id.substring(0, 8)}...</span>
                  <span className={`px-2 py-1 rounded text-xs ${
                    access.access_type === 'Read' ? 'bg-blue-100' : 'bg-green-100'
                  }`}>
                    {access.access_type}
                  </span>
                </div>
                <div className="text-xs text-gray-500">
                  {new Date(access.timestamp * 1000).toLocaleTimeString()}
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Selected Thought Details */}
      {selectedThought && (
        <div className="bg-white rounded-lg shadow p-4">
          <h2 className="text-xl font-semibold mb-4">Thought Details</h2>
          <div className="space-y-2">
            <div><strong>ID:</strong> {selectedThought.id}</div>
            <div><strong>Thread ID:</strong> {selectedThought.thread_id}</div>
            <div><strong>State:</strong> {selectedThought.state}</div>
            <div><strong>Priority:</strong> {selectedThought.priority}</div>
            <div><strong>Associations:</strong> {selectedThought.associations.length}</div>
            <div><strong>Content:</strong></div>
            <pre className="bg-gray-100 p-2 rounded text-sm overflow-auto">
              {JSON.stringify(selectedThought.content, null, 2)}
            </pre>
          </div>
        </div>
      )}

      {/* Timeline Graph */}
      <div className="bg-white rounded-lg shadow p-4">
        <h2 className="text-xl font-semibold mb-4">Thought Timeline</h2>
        <div className="relative">
          <div className="flex space-x-4 overflow-x-auto pb-4">
            {thoughtTimeline.map((event, idx) => (
              <div
                key={idx}
                className="flex-shrink-0 w-32 p-2 border rounded bg-gray-50"
              >
                <div className="text-xs font-medium">{event.type}</div>
                <div className="text-xs text-gray-600 mt-1">
                  {new Date(event.timestamp * 1000).toLocaleTimeString()}
                </div>
                <div className="text-xs font-mono mt-1 truncate">
                  {event.thought_id.substring(0, 8)}...
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Conflicts */}
      {conflicts.length > 0 && (
        <div className="bg-red-50 border border-red-200 rounded-lg shadow p-4">
          <h2 className="text-xl font-semibold mb-4 text-red-800">Conflicts ({conflicts.length})</h2>
          <div className="space-y-2">
            {conflicts.map((conflict, idx) => (
              <div key={idx} className="p-2 bg-white rounded">
                <div className="font-medium text-red-800">{conflict.type}</div>
                <div className="text-sm text-gray-600">{conflict.description}</div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

