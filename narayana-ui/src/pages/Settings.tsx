export default function Settings() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold text-gray-900">Settings</h1>
        <p className="text-gray-600 mt-1">Configure your NarayanaDB instance</p>
      </div>

      <div className="card">
        <h2 className="text-lg font-semibold text-gray-900 mb-4">Server Configuration</h2>
        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Server URL
            </label>
            <input
              type="text"
              defaultValue="http://localhost:8080"
              className="input"
              disabled
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              API Version
            </label>
            <input type="text" defaultValue="v1" className="input" disabled />
          </div>
        </div>
      </div>

      <div className="card">
        <h2 className="text-lg font-semibold text-gray-900 mb-4">Performance Settings</h2>
        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Refresh Interval (ms)
            </label>
            <input type="number" defaultValue="5000" className="input" />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Max Query Results
            </label>
            <input type="number" defaultValue="10000" className="input" />
          </div>
        </div>
      </div>

      <div className="card">
        <h2 className="text-lg font-semibold text-gray-900 mb-4">About</h2>
        <div className="space-y-2 text-sm text-gray-600">
          <p><strong>Version:</strong> 0.1.0</p>
          <p><strong>Build:</strong> Development</p>
          <p><strong>Framework:</strong> React + TypeScript + Vite</p>
        </div>
      </div>
    </div>
  )
}

