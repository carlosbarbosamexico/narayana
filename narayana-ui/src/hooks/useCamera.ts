import { useEffect, useRef, useState, useCallback } from 'react'

export interface UseCameraOptions {
  enabled?: boolean
  onFrame?: (frame: ImageData) => void
  width?: number
  height?: number
  fps?: number
}

export function useCamera(options: UseCameraOptions = {}) {
  const {
    enabled = true,
    onFrame,
    width = 640,
    height = 480,
    fps = 30,
  } = options

  const [isActive, setIsActive] = useState(false)
  const [error, setError] = useState<Error | null>(null)
  const videoRef = useRef<HTMLVideoElement | null>(null)
  const streamRef = useRef<MediaStream | null>(null)
  const frameIntervalRef = useRef<number | null>(null)

  const start = useCallback(async () => {
    if (!enabled) return

    try {
      // Request camera access
      const stream = await navigator.mediaDevices.getUserMedia({
        video: {
          width: { ideal: width },
          height: { ideal: height },
          frameRate: { ideal: fps },
        },
        audio: false,
      })

      streamRef.current = stream

      // Create video element if not exists
      if (!videoRef.current) {
        const video = document.createElement('video')
        video.autoplay = true
        video.playsInline = true
        video.width = width
        video.height = height
        videoRef.current = video
      }

      const video = videoRef.current
      video.srcObject = stream
      await video.play()

      setIsActive(true)
      setError(null)

      // Capture frames at specified FPS
      if (onFrame) {
        const canvas = document.createElement('canvas')
        canvas.width = width
        canvas.height = height
        const ctx = canvas.getContext('2d')
        if (!ctx) {
          throw new Error('Failed to get canvas context')
        }

        const captureFrame = () => {
          if (video.readyState === video.HAVE_ENOUGH_DATA) {
            ctx.drawImage(video, 0, 0, width, height)
            const imageData = ctx.getImageData(0, 0, width, height)
            onFrame(imageData)
          }
        }

        const interval = 1000 / fps
        frameIntervalRef.current = window.setInterval(captureFrame, interval)
      }
    } catch (err) {
      const error = err instanceof Error ? err : new Error('Failed to access camera')
      setError(error)
      setIsActive(false)
      console.error('Camera access error:', error)
    }
  }, [enabled, width, height, fps, onFrame])

  const stop = useCallback(() => {
    if (frameIntervalRef.current !== null) {
      clearInterval(frameIntervalRef.current)
      frameIntervalRef.current = null
    }

    if (streamRef.current) {
      streamRef.current.getTracks().forEach(track => track.stop())
      streamRef.current = null
    }

    if (videoRef.current) {
      videoRef.current.srcObject = null
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
    videoRef,
  }
}

