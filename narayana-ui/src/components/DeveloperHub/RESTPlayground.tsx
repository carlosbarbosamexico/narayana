import { useState } from 'react'
import { Play, Copy, Check, Trash2, Save } from 'lucide-react'

type Method = 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH'

interface Request {
  method: Method
  url: string
  headers: Record<string, string>
  body: string
}

export default function RESTPlayground() {
  const [method, setMethod] = useState<Method>('GET')
  const [url, setUrl] = useState('/api/v1/tables')
  const [headers, setHeaders] = useState<Record<string, string>>({
    'Content-Type': 'application/json',
  })
  const [body, setBody] = useState('')
  const [response, setResponse] = useState<any>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const [copied, setCopied] = useState(false)
  const [savedRequests, setSavedRequests] = useState<Request[]>([])

  const baseUrl = 'http://localhost:8080'

  const handleExecute = async () => {
    setLoading(true)
    setError('')
    setResponse(null)

    try {
      const fullUrl = url.startsWith('http') ? url : `${baseUrl}${url}`
      const options: RequestInit = {
        method,
        headers: {
          ...headers,
          Authorization: `Bearer ${localStorage.getItem('token') || ''}`,
        },
      }

      if (method !== 'GET' && body) {
        try {
          options.body = JSON.stringify(JSON.parse(body))
        } catch {
          options.body = body
        }
      }

      const res = await fetch(fullUrl, options)
      const data = await res.json()

      setResponse({
        status: res.status,
        statusText: res.statusText,
        headers: Object.fromEntries(res.headers.entries()),
        data,
      })
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Request failed')
    } finally {
      setLoading(false)
    }
  }

  const copyResponse = () => {
    if (!response) return
    navigator.clipboard.writeText(JSON.stringify(response, null, 2))
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const saveRequest = () => {
    const request: Request = { method, url, headers, body }
    setSavedRequests([...savedRequests, request])
  }

  const loadRequest = (request: Request) => {
    setMethod(request.method)
    setUrl(request.url)
    setHeaders(request.headers)
    setBody(request.body)
  }

  const addHeader = () => {
    setHeaders({ ...headers, '': '' })
  }

  const updateHeader = (key: string, value: string, oldKey?: string) => {
    const newHeaders = { ...headers }
    if (oldKey && oldKey !== key) {
      delete newHeaders[oldKey]
    }
    if (key && value) {
      newHeaders[key] = value
    } else if (!key || !value) {
      delete newHeaders[key]
    }
    setHeaders(newHeaders)
  }

  return (
    <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
      {/* Request Panel */}
      <div className="lg:col-span-2 space-y-6">
        <div className="card">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">Request</h2>
          
          <div className="space-y-4">
            {/* Method and URL */}
            <div className="flex gap-2">
              <select
                value={method}
                onChange={(e) => setMethod(e.target.value as Method)}
                className="input w-32"
              >
                <option>GET</option>
                <option>POST</option>
                <option>PUT</option>
                <option>PATCH</option>
                <option>DELETE</option>
              </select>
              <input
                type="text"
                value={url}
                onChange={(e) => setUrl(e.target.value)}
                placeholder="/api/v1/tables"
                className="input flex-1 font-mono text-sm"
              />
              <button
                onClick={handleExecute}
                disabled={loading}
                className="btn-primary flex items-center gap-2"
              >
                <Play className="w-4 h-4" />
                Send
              </button>
            </div>

            {/* Headers */}
            <div>
              <div className="flex items-center justify-between mb-2">
                <label className="text-sm font-medium text-gray-700">Headers</label>
                <button
                  onClick={addHeader}
                  className="text-sm text-primary-600 hover:text-primary-700"
                >
                  + Add Header
                </button>
              </div>
              <div className="space-y-2 border rounded-lg p-3 bg-gray-50">
                {Object.entries(headers).map(([key, value], idx) => (
                  <div key={idx} className="flex gap-2">
                    <input
                      type="text"
                      value={key}
                      onChange={(e) => updateHeader(e.target.value, value, key)}
                      placeholder="Header name"
                      className="input flex-1 text-sm font-mono"
                    />
                    <input
                      type="text"
                      value={value}
                      onChange={(e) => updateHeader(key, e.target.value)}
                      placeholder="Header value"
                      className="input flex-1 text-sm font-mono"
                    />
                  </div>
                ))}
              </div>
            </div>

            {/* Body */}
            {method !== 'GET' && (
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Request Body
                </label>
                <textarea
                  value={body}
                  onChange={(e) => setBody(e.target.value)}
                  placeholder='{"key": "value"}'
                  className="input font-mono text-sm min-h-[200px]"
                  spellCheck={false}
                />
              </div>
            )}

            {/* Actions */}
            <div className="flex gap-2">
              <button onClick={saveRequest} className="btn-secondary flex items-center gap-2">
                <Save className="w-4 h-4" />
                Save Request
              </button>
            </div>

            {error && (
              <div className="bg-red-50 border border-red-200 text-red-800 px-4 py-3 rounded-lg">
                <strong>Error:</strong> {error}
              </div>
            )}
          </div>
        </div>

        {/* Response */}
        {response && (
          <div className="card">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold text-gray-900">Response</h2>
              <div className="flex items-center gap-2">
                <span
                  className={`px-3 py-1 rounded-full text-sm font-medium ${
                    response.status >= 200 && response.status < 300
                      ? 'bg-green-100 text-green-800'
                      : response.status >= 400
                      ? 'bg-red-100 text-red-800'
                      : 'bg-yellow-100 text-yellow-800'
                  }`}
                >
                  {response.status} {response.statusText}
                </span>
                <button
                  onClick={copyResponse}
                  className="btn-secondary flex items-center gap-2"
                >
                  {copied ? (
                    <>
                      <Check className="w-4 h-4" />
                      Copied
                    </>
                  ) : (
                    <>
                      <Copy className="w-4 h-4" />
                      Copy
                    </>
                  )}
                </button>
              </div>
            </div>

            <div className="space-y-4">
              <div>
                <h3 className="text-sm font-medium text-gray-700 mb-2">Headers</h3>
                <pre className="bg-gray-50 p-3 rounded text-xs overflow-x-auto">
                  {JSON.stringify(response.headers, null, 2)}
                </pre>
              </div>
              <div>
                <h3 className="text-sm font-medium text-gray-700 mb-2">Body</h3>
                <pre className="bg-gray-50 p-3 rounded text-xs overflow-x-auto max-h-[500px] overflow-y-auto">
                  {JSON.stringify(response.data, null, 2)}
                </pre>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Sidebar */}
      <div className="space-y-6">
        {/* Saved Requests */}
        {savedRequests.length > 0 && (
          <div className="card">
            <h3 className="text-sm font-semibold text-gray-900 mb-3">Saved Requests</h3>
            <div className="space-y-2">
              {savedRequests.map((req, idx) => (
                <div
                  key={idx}
                  className="p-2 bg-gray-50 rounded hover:bg-gray-100 cursor-pointer"
                  onClick={() => loadRequest(req)}
                >
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-mono text-gray-600">{req.method}</span>
                    <button
                      onClick={(e) => {
                        e.stopPropagation()
                        setSavedRequests(savedRequests.filter((_, i) => i !== idx))
                      }}
                      className="text-red-600 hover:text-red-700"
                    >
                      <Trash2 className="w-3 h-3" />
                    </button>
                  </div>
                  <div className="text-xs text-gray-500 truncate mt-1">{req.url}</div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Quick Actions */}
        <div className="card">
          <h3 className="text-sm font-semibold text-gray-900 mb-3">Quick Actions</h3>
          <div className="space-y-2">
            <button
              onClick={() => {
                setMethod('GET')
                setUrl('/api/v1/tables')
                setBody('')
              }}
              className="w-full text-left px-3 py-2 text-sm bg-gray-50 hover:bg-gray-100 rounded"
            >
              GET /api/v1/tables
            </button>
            <button
              onClick={() => {
                setMethod('GET')
                setUrl('/api/v1/stats')
                setBody('')
              }}
              className="w-full text-left px-3 py-2 text-sm bg-gray-50 hover:bg-gray-100 rounded"
            >
              GET /api/v1/stats
            </button>
            <button
              onClick={() => {
                setMethod('POST')
                setUrl('/api/v1/tables')
                setBody(JSON.stringify({ table_name: 'my_table', schema: { fields: [] } }, null, 2))
              }}
              className="w-full text-left px-3 py-2 text-sm bg-gray-50 hover:bg-gray-100 rounded"
            >
              POST /api/v1/tables
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}



