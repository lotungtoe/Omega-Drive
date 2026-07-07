import { useState, useEffect, useRef, useCallback } from 'react'
import { motion, AnimatePresence, useMotionValue, animate, useTransform, useMotionValueEvent } from 'framer-motion'
import { X, Download, Loader2, AlertCircle, ZoomIn, ZoomOut, Maximize, Maximize2, Minimize2, RotateCcw, QrCode, Copy, ExternalLink, Check, ChevronDown } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { invoke } from '@tauri-apps/api/core'
import { FileIcon } from '../../../shared/components/Icons'
import { getColor, formatSize, cn } from '../../../shared/utils'

export function ImagePreview({ file, onClose, onDownload }) {
  const { t } = useTranslation()
  const displayName = file.filename || file.name || ''
  const color = getColor(displayName, file.kind)

  const [imageSrc, setImageSrc] = useState(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(null)
  const [scale, setScale] = useState(1)
  const [foundCodes, setFoundCodes] = useState([])
  const [showQrMenu, setShowQrMenu] = useState(false)
  const [showZoomMenu, setShowZoomMenu] = useState(false)
  const [recentCopy, setRecentCopy] = useState(null)
  const [isFullscreen, setIsFullscreen] = useState(false)

  const ZOOM_LEVELS = [0.1, 0.25, 0.5, 0.75, 1, 1.25, 1.5, 2, 3, 5, 10]

  // Keep pan and zoom on motion values so wheel interactions stay off the React render path.
  const x = useMotionValue(0)
  const y = useMotionValue(0)
  const motionScale = useMotionValue(1)

  const [dragConstraints, setDragConstraints] = useState({ left: 0, right: 0, top: 0, bottom: 0 })
  const containerRef = useRef(null)
  const imageRef = useRef(null)
  const modalRef = useRef(null)
  const sliderRef = useRef(null)

  const scalePercentText = useTransform(motionScale, (currentValue) => `${Math.round(currentValue * 100)}%`)

  const sliderBackground = useTransform(motionScale, (currentValue) => {
    const percent = Math.round(currentValue * 100)
    const progress = Math.max(0, Math.min(100, ((percent - 10) / (1000 - 10)) * 100))
    return `linear-gradient(to right, #F43F5E 0%, #F43F5E ${progress}%, rgba(255,255,255,0.4) ${progress}%, rgba(255,255,255,0.4) 100%)`
  })

  useMotionValueEvent(motionScale, 'change', (latest) => {
    if (sliderRef.current) {
      sliderRef.current.value = Math.round(latest * 100)
    }
  })

  const updateConstraints = useCallback((currentScale) => {
    if (!containerRef.current || !imageRef.current) return { xDist: 0, yDist: 0 }

    const imgWidth = imageRef.current.offsetWidth
    const imgHeight = imageRef.current.offsetHeight
    const contWidth = containerRef.current.offsetWidth
    const contHeight = containerRef.current.offsetHeight

    const scaledW = imgWidth * currentScale
    const scaledH = imgHeight * currentScale

    const xDist = Math.max(0, (scaledW - contWidth) / 2)
    const yDist = Math.max(0, (scaledH - contHeight) / 2)

    setDragConstraints({
      left: -xDist,
      right: xDist,
      top: -yDist,
      bottom: yDist,
    })

    return { xDist, yDist }
  }, [])

  useEffect(() => {
    const handleResize = () => updateConstraints(scale)
    globalThis.addEventListener('resize', handleResize)
    return () => globalThis.removeEventListener('resize', handleResize)
  }, [scale, updateConstraints])

  useEffect(() => {
    let objectUrl = null
    let active = true

    const loadImage = async () => {
      try {
        setLoading(true)
        setError(null)
        const binaryData = await invoke('retrieve_full_file', { fileId: file.id })

        void invoke('scan_qr_by_file_id', { fileId: file.id })
          .then((codes) => {
            if (active && codes && (codes as any).length > 0) {
              setFoundCodes(codes as any)
            }
          })
          .catch((err) => console.error('QR scan failed:', err))

        const blob = new Blob([new Uint8Array(binaryData as any)], { type: 'image/*' })
        objectUrl = URL.createObjectURL(blob)
        if (active) {
          setImageSrc(objectUrl)
        }
      } catch (err) {
        console.error('Failed to load preview:', err)
        if (active) {
          setError(err?.toString?.() ?? String(err))
        }
      } finally {
        if (active) {
          setLoading(false)
        }
      }
    }

    void loadImage()
    return () => {
      active = false
      if (objectUrl) {
        URL.revokeObjectURL(objectUrl)
      }
    }
  }, [file.id])

  const resetZoom = useCallback(() => {
    x.set(0)
    y.set(0)
    motionScale.set(1)
    setScale(1)
  }, [motionScale, x, y])

  const handleFullscreen = useCallback(() => {
    if (document.fullscreenElement) {
      document.exitFullscreen().catch(() => {})
      setIsFullscreen(false)
    } else {
      document.documentElement.requestFullscreen().catch(() => {})
      setIsFullscreen(true)
    }
  }, [])

  useEffect(() => {
    const onFsChange = () => setIsFullscreen(!!document.fullscreenElement)
    document.addEventListener('fullscreenchange', onFsChange)

    return () => {
      document.removeEventListener('fullscreenchange', onFsChange)
      if (document.fullscreenElement) {
        document.exitFullscreen().catch(() => {})
      }
    }
  }, [])

  useEffect(() => {
    const handleClickOutside = () => {
      setShowZoomMenu(false)
      setShowQrMenu(false)
    }

    globalThis.addEventListener('click', handleClickOutside)
    return () => globalThis.removeEventListener('click', handleClickOutside)
  }, [])

  const applyZoom = useCallback((targetScale) => {
    const currentScale = motionScale.get()
    if (targetScale === currentScale) {
      setShowZoomMenu(false)
      return
    }

    const { xDist, yDist } = updateConstraints(targetScale)
    const ratio = targetScale / currentScale
    const nextX = x.get() * ratio
    const nextY = y.get() * ratio

    animate(motionScale, targetScale, { type: 'spring', stiffness: 300, damping: 30, mass: 0.5 })
    animate(x, Math.min(Math.max(nextX, -xDist), xDist), { type: 'spring', stiffness: 300, damping: 30, mass: 0.5 })
    animate(y, Math.min(Math.max(nextY, -yDist), yDist), { type: 'spring', stiffness: 300, damping: 30, mass: 0.5 })

    setScale(targetScale)
    setShowZoomMenu(false)
  }, [motionScale, updateConstraints, x, y])

  const handleDoubleClick = useCallback((e) => {
    e.stopPropagation()
    const currentScale = motionScale.get()
    const targetScale = Math.abs(currentScale - 1) < 0.01 ? 3 : 1
    applyZoom(targetScale)
  }, [applyZoom, motionScale])

  const handleZoom = (delta) => {
    const currentScale = motionScale.get()
    const newScale = Math.min(Math.max(currentScale + delta, 0.1), 10)
    if (newScale === currentScale) return

    animate(motionScale, newScale, {
      type: 'spring',
      stiffness: 300,
      damping: 30,
      mass: 0.5,
    })

    setScale(newScale)
    updateConstraints(newScale)
  }

  const handleWheel = useCallback((e) => {
    e.preventDefault()

    const rect = containerRef.current.getBoundingClientRect()
    const centerX = rect.left + rect.width / 2
    const centerY = rect.top + rect.height / 2

    const relativeX = e.clientX - centerX
    const relativeY = e.clientY - centerY

    const currentScale = motionScale.get()
    const currentX = x.get()
    const currentY = y.get()

    let zoomFactor = Math.exp(-e.deltaY * 0.002)
    zoomFactor = Math.min(Math.max(zoomFactor, 0.6), 1.5)

    let newScale = currentScale * zoomFactor
    if (currentScale < 1 && newScale >= 1) newScale = 1
    if (currentScale > 1 && newScale <= 1) newScale = 1
    newScale = Math.min(Math.max(newScale, 0.1), 10)

    if (newScale !== currentScale) {
      const ratio = newScale / currentScale
      const nextX = relativeX - (relativeX - currentX) * ratio
      const nextY = relativeY - (relativeY - currentY) * ratio
      const { xDist, yDist } = updateConstraints(newScale)
      const clampX = Math.min(Math.max(nextX, -xDist), xDist)
      const clampY = Math.min(Math.max(nextY, -yDist), yDist)
      const isTrackpad = e.ctrlKey || e.deltaY % 1 !== 0 || (e.deltaMode === 0 && Math.abs(e.deltaY) < 50)

      if (isTrackpad) {
        x.set(clampX)
        y.set(clampY)
        motionScale.set(newScale)
      } else {
        const springConfig = { type: 'spring', stiffness: 400, damping: 35, mass: 0.5 }
        // @ts-ignore
        animate(x, clampX, springConfig)
        // @ts-ignore
        animate(y, clampY, springConfig)
        // @ts-ignore
        animate(motionScale, newScale, springConfig)
      }
    }
  }, [motionScale, updateConstraints, x, y])

  useEffect(() => {
    const handleKeyDown = (e) => {
      if (e.key === 'Escape') onClose()
    }

    globalThis.addEventListener('keydown', handleKeyDown)
    return () => globalThis.removeEventListener('keydown', handleKeyDown)
  }, [onClose])

  useEffect(() => {
    const element = modalRef.current
    if (!element) return undefined

    const onWheel = (e) => {
      if (e.target.closest('.custom-scrollbar')) {
        return
      }
      handleWheel(e)
    }

    element.addEventListener('wheel', onWheel, { passive: false })
    return () => element.removeEventListener('wheel', onWheel)
  }, [handleWheel])

  const renderContent = () => {
    if (loading) {
      return (
        <motion.div
          key='loading'
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          className='flex flex-col items-center gap-4'
          onClick={(e) => e.stopPropagation()}
        >
          <Loader2 className='w-10 h-10 text-blue-500 animate-spin' />
          <p className='text-white/60 text-sm font-medium'>
            {t('modal.preview.loadingFullFile', 'Dang tai anh chat luong goc...')}
          </p>
        </motion.div>
      )
    }

    if (error) {
      return (
        <motion.div
          key='error'
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          className='flex flex-col items-center gap-4 max-w-md text-center'
          onClick={(e) => e.stopPropagation()}
        >
          <div className='w-16 h-16 rounded-full bg-red-500/20 flex items-center justify-center text-red-500 mb-2'>
            <AlertCircle size={32} />
          </div>
          <h4 className='text-white font-semibold'>{t('modal.preview.loadError', 'Loi khi tai anh')}</h4>
          <p className='text-white/40 text-sm'>{error}</p>
          <button type="button"
            onClick={(e) => {
              e.stopPropagation()
              globalThis.location.reload()
            }}
            className='mt-2 px-4 py-2 bg-white/10 hover:bg-white/20 text-white rounded-xl text-sm transition-colors flex items-center gap-2'
          >
            <RotateCcw size={14} /> {t('common.retry', 'Thu lai')}
          </button>
        </motion.div>
      )
    }

    return (
      <motion.div
        key='image-container'
        style={{ x, y, scale: motionScale }}
        drag={true}
        dragConstraints={dragConstraints}
        dragElastic={0}
        className='relative flex items-center justify-center'
        onClick={(e) => e.stopPropagation()}
        onDoubleClick={handleDoubleClick}
      >
        <img
          ref={imageRef}
          src={imageSrc}
          onLoad={() => updateConstraints(scale)}
          alt={displayName}
          className='max-w-full max-h-full object-contain shadow-2xl rounded-sm pointer-events-none'
          draggable={false}
        />
      </motion.div>
    )
  }

  return (
    <motion.div
      ref={modalRef}
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className='fixed inset-0 z-[100] flex flex-col bg-black/95 backdrop-blur-xl select-none'
      onContextMenu={(e) => e.preventDefault()}
    >
      <div className='flex items-center justify-between p-4 bg-black/40 border-b border-white/10 z-20'>
        <div className='flex items-center gap-3 min-w-0'>
          <div className='w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0' style={{ backgroundColor: `${color}20` }}>
            <FileIcon filename={displayName} kind={file.kind} size={16} />
          </div>
          <div className='min-w-0'>
            <h3 className='text-white text-sm font-medium truncate'>{displayName}</h3>
            <p className='text-white/40 text-xs'>{formatSize(file.size)}</p>
          </div>
        </div>

        {!loading && !error && (
          <div className='absolute left-1/2 -translate-x-1/2 hidden md:flex items-center gap-3 p-2 z-30'>
            <button type="button"
              onClick={(e) => {
                e.stopPropagation()
                resetZoom()
              }}
              className='p-1.5 bg-[#2B2D31]/90 hover:bg-[#313338] backdrop-blur-md text-white border border-[#1E1F22] rounded-md transition-colors shadow-lg'
              title={t('preview.zoomFit', 'Vua khung hinh')}
            >
              <Maximize size={16} />
            </button>

            <div className='relative'>
              <button type="button"
                onClick={(e) => {
                  e.stopPropagation()
                  setScale(motionScale.get())
                  setShowZoomMenu(!showZoomMenu)
                  setShowQrMenu(false)
                }}
                className={cn(
                  'flex items-center gap-2 px-3 py-1.5 bg-[#2B2D31]/90 hover:bg-[#313338] backdrop-blur-md text-white border rounded-md transition-all shadow-lg min-w-[80px] justify-between',
                  showZoomMenu ? 'border-blue-500/50 ring-2 ring-blue-500/20' : 'border-[#1E1F22]'
                )}
                title={t('preview.zoomLevel', 'Chon muc thu phong')}
              >
                <motion.span className='text-[13px] font-bold'>
                  {scalePercentText}
                </motion.span>
                <ChevronDown size={14} className={cn('text-white/60 transition-transform duration-200', showZoomMenu && 'rotate-180')} />
              </button>

              <AnimatePresence>
                {showZoomMenu && (
                  <motion.div
                    initial={{ opacity: 0, scale: 0.95, y: -10 }}
                    animate={{ opacity: 1, scale: 1, y: 0 }}
                    exit={{ opacity: 0, scale: 0.95, y: -10 }}
                    className='absolute top-full left-0 mt-2 w-32 bg-[#1E1F22] border border-white/10 rounded-xl shadow-2xl p-1.5 z-[60] overflow-hidden'
                    onClick={(e) => e.stopPropagation()}
                  >
                    <div className='max-h-64 overflow-y-auto space-y-0.5 custom-scrollbar pr-0.5'>
                      {ZOOM_LEVELS.map((level) => (
                        <button type="button"
                          key={level}
                          onClick={() => applyZoom(level)}
                          className={cn(
                            'w-full text-left px-3 py-2 rounded-lg text-[13px] font-medium transition-colors flex items-center justify-between group',
                            Math.round(scale * 100) === Math.round(level * 100)
                              ? 'bg-blue-500 text-white'
                              : 'text-white/70 hover:bg-white/5 hover:text-white'
                          )}
                        >
                          {Math.round(level * 100)}%
                          {level === 1 && Math.round(scale * 100) !== 100 && (
                            <span className='text-[10px] opacity-40 group-hover:opacity-60'>Reset</span>
                          )}
                        </button>
                      ))}
                    </div>
                  </motion.div>
                )}
              </AnimatePresence>
            </div>

            <div className='flex items-center gap-3 px-1 ml-1 text-white'>
              <button type="button"
                onClick={(e) => {
                  e.stopPropagation()
                  handleZoom(-0.2)
                }}
                className='hover:text-white/80 transition-colors drop-shadow-md'
                title={t('preview.zoomOut', 'Thu nho')}
              >
                <ZoomOut size={20} />
              </button>

              <motion.input
                ref={sliderRef}
                type='range'
                min='10'
                max='1000'
                step='1'
                defaultValue={100}
                onChange={(e) => {
                  e.stopPropagation()
                  const targetScale = Number.parseFloat(e.target.value) / 100

                  animate(motionScale, targetScale, { duration: 0.2, type: 'spring', bounce: 0 })

                  const { xDist, yDist } = updateConstraints(targetScale)
                  animate(x, Math.min(Math.max(x.get(), -xDist), xDist), { duration: 0.2, type: 'spring', bounce: 0 })
                  animate(y, Math.min(Math.max(y.get(), -yDist), yDist), { duration: 0.2, type: 'spring', bounce: 0 })

                  setScale(targetScale)
                }}
                className='w-32 h-1.5 rounded-full appearance-none cursor-pointer outline-none
                  [&::-webkit-slider-thumb]:appearance-none
                  [&::-webkit-slider-thumb]:w-3 [&::-webkit-slider-thumb]:h-3
                  [&::-webkit-slider-thumb]:bg-[#F43F5E]
                  [&::-webkit-slider-thumb]:rounded-full
                  [&::-webkit-slider-thumb]:ring-[5px] [&::-webkit-slider-thumb]:ring-[#2B2D31]
                  [&::-webkit-slider-thumb]:shadow-lg
                  [&::-moz-range-thumb]:appearance-none
                  [&::-moz-range-thumb]:w-3 [&::-moz-range-thumb]:h-3
                  [&::-moz-range-thumb]:bg-[#F43F5E]
                  [&::-moz-range-thumb]:border-none
                  [&::-moz-range-thumb]:rounded-full
                  [&::-moz-range-thumb]:ring-[5px] [&::-moz-range-thumb]:ring-[#2B2D31]'
                style={{
                  background: sliderBackground,
                }}
              />

              <button type="button"
                onClick={(e) => {
                  e.stopPropagation()
                  handleZoom(0.2)
                }}
                className='hover:text-white/80 transition-colors drop-shadow-md'
                title={t('preview.zoomIn', 'Phong to')}
              >
                <ZoomIn size={20} />
              </button>
            </div>

            {foundCodes.length > 0 && (
              <>
                <div className='w-px h-6 bg-white/10 mx-2' />
                <div className='relative'>
                  <button type="button"
                    onClick={(e) => {
                      e.stopPropagation()
                      setShowQrMenu(!showQrMenu)
                      setShowZoomMenu(false)
                    }}
                    className={cn(
                      'px-3 py-1.5 rounded-md transition-all flex items-center gap-1.5 bg-[#2B2D31]/90 hover:bg-[#313338] backdrop-blur-md border shadow-lg',
                      showQrMenu ? 'text-blue-400 border-blue-500/30' : 'text-white/80 border-[#1E1F22]'
                    )}
                    title={t('preview.qrFound', { count: foundCodes.length })}
                  >
                    <QrCode size={16} />
                    <span className='text-[12px] font-bold'>
                      {foundCodes.length}
                    </span>
                  </button>

                  <AnimatePresence>
                    {showQrMenu && (
                      <motion.div
                        initial={{ opacity: 0, scale: 0.95, y: -10 }}
                        animate={{ opacity: 1, scale: 1, y: 0 }}
                        exit={{ opacity: 0, scale: 0.95, y: -10 }}
                        className='absolute top-full right-0 mt-3 w-72 bg-[#1e1e1e] border border-white/10 rounded-xl shadow-2xl p-2 z-[60] text-left'
                        onClick={(e) => e.stopPropagation()}
                      >
                        <div className='px-3 py-2 border-b border-white/5 mb-2'>
                          <span className='text-[11px] font-bold text-white/40 uppercase tracking-widest'>
                            QR / Barcode ({foundCodes.length})
                          </span>
                        </div>
                        <div className='max-h-64 overflow-y-auto space-y-1.5 custom-scrollbar pr-1'>
                          {foundCodes.map((code, idx) => {
                            const isUrl = /^https?:\/\//i.test(code.text)
                            const key = `${code.text}-${idx}`
                            return (
                              <div key={key} className='bg-white/5 rounded-lg p-3 group border border-transparent hover:border-white/5 transition-all'>
                                <div className='text-[12px] text-white/90 break-all font-mono leading-relaxed mb-3 selection:bg-blue-500/30'>
                                  {code.text}
                                </div>
                                <div className='flex items-center gap-2'>
                                  <button type="button"
                                    onClick={() => {
                                      navigator.clipboard.writeText(code.text)
                                      setRecentCopy(idx)
                                      setTimeout(() => setRecentCopy(null), 2000)
                                    }}
                                    className='flex-1 flex items-center justify-center gap-2 py-1.5 rounded-md bg-white/5 hover:bg-white/10 text-[11px] font-medium text-white/70 transition-colors'
                                  >
                                    {recentCopy === idx ? <Check size={12} className='text-green-400' /> : <Copy size={12} />}
                                    {t('preview.copy')}
                                  </button>
                                  {isUrl && (
                                    <button type="button"
                                      onClick={() => invoke('open_external_url', { url: code.text })}
                                      className='flex-1 flex items-center justify-center gap-2 py-1.5 rounded-md bg-blue-400 hover:bg-blue-500 text-[11px] font-medium text-white transition-colors'
                                    >
                                      <ExternalLink size={12} />
                                      {t('preview.openLink')}
                                    </button>
                                  )}
                                </div>
                              </div>
                            )
                          })}
                        </div>
                      </motion.div>
                    )}
                  </AnimatePresence>
                </div>
              </>
            )}
          </div>
        )}

        <div className='flex items-center gap-2'>
          <button type="button"
            onClick={(e) => {
              e.stopPropagation()
              onDownload()
            }}
            className='flex items-center gap-2 px-3 py-1.5 rounded-lg bg-white/10 hover:bg-white/20 text-white text-xs font-medium transition-colors'
          >
            <Download size={14} /> {t('drive.download')}
          </button>
          <button type="button"
            onClick={(e) => {
              e.stopPropagation()
              handleFullscreen()
            }}
            className='p-2 rounded-lg hover:bg-white/10 text-white/60 hover:text-white transition-colors'
            title={isFullscreen ? t('preview.exitFullscreen', 'Thoat toan man hinh') : t('preview.fullscreen', 'Toan man hinh')}
          >
            {isFullscreen ? <Minimize2 size={18} /> : <Maximize2 size={18} />}
          </button>
          <button type="button"
            onClick={(e) => {
              e.stopPropagation()
              onClose()
            }}
            className='p-2 rounded-lg hover:bg-white/10 text-white/60 hover:text-white transition-colors'
          >
            <X size={20} />
          </button>
        </div>
      </div>

      <div
        ref={containerRef}
        className='flex-1 relative overflow-hidden flex items-center justify-center p-4 cursor-grab active:cursor-grabbing'
      >
        <AnimatePresence mode='wait'>
          {renderContent()}
        </AnimatePresence>
      </div>
    </motion.div>
  )
}
