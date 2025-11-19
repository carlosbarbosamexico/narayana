import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { apiClient } from '../lib/api'
import { Plus, Trash2, Database } from 'lucide-react'
import { useState } from 'react'

export default function Tables() {
  const queryClient = useQueryClient()
  const [showCreateModal, setShowCreateModal] = useState(false)
  const [tableName, setTableName] = useState('')

  const { data: tables, isLoading } = useQuery({
    queryKey: ['tables'],
    queryFn: apiClient.getTables,
  })

  const createMutation = useMutation({
    mutationFn: (data: { name: string; schema: any }) => apiClient.createTable(data.name, data.schema),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['tables'] })
      setShowCreateModal(false)
      setTableName('')
    },
  })

  const deleteMutation = useMutation({
    mutationFn: (tableId: number) => apiClient.deleteTable(tableId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['tables'] })
    },
  })

  const handleCreate = () => {
    const schema = {
      fields: [
        { name: 'id', data_type: 'Int64', nullable: false },
        { name: 'created_at', data_type: 'Timestamp', nullable: false },
      ],
    }
    createMutation.mutate({ name: tableName, schema })
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-gray-900">Tables</h1>
          <p className="text-gray-600 mt-1">Manage your database tables</p>
        </div>
        <button
          onClick={() => setShowCreateModal(true)}
          className="btn-primary flex items-center gap-2"
        >
          <Plus className="w-5 h-5" />
          Create Table
        </button>
      </div>

      {isLoading ? (
        <div className="card text-center py-12">
          <p className="text-gray-500">Loading tables...</p>
        </div>
      ) : tables && tables.length > 0 ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {tables.map((table) => (
            <div key={table.id} className="card">
              <div className="flex items-start justify-between">
                <div className="flex items-center gap-3">
                  <div className="p-2 bg-primary-50 rounded-lg">
                    <Database className="w-5 h-5 text-primary-600" />
                  </div>
                  <div>
                    <h3 className="font-semibold text-gray-900">{table.name}</h3>
                    <p className="text-sm text-gray-500">ID: {table.id}</p>
                  </div>
                </div>
                <button
                  onClick={() => deleteMutation.mutate(table.id)}
                  className="p-2 text-red-600 hover:bg-red-50 rounded-lg transition-colors"
                >
                  <Trash2 className="w-4 h-4" />
                </button>
              </div>
              <div className="mt-4 pt-4 border-t border-gray-200">
                <div className="flex items-center justify-between text-sm">
                  <span className="text-gray-600">Columns</span>
                  <span className="font-medium text-gray-900">
                    {table.schema?.fields?.length || 0}
                  </span>
                </div>
                <div className="flex items-center justify-between text-sm mt-2">
                  <span className="text-gray-600">Rows</span>
                  <span className="font-medium text-gray-900">
                    {table.row_count || 0}
                  </span>
                </div>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="card text-center py-12">
          <Database className="w-12 h-12 text-gray-400 mx-auto mb-4" />
          <h3 className="text-lg font-medium text-gray-900 mb-2">No tables yet</h3>
          <p className="text-gray-500 mb-4">Get started by creating your first table</p>
          <button onClick={() => setShowCreateModal(true)} className="btn-primary">
            Create Table
          </button>
        </div>
      )}

      {/* Create Modal */}
      {showCreateModal && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-full max-w-md">
            <h2 className="text-xl font-bold text-gray-900 mb-4">Create New Table</h2>
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Table Name
                </label>
                <input
                  type="text"
                  value={tableName}
                  onChange={(e) => setTableName(e.target.value)}
                  className="input"
                  placeholder="Enter table name"
                />
              </div>
              <div className="flex gap-3 justify-end">
                <button
                  onClick={() => setShowCreateModal(false)}
                  className="btn-secondary"
                >
                  Cancel
                </button>
                <button
                  onClick={handleCreate}
                  disabled={!tableName || createMutation.isPending}
                  className="btn-primary"
                >
                  {createMutation.isPending ? 'Creating...' : 'Create'}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

