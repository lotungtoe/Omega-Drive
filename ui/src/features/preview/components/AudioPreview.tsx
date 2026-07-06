import { useState, useCallback } from 'react'
import { motion } from 'framer-motion'
import { invoke } from '@tauri-apps/api/core'
import { 
  X, Download, Play, Pause, SkipForward, SkipBack, 
  Repeat, Shuffle, Volume2, VolumeX, Music, ListMusic, 
  Disc, AlertCircle, Loader2
} from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { getColor, formatSize, cn } from '../../../shared/utils'
import { useMainAppUiStateContext, useMainAppUiActions } from '../../drive/pages/useMainAppContext'

export function AudioPreview({ file, onClose, onDownload }) {
  const { t } = useTranslation()
  const displayName = file.filename || file.name || ''
  const color = getColor(displayName, file.kind)
  
  const { audioPlayback } = useMainAppUiStateContext()
  const { setAudioPlayback } = useMainAppUiActions()
  
  const { playing, position: currentTime, duration, loading } = audioPlayback
  const progress = duration > 0 ? (currentTime / duration) * 100 : 0
  
  const [volume, setVolume] = useState(0.8)
  const [muted, setMuted] = useState(false)
  const [shuffle, setShuffle] = useState(false)
  const [repeat, setRepeat] = useState('none')
  const [error] = useState(null)

  // -- Helper: Bridge Command (Local proxy to communicate with global bridge via side effects/server) --
  // We still need a way to send commands. The global bridge handles polling, 
  // but specific actions (seek, volume) can be sent directly.
  const callBridge = useCallback(async (endpoint, method = 'POST', body = null) => {
    try {
      const port = await invoke('get_bridge_port')
      const url = `http://127.0.0.1:${port}${endpoint}${endpoint.includes('?') ? '&' : '?'}type=audio`
      const options = { method }
      if (body) {
        options.headers = { 'Content-Type': 'application/json' }
        options.body = JSON.stringify(body)
      }
      const res = await fetch(url, options)
      if (!res.ok) throw new Error(await res.text())
      return res.status === 204 ? null : await res.json()
    } catch (err) {
      console.error(`AudioPreview: Command ${endpoint} failed:`, err)
      return null
    }
  }, [])

  // Format time (seconds to MM:SS)
  const formatTime = (secs) => {
    const minutes = Math.floor(secs / 60) || 0
    const seconds = Math.floor(secs % 60) || 0
    return `${minutes}:${seconds < 10 ? '0' : ''}${seconds}`
  }

  const togglePlay = () => {
    setAudioPlayback(prev => ({ ...prev, playing: !prev.playing }))
  }

  const handleSeek = async (e) => {
    if (!duration || Number.isNaN(duration)) return
    const rect = e.currentTarget.getBoundingClientRect()
    const pct = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width))
    const seekTime = Number.parseFloat((pct * duration).toFixed(3))
    await callBridge('/player/seek', 'POST', { position: seekTime })
  }

  const handleVolumeChange = async (e) => {
    const vol = Number.parseFloat(e.target.value)
    setVolume(vol)
    if (vol > 0 && muted) setMuted(false)
    await callBridge('/player/volume', 'POST', { volume: vol * 100 })
  }

  const toggleMute = async () => {
    const nextMuted = !muted
    setMuted(nextMuted)
    await callBridge('/player/volume', 'POST', { volume: nextMuted ? 0 : volume * 100 })
  }

  const renderPlayIcon = () => {
    if (loading) return <Loader2 size={24} className="animate-spin" />
    if (playing) return <Pause size={24} fill="currentColor" />
    return <Play size={24} className="ml-1" fill="currentColor" />
  }

  return (
    <motion.div 
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="fixed inset-0 z-50 flex flex-col bg-[#0c0e13] text-white overflow-hidden"
    >
      <div className="absolute inset-0 pointer-events-none opacity-20 overflow-hidden">
        <div 
          className="absolute -top-[20%] -left-[10%] w-[60%] h-[60%] rounded-full blur-[120px]"
          style={{ background: `radial-gradient(circle, ${color}33 0%, transparent 70%)` }}
        />
        <div 
          className="absolute -bottom-[10%] -right-[10%] w-[50%] h-[50%] rounded-full blur-[100px]"
          style={{ background: `radial-gradient(circle, ${color}22 0%, transparent 70%)` }}
        />
      </div>

      <div className="relative z-10 flex items-center justify-between px-6 py-4 bg-[#111318]/40 backdrop-blur-md border-b border-white/5">
        <div className="flex items-center gap-4 overflow-hidden">
          <div 
            className="p-2 rounded-lg bg-white/5 flex items-center justify-center shrink-0"
            style={{ color }}
          >
            <Music size={20} />
          </div>
          <div className="flex flex-col overflow-hidden">
            <h2 className="text-sm font-semibold truncate leading-tight">{displayName}</h2>
            <span className="text-[10px] text-white/40 font-mono tracking-wider uppercase">
              {t('preview.audioFile', 'Audio Archive')} â€¢ {formatSize(file.size)}
            </span>
          </div>
        </div>

        <div className="flex items-center gap-3">
          <button type="button" onClick={onDownload} className="flex items-center gap-2 px-4 py-1.5 rounded-full bg-white/10 hover:bg-white/20 text-white text-xs font-semibold transition-all hover:scale-105 active:scale-95">
            <Download size={14} /> {t('drive.download')}
          </button>
          <button type="button" onClick={onClose} className="p-2 rounded-full hover:bg-white/10 text-white/60 hover:text-white transition-colors">
            <X size={20} />
          </button>
        </div>
      </div>

      <div className="flex-1 relative z-10 flex overflow-hidden">
        <div className="w-80 hidden lg:flex flex-col p-6 border-r border-white/5 bg-[#111318]/20 backdrop-blur-sm">
          <div className="flex flex-col gap-6">
            <section>
              <h3 className="text-[10px] font-bold text-white/30 uppercase tracking-[0.2em] mb-4">{t('preview.sourceInfo', 'Information')}</h3>
              <div className="space-y-4">
                <div className="flex items-center justify-between text-xs p-3 rounded-xl bg-white/5">
                  <span className="text-white/40">{t('preview.provider', 'Provider')}</span>
                  <span className="text-white/80 font-medium">{file.provider || 'Cloud'}</span>
                </div>
                <div className="flex items-center justify-between text-xs p-3 rounded-xl bg-white/5">
                  <span className="text-white/40">{t('preview.bitrate', 'Quality')}</span>
                  <span className="text-white/80 font-medium">Auto</span>
                </div>
                <div className="flex items-center justify-between text-xs p-3 rounded-xl bg-white/5">
                  <span className="text-white/40">{t('preview.id', 'File ID')}</span>
                  <span className="text-white/80 font-medium">{file.id}</span>
                </div>
              </div>
            </section>
            {error && (
              <div className="p-4 rounded-xl bg-red-500/10 border border-red-500/20 text-red-400 text-xs flex gap-3">
                <AlertCircle size={16} className="shrink-0" />
                <p>{error}</p>
              </div>
            )}
          </div>
        </div>

        <div className="flex-1 flex flex-col items-center justify-center p-12 lg:p-24 relative">
          <motion.div 
            animate={{ scale: playing ? [1, 1.15, 1] : 1, opacity: playing ? [0.4, 0.7, 0.4] : 0.4 }}
            transition={{ repeat: Infinity, duration: 3, ease: "easeInOut" }}
            className="absolute rounded-full blur-[80px]"
            style={{ width: '400px', height: '400px', backgroundColor: color }}
          />

          <div className="relative group">
            <motion.div 
              animate={{ rotate: playing ? 360 : 0 }}
              transition={{ repeat: Infinity, duration: 20, ease: "linear" }}
              className="relative w-64 h-64 md:w-80 md:h-80 lg:w-96 lg:h-96 rounded-full bg-[#111318] shadow-2xl flex items-center justify-center p-2 border border-white/10"
              style={{ boxShadow: `0 0 50px ${color}15, inset 0 0 100px rgba(0,0,0,1)` }}
            >
              <div className="absolute inset-0 rounded-full opacity-20 pointer-events-none" style={{ background: 'repeating-radial-gradient(circle, #000, #000 2px, #444 3px, #000 4px)' }} />
              <div className="w-full h-full rounded-full overflow-hidden bg-gradient-to-br from-white/10 to-transparent flex items-center justify-center relative">
                {loading ? <Loader2 size={48} className="text-white/20 animate-spin" /> : <Disc size={120} strokeWidth={0.5} className="text-white/5 opacity-50" />}
                <div className="absolute inset-0 flex items-center justify-center bg-black/20">
                  <div className="w-1/4 h-1/4 rounded-full bg-[#0c0e13] border border-white/10 flex items-center justify-center">
                     <div className="w-4 h-4 rounded-full bg-white/5" />
                  </div>
                </div>
              </div>
            </motion.div>
          </div>

          <div className="mt-12 text-center max-w-2xl">
            <motion.h1 initial={{ y: 20, opacity: 0 }} animate={{ y: 0, opacity: 1 }} className="text-3xl md:text-5xl font-bold tracking-tight mb-2 truncate">{displayName}</motion.h1>
            <motion.p initial={{ y: 20, opacity: 0 }} animate={{ y: 0, opacity: 1 }} transition={{ delay: 0.1 }} className="text-white/60 text-lg font-medium">{t('preview.unknownArtist', 'Unknown Artist')}</motion.p>
          </div>
        </div>
      </div>

      <div className="relative z-20 bg-[#111318]/80 backdrop-blur-xl border-t border-white/5 px-6 pt-4 pb-8 lg:pb-6">
        <div className="flex flex-col gap-2 max-w-4xl mx-auto mb-4">
          <div className="flex justify-between text-[10px] font-medium font-mono text-white/40 uppercase tracking-widest">
            <span>{formatTime(currentTime)}</span>
            <span>{formatTime(duration)}</span>
          </div>
          <div 
            className="group relative h-1.5 w-full bg-white/5 rounded-full cursor-pointer overflow-hidden"
            onClick={handleSeek}
            role="slider"
            aria-valuemin="0"
            aria-valuemax={duration}
            aria-valuenow={currentTime}
            tabIndex={0}
            onKeyDown={async (e) => {
              if (e.key === 'ArrowRight') {
                const target = Math.min(duration, currentTime + 5)
                await callBridge('/player/seek', 'POST', { position: target })
              }
              if (e.key === 'ArrowLeft') {
                const target = Math.max(0, currentTime - 5)
                await callBridge('/player/seek', 'POST', { position: target })
              }
            }}
          >
            <motion.div 
              className="absolute left-0 top-0 h-full rounded-full"
              style={{ 
                width: `${progress}%`,
                background: `linear-gradient(to right, ${color}, ${color}dd)`,
                boxShadow: `0 0 10px ${color}88`
              }}
            />
            <div className="absolute h-3 w-3 bg-white rounded-full -top-[0.4rem] opacity-0 group-hover:opacity-100 shadow-lg transition-opacity" style={{ left: `calc(${progress}% - 6px)` }} />
          </div>
        </div>

        <div className="grid grid-cols-3 items-center max-w-6xl mx-auto">
          <div className="flex items-center gap-3 overflow-hidden">
             <div className="w-10 h-10 rounded-lg bg-white/5 flex items-center justify-center shrink-0">
               <Music size={16} style={{ color }} />
             </div>
             <div className="flex flex-col truncate">
               <span className="text-xs font-bold truncate leading-tight">{displayName}</span>
               <span className="text-[10px] text-white/40 truncate italic">
                 {file.provider || t('preview.audioFile', 'Audio File')}
               </span>
             </div>
          </div>

          <div className="flex items-center justify-center gap-6">
            <button type="button" onClick={() => setShuffle(!shuffle)} className={cn("p-2 transition-colors", shuffle ? "text-primary" : "text-white/40 hover:text-white")} style={shuffle ? { color } : {}}><Shuffle size={18} /></button>
            <button type="button" className="text-white/60 hover:text-white transition-colors"><SkipBack size={24} fill="currentColor" /></button>
            <button type="button" onClick={togglePlay} disabled={loading} className="w-12 h-12 rounded-full bg-white text-black flex items-center justify-center hover:scale-105 active:scale-95 transition-all shadow-lg disabled:opacity-50 disabled:hover:scale-100">{renderPlayIcon()}</button>
            <button type="button" className="text-white/60 hover:text-white transition-colors"><SkipForward size={24} fill="currentColor" /></button>
            <button type="button" onClick={() => {
                const modes = ['none', 'all', 'one']
                setRepeat(modes[(modes.indexOf(repeat) + 1) % modes.length])
              }} className={cn("p-2 transition-colors relative", repeat === 'none' ? "text-white/40 hover:text-white" : "text-primary")} style={repeat === 'none' ? {} : { color }}>
              <Repeat size={18} />
              {repeat === 'one' && <span className="absolute top-0 right-0 text-[8px] font-bold">1</span>}
            </button>
          </div>

          <div className="flex items-center justify-end gap-4">
             <button type="button" className="text-white/40 hover:text-white transition-colors hidden sm:block"><ListMusic size={18} /></button>
             <div className="flex items-center gap-2 group">
                <button type="button" onClick={toggleMute} className="text-white/40 hover:text-white transition-colors">{muted || volume === 0 ? <VolumeX size={18} /> : <Volume2 size={18} />}</button>
                <input type="range" min="0" max="1" step="0.01" value={volume} onChange={handleVolumeChange} className="w-24 h-1 bg-white/10 rounded-full appearance-none cursor-pointer accent-white" />
             </div>
          </div>
        </div>
      </div>
    </motion.div>
  )
}
