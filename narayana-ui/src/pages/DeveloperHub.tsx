import { useState } from 'react'
import { Code, Database, Book, Zap, Terminal } from 'lucide-react'
import SQLQuery from '../components/DeveloperHub/SQLQuery'
import RESTPlayground from '../components/DeveloperHub/RESTPlayground'
import GraphQLPlayground from '../components/DeveloperHub/GraphQLPlayground'
import APIDocs from '../components/DeveloperHub/APIDocs'

type Tab = 'sql' | 'rest' | 'graphql' | 'docs'

export default function DeveloperHub() {
  const [activeTab, setActiveTab] = useState<Tab>('sql')

  const tabs = [
    { id: 'sql' as Tab, name: 'SQL Query', icon: Terminal },
    { id: 'rest' as Tab, name: 'REST API', icon: Code },
    { id: 'graphql' as Tab, name: 'GraphQL', icon: Zap },
    { id: 'docs' as Tab, name: 'API Docs', icon: Book },
  ]

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold text-gray-900 flex items-center gap-3">
          <Database className="w-8 h-8 text-primary-600" />
          Developer Hub
        </h1>
        <p className="text-gray-600 mt-1">
          Comprehensive tools for developing with NarayanaDB
        </p>
      </div>

      {/* Tabs */}
      <div className="border-b border-gray-200">
        <nav className="-mb-px flex space-x-8">
          {tabs.map((tab) => {
            const Icon = tab.icon
            return (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={`
                  flex items-center gap-2 py-4 px-1 border-b-2 font-medium text-sm transition-colors
                  ${
                    activeTab === tab.id
                      ? 'border-primary-500 text-primary-600'
                      : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
                  }
                `}
              >
                <Icon className="w-5 h-5" />
                {tab.name}
              </button>
            )
          })}
        </nav>
      </div>

      {/* Tab Content */}
      <div className="mt-6">
        {activeTab === 'sql' && <SQLQuery />}
        {activeTab === 'rest' && <RESTPlayground />}
        {activeTab === 'graphql' && <GraphQLPlayground />}
        {activeTab === 'docs' && <APIDocs />}
      </div>
    </div>
  )
}



