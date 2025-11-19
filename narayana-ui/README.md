# NarayanaDB UI

Enterprise-grade web interface for NarayanaDB performance monitoring and management.

## Features

- **Real-time Dashboard**: Monitor queries, latency, and throughput
- **Table Management**: Create, view, and delete tables
- **Query Interface**: Execute queries with visual results
- **Performance Monitoring**: Real-time metrics and charts
- **Settings**: Configure your database instance

## Tech Stack

- **React 18** - Modern UI library
- **TypeScript** - Type-safe development
- **Vite** - Fast build tool
- **Tailwind CSS** - Utility-first styling
- **Recharts** - Beautiful charts
- **TanStack Query** - Data fetching and caching
- **React Router** - Client-side routing

## Development

```bash
# Install dependencies
npm install

# Start development server
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview
```

The UI will be available at `http://localhost:3000` and proxies API requests to `http://localhost:8080`.

## Integration

The UI is automatically served by the NarayanaDB server when built. The server will:
1. Serve the built UI files on the main port
2. Proxy API requests to `/api/v1/*`
3. Provide real-time metrics via WebSocket (future)

## Building for Production

```bash
npm run build
```

The `dist/` folder contains the production-ready static files that can be served by the Rust server.

