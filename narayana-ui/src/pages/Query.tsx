import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { apiClient } from '../lib/api'
import { Play } from 'lucide-react'

export default function Query() {
  const [tableId, setTableId] = useState('')
  const [columns, setColumns] = useState('')
  const [limit, setLimit] = useState('1000')
  const [results, setResults] = useState<any>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  const { data: tables } = useQuery({
    queryKey: ['tables'],
    queryFn: apiClient.getTables,
  })

  const handleQuery = async () => {
    if (!tableId) return

    setLoading(true)
    try {
      const params: any = {}
      if (columns) params.columns = columns
      if (limit) {
        // SECURITY: Validate limit is a valid number
        const parsedLimit = parseInt(limit, 10)
        if (!isNaN(parsedLimit) && isFinite(parsedLimit) && parsedLimit > 0 && parsedLimit <= 10000) {
          params.limit = parsedLimit
        } else {
          setError('Limit must be a number between 1 and 10000')
          setLoading(false)
          return
        }
      }

      // SECURITY: Validate tableId
      const parsedTableId = parseInt(tableId, 10)
      if (isNaN(parsedTableId) || !isFinite(parsedTableId) || parsedTableId <= 0) {
        setError('Invalid table ID')
        setLoading(false)
        return
      }

      const data = await apiClient.queryData(parsedTableId, params)
      setResults(data)
      setError('') // Clear any previous errors
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Query failed'
      setError(errorMessage)
      console.error('Query error:', err)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold text-gray-900">Query Interface</h1>
        <p className="text-gray-600 mt-1">Execute queries against your tables</p>
      </div>

      <div className="card">
        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Select Table
            </label>
            <select
              value={tableId}
              onChange={(e) => setTableId(e.target.value)}
              className="input"
            >
              <option value="">Select a table</option>
              {tables?.map((table) => (
                <option key={table.id} value={table.id}>
                  {table.name} (ID: {table.id})
                </option>
              ))}
            </select>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Columns (comma-separated indices)
              </label>
              <input
                type="text"
                value={columns}
                onChange={(e) => setColumns(e.target.value)}
                className="input"
                placeholder="0,1,2"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Limit
              </label>
              <input
                type="number"
                value={limit}
                onChange={(e) => setLimit(e.target.value)}
                className="input"
                placeholder="1000"
              />
            </div>
          </div>

          <button
            onClick={handleQuery}
            disabled={!tableId || loading}
            className="btn-primary flex items-center gap-2"
          >
            <Play className="w-5 h-5" />
            {loading ? 'Executing...' : 'Execute Query'}
          </button>

          {error && (
            <div className="bg-red-50 border border-red-200 text-red-800 px-4 py-3 rounded-lg">
              {error}
            </div>
          )}
        </div>
      </div>

      {results && (
        <div className="card">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-gray-900">Query Results</h2>
            <span className="text-sm text-gray-500">
              {results.row_count || 0} rows returned
            </span>
          </div>
          <div className="overflow-x-auto">
            <pre className="bg-gray-50 p-4 rounded-lg overflow-x-auto text-sm">
              {JSON.stringify(results, null, 2)}
            </pre>
          </div>
        </div>
      )}
    </div>
  )
}

