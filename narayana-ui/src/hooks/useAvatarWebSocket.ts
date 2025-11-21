import { useEffect, useState, useCallback, useRef } from 'react'

export interface AvatarMessage {
  type: 'expression' | 'gesture' | 'state' | 'streamUrl' | 'audio'
  emotion?: string
  intensity?: number
  gesture?: string
  duration_ms?: number
  state?: string
  url?: string
  data?: Uint8Array
}

export interface UseAvatarWebSocketOptions {
  url?: string
  port?: number
  enabled?: boolean
  onMessage?: (message: AvatarMessage) => void
  onError?: (error: Event) => void
  onOpen?: () => void
  onClose?: () => void
}

export function useAvatarWebSocket(options: UseAvatarWebSocketOptions = {}) {
  const {
    url,
    port = 8081,
    enabled = true,
    onMessage,
    onError,
    onOpen,
    onClose,
  } = options

  const [isConnected, setIsConnected] = useState(false)
  const [currentExpression, setCurrentExpression] = useState<string>('neutral')
  const [expressionIntensity, setExpressionIntensity] = useState<number>(0.7)
  const [currentGesture, setCurrentGesture] = useState<string | null>(null)
  const [avatarState, setAvatarState] = useState<string>('idle')
  const [streamUrl, setStreamUrl] = useState<string | null>(null)
  
  const wsRef = useRef<WebSocket | null>(null)
  const reconnectTimeoutRef = useRef<number | null>(null)
  const gestureTimeoutRef = useRef<number | null>(null)
  const reconnectAttempts = useRef(0)
  const consecutiveFailures = useRef(0) // Track consecutive failures to stop immediately failing connections
  const maxReconnectAttempts = 5
  const maxConsecutiveFailures = 3 // Stop after 3 immediate failures
  const reconnectDelay = 3000
  const lastSuccessfulConnection = useRef<number | null>(null) // Track when we last had a successful connection

  // Validate WebSocket URL
  const validateWebSocketUrl = useCallback((url: string): boolean => {
    try {
      const urlObj = new URL(url)
      // Only allow ws:// or wss:// protocols
      if (urlObj.protocol !== 'ws:' && urlObj.protocol !== 'wss:') {
        return false
      }
      // Check URL length (prevent DoS)
      if (url.length > 2048) {
        return false
      }
      // Check for invalid characters
      if (/[\x00-\x1F\x7F]/.test(url)) {
        return false
      }
      return true
    } catch {
      return false
    }
  }, [])

  const connect = useCallback(() => {
    if (!enabled) return

    // Validate port number
    if (port < 1 || port > 65535 || !Number.isInteger(port)) {
      console.error('Invalid WebSocket port:', port)
      return
    }

    // For development: connect directly to port 8081 (bypass Vite proxy which has WebSocket upgrade issues)
    // In production, this should use the relative URL through the proxy
    const isDevelopment = window.location.hostname === 'localhost' || window.location.hostname === 'demo.localhost'
    const wsUrl = url || (isDevelopment 
      ? `ws://localhost:8081/avatar/ws`  // Direct connection in dev (bypass proxy)
      : `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}/avatar/ws`)  // Production: through proxy
    
    // Validate WebSocket URL
    if (!validateWebSocketUrl(wsUrl)) {
      console.error('Invalid WebSocket URL:', wsUrl)
      if (onError) {
        onError(new Event('Invalid URL'))
      }
      return
    }
    
    try {
      const ws = new WebSocket(wsUrl)
      wsRef.current = ws

      ws.onopen = () => {
        console.log('Avatar WebSocket connected')
        setIsConnected(true)
        reconnectAttempts.current = 0
        consecutiveFailures.current = 0 // Reset consecutive failures on successful connection
        lastSuccessfulConnection.current = Date.now()
        if (onOpen) onOpen()
      }

      ws.onmessage = (event) => {
        try {
          // Validate message size to prevent DoS
          const MAX_MESSAGE_SIZE = 10 * 1024 * 1024 // 10MB max
          if (event.data instanceof Blob && event.data.size > MAX_MESSAGE_SIZE) {
            console.warn('Avatar message too large, ignoring')
            return
          }
          if (typeof event.data === 'string' && event.data.length > MAX_MESSAGE_SIZE) {
            console.warn('Avatar message too large, ignoring')
            return
          }

          // Parse message - handle both Rust enum format and flat format
          const rawMessage = JSON.parse(event.data)
          let message: AvatarMessage | null = null
          
          // Handle Rust enum serialization format: {"Expression": {...}}, {"Gesture": {...}}, etc.
          if (rawMessage.Expression) {
            const expr = rawMessage.Expression
            message = {
              type: 'expression',
              emotion: expr.emotion || 'neutral',
              intensity: expr.intensity ?? 0.7,
            }
          } else if (rawMessage.Gesture) {
            const gest = rawMessage.Gesture
            message = {
              type: 'gesture',
              gesture: gest.gesture || 'neutral',
              duration_ms: gest.duration_ms || 1000,
            }
          } else if (rawMessage.State) {
            const state = rawMessage.State
            message = {
              type: 'state',
              state: state.state || 'idle',
            }
          } else if (rawMessage.StreamUrl) {
            const url = rawMessage.StreamUrl
            message = {
              type: 'streamUrl',
              url: url.url || '',
            }
          } else if (rawMessage.Audio) {
            const audio = rawMessage.Audio
            message = {
              type: 'audio',
              data: audio.data || [],
            }
          } else if (rawMessage.TTSRequest) {
            // Handle TTS request (for browser TTS)
            const tts = rawMessage.TTSRequest
            if (tts.text) {
              // Trigger browser TTS if available
              if ('speechSynthesis' in window) {
                const utterance = new SpeechSynthesisUtterance(tts.text)
                utterance.rate = 1.0
                utterance.pitch = 1.0
                utterance.volume = 0.8
                speechSynthesis.speak(utterance)
              }
            }
            return // TTS handled, don't process as avatar message
          } else if (rawMessage.TTSAudio) {
            // TTS audio data (not used currently, but handle gracefully)
            return
          } else if (rawMessage.type) {
            // Fallback to flat format
            message = rawMessage as AvatarMessage
          } else {
            // Unknown format, ignore
            console.debug('Unknown message format:', rawMessage)
            return
          }
          
          if (!message) return
          
          // Validate message type
          const validTypes = ['expression', 'gesture', 'state', 'streamUrl', 'audio']
          if (!validTypes.includes(message.type)) {
            console.warn('Invalid message type:', message.type)
            return
          }
          
          switch (message.type) {
            case 'expression':
              if (message.emotion) {
                // Validate expression string (prevent XSS/command injection)
                const sanitizedEmotion = String(message.emotion).slice(0, 256).replace(/[^\w-]/g, '')
                if (sanitizedEmotion) {
                  setCurrentExpression(sanitizedEmotion)
                }
                // Validate and clamp intensity
                const intensity = typeof message.intensity === 'number' 
                  ? Math.max(0, Math.min(1, message.intensity)) 
                  : 0.7
                setExpressionIntensity(intensity)
              }
              break
            case 'gesture':
              if (message.gesture) {
                // Validate gesture string
                const sanitizedGesture = String(message.gesture).slice(0, 256).replace(/[^\w-]/g, '')
                if (sanitizedGesture) {
                  setCurrentGesture(sanitizedGesture)
                  // Clear gesture after duration
                  if (gestureTimeoutRef.current !== null) {
                    clearTimeout(gestureTimeoutRef.current)
                  }
                  if (message.duration_ms && message.duration_ms > 0 && message.duration_ms <= 300000) {
                    gestureTimeoutRef.current = window.setTimeout(() => {
                      setCurrentGesture(null)
                      gestureTimeoutRef.current = null
                    }, message.duration_ms)
                  }
                }
              }
              break
            case 'state':
              if (message.state) {
                // Validate state string
                const sanitizedState = String(message.state).slice(0, 64).replace(/[^\w-]/g, '')
                if (sanitizedState) {
                  setAvatarState(sanitizedState)
                }
              }
              break
            case 'streamUrl':
              if (message.url) {
                // Validate URL to prevent XSS
                try {
                  const urlObj = new URL(message.url)
                  // Only allow ws://, wss://, http://, https://
                  if (['ws:', 'wss:', 'http:', 'https:'].includes(urlObj.protocol)) {
                    if (urlObj.href.length <= 2048) {
                      setStreamUrl(urlObj.href)
                    }
                  }
                } catch {
                  console.warn('Invalid stream URL:', message.url)
                }
              }
              break
            case 'audio':
              // Audio data for lip sync (handled separately)
              // Validate audio data size if present
              if (message.data && message.data.length > 10 * 1024 * 1024) {
                console.warn('Audio data too large, ignoring')
                return
              }
              break
          }

          if (onMessage) {
            onMessage(message)
          }
        } catch (err) {
          console.error('Failed to parse avatar message:', err)
        }
      }

      ws.onerror = (error) => {
        // Don't log errors if connection is already closed or closing
        // This prevents spam from repeated connection attempts
        if (wsRef.current && wsRef.current.readyState !== WebSocket.CLOSED) {
          console.error('Avatar WebSocket error:', error)
          if (onError) onError(error)
        }
      }

      ws.onclose = (event) => {
        console.log('Avatar WebSocket disconnected', event.code, event.reason)
        setIsConnected(false)
        wsRef.current = null
        
        if (onClose) onClose()

        // Code 1000 = normal closure, don't reconnect
        if (event.code === 1000) {
          consecutiveFailures.current = 0 // Reset on normal closure
          return
        }
        
        // Track consecutive failures (connections that fail immediately)
        const connectionDuration = lastSuccessfulConnection.current 
          ? Date.now() - lastSuccessfulConnection.current 
          : 0
        
        // If connection lasted less than 1 second, consider it an immediate failure
        if (connectionDuration < 1000 || !lastSuccessfulConnection.current) {
          consecutiveFailures.current += 1
          
          // Stop reconnecting if we've had too many immediate failures
          if (consecutiveFailures.current >= maxConsecutiveFailures) {
            console.warn(`Avatar bridge appears to be unavailable (${consecutiveFailures.current} consecutive failures). Stopping reconnection attempts.`)
            console.warn('To fix: Ensure the server is running with --features avatar and check that port 8081 is listening.')
            return // Stop trying to reconnect
          }
        } else {
          // Connection was successful (lasted > 1 second), reset consecutive failures
          consecutiveFailures.current = 0
        }
        
        // Only reconnect if we haven't exceeded max attempts
        if (enabled && reconnectAttempts.current < maxReconnectAttempts && consecutiveFailures.current < maxConsecutiveFailures) {
          reconnectAttempts.current += 1
          // Exponential backoff: 3s, 6s, 12s, 24s, 48s
          const delay = reconnectDelay * Math.pow(2, Math.min(reconnectAttempts.current - 1, 4))
          reconnectTimeoutRef.current = window.setTimeout(() => {
            console.log(`Reconnecting to avatar WebSocket (attempt ${reconnectAttempts.current}/${maxReconnectAttempts}, failures: ${consecutiveFailures.current}/${maxConsecutiveFailures})...`)
            connect()
          }, delay)
        } else {
          if (reconnectAttempts.current >= maxReconnectAttempts) {
            console.warn(`Max reconnection attempts reached (${reconnectAttempts.current}). Avatar bridge may not be running.`)
          }
        }
      }
    } catch (err) {
      console.error('Failed to create avatar WebSocket:', err)
    }
  }, [enabled, url, port, onMessage, onError, onOpen, onClose, validateWebSocketUrl])

  const disconnect = useCallback(() => {
    // Clear all timeouts
    if (reconnectTimeoutRef.current !== null) {
      clearTimeout(reconnectTimeoutRef.current)
      reconnectTimeoutRef.current = null
    }
    if (gestureTimeoutRef.current !== null) {
      clearTimeout(gestureTimeoutRef.current)
      gestureTimeoutRef.current = null
    }

    if (wsRef.current) {
      try {
        wsRef.current.close(1000, 'Manual disconnect') // Normal closure
      } catch (err) {
        // Ignore errors when closing
      }
      wsRef.current = null
    }

    setIsConnected(false)
    // Reset counters on manual disconnect
    reconnectAttempts.current = 0
    consecutiveFailures.current = 0
  }, [])

  useEffect(() => {
    if (enabled) {
      connect()
    } else {
      disconnect()
    }

    return () => {
      disconnect()
    }
  }, [enabled, connect, disconnect])

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      disconnect()
    }
  }, [disconnect])

  return {
    isConnected,
    currentExpression,
    expressionIntensity,
    currentGesture,
    avatarState,
    streamUrl,
    connect,
    disconnect,
  }
}

