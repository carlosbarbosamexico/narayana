import { useState } from 'react'
import { ChevronRight, ChevronDown, Code, Database, Zap, Brain } from 'lucide-react'

type Section = 'overview' | 'tables' | 'brains' | 'webhooks' | 'stats' | 'graphql'

export default function APIDocs() {
  const [expandedSections, setExpandedSections] = useState<Set<Section>>(new Set(['overview']))

  const toggleSection = (section: Section) => {
    const newExpanded = new Set(expandedSections)
    if (newExpanded.has(section)) {
      newExpanded.delete(section)
    } else {
      newExpanded.add(section)
    }
    setExpandedSections(newExpanded)
  }

  const sections = [
    {
      id: 'overview' as Section,
      title: 'API Overview',
      icon: Code,
      content: (
        <div className="space-y-4">
          <div>
            <h3 className="font-semibold text-gray-900 mb-2">Base URL</h3>
            <code className="bg-gray-100 px-2 py-1 rounded text-sm">http://localhost:8080/api/v1</code>
          </div>
          <div>
            <h3 className="font-semibold text-gray-900 mb-2">Authentication</h3>
            <p className="text-gray-700 text-sm mb-2">
              Most endpoints require authentication via Bearer token:
            </p>
            <pre className="bg-gray-100 p-3 rounded text-xs overflow-x-auto">
{`Authorization: Bearer <your_token>`}
            </pre>
          </div>
          <div>
            <h3 className="font-semibold text-gray-900 mb-2">Response Format</h3>
            <p className="text-gray-700 text-sm">
              All responses are JSON. Errors follow this format:
            </p>
            <pre className="bg-gray-100 p-3 rounded text-xs overflow-x-auto mt-2">
{`{
  "error": "Error message",
  "code": "ERROR_CODE"
}`}
            </pre>
          </div>
        </div>
      ),
    },
    {
      id: 'tables' as Section,
      title: 'Tables API',
      icon: Database,
      content: (
        <div className="space-y-6">
          <div>
            <h3 className="font-semibold text-gray-900 mb-2">GET /api/v1/tables</h3>
            <p className="text-gray-700 text-sm mb-3">List all tables</p>
            <pre className="bg-gray-100 p-3 rounded text-xs overflow-x-auto">
{`GET /api/v1/tables
Authorization: Bearer <token>

Response:
{
  "tables": [
    {
      "id": 1,
      "name": "users",
      "schema": {
        "fields": [
          { "name": "id", "data_type": "Int64" },
          { "name": "name", "data_type": "String" }
        ]
      },
      "row_count": 100
    }
  ]
}`}
            </pre>
          </div>
          <div>
            <h3 className="font-semibold text-gray-900 mb-2">POST /api/v1/tables</h3>
            <p className="text-gray-700 text-sm mb-3">Create a new table</p>
            <pre className="bg-gray-100 p-3 rounded text-xs overflow-x-auto">
{`POST /api/v1/tables
Content-Type: application/json
Authorization: Bearer <token>

{
  "table_name": "users",
  "schema": {
    "fields": [
      { "name": "id", "data_type": "Int64", "nullable": false },
      { "name": "name", "data_type": "String", "nullable": false }
    ]
  }
}`}
            </pre>
          </div>
          <div>
            <h3 className="font-semibold text-gray-900 mb-2">GET /api/v1/tables/:id/query</h3>
            <p className="text-gray-700 text-sm mb-3">Query table data</p>
            <pre className="bg-gray-100 p-3 rounded text-xs overflow-x-auto">
{`GET /api/v1/tables/1/query?columns=0,1,2&limit=100
Authorization: Bearer <token>

Response:
{
  "columns": [
    [1, 2, 3],
    ["Alice", "Bob", "Charlie"]
  ],
  "row_count": 3
}`}
            </pre>
          </div>
          <div>
            <h3 className="font-semibold text-gray-900 mb-2">POST /api/v1/tables/:id/insert</h3>
            <p className="text-gray-700 text-sm mb-3">Insert data into a table</p>
            <pre className="bg-gray-100 p-3 rounded text-xs overflow-x-auto">
{`POST /api/v1/tables/1/insert
Content-Type: application/json
Authorization: Bearer <token>

{
  "columns": [
    [1, 2, 3],
    ["Alice", "Bob", "Charlie"]
  ]
}`}
            </pre>
          </div>
          <div>
            <h3 className="font-semibold text-gray-900 mb-2">DELETE /api/v1/tables/:id</h3>
            <p className="text-gray-700 text-sm mb-3">Delete a table</p>
            <pre className="bg-gray-100 p-3 rounded text-xs overflow-x-auto">
{`DELETE /api/v1/tables/1
Authorization: Bearer <token>`}
            </pre>
          </div>
        </div>
      ),
    },
    {
      id: 'brains' as Section,
      title: 'Brains API',
      icon: Brain,
      content: (
        <div className="space-y-6">
          <div>
            <h3 className="font-semibold text-gray-900 mb-2">GET /api/v1/brains</h3>
            <p className="text-gray-700 text-sm mb-3">List all cognitive brains</p>
            <pre className="bg-gray-100 p-3 rounded text-xs overflow-x-auto">
{`GET /api/v1/brains
Authorization: Bearer <token>

Response:
{
  "brains": [
    {
      "brain_id": "robot-1",
      "memory_types": ["episodic", "semantic"],
      "created_at": 1234567890
    }
  ]
}`}
            </pre>
          </div>
          <div>
            <h3 className="font-semibold text-gray-900 mb-2">POST /api/v1/brains</h3>
            <p className="text-gray-700 text-sm mb-3">Create a new brain</p>
            <pre className="bg-gray-100 p-3 rounded text-xs overflow-x-auto">
{`POST /api/v1/brains
Content-Type: application/json
Authorization: Bearer <token>

{
  "brain_id": "robot-1",
  "memory_types": ["episodic", "semantic"]
}`}
            </pre>
          </div>
        </div>
      ),
    },
    {
      id: 'stats' as Section,
      title: 'Stats API',
      icon: Zap,
      content: (
        <div className="space-y-6">
          <div>
            <h3 className="font-semibold text-gray-900 mb-2">GET /api/v1/stats</h3>
            <p className="text-gray-700 text-sm mb-3">Get query statistics</p>
            <pre className="bg-gray-100 p-3 rounded text-xs overflow-x-auto">
{`GET /api/v1/stats
Authorization: Bearer <token>

Response:
{
  "total_queries": 1000,
  "avg_duration_ms": 12.5,
  "total_rows_read": 50000,
  "total_rows_inserted": 10000
}`}
            </pre>
          </div>
          <div>
            <h3 className="font-semibold text-gray-900 mb-2">GET /api/v1/system/stats</h3>
            <p className="text-gray-700 text-sm mb-3">Get system statistics</p>
            <pre className="bg-gray-100 p-3 rounded text-xs overflow-x-auto">
{`GET /api/v1/system/stats
Authorization: Bearer <token>

Response:
{
  "total_queries": 1000,
  "avg_latency_ms": 12.5,
  "total_rows_read": 50000,
  "total_rows_inserted": 10000,
  "tables": 10,
  "brains": 2,
  "workers": 5,
  "active_connections": 3
}`}
            </pre>
          </div>
        </div>
      ),
    },
    {
      id: 'graphql' as Section,
      title: 'GraphQL API',
      icon: Code,
      content: (
        <div className="space-y-6">
          <div>
            <h3 className="font-semibold text-gray-900 mb-2">Endpoint</h3>
            <code className="bg-gray-100 px-2 py-1 rounded text-sm">POST /graphql</code>
          </div>
          <div>
            <h3 className="font-semibold text-gray-900 mb-2">Query Tables</h3>
            <pre className="bg-gray-100 p-3 rounded text-xs overflow-x-auto">
{`query {
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
}`}
            </pre>
          </div>
          <div>
            <h3 className="font-semibold text-gray-900 mb-2">Create Table Mutation</h3>
            <pre className="bg-gray-100 p-3 rounded text-xs overflow-x-auto">
{`mutation {
  createTable(input: {
    name: "users"
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
}`}
            </pre>
          </div>
        </div>
      ),
    },
  ]

  return (
    <div className="space-y-4">
      {sections.map((section) => {
        const Icon = section.icon
        const isExpanded = expandedSections.has(section.id)
        return (
          <div key={section.id} className="card">
            <button
              onClick={() => toggleSection(section.id)}
              className="w-full flex items-center justify-between text-left"
            >
              <div className="flex items-center gap-3">
                <Icon className="w-5 h-5 text-primary-600" />
                <h2 className="text-lg font-semibold text-gray-900">{section.title}</h2>
              </div>
              {isExpanded ? (
                <ChevronDown className="w-5 h-5 text-gray-500" />
              ) : (
                <ChevronRight className="w-5 h-5 text-gray-500" />
              )}
            </button>
            {isExpanded && (
              <div className="mt-4 pt-4 border-t border-gray-200">
                {section.content}
              </div>
            )}
          </div>
        )
      })}
    </div>
  )
}



