import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { apiClient } from '../../lib/api'
import { Play, Download, Copy, Check } from 'lucide-react'

export default function SQLQuery() {
  const [query, setQuery] = useState('')
  const [results, setResults] = useState<any>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const [copied, setCopied] = useState(false)

  const { data: tables } = useQuery({
    queryKey: ['tables'],
    queryFn: apiClient.getTables,
  })

  const handleExecute = async () => {
    if (!query.trim()) return

    setLoading(true)
    setError('')
    try {
      // Parse SQL-like query (simplified - in production, use proper SQL parser)
      const trimmed = query.trim().toUpperCase()
      
      if (trimmed.startsWith('SELECT')) {
        // Extract table name and columns
        const match = query.match(/FROM\s+(\w+)/i)
        if (match) {
          const tableName = match[1]
          const table = tables?.find((t) => t.name === tableName)
          
          if (table) {
            const data = await apiClient.queryData(table.id, { limit: 1000 })
            setResults(data)
          } else {
            setError(`Table '${tableName}' not found`)
          }
        } else {
          setError('Invalid SELECT query. Use: SELECT * FROM table_name')
        }
      } else if (trimmed.startsWith('SHOW TABLES')) {
        setResults({
          tables: tables || [],
          row_count: tables?.length || 0,
        })
      } else {
        setError('Only SELECT and SHOW TABLES queries are supported in this interface')
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Query execution failed')
    } finally {
      setLoading(false)
    }
  }

  const exportResults = () => {
    if (!results) return
    
    const json = JSON.stringify(results, null, 2)
    const blob = new Blob([json], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `query-results-${Date.now()}.json`
    a.click()
    URL.revokeObjectURL(url)
  }

  const copyResults = () => {
    if (!results) return
    navigator.clipboard.writeText(JSON.stringify(results, null, 2))
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <div className="space-y-6">
      <div className="card">
        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              SQL Query
            </label>
            <textarea
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="SELECT * FROM table_name&#10;SHOW TABLES"
              className="input font-mono text-sm min-h-[200px]"
              spellCheck={false}
            />
          </div>

          <div className="flex items-center gap-3">
            <button
              onClick={handleExecute}
              disabled={!query.trim() || loading}
              className="btn-primary flex items-center gap-2"
            >
              <Play className="w-4 h-4" />
              {loading ? 'Executing...' : 'Execute Query'}
            </button>
            
            {results && (
              <>
                <button
                  onClick={exportResults}
                  className="btn-secondary flex items-center gap-2"
                >
                  <Download className="w-4 h-4" />
                  Export JSON
                </button>
                <button
                  onClick={copyResults}
                  className="btn-secondary flex items-center gap-2"
                >
                  {copied ? (
                    <>
                      <Check className="w-4 h-4" />
                      Copied!
                    </>
                  ) : (
                    <>
                      <Copy className="w-4 h-4" />
                      Copy
                    </>
                  )}
                </button>
              </>
            )}
          </div>

          {error && (
            <div className="bg-red-50 border border-red-200 text-red-800 px-4 py-3 rounded-lg">
              <strong>Error:</strong> {error}
            </div>
          )}
        </div>
      </div>

      {results && (
        <div className="card">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-gray-900">Results</h2>
            <span className="text-sm text-gray-500">
              {results.row_count || results.tables?.length || 0} rows
            </span>
          </div>
          
          <div className="overflow-x-auto">
            {results.tables ? (
              <table className="w-full text-sm">
                <thead className="bg-gray-50">
                  <tr>
                    <th className="px-4 py-3 text-left font-semibold text-gray-700">Table Name</th>
                    <th className="px-4 py-3 text-left font-semibold text-gray-700">ID</th>
                    <th className="px-4 py-3 text-left font-semibold text-gray-700">Rows</th>
                    <th className="px-4 py-3 text-left font-semibold text-gray-700">Columns</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-200">
                  {results.tables.map((table: any) => (
                    <tr key={table.id} className="hover:bg-gray-50">
                      <td className="px-4 py-3 text-gray-900">{table.name}</td>
                      <td className="px-4 py-3 text-gray-600">{table.id}</td>
                      <td className="px-4 py-3 text-gray-600">{table.row_count || 0}</td>
                      <td className="px-4 py-3 text-gray-600">
                        {table.schema?.fields?.length || 0}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            ) : results.columns ? (
              <div className="overflow-x-auto">
                <table className="w-full text-sm border-collapse">
                  <thead className="bg-gray-50">
                    <tr>
                      {results.columns.map((_col: any, idx: number) => (
                        <th key={idx} className="px-4 py-3 text-left font-semibold text-gray-700 border border-gray-200">
                          Column {idx}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-gray-200">
                    {Array.from({ length: results.row_count || 0 }).map((_, rowIdx) => (
                      <tr key={rowIdx} className="hover:bg-gray-50">
                        {results.columns.map((col: any, colIdx: number) => (
                          <td key={colIdx} className="px-4 py-3 text-gray-900 border border-gray-200">
                            {col[rowIdx] !== undefined && col[rowIdx] !== null
                              ? String(col[rowIdx])
                              : 'NULL'}
                          </td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <pre className="bg-gray-50 p-4 rounded-lg overflow-x-auto text-sm">
                {JSON.stringify(results, null, 2)}
              </pre>
            )}
          </div>
        </div>
      )}
    </div>
  )
}



