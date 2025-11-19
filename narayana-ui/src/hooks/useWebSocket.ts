import { useEffect, useRef, useState, useCallback } from 'react'

export type WsMessage =
  | { type: 'subscribe'; channel: string; filter?: any }
  | { type: 'unsubscribe'; channel: string }
  | { type: 'ping'; id?: string }
  | { type: 'event'; channel: string; event: any; timestamp?: number }
  | { type: 'subscribed'; channel: string }
  | { type: 'unsubscribed'; channel: string }
  | { type: 'error'; code: string; message: string }
  | { type: 'pong'; id?: string }

interface UseWebSocketOptions {
  url?: string
  onMessage?: (message: WsMessage) => void
  onError?: (error: Event) => void
  onOpen?: () => void
  onClose?: () => void
  reconnectInterval?: number
  maxReconnectAttempts?: number
}

export function useWebSocket(options: UseWebSocketOptions = {}) {
  const {
    url = 'ws://localhost:8080/ws',
    onMessage,
    onError,
    onOpen,
    onClose,
    reconnectInterval = 3000,
    maxReconnectAttempts = 10,
  } = options

  const [isConnected, setIsConnected] = useState(false)
  const [lastMessage, setLastMessage] = useState<WsMessage | null>(null)
  const wsRef = useRef<WebSocket | null>(null)
  const reconnectAttemptsRef = useRef(0)
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const subscriptionsRef = useRef<Set<string>>(new Set())

  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      return
    }

    try {
      const ws = new WebSocket(url)
      wsRef.current = ws

      ws.onopen = () => {
        setIsConnected(true)
        reconnectAttemptsRef.current = 0
        onOpen?.()

        // Resubscribe to all channels
        subscriptionsRef.current.forEach((channel) => {
          ws.send(
            JSON.stringify({
              type: 'subscribe',
              channel,
            })
          )
        })
      }

      ws.onmessage = (event) => {
        try {
          const message: WsMessage = JSON.parse(event.data)
          setLastMessage(message)

          if (message.type === 'subscribed') {
            subscriptionsRef.current.add(message.channel)
          } else if (message.type === 'unsubscribed') {
            subscriptionsRef.current.delete(message.channel)
          }

          onMessage?.(message)
        } catch (error) {
          console.error('Failed to parse WebSocket message:', error)
        }
      }

      ws.onerror = (error) => {
        console.error('WebSocket error:', error)
        onError?.(error)
      }

      ws.onclose = () => {
        setIsConnected(false)
        onClose?.()

        // Attempt to reconnect
        if (reconnectAttemptsRef.current < maxReconnectAttempts) {
          reconnectAttemptsRef.current++
          reconnectTimeoutRef.current = setTimeout(() => {
            connect()
          }, reconnectInterval)
        }
      }
    } catch (error) {
      console.error('Failed to create WebSocket:', error)
    }
  }, [url, onMessage, onError, onOpen, onClose, reconnectInterval, maxReconnectAttempts])

  const disconnect = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current)
      reconnectTimeoutRef.current = null
    }
    if (wsRef.current) {
      wsRef.current.close()
      wsRef.current = null
    }
    setIsConnected(false)
    subscriptionsRef.current.clear()
  }, [])

  const sendMessage = useCallback(
    (message: WsMessage) => {
      if (wsRef.current?.readyState === WebSocket.OPEN) {
        wsRef.current.send(JSON.stringify(message))
      } else {
        console.warn('WebSocket is not connected')
      }
    },
    []
  )

  const subscribe = useCallback(
    (channel: string, filter?: any) => {
      if (!subscriptionsRef.current.has(channel)) {
        sendMessage({
          type: 'subscribe',
          channel,
          filter,
        })
        subscriptionsRef.current.add(channel)
      }
    },
    [sendMessage]
  )

  const unsubscribe = useCallback(
    (channel: string) => {
      if (subscriptionsRef.current.has(channel)) {
        sendMessage({
          type: 'unsubscribe',
          channel,
        })
        subscriptionsRef.current.delete(channel)
      }
    },
    [sendMessage]
  )

  useEffect(() => {
    connect()
    return () => {
      disconnect()
    }
  }, [connect, disconnect])

  return {
    isConnected,
    lastMessage,
    sendMessage,
    subscribe,
    unsubscribe,
    connect,
    disconnect,
  }
}

