import { useRef, useState, useCallback } from 'react'

export interface UseTTSOptions {
  enabled?: boolean
  voice?: SpeechSynthesisVoice | null
  rate?: number
  pitch?: number
  volume?: number
  onAudioData?: (audioData: ArrayBuffer) => void
}

export function useTTS(options: UseTTSOptions = {}) {
  const {
    enabled = true,
    voice = null,
    rate = 1.0,
    pitch = 1.0,
    volume = 1.0,
    onAudioData,
  } = options

  const [isSpeaking, setIsSpeaking] = useState(false)
  const [error, setError] = useState<Error | null>(null)
  const utteranceRef = useRef<SpeechSynthesisUtterance | null>(null)

  const speak = useCallback(async (text: string) => {
    if (!enabled || !('speechSynthesis' in window)) {
      setError(new Error('Speech synthesis not available'))
      return
    }

    try {
      // Cancel any ongoing speech
      window.speechSynthesis.cancel()

      const utterance = new SpeechSynthesisUtterance(text)
      
      if (voice) {
        utterance.voice = voice
      }
      
      utterance.rate = rate
      utterance.pitch = pitch
      utterance.volume = volume

      utterance.onstart = () => {
        setIsSpeaking(true)
        setError(null)
      }

      utterance.onend = () => {
        setIsSpeaking(false)
      }

      utterance.onerror = (event) => {
        setIsSpeaking(false)
        setError(new Error(`Speech synthesis error: ${event.error}`))
      }

      utteranceRef.current = utterance
      window.speechSynthesis.speak(utterance)

      // Note: Browser TTS doesn't directly provide audio data
      // For audio data capture, would need Web Audio API worklet
      // This is a placeholder for future implementation
      if (onAudioData) {
        // Could use AudioWorklet to capture synthesized audio
        console.warn('Audio data capture not yet implemented for browser TTS')
      }
    } catch (err) {
      const error = err instanceof Error ? err : new Error('Failed to speak')
      setError(error)
      setIsSpeaking(false)
    }
  }, [enabled, voice, rate, pitch, volume, onAudioData])

  const stop = useCallback(() => {
    window.speechSynthesis.cancel()
    setIsSpeaking(false)
  }, [])

  const pause = useCallback(() => {
    window.speechSynthesis.pause()
    setIsSpeaking(false)
  }, [])

  const resume = useCallback(() => {
    window.speechSynthesis.resume()
    setIsSpeaking(true)
  }, [])

  return {
    speak,
    stop,
    pause,
    resume,
    isSpeaking,
    error,
    voices: typeof window !== 'undefined' && 'speechSynthesis' in window
      ? window.speechSynthesis.getVoices()
      : [],
  }
}

