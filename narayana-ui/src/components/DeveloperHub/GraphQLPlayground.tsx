import { useState } from 'react'
import { Play, Copy, Check, BookOpen } from 'lucide-react'

export default function GraphQLPlayground() {
  const [query, setQuery] = useState(`query {
  tables {
    id
    name
    row_count
    schema {
      fields {
        name
        data_type
      }
    }
  }
}`)
  const [variables, setVariables] = useState('')
  const [response, setResponse] = useState<any>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const [copied, setCopied] = useState(false)

  const handleExecute = async () => {
    setLoading(true)
    setError('')
    setResponse(null)

    try {
      const baseUrl = 'http://localhost:8080'
      const graphqlUrl = `${baseUrl}/graphql` // Assuming GraphQL endpoint exists

      let parsedVariables = {}
      if (variables.trim()) {
        try {
          parsedVariables = JSON.parse(variables)
        } catch {
          throw new Error('Invalid JSON in variables')
        }
      }

      const res = await fetch(graphqlUrl, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${localStorage.getItem('token') || ''}`,
        },
        body: JSON.stringify({
          query,
          variables: parsedVariables,
        }),
      })

      const data = await res.json()
      
      if (data.errors) {
        setError(data.errors.map((e: any) => e.message).join(', '))
      } else {
        setResponse(data)
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'GraphQL request failed')
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

  const loadExample = (example: string) => {
    switch (example) {
      case 'query':
        setQuery(`query {
  tables {
    id
    name
    row_count
    schema {
      fields {
        name
        data_type
      }
    }
  }
}`)
        break
      case 'mutation':
        setQuery(`mutation {
  createTable(input: {
    name: "my_table"
    schema: {
      fields: [
        { name: "id", data_type: "Int64", nullable: false }
        { name: "name", data_type: "String", nullable: false }
      ]
    }
  }) {
    id
    name
  }
}`)
        break
    }
  }

  return (
    <div className="space-y-6">
      <div className="card">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-gray-900">GraphQL Query</h2>
          <div className="flex gap-2">
            <button
              onClick={() => loadExample('query')}
              className="text-sm text-primary-600 hover:text-primary-700"
            >
              Query Example
            </button>
            <button
              onClick={() => loadExample('mutation')}
              className="text-sm text-primary-600 hover:text-primary-700"
            >
              Mutation Example
            </button>
          </div>
        </div>

        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Query
            </label>
            <textarea
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              className="input font-mono text-sm min-h-[300px]"
              spellCheck={false}
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Variables (JSON)
            </label>
            <textarea
              value={variables}
              onChange={(e) => setVariables(e.target.value)}
              placeholder='{"key": "value"}'
              className="input font-mono text-sm min-h-[100px]"
              spellCheck={false}
            />
          </div>

          <button
            onClick={handleExecute}
            disabled={loading || !query.trim()}
            className="btn-primary flex items-center gap-2"
          >
            <Play className="w-4 h-4" />
            {loading ? 'Executing...' : 'Execute Query'}
          </button>

          {error && (
            <div className="bg-red-50 border border-red-200 text-red-800 px-4 py-3 rounded-lg">
              <strong>Error:</strong> {error}
            </div>
          )}
        </div>
      </div>

      {response && (
        <div className="card">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-gray-900">Response</h2>
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
          <pre className="bg-gray-50 p-4 rounded-lg overflow-x-auto text-sm max-h-[600px] overflow-y-auto">
            {JSON.stringify(response, null, 2)}
          </pre>
        </div>
      )}

      {/* Info Box */}
      <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
        <div className="flex items-start gap-3">
          <BookOpen className="w-5 h-5 text-blue-600 mt-0.5" />
          <div>
            <h3 className="font-semibold text-blue-900 mb-1">GraphQL Endpoint</h3>
            <p className="text-sm text-blue-800">
              The GraphQL endpoint is available at <code className="bg-blue-100 px-1 rounded">/graphql</code>.
              Note: GraphQL introspection may be disabled for security. Use the API documentation
              to learn about available queries and mutations.
            </p>
          </div>
        </div>
      </div>
    </div>
  )
}



