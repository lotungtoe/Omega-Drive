import { useCallback, useEffect, useState } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import { invoke } from '@tauri-apps/api/core'
import { 
  Play, Pause, X, Maximize2, Music, Loader2,
  SkipBack, SkipForward, Shuffle, Repeat,
  Heart, Mic2, ListMusic, Volume2 
} from 'lucide-react'
import { useMainAppUiStateContext, useMainAppUiActions } from '../../drive/pages/useMainAppContext'
import { getColor, cn } from '../../../shared/utils/index'

export function MiniAudioPlayer() {
  const { activeAudioFile, audioPlayback, isAudioMinimized, dark } = useMainAppUiStateContext()
  const { setAudioPlayback, closeAudio, expandAudio } = useMainAppUiActions()
  
  const [isLiked, setIsLiked] = useState(false)
  const [shuffle, setShuffle] = useState(false)
  const [repeat, setRepeat] = useState('none') // 'none', 'all', 'one'

  const [isDragging, setIsDragging] = useState(false)
  const [scrubPosition, setScrubPosition] = useState(0)
  const audioDuration = audioPlayback?.duration ?? 0

  // 1. Actions logic
  const performSeek = async (time) => {
    if (Number.isNaN(time) || time < 0) return
    try {
      const port = await invoke('get_bridge_port')
      const seekTime = Number.parseFloat(time.toFixed(3))
      await fetch(`http://127.0.0.1:${port}/player/seek?type=audio`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ position: seekTime })
      })
    } catch (err) {
      console.error("MiniPlayer Seek failed:", err)
    }
  }

  const performVolume = async (vol) => {
    try {
      const port = await invoke('get_bridge_port')
      await fetch(`http://127.0.0.1:${port}/player/volume?type=audio`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ volume: vol })
      })
    } catch (err) {
      console.error("MiniPlayer Volume failed:", err)
    }
  }

  const handleScrub = useCallback((e) => {
    const rail = document.getElementById('audio-rail')
    if (!rail || !audioDuration) return
    const rect = rail.getBoundingClientRect()
    const pct = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width))
    setScrubPosition(pct * audioDuration)
  }, [audioDuration])

  const handleMouseDown = (e) => {
    if (!audioDuration) return
    setIsDragging(true)
    handleScrub(e)
  }

  // 2. Event Listeners for Dragging
  useEffect(() => {
    if (!isDragging) return

    const onMouseMove = (e) => handleScrub(e)
    const onMouseUp = (e) => {
      const rail = document.getElementById('audio-rail')
      if (rail && audioDuration) {
        const rect = rail.getBoundingClientRect()
        const pct = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width))
        performSeek(pct * audioDuration)
      }
      setIsDragging(false)
    }

    globalThis.addEventListener('mousemove', onMouseMove)
    globalThis.addEventListener('mouseup', onMouseUp)
    return () => {
      globalThis.removeEventListener('mousemove', onMouseMove)
      globalThis.removeEventListener('mouseup', onMouseUp)
    }
  }, [audioDuration, handleScrub, isDragging])

  // 3. Early Return (Must be after Hooks)
  if (!activeAudioFile || !isAudioMinimized) return null

  const { playing, position, duration, loading } = audioPlayback
  const currentPos = isDragging ? scrubPosition : position
  const progress = duration > 0 ? (currentPos / duration) * 100 : 0
  const fileName = activeAudioFile.filename || activeAudioFile.name || 'Unknown Song'
  const color = getColor(fileName, activeAudioFile.kind)

  const formatTime = (secs) => {
    const minutes = Math.floor(secs / 60) || 0
    const seconds = Math.floor(secs % 60) || 0
    return `${minutes}:${seconds < 10 ? '0' : ''}${seconds}`
  }

  const togglePlay = () => {
    setAudioPlayback(prev => ({ ...prev, playing: !prev.playing }))
  }

  return (
    <AnimatePresence>
      <motion.div
        initial={{ y: 20, opacity: 0 }}
        animate={{ y: 0, opacity: 1 }}
        exit={{ y: 100, opacity: 0 }}
        className={cn(
          "fixed bottom-0 left-0 right-0 z-[5000] h-24 px-4 border-t",
          "bg-[#09090b] border-white/5", // Spotify deep black
          dark ? "dark" : ""
        )}
      >
        <div className="max-w-[100vw] h-full mx-auto flex items-center justify-between">
          
          {/* LEFT: Song Info */}
          <div className="flex items-center gap-4 w-[30%] min-w-0 px-2">
            <div 
              className="w-14 h-14 rounded-md flex items-center justify-center shrink-0 shadow-lg relative group overflow-hidden"
              style={{ background: `linear-gradient(135deg, ${color}44, ${color}22)` }}
            >
              <div className="absolute inset-0 bg-black/20 group-hover:bg-black/40 transition-colors" />
              {loading ? <Loader2 className="w-6 h-6 animate-spin text-white/50" /> : <Music className="w-6 h-6" style={{ color }} />}
            </div>
            <div className="flex flex-col min-w-0 pr-2">
              <span className="text-sm font-bold truncate text-white hover:underline cursor-pointer">
                {fileName}
              </span>
              <span className="text-[11px] text-zinc-500 font-medium hover:text-zinc-300 cursor-pointer transition-colors">
                Nghá»‡ sÄ© chÆ°a rĂµ
              </span>
            </div>
            <div className="flex items-center gap-1 shrink-0">
               <button type="button" 
                onClick={() => setIsLiked(!isLiked)}
                className={cn("p-2 transition-transform active:scale-90", isLiked ? "text-[#1ed760]" : "text-zinc-500 hover:text-white")}
               >
                 <Heart size={18} fill={isLiked ? "currentColor" : "none"} />
               </button>
            </div>
          </div>

          {/* CENTER: Main Controls & Progress */}
          <div className="flex flex-col items-center gap-2 flex-1 max-w-[40%]">
            <div className="flex items-center gap-5">
              <button type="button" 
                onClick={() => setShuffle(!shuffle)}
                className={cn("transition-colors", shuffle ? "text-[#1ed760]" : "text-zinc-500 hover:text-white")}
              >
                <Shuffle size={16} />
              </button>
              <button type="button" className="text-zinc-400 hover:text-white transition-colors">
                <SkipBack size={20} fill="currentColor" />
              </button>
              <button type="button"
                onClick={togglePlay}
                disabled={loading}
                className="w-8 h-8 rounded-full bg-white text-black flex items-center justify-center hover:scale-105 active:scale-95 transition-all shadow-md disabled:opacity-50"
              >
                {playing ? <Pause size={18} fill="currentColor" /> : <Play size={18} fill="currentColor" className="ml-0.5" />}
              </button>
              <button type="button" className="text-zinc-400 hover:text-white transition-colors">
                <SkipForward size={20} fill="currentColor" />
              </button>
              <button type="button" 
                onClick={() => {
                  const modes = ['none', 'all', 'one']
                  setRepeat(modes[(modes.indexOf(repeat) + 1) % modes.length])
                }}
                className={cn("relative transition-colors", repeat === 'none' ? "text-zinc-500 hover:text-white" : "text-[#1ed760]")}
              >
                <Repeat size={16} />
                {repeat === 'one' && <span className="absolute -top-1 -right-1 text-[8px] font-bold">1</span>}
              </button>
            </div>

            <div className="w-full flex items-center gap-2 group">
              <span className="text-[10px] text-zinc-500 font-mono w-10 text-right">
                {formatTime(currentPos)}
              </span>
              <div 
                id="audio-rail"
                className="relative h-1 flex-1 bg-zinc-800 rounded-full cursor-pointer group/rail"
                onMouseDown={handleMouseDown}
                onKeyDown={(e) => {
                  if (e.key === 'ArrowRight') performSeek(Math.min(duration, position + 5))
                  if (e.key === 'ArrowLeft') performSeek(Math.max(0, position - 5))
                }}
                role="slider"
                aria-valuemin="0"
                aria-valuemax={duration}
                aria-valuenow={currentPos}
                tabIndex={0}
              >
                <div className="absolute inset-0 py-2 -top-2 flex items-center">
                  <div className="w-full h-1 relative overflow-hidden rounded-full">
                    <motion.div 
                      initial={false}
                      className={cn(
                        "absolute left-0 top-0 h-full rounded-full transition-colors",
                        isDragging ? "bg-[#1ed760]" : "bg-white group-hover/rail:bg-[#1ed760]"
                      )}
                      style={{ width: `${progress}%` }}
                    />
                  </div>
                  {/* The Knob */}
                  <motion.div 
                    initial={false}
                    className={cn(
                      "absolute w-3 h-3 bg-white rounded-full shadow-lg -ml-1.5 transition-opacity",
                      isDragging ? "opacity-100" : "opacity-0 group-hover/rail:opacity-100"
                    )}
                    style={{ left: `${progress}%` }}
                  />
                </div>
              </div>
              <span className="text-[10px] text-zinc-500 font-mono w-10">
                {formatTime(duration)}
              </span>
            </div>
          </div>

          {/* RIGHT: Extra Actions */}
          <div className="flex items-center justify-end gap-3 w-[30%] px-2">
            <div className="px-2 py-0.5 rounded border border-zinc-700 text-[9px] font-bold text-zinc-500 uppercase tracking-wider">
              320 kbps
            </div>
            
            <button type="button" className="p-2 text-zinc-500 hover:text-white transition-colors">
              <Mic2 size={16} />
            </button>
            <button type="button" className="p-2 text-zinc-500 hover:text-white transition-colors">
              <ListMusic size={18} />
            </button>
            
            <div className="flex items-center gap-2 group ml-2 w-32">
              <button type="button" 
                onClick={() => {
                  const newVol = audioPlayback.volume > 0 ? 0 : 80
                  setAudioPlayback(p => ({ ...p, volume: newVol }))
                  performVolume(newVol)
                }}
                className="text-zinc-500 hover:text-white transition-colors"
                title="Mute/Unmute"
              >
                <Volume2 size={16} />
              </button>
              <div 
                id="volume-rail"
                className="relative h-1 flex-1 bg-zinc-800 rounded-full cursor-pointer group/vol"
                role="slider"
                aria-valuemin="0"
                aria-valuemax="100"
                aria-valuenow={audioPlayback.volume || 0}
                tabIndex={0}
                onKeyDown={(e) => {
                  const currentVol = audioPlayback.volume || 0
                  if (e.key === 'ArrowUp') {
                    const next = Math.min(100, currentVol + 5)
                    setAudioPlayback(p => ({ ...p, volume: next }))
                    performVolume(next)
                  }
                  if (e.key === 'ArrowDown') {
                    const next = Math.max(0, currentVol - 5)
                    setAudioPlayback(p => ({ ...p, volume: next }))
                    performVolume(next)
                  }
                }}
                onClick={(e) => {
                  const rect = e.currentTarget.getBoundingClientRect()
                  const pct = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width))
                  const newVol = Math.round(pct * 100)
                  setAudioPlayback(p => ({ ...p, volume: newVol }))
                  performVolume(newVol)
                }}
              >
                <div 
                  className="absolute left-0 top-0 h-full bg-zinc-500 group-hover/vol:bg-[#1ed760] rounded-full transition-colors" 
                  style={{ width: `${audioPlayback.volume || 0}%` }}
                />
                {/* Volume Knob */}
                <div 
                  className="absolute w-3 h-3 bg-white rounded-full shadow-lg -top-1 -ml-1.5 opacity-0 group-hover/vol:opacity-100 transition-opacity"
                  style={{ left: `${audioPlayback.volume || 0}%` }}
                />
              </div>
            </div>

            <div className="flex items-center gap-1 ml-4 border-l border-white/5 pl-4">
               <button type="button" onClick={expandAudio} className="p-2 text-zinc-500 hover:text-white transition-colors">
                 <Maximize2 size={16} />
               </button>
               <button type="button" onClick={closeAudio} className="p-2 text-zinc-500 hover:text-red-500 transition-colors">
                 <X size={18} />
               </button>
            </div>
          </div>

        </div>
      </motion.div>
    </AnimatePresence>
  )
}
