import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useParams, useNavigate } from 'react-router-dom'
import { apiClient } from '../lib/api'
import { ArrowLeft, Edit2, Save, X, Plus, Download, Search } from 'lucide-react'
import { useState } from 'react'

export default function TableDetail() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [editingRow, setEditingRow] = useState<number | null>(null)
  const [editedData, setEditedData] = useState<any>(null)
  const [showInsertModal, setShowInsertModal] = useState(false)
  const [newRowData, setNewRowData] = useState<any[]>([])
  const [searchTerm, setSearchTerm] = useState('')
  const [currentPage, setCurrentPage] = useState(1)
  const [rowsPerPage, setRowsPerPage] = useState(50)

  const tableId = id ? parseInt(id, 10) : 0

  const { data: tables } = useQuery({
    queryKey: ['tables'],
    queryFn: apiClient.getTables,
  })

  const table = tables?.find((t) => t.id === tableId)

  const { data: tableData, isLoading } = useQuery({
    queryKey: ['tableData', tableId],
    queryFn: () => apiClient.queryData(tableId, { limit: 1000 }),
    enabled: !!tableId && tableId > 0,
  })

  const startEdit = (rowIndex: number) => {
    if (!tableData?.columns) return
    
    const row: any = {}
    tableData.columns.forEach((col: any, idx: number) => {
      row[idx] = col[rowIndex] !== undefined ? col[rowIndex] : null
    })
    setEditedData(row)
    setEditingRow(rowIndex)
  }

  const cancelEdit = () => {
    setEditingRow(null)
    setEditedData(null)
  }

  const saveEdit = async () => {
    if (!tableData?.columns || editingRow === null || !editedData) return

    // For now, we'll need to delete and re-insert
    // In a real implementation, you'd have an update endpoint
    try {
      // Prepare new row data from edited values
      const newRow: any[] = tableData.columns.map((col: any, idx: number) => {
        return editedData[idx] !== undefined ? editedData[idx] : col[editingRow]
      })

      // Note: This is a simplified implementation
      // In production, you'd want an update endpoint
      await apiClient.insertData(tableId, [newRow])
      
      queryClient.invalidateQueries({ queryKey: ['tableData', tableId] })
      cancelEdit()
    } catch (error) {
      console.error('Failed to save row:', error)
      alert('Failed to save changes. Note: Update functionality requires a proper update endpoint.')
    }
  }

  const handleInsert = async () => {
    if (newRowData.length === 0) return

    try {
      await apiClient.insertData(tableId, [newRowData])
      setShowInsertModal(false)
      setNewRowData([])
      queryClient.invalidateQueries({ queryKey: ['tableData', tableId] })
    } catch (error) {
      console.error('Failed to insert row:', error)
      alert('Failed to insert row')
    }
  }

  // Convert columnar data to rows
  const allRows: any[][] = []
  if (tableData?.columns && tableData.columns.length > 0) {
    const rowCount = tableData.columns[0].length
    for (let i = 0; i < rowCount; i++) {
      const row: any[] = tableData.columns.map((col: any) => col[i])
      allRows.push(row)
    }
  }

  // Filter rows based on search term
  const filteredRows = allRows.filter((row) => {
    if (!searchTerm) return true
    const searchLower = searchTerm.toLowerCase()
    return row.some((cell) => 
      cell !== null && cell !== undefined && String(cell).toLowerCase().includes(searchLower)
    )
  })

  // Pagination
  const totalPages = Math.ceil(filteredRows.length / rowsPerPage)
  const startIndex = (currentPage - 1) * rowsPerPage
  const endIndex = startIndex + rowsPerPage
  const rows = filteredRows.slice(startIndex, endIndex)

  const columnNames = table?.schema?.fields?.map((f: any) => f.name) || []

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <button
            onClick={() => navigate('/tables')}
            className="p-2 hover:bg-gray-100 rounded-lg transition-colors"
          >
            <ArrowLeft className="w-5 h-5" />
          </button>
          <div>
            <h1 className="text-3xl font-bold text-gray-900">
              {table?.name || `Table ${tableId}`}
            </h1>
            <p className="text-gray-600 mt-1">
              {table?.row_count || 0} rows • {columnNames.length} columns
            </p>
          </div>
        </div>
        <div className="flex items-center gap-3">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 w-4 h-4 text-gray-400" />
            <input
              type="text"
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              placeholder="Search rows..."
              className="input pl-10 w-64"
            />
          </div>
          <button
            onClick={() => {
              if (table?.schema?.fields) {
                setNewRowData(new Array(table.schema.fields.length).fill(null))
                setShowInsertModal(true)
              }
            }}
            className="btn-primary flex items-center gap-2"
          >
            <Plus className="w-5 h-5" />
            Insert Row
          </button>
          <button
            onClick={() => {
              if (tableData?.columns) {
                const json = JSON.stringify(tableData, null, 2)
                const blob = new Blob([json], { type: 'application/json' })
                const url = URL.createObjectURL(blob)
                const a = document.createElement('a')
                a.href = url
                a.download = `${table?.name || 'table'}-${Date.now()}.json`
                a.click()
                URL.revokeObjectURL(url)
              }
            }}
            className="btn-secondary flex items-center gap-2"
          >
            <Download className="w-5 h-5" />
            Export
          </button>
        </div>
      </div>

      {isLoading ? (
        <div className="card text-center py-12">
          <p className="text-gray-500">Loading table data...</p>
        </div>
      ) : rows.length === 0 ? (
        <div className="card text-center py-12">
          <p className="text-gray-500">No data in this table</p>
        </div>
      ) : (
        <div className="card overflow-x-auto">
          <table className="w-full">
            <thead>
              <tr className="border-b border-gray-200">
                {columnNames.map((name: string, idx: number) => (
                  <th key={idx} className="text-left p-3 font-semibold text-gray-700">
                    {name}
                  </th>
                ))}
                <th className="text-right p-3 font-semibold text-gray-700">Actions</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((row, displayIndex) => {
                const actualRowIndex = startIndex + displayIndex
                return (
                <tr key={actualRowIndex} className="border-b border-gray-100 hover:bg-gray-50">
                  {row.map((cell, cellIndex) => (
                    <td key={cellIndex} className="p-3 text-gray-900">
                      {editingRow === actualRowIndex ? (
                        <input
                          type="text"
                          value={editedData?.[cellIndex] ?? ''}
                          onChange={(e) => {
                            setEditedData({
                              ...editedData,
                              [cellIndex]: e.target.value,
                            })
                          }}
                          className="input text-sm"
                        />
                      ) : (
                        <span className="text-sm">
                          {cell !== null && cell !== undefined ? String(cell) : 'NULL'}
                        </span>
                      )}
                    </td>
                  ))}
                  <td className="p-3 text-right">
                    {editingRow === actualRowIndex ? (
                      <div className="flex items-center justify-end gap-2">
                        <button
                          onClick={saveEdit}
                          className="p-2 text-green-600 hover:bg-green-50 rounded-lg transition-colors"
                          title="Save"
                        >
                          <Save className="w-4 h-4" />
                        </button>
                        <button
                          onClick={cancelEdit}
                          className="p-2 text-gray-600 hover:bg-gray-100 rounded-lg transition-colors"
                          title="Cancel"
                        >
                          <X className="w-4 h-4" />
                        </button>
                      </div>
                    ) : (
                      <button
                        onClick={() => startEdit(actualRowIndex)}
                        className="p-2 text-blue-600 hover:bg-blue-50 rounded-lg transition-colors"
                        title="Edit"
                      >
                        <Edit2 className="w-4 h-4" />
                      </button>
                    )}
                  </td>
                </tr>
                )
              })}
            </tbody>
          </table>
          
          {/* Pagination */}
          {filteredRows.length > rowsPerPage && (
            <div className="flex items-center justify-between px-4 py-3 border-t border-gray-200">
              <div className="flex items-center gap-2">
                <span className="text-sm text-gray-700">Rows per page:</span>
                <select
                  value={rowsPerPage}
                  onChange={(e) => {
                    setRowsPerPage(Number(e.target.value))
                    setCurrentPage(1)
                  }}
                  className="input text-sm w-20"
                >
                  <option value={25}>25</option>
                  <option value={50}>50</option>
                  <option value={100}>100</option>
                  <option value={250}>250</option>
                </select>
              </div>
              <div className="flex items-center gap-2">
                <span className="text-sm text-gray-700">
                  Showing {startIndex + 1} to {Math.min(endIndex, filteredRows.length)} of {filteredRows.length} rows
                </span>
                <div className="flex gap-1">
                  <button
                    onClick={() => setCurrentPage((p) => Math.max(1, p - 1))}
                    disabled={currentPage === 1}
                    className="px-3 py-1 text-sm border rounded hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    Previous
                  </button>
                  <button
                    onClick={() => setCurrentPage((p) => Math.min(totalPages, p + 1))}
                    disabled={currentPage === totalPages}
                    className="px-3 py-1 text-sm border rounded hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    Next
                  </button>
                </div>
              </div>
            </div>
          )}
        </div>
      )}

      {/* Insert Modal */}
      {showInsertModal && table?.schema?.fields && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 w-full max-w-2xl max-h-[80vh] overflow-y-auto">
            <h2 className="text-xl font-bold text-gray-900 mb-4">Insert New Row</h2>
            <div className="space-y-4">
              {table.schema.fields.map((field: any, idx: number) => (
                <div key={idx}>
                  <label className="block text-sm font-medium text-gray-700 mb-2">
                    {field.name} ({field.data_type})
                  </label>
                  <input
                    type="text"
                    value={newRowData[idx] ?? ''}
                    onChange={(e) => {
                      const updated = [...newRowData]
                      updated[idx] = e.target.value
                      setNewRowData(updated)
                    }}
                    className="input"
                    placeholder={`Enter ${field.name}`}
                  />
                </div>
              ))}
              <div className="flex gap-3 justify-end pt-4">
                <button
                  onClick={() => {
                    setShowInsertModal(false)
                    setNewRowData([])
                  }}
                  className="btn-secondary"
                >
                  Cancel
                </button>
                <button onClick={handleInsert} className="btn-primary">
                  Insert
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

