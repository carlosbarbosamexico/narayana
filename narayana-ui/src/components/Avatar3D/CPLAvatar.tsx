import { useAvatarWebSocket } from '../../hooks/useAvatarWebSocket'
import Avatar3D from './Avatar3D'
import { Wifi, WifiOff } from 'lucide-react'

interface CPLAvatarProps {
  cplId?: string
  enabled?: boolean
  className?: string
}

export default function CPLAvatar({ cplId, enabled = true, className = '' }: CPLAvatarProps) {
  const {
    isConnected,
    currentExpression,
    expressionIntensity,
    currentGesture,
    avatarState,
    streamUrl,
  } = useAvatarWebSocket({
    enabled,
    port: 8081,
    onOpen: () => {
      console.log('Avatar WebSocket connected for CPL:', cplId)
    },
    onClose: () => {
      console.log('Avatar WebSocket disconnected for CPL:', cplId)
    },
    onError: (error) => {
      console.error('Avatar WebSocket error:', error)
    },
  })

  if (!enabled) {
    return null
  }

  return (
    <div className={`cpl-avatar-container ${className}`}>
      <div className="relative w-full h-full min-h-[500px] bg-gray-50 rounded-lg overflow-hidden border border-gray-200">
        {/* Connection Status */}
        <div className="absolute top-2 right-2 z-10">
          <div
            className={`flex items-center gap-2 px-3 py-1 rounded-full text-xs font-medium ${
              isConnected
                ? 'bg-green-100 text-green-800'
                : 'bg-yellow-100 text-yellow-800'
            }`}
          >
            {isConnected ? (
              <>
                <Wifi className="w-3 h-3" />
                Connected
              </>
            ) : (
              <>
                <WifiOff className="w-3 h-3" />
                Connecting...
              </>
            )}
          </div>
        </div>

        {/* Avatar State Indicator */}
        {avatarState && avatarState !== 'idle' && (
          <div className="absolute top-2 left-2 z-10">
            <div className="px-3 py-1 bg-blue-100 text-blue-800 rounded-full text-xs font-medium">
              {avatarState.charAt(0).toUpperCase() + avatarState.slice(1)}
            </div>
          </div>
        )}

        {/* 3D Avatar */}
        <Avatar3D
          expression={currentExpression}
          expressionIntensity={expressionIntensity}
          gesture={currentGesture || undefined}
          gestureDuration={2000}
          className="w-full h-full"
        />

        {/* Info Panel */}
        <div className="absolute bottom-0 left-0 right-0 bg-black bg-opacity-50 text-white p-3 text-xs">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <span>
                Expression: <strong>{currentExpression}</strong> ({expressionIntensity.toFixed(2)})
              </span>
              {currentGesture && (
                <span>
                  Gesture: <strong>{currentGesture}</strong>
                </span>
              )}
              <span>State: <strong>{avatarState}</strong></span>
            </div>
            {streamUrl && (() => {
              // Validate URL before rendering link
              try {
                const url = new URL(streamUrl)
                if (['http:', 'https:', 'ws:', 'wss:'].includes(url.protocol)) {
                  return (
                    <a
                      href={streamUrl}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-blue-300 hover:text-blue-200 underline"
                    >
                      Stream URL
                    </a>
                  )
                }
              } catch {
                // Invalid URL, don't render link
              }
              return null
            })()}
          </div>
        </div>
      </div>
    </div>
  )
}

