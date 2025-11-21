import { useParams, useNavigate } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { apiClient } from '../lib/api'
import { X, Loader2, Camera, Mic, Volume2, Send } from 'lucide-react'
import CPLAvatar from '../components/Avatar3D/CPLAvatar'
import React, { useState, useEffect, useCallback, useRef } from 'react'
import { useCamera } from '../hooks/useCamera'
import { useMicrophone } from '../hooks/useMicrophone'
import { useTTS } from '../hooks/useTTS'
import { useAvatarWebSocket } from '../hooks/useAvatarWebSocket'

export default function CPLAvatarView() {
  const { cplId } = useParams<{ cplId: string }>()
  const navigate = useNavigate()
  const [isStandalone, setIsStandalone] = useState(false)
  const [chatMessage, setChatMessage] = useState('')
  const [chatHistory, setChatHistory] = useState<Array<{ role: 'user' | 'avatar', text: string, timestamp: Date }>>([])
  const [cameraEnabled, setCameraEnabled] = useState(true)
  const [audioEnabled, setAudioEnabled] = useState(true)
  const chatEndRef = useRef<HTMLDivElement>(null)

  // Shared WebSocket connection - use the same one as CPLAvatar
  const {
    isConnected,
  } = useAvatarWebSocket({
    enabled: true,
    port: 8081,
    onOpen: () => {
      console.log('Avatar WebSocket connected in CPLAvatarView')
    },
    onClose: () => {
      console.log('Avatar WebSocket disconnected in CPLAvatarView')
    },
  })

  // Create a separate WebSocket connection for multimodal data (video/audio) and chat
  const wsRef = useRef<WebSocket | null>(null)
  
  // Set up WebSocket connection for multimodal data and chat
  useEffect(() => {
    // Always connect if CPL is running and avatar is enabled (chat should always work)
    if (cplId) {
      const isDevelopment = window.location.hostname === 'localhost' || window.location.hostname === 'demo.localhost'
      const wsUrl = isDevelopment 
        ? `ws://localhost:8081/avatar/ws`
        : `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}/avatar/ws`
      
      if (!wsRef.current || wsRef.current.readyState === WebSocket.CLOSED || wsRef.current.readyState === WebSocket.CONNECTING) {
        const ws = new WebSocket(wsUrl)
        wsRef.current = ws
        
        ws.onopen = () => {
          console.log('Multimodal WebSocket connected for chat')
        }
        
        ws.onmessage = (event) => {
          try {
            const message = JSON.parse(event.data)
            // Handle TTS requests - add to chat history
            if (message.TTSRequest && message.TTSRequest.text) {
              setChatHistory(prev => [...prev, {
                role: 'avatar',
                text: message.TTSRequest.text,
                timestamp: new Date()
              }])
            } else if (message.State && message.State.state === 'connected') {
              console.log('Avatar bridge connected')
            }
          } catch (e) {
            // Not a JSON message, ignore
          }
        }
        
        ws.onerror = (error) => {
          console.error('Multimodal WebSocket error:', error)
        }
        
        ws.onclose = (event) => {
          console.log('Multimodal WebSocket closed:', event.code, event.reason)
          wsRef.current = null
          // Reconnect if still needed (but not if it's a normal closure)
          if (event.code !== 1000 && cplId) {
            setTimeout(() => {
              if (cplId && (!wsRef.current || wsRef.current.readyState === WebSocket.CLOSED)) {
                console.log('Reconnecting multimodal WebSocket...')
                wsRef.current = new WebSocket(wsUrl)
              }
            }, 3000)
          }
        }
      }
      
      return () => {
        if (wsRef.current) {
          wsRef.current.close(1000, 'Component unmounting')
          wsRef.current = null
        }
      }
    }
  }, [cplId])

  // Scroll chat to bottom when new messages arrive
  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [chatHistory])

  // ALL HOOKS MUST BE CALLED BEFORE ANY EARLY RETURNS
  const { data: cpl, isLoading, error } = useQuery({
    queryKey: ['cpl', cplId],
    queryFn: () => apiClient.getCPL(cplId!),
    enabled: !!cplId,
    refetchInterval: 5000, // Refresh every 5 seconds
  })

  // Extract config values (safe even if cpl is undefined)
  const avatarEnabled = cpl?.config?.enable_avatar === true || cpl?.config?.avatar_config?.enabled === true
  const isRunning = cpl?.is_running === true
  const enableVision = cpl?.config?.avatar_config?.enable_vision === true
  const enableAudioInput = cpl?.config?.avatar_config?.enable_audio_input === true
  const enableTTS = cpl?.config?.avatar_config?.enable_tts === true

  useEffect(() => {
    // Check if opened in new window (standalone mode)
    if (window.opener || window.history.length === 1) {
      setIsStandalone(true)
    }
  }, [])

  // Callbacks for hooks - must be defined before hooks
  const handleFrame = useCallback((frame: ImageData) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      // Convert ImageData to bytes
      const canvas = document.createElement('canvas')
      canvas.width = frame.width
      canvas.height = frame.height
      const ctx = canvas.getContext('2d')
      if (ctx) {
        ctx.putImageData(frame, 0, 0)
        canvas.toBlob((blob) => {
          if (blob) {
            blob.arrayBuffer().then((buffer) => {
              // Match Rust enum serialization format
              const message = JSON.stringify({
                VideoFrame: {
                  data: Array.from(new Uint8Array(buffer)),
                  width: frame.width,
                  height: frame.height,
                  timestamp: Date.now(),
                }
              })
              wsRef.current?.send(message)
            })
          }
        }, 'image/jpeg', 0.8) // Compress to JPEG
      }
    }
  }, [])

  const handleAudioData = useCallback((audioData: Float32Array) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      // Convert Float32Array to bytes (PCM)
      const bytes = new Int16Array(audioData.length)
      for (let i = 0; i < audioData.length; i++) {
        bytes[i] = Math.max(-32768, Math.min(32767, Math.floor(audioData[i] * 32768)))
      }
      const message = JSON.stringify({
        AudioSample: {
          data: Array.from(new Uint8Array(bytes.buffer)),
          sample_rate: 16000,
          channels: 1,
          timestamp: Date.now(),
        }
      })
      wsRef.current?.send(message)
    }
  }, [])

  // Vision (camera) - toggle with button
  const { isActive: cameraActive, error: cameraError, start: startCamera, stop: stopCamera } = useCamera({
    enabled: cameraEnabled && enableVision && isRunning && !!cplId,
    width: 640,
    height: 480,
    fps: 30,
    onFrame: handleFrame,
  })

  // Audio Input (microphone) - toggle with button
  const { isActive: micActive, error: micError, start: startMic, stop: stopMic } = useMicrophone({
    enabled: audioEnabled && enableAudioInput && isRunning && !!cplId,
    sampleRate: 16000,
    channels: 1,
    onAudioData: handleAudioData,
  })

  // Text-to-Speech - ALWAYS call hook, but disable conditionally
  const { speak, isSpeaking } = useTTS({
    enabled: enableTTS && isRunning && !!cplId,
    rate: 1.0,
    pitch: 1.0,
    volume: 0.8,
  })

  // Toggle camera
  const toggleCamera = useCallback(() => {
    setCameraEnabled(prev => !prev)
  }, [])

  // Toggle microphone
  const toggleAudio = useCallback(() => {
    setAudioEnabled(prev => !prev)
  }, [])

  // Send chat message
  const sendChatMessage = useCallback(() => {
    if (!chatMessage.trim()) {
      return
    }

    const messageText = chatMessage.trim()
    
    // Add user message to chat history
    setChatHistory(prev => [...prev, {
      role: 'user',
      text: messageText,
      timestamp: new Date()
    }])

    // Send via WebSocket - use ClientMessage format for text input
    // This will trigger LLM processing on the backend
    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
      // Send as ClientMessage::TTSRequest which the backend will process
      const message = JSON.stringify({
        TTSRequest: {
          text: messageText
        }
      })
      try {
        wsRef.current.send(message)
        console.log('Chat message sent:', messageText)
      } catch (e) {
        console.error('Failed to send chat message:', e)
        // Show error to user
        setChatHistory(prev => [...prev, {
          role: 'avatar',
          text: 'Error: Failed to send message. Please check connection.',
          timestamp: new Date()
        }])
      }
    } else {
      console.warn('WebSocket not connected, cannot send message')
      setChatHistory(prev => [...prev, {
        role: 'avatar',
        text: 'Error: Not connected. Please wait for connection...',
        timestamp: new Date()
      }])
    }
    
    setChatMessage('')
  }, [chatMessage])

  // NOW we can do early returns after all hooks have been called
  if (!cplId) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="text-center">
          <p className="text-gray-500">No CPL ID provided</p>
        </div>
      </div>
    )
  }

  if (isLoading) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="text-center">
          <Loader2 className="w-8 h-8 text-indigo-600 animate-spin mx-auto mb-4" />
          <p className="text-gray-500">Loading CPL avatar...</p>
        </div>
      </div>
    )
  }

  if (error || !cpl) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="text-center">
          <p className="text-red-600 mb-4">Failed to load CPL</p>
          <button
            onClick={() => (isStandalone ? window.close() : navigate('/cpls'))}
            className="btn-secondary"
          >
            Close
          </button>
        </div>
      </div>
    )
  }

  if (!avatarEnabled) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center p-4">
        <div className="bg-white rounded-lg shadow-lg p-6 max-w-md text-center">
          <p className="text-gray-700 mb-4">Avatar is not enabled for this CPL</p>
          <button
            onClick={() => (isStandalone ? window.close() : navigate('/cpls'))}
            className="btn-secondary"
          >
            Close
          </button>
        </div>
      </div>
    )
  }

  if (!isRunning) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center p-4">
        <div className="bg-white rounded-lg shadow-lg p-6 max-w-md text-center">
          <p className="text-gray-700 mb-2">CPL is not running</p>
          <p className="text-sm text-gray-500 mb-4">Start the CPL to view the avatar</p>
          <button
            onClick={() => (isStandalone ? window.close() : navigate('/cpls'))}
            className="btn-secondary"
          >
            Close
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-gray-900 flex flex-col">
      {/* Header */}
      <div className="bg-gray-800 border-b border-gray-700 px-4 py-3 flex items-center justify-between">
        <div>
          <h1 className="text-lg font-semibold text-white">CPL Avatar: {cplId}</h1>
          <p className="text-sm text-gray-400">Real-time avatar interaction</p>
        </div>
        <div className="flex items-center gap-2">
          {/* Toggle Camera Button */}
          {enableVision && (
            <button
              onClick={toggleCamera}
              className={`p-2 rounded-lg transition-colors ${
                cameraActive 
                  ? 'bg-green-600 hover:bg-green-700' 
                  : cameraError 
                    ? 'bg-red-600 hover:bg-red-700' 
                    : 'bg-gray-700 hover:bg-gray-600'
              }`}
              title={cameraActive ? 'Camera active - Click to disable' : cameraError ? 'Camera error - Click to retry' : 'Camera inactive - Click to enable'}
            >
              <Camera className="w-4 h-4 text-white" />
            </button>
          )}
          {/* Toggle Microphone Button */}
          {enableAudioInput && (
            <button
              onClick={toggleAudio}
              className={`p-2 rounded-lg transition-colors ${
                micActive 
                  ? 'bg-green-600 hover:bg-green-700' 
                  : micError 
                    ? 'bg-red-600 hover:bg-red-700' 
                    : 'bg-gray-700 hover:bg-gray-600'
              }`}
              title={micActive ? 'Microphone active - Click to disable' : micError ? 'Microphone error - Click to retry' : 'Microphone inactive - Click to enable'}
            >
              <Mic className="w-4 h-4 text-white" />
            </button>
          )}
          {/* TTS Status Indicator */}
          {enableTTS && (
            <div
              className={`p-2 rounded-lg ${isSpeaking ? 'bg-blue-600' : 'bg-gray-700'}`}
              title={isSpeaking ? 'Speaking' : 'TTS ready'}
            >
              <Volume2 className="w-4 h-4 text-white" />
            </div>
          )}
          {/* Connection Status */}
          <div
            className={`px-3 py-1 rounded-lg text-xs font-medium ${
              isConnected ? 'bg-green-600' : 'bg-yellow-600'
            }`}
            title={isConnected ? 'WebSocket connected' : 'WebSocket disconnected'}
          >
            {isConnected ? '●' : '○'}
          </div>
          <button
            onClick={() => (isStandalone ? window.close() : navigate('/cpls'))}
            className="p-2 text-gray-400 hover:text-white hover:bg-gray-700 rounded-lg transition-colors"
            title="Close"
          >
            <X className="w-5 h-5" />
          </button>
        </div>
      </div>

      {/* Avatar Container */}
      <div className="flex-1 overflow-hidden">
        <CPLAvatar
          cplId={cplId}
          enabled={true}
          className="w-full h-full"
        />
      </div>

      {/* Chat UI */}
      <div className="bg-gray-800 border-t border-gray-700 flex flex-col" style={{ height: '250px' }}>
        {/* Chat History */}
        <div className="flex-1 overflow-y-auto p-4 space-y-2">
          {chatHistory.length === 0 ? (
            <div className="text-center text-gray-500 text-sm py-8">
              Start a conversation with the avatar...
            </div>
          ) : (
            chatHistory.map((msg, idx) => (
              <div
                key={idx}
                className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}
              >
                <div
                  className={`max-w-[80%] rounded-lg px-4 py-2 ${
                    msg.role === 'user'
                      ? 'bg-indigo-600 text-white'
                      : 'bg-gray-700 text-gray-100'
                  }`}
                >
                  <p className="text-sm">{msg.text}</p>
                  <p className="text-xs opacity-70 mt-1">
                    {msg.timestamp.toLocaleTimeString()}
                  </p>
                </div>
              </div>
            ))
          )}
          <div ref={chatEndRef} />
        </div>
        
        {/* Chat Input */}
        <div className="border-t border-gray-700 p-4">
          <div className="flex gap-2">
            <input
              type="text"
              value={chatMessage}
              onChange={(e) => setChatMessage(e.target.value)}
              onKeyPress={(e) => {
                if (e.key === 'Enter' && !e.shiftKey) {
                  e.preventDefault()
                  sendChatMessage()
                }
              }}
              placeholder={isConnected ? "Type a message..." : "Connecting..."}
              disabled={!isConnected}
              className="flex-1 bg-gray-700 text-white px-4 py-2 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500 disabled:opacity-50 disabled:cursor-not-allowed"
            />
            <button
              onClick={sendChatMessage}
              disabled={!isConnected || !chatMessage.trim()}
              className="bg-indigo-600 hover:bg-indigo-700 disabled:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed text-white px-4 py-2 rounded-lg transition-colors flex items-center gap-2"
            >
              <Send className="w-4 h-4" />
              Send
            </button>
          </div>
        </div>
      </div>

      {/* Footer Info */}
      <div className="bg-gray-800 border-t border-gray-700 px-4 py-2 text-xs text-gray-400">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <span>CPL ID: {cplId}</span>
            <span className="mx-2">•</span>
            <span>Provider: {cpl.config?.avatar_config?.provider || 'Unknown'}</span>
            {(enableVision || enableAudioInput || enableTTS) && (
              <>
                <span className="mx-2">•</span>
                <span className="text-xs">
                  {enableVision && '👁️ Vision'} {enableAudioInput && '🎤 Hearing'} {enableTTS && '🗣️ Speech'}
                </span>
              </>
            )}
          </div>
          <div>
            Status: <span className={isConnected ? 'text-green-400' : 'text-yellow-400'}>
              {isConnected ? 'Connected' : 'Disconnected'}
            </span>
          </div>
        </div>
      </div>
    </div>
  )
}