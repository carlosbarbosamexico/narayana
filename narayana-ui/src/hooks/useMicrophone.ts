import { useEffect, useRef, useState, useCallback } from 'react'

export interface UseMicrophoneOptions {
  enabled?: boolean
  onAudioData?: (audioData: Float32Array) => void
  sampleRate?: number
  channels?: number
}

export function useMicrophone(options: UseMicrophoneOptions = {}) {
  const {
    enabled = true,
    onAudioData,
    sampleRate = 16000,
    channels = 1,
  } = options

  const [isActive, setIsActive] = useState(false)
  const [error, setError] = useState<Error | null>(null)
  const audioContextRef = useRef<AudioContext | null>(null)
  const mediaStreamRef = useRef<MediaStream | null>(null)
  const processorRef = useRef<ScriptProcessorNode | null>(null)

  const start = useCallback(async () => {
    if (!enabled) return

    try {
      // Request microphone access
      const stream = await navigator.mediaDevices.getUserMedia({
        audio: {
          sampleRate: { ideal: sampleRate },
          channelCount: { ideal: channels },
          echoCancellation: true,
          noiseSuppression: true,
          autoGainControl: true,
        },
        video: false,
      })

      mediaStreamRef.current = stream

      // Create AudioContext
      const AudioContextClass = window.AudioContext || (window as any).webkitAudioContext
      const audioContext = new AudioContextClass({ sampleRate })
      audioContextRef.current = audioContext

      const source = audioContext.createMediaStreamSource(stream)

      // Use ScriptProcessorNode for audio processing (deprecated but widely supported)
      // For better performance, could use AudioWorklet when available
      const processor = audioContext.createScriptProcessor(4096, channels, channels)
      processor.onaudioprocess = (event) => {
        if (onAudioData) {
          const inputBuffer = event.inputBuffer
          const inputData = inputBuffer.getChannelData(0)
          onAudioData(new Float32Array(inputData))
        }
      }

      source.connect(processor)
      processor.connect(audioContext.destination)
      processorRef.current = processor

      setIsActive(true)
      setError(null)
    } catch (err) {
      const error = err instanceof Error ? err : new Error('Failed to access microphone')
      setError(error)
      setIsActive(false)
      console.error('Microphone access error:', error)
    }
  }, [enabled, sampleRate, channels, onAudioData])

  const stop = useCallback(() => {
    if (processorRef.current) {
      processorRef.current.disconnect()
      processorRef.current = null
    }

    if (audioContextRef.current) {
      audioContextRef.current.close()
      audioContextRef.current = null
    }

    if (mediaStreamRef.current) {
      mediaStreamRef.current.getTracks().forEach(track => track.stop())
      mediaStreamRef.current = null
    }

    setIsActive(false)
  }, [])

  useEffect(() => {
    if (enabled) {
      start()
    } else {
      stop()
    }

    return () => {
      stop()
    }
  }, [enabled, start, stop])

  return {
    isActive,
    error,
    start,
    stop,
  }
}

