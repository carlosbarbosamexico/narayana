import { useRef, useEffect, useState } from 'react'
import { Canvas, useFrame } from '@react-three/fiber'
import { OrbitControls, PerspectiveCamera, Environment } from '@react-three/drei'
import * as THREE from 'three'

interface Avatar3DProps {
  modelUrl?: string
  expression?: string
  expressionIntensity?: number
  gesture?: string
  gestureDuration?: number
  onReady?: () => void
  className?: string
}

// Facial expression blendshape mappings
// TODO: Use this when implementing actual blendshape animations
// const EXPRESSION_BLENDSHAPES: Record<string, string> = {
//   neutral: 'neutral',
//   happy: 'happy',
//   sad: 'sad',
//   angry: 'angry',
//   surprised: 'surprised',
//   thinking: 'thinking',
//   confused: 'confused',
//   excited: 'excited',
//   tired: 'tired',
//   recognition: 'recognition',
// }

// Avatar scene component
function AvatarScene({
  modelUrl,
  expression,
  expressionIntensity,
  gesture,
  gestureDuration,
  onReady,
}: Omit<Avatar3DProps, 'className'>) {
  const meshRef = useRef<THREE.Mesh>(null)
  const [model, setModel] = useState<THREE.Group | null>(null)
  const [expressionWeight, setExpressionWeight] = useState(0)
  const previousExpression = useRef<string | undefined>()

  // Load 3D model
  useEffect(() => {
    let objectUrl: string | null = null
    let animationFrameId: number | null = null
    let currentModel: THREE.Group | null = null
    let isCancelled = false
    
    // Validate model URL if provided
    if (modelUrl) {
      try {
        // Validate URL format and protocol
        const urlObj = new URL(modelUrl)
        if (!['http:', 'https:', 'blob:', 'data:'].includes(urlObj.protocol)) {
          console.error('Invalid model URL protocol:', modelUrl)
          return
        }
        // Check URL length
        if (modelUrl.length > 2048) {
          console.error('Model URL too long')
          return
        }
      } catch {
        console.error('Invalid model URL:', modelUrl)
        return
      }
    }

    const createPlaceholderModel = (): THREE.Group => {
      const group = new THREE.Group()
      
      // Body - larger and more visible (using cylinder instead of capsule for compatibility)
      const bodyGeometry = new THREE.CylinderGeometry(0.4, 0.35, 1.2, 16)
      const bodyMaterial = new THREE.MeshStandardMaterial({ 
        color: 0x4a5568,
        metalness: 0.3,
        roughness: 0.7
      })
      const body = new THREE.Mesh(bodyGeometry, bodyMaterial)
      body.position.y = 0.6
      group.add(body)
      
      // Head - larger and more proportional
      const headGeometry = new THREE.SphereGeometry(0.4, 32, 32)
      const headMaterial = new THREE.MeshStandardMaterial({ 
        color: 0xffdbac,
        metalness: 0.1,
        roughness: 0.9
      })
      const head = new THREE.Mesh(headGeometry, headMaterial)
      head.position.set(0, 1.85, 0)
      group.add(head)
      
      // Simple eyes
      const eyeGeometry = new THREE.SphereGeometry(0.06, 8, 8)
      const eyeMaterial = new THREE.MeshStandardMaterial({ color: 0x000000 })
      const leftEye = new THREE.Mesh(eyeGeometry, eyeMaterial)
      leftEye.position.set(-0.12, 1.9, 0.35)
      group.add(leftEye)
      const rightEye = new THREE.Mesh(eyeGeometry, eyeMaterial)
      rightEye.position.set(0.12, 1.9, 0.35)
      group.add(rightEye)
      
      // Simple mouth
      const mouthGeometry = new THREE.TorusGeometry(0.08, 0.02, 8, 16)
      const mouthMaterial = new THREE.MeshStandardMaterial({ color: 0x8b0000 })
      const mouth = new THREE.Mesh(mouthGeometry, mouthMaterial)
      mouth.position.set(0, 1.7, 0.35)
      mouth.rotation.x = Math.PI / 2
      group.add(mouth)
      
      // Scale the whole group to be more visible
      group.scale.set(1.5, 1.5, 1.5)
      
      return group
    }

    if (modelUrl) {
      fetch(modelUrl)
        .then((res) => {
          if (isCancelled) return null
          // Validate response size
          const contentLength = res.headers.get('content-length')
          const MAX_SIZE = 100 * 1024 * 1024 // 100MB max
          if (contentLength && parseInt(contentLength, 10) > MAX_SIZE) {
            throw new Error('Model file too large')
          }
          return res.blob()
        })
        .then((blob) => {
          if (isCancelled) return
          if (!blob) return
          
          // Validate blob size
          if (blob.size > 100 * 1024 * 1024) {
            throw new Error('Model file too large')
          }
          objectUrl = URL.createObjectURL(blob)
          // Load GLTF model using drei's useGLTF hook
          // For now, we'll create a placeholder mesh
          currentModel = createPlaceholderModel()
          
          if (!isCancelled) {
            setModel(currentModel)
            if (onReady) onReady()
          } else {
            // Cleanup if cancelled
            currentModel.traverse((child) => {
              if (child instanceof THREE.Mesh) {
                if (child.geometry) child.geometry.dispose()
                if (child.material) {
                  if (Array.isArray(child.material)) {
                    child.material.forEach((mat) => mat.dispose())
                  } else {
                    child.material.dispose()
                  }
                }
              }
            })
            if (objectUrl) URL.revokeObjectURL(objectUrl)
          }
        })
        .catch((err) => {
          if (isCancelled) return
          console.error('Failed to load avatar model:', err)
          // Create fallback avatar
          currentModel = createPlaceholderModel()
          if (!isCancelled) {
            setModel(currentModel)
            if (onReady) onReady()
          } else {
            // Cleanup if cancelled
            currentModel.traverse((child) => {
              if (child instanceof THREE.Mesh) {
                if (child.geometry) child.geometry.dispose()
                if (child.material) {
                  if (Array.isArray(child.material)) {
                    child.material.forEach((mat) => mat.dispose())
                  } else {
                    child.material.dispose()
                  }
                }
              }
            })
          }
        })
    } else {
      // Default placeholder avatar
      currentModel = createPlaceholderModel()
      if (!isCancelled) {
        setModel(currentModel)
        if (onReady) onReady()
      } else {
        // Cleanup if cancelled
        currentModel.traverse((child) => {
          if (child instanceof THREE.Mesh) {
            if (child.geometry) child.geometry.dispose()
            if (child.material) {
              if (Array.isArray(child.material)) {
                child.material.forEach((mat) => mat.dispose())
              } else {
                child.material.dispose()
              }
            }
          }
        })
      }
    }

    // Cleanup function
    return () => {
      isCancelled = true
      
      // Clean up object URL if created
      if (objectUrl) {
        URL.revokeObjectURL(objectUrl)
        objectUrl = null
      }
      
      // Cancel any pending animation frames
      if (animationFrameId !== null) {
        cancelAnimationFrame(animationFrameId)
        animationFrameId = null
      }
      
      // Clean up Three.js resources from current effect
      if (currentModel) {
        currentModel.traverse((child) => {
          if (child instanceof THREE.Mesh) {
            if (child.geometry) child.geometry.dispose()
            if (child.material) {
              if (Array.isArray(child.material)) {
                child.material.forEach((mat) => mat.dispose())
              } else {
                child.material.dispose()
              }
            }
          }
        })
        currentModel = null
      }
      
      // Also clean up previous model from state if it exists
      // Use a callback to get current state
      setModel((prevModel) => {
        if (prevModel && prevModel !== currentModel) {
          prevModel.traverse((child) => {
            if (child instanceof THREE.Mesh) {
              if (child.geometry) child.geometry.dispose()
              if (child.material) {
                if (Array.isArray(child.material)) {
                  child.material.forEach((mat) => mat.dispose())
                } else {
                  child.material.dispose()
                }
              }
            }
          })
        }
        return null
      })
    }
  }, [modelUrl, onReady])

  // Animate expression changes
  useEffect(() => {
    let animationFrameId: number | null = null
    
    if (expression && expression !== previousExpression.current) {
      // Validate expression string
      const sanitizedExpression = String(expression).slice(0, 256).replace(/[^\w-]/g, '')
      if (!sanitizedExpression) return
      
      previousExpression.current = sanitizedExpression
      // Smoothly transition expression intensity
      const targetWeight = Math.max(0, Math.min(1, expressionIntensity || 0.7))
      const duration = Math.max(0, Math.min(5000, 500)) // Clamp duration
      const startWeight = expressionWeight
      const startTime = Date.now()

      const animate = () => {
        const elapsed = Date.now() - startTime
        const progress = Math.min(elapsed / duration, 1)
        const eased = 0.5 - Math.cos(progress * Math.PI) / 2 // Ease in-out
        const currentWeight = startWeight + (targetWeight - startWeight) * eased

        setExpressionWeight(currentWeight)

        if (progress < 1) {
          animationFrameId = requestAnimationFrame(animate)
        } else {
          animationFrameId = null
        }
      }

      animationFrameId = requestAnimationFrame(animate)
    }

    // Cleanup
    return () => {
      if (animationFrameId !== null) {
        cancelAnimationFrame(animationFrameId)
      }
    }
  }, [expression, expressionIntensity, expressionWeight])

  // Animate gesture
  useEffect(() => {
    let animationFrameId: number | null = null
    let returnAnimationFrameId: number | null = null
    
    if (gesture && meshRef.current) {
      // Validate gesture string
      const sanitizedGesture = String(gesture).slice(0, 256).replace(/[^\w-]/g, '')
      if (!sanitizedGesture) return
      
      // Gesture animations (simplified)
      const duration = Math.max(0, Math.min(300000, gestureDuration || 1000)) // Clamp to 5 min max
      const startRotation = meshRef.current.rotation.clone()
      
      let targetRotation = new THREE.Euler()
      switch (sanitizedGesture) {
        case 'wave':
          targetRotation = new THREE.Euler(0, 0, Math.PI / 4)
          break
        case 'nod':
          targetRotation = new THREE.Euler(0.3, 0, 0)
          break
        case 'shake':
          targetRotation = new THREE.Euler(0, 0.3, 0)
          break
        default:
          targetRotation = startRotation
      }

      const startTime = Date.now()
      const animate = () => {
        const elapsed = Date.now() - startTime
        const progress = Math.min(elapsed / duration, 1)
        const eased = 0.5 - Math.cos(progress * Math.PI) / 2

        if (meshRef.current) {
          meshRef.current.rotation.x = startRotation.x + (targetRotation.x - startRotation.x) * eased
          meshRef.current.rotation.y = startRotation.y + (targetRotation.y - startRotation.y) * eased
          meshRef.current.rotation.z = startRotation.z + (targetRotation.z - startRotation.z) * eased

          if (progress < 1) {
            animationFrameId = requestAnimationFrame(animate)
          } else {
            animationFrameId = null
            // Return to neutral
            const returnStart = meshRef.current.rotation.clone()
            const returnDuration = 300
            const returnStartTime = Date.now()
            const returnAnimate = () => {
              const returnElapsed = Date.now() - returnStartTime
              const returnProgress = Math.min(returnElapsed / returnDuration, 1)
              const returnEased = 0.5 - Math.cos(returnProgress * Math.PI) / 2

              if (meshRef.current) {
                meshRef.current.rotation.x = returnStart.x + (startRotation.x - returnStart.x) * returnEased
                meshRef.current.rotation.y = returnStart.y + (startRotation.y - returnStart.y) * returnEased
                meshRef.current.rotation.z = returnStart.z + (startRotation.z - returnStart.z) * returnEased

                if (returnProgress < 1) {
                  returnAnimationFrameId = requestAnimationFrame(returnAnimate)
                } else {
                  returnAnimationFrameId = null
                }
              }
            }
            returnAnimationFrameId = requestAnimationFrame(returnAnimate)
          }
        }
      }
      animationFrameId = requestAnimationFrame(animate)
    }

    // Cleanup
    return () => {
      if (animationFrameId !== null) {
        cancelAnimationFrame(animationFrameId)
      }
      if (returnAnimationFrameId !== null) {
        cancelAnimationFrame(returnAnimationFrameId)
      }
    }
  }, [gesture, gestureDuration])

  // Render animation loop - subtle idle animation
  useFrame((state) => {
    if (meshRef.current && model) {
      // Subtle idle animation (prevent accumulation by using absolute time)
      const baseYRotation = 0
      const animationOffset = Math.sin(state.clock.elapsedTime) * 0.0001
      meshRef.current.rotation.y = baseYRotation + animationOffset
    }
  })

  if (!model) {
    return null
  }

  return (
    <primitive ref={meshRef} object={model} scale={[1, 1, 1]} />
  )
}

export default function Avatar3D({
  modelUrl,
  expression = 'neutral',
  expressionIntensity = 0.7,
  gesture,
  gestureDuration = 1000,
  onReady,
  className = '',
}: Avatar3DProps) {
  const [isReady, setIsReady] = useState(false)

  const handleReady = () => {
    setIsReady(true)
    if (onReady) onReady()
  }

  return (
    <div className={`avatar-3d-container ${className}`} style={{ width: '100%', height: '100%', minHeight: '400px' }}>
      <Canvas
        shadows
        gl={{ antialias: true, alpha: true }}
        style={{ background: 'linear-gradient(to bottom, #87CEEB 0%, #E0F6FF 100%)' }}
      >
        <PerspectiveCamera makeDefault position={[0, 1.5, 5]} fov={50} />
        <ambientLight intensity={0.6} />
        <directionalLight position={[5, 5, 5]} intensity={1} castShadow />
        <pointLight position={[-5, 5, -5]} intensity={0.5} />
        
        <AvatarScene
          modelUrl={modelUrl}
          expression={expression}
          expressionIntensity={expressionIntensity}
          gesture={gesture}
          gestureDuration={gestureDuration}
          onReady={handleReady}
        />
        
        <OrbitControls
          enablePan={false}
          enableZoom={true}
          enableRotate={true}
          minDistance={3}
          maxDistance={10}
          minPolarAngle={Math.PI / 6}
          maxPolarAngle={Math.PI / 2.2}
        />
        
        <Environment preset="sunset" />
      </Canvas>
      
      {!isReady && (
        <div className="absolute inset-0 flex items-center justify-center bg-gray-100 bg-opacity-75">
          <div className="text-center">
            <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto"></div>
            <p className="mt-4 text-gray-600">Loading avatar...</p>
          </div>
        </div>
      )}
    </div>
  )
}

