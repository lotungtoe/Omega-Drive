import { useEffect, useCallback, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useMainAppUiStateContext, useMainAppUiActions } from '../../drive/pages/useMainAppContext'

export function GlobalAudioBridge() {
  const { activeAudioFile, audioPlayback } = useMainAppUiStateContext()
  const { setAudioPlayback } = useMainAppUiActions()
  const isPlaying = audioPlayback.playing
  const playbackPosition = audioPlayback.position
  
  const bridgePortRef = useRef(null)
  const pollingRef = useRef(null)
  const isOpeningRef = useRef(false)

  // -- Helper: Bridge Command --
  const callBridge = useCallback(async (endpoint, method = 'POST', body = null) => {
    if (!bridgePortRef.current) {
        try {
            bridgePortRef.current = await invoke('get_bridge_port')
        } catch (e) {
            console.error("GlobalAudioBridge: Failed to get bridge port, using default:", e)
            bridgePortRef.current = 13370
        }
    }
    
    try {
      const url = `http://127.0.0.1:${bridgePortRef.current}${endpoint}${endpoint.includes('?') ? '&' : '?'}type=audio`
      const options = { method }
      if (body) {
        options.headers = { 'Content-Type': 'application/json' }
        options.body = JSON.stringify(body)
      }
      const res = await fetch(url, options)
      if (!res.ok) throw new Error(await res.text())
      return res.status === 204 ? null : await res.json()
    } catch (err) {
      console.error(`GlobalAudioBridge: ${endpoint} failed:`, err)
      return null
    }
  }, [])

  // -- Polling Status --
  const startPolling = useCallback(() => {
    if (pollingRef.current) clearInterval(pollingRef.current)
    pollingRef.current = setInterval(async () => {
      const status = await callBridge('/player/status', 'GET')
      if (status) {
        setAudioPlayback(prev => ({
          ...prev,
          playing: !status.paused && status.alive,
          position: status.position || 0,
          duration: status.duration || 0,
          loading: false
        }))
        
        // Neu MPV bao da ket thuc (alive = false nhung position > 0)
        if (!status.alive && status.position >= status.duration && status.duration > 0) {
            // Tu dong close hoac chuyen bai? Tam thoi close.
            // closeAudio()
        }
      }
    }, 500)
  }, [callBridge, setAudioPlayback])

  // EFFECT: Handle Open/Play/Pause logic
  useEffect(() => {
    if (!activeAudioFile) {
        if (pollingRef.current) {
            clearInterval(pollingRef.current)
            pollingRef.current = null
            callBridge('/player/shutdown', 'POST')
        }
        return
    }

    // CASE 1: Need to Start Playback (Initial open or resume from pause)
    if (isPlaying && !pollingRef.current && !isOpeningRef.current) {
        isOpeningRef.current = true
        setAudioPlayback(p => ({ ...p, loading: true }))
        
        const initMpv = async () => {
            try {
                await invoke('prepare_audio_bridge')
                await callBridge('/player/open', 'POST', {
                    file_id: activeAudioFile.id,
                    title: activeAudioFile.filename || activeAudioFile.name,
                    start_pos: playbackPosition > 0 ? playbackPosition : undefined,
                    session_type: 'audio'
                })
                startPolling()
            } catch (err) {
                console.error("GlobalAudioBridge: Failed to init:", err)
            } finally {
                isOpeningRef.current = false
                setAudioPlayback(p => ({ ...p, loading: false }))
            }
        }
        initMpv()
    }

    // CASE 2: Need to Pause -> Shutdown MPV to save RAM
    if (!isPlaying && pollingRef.current) {
        clearInterval(pollingRef.current)
        pollingRef.current = null
        callBridge('/player/shutdown', 'POST')
        console.info("GlobalAudioBridge: paused, shutting down MPV to save RAM.")
    }

  }, [activeAudioFile, callBridge, isPlaying, playbackPosition, setAudioPlayback, startPolling])

  // EFFECT: Heartbeat
  useEffect(() => {
    const interval = setInterval(() => {
        if (bridgePortRef.current && pollingRef.current) {
            fetch(`http://127.0.0.1:${bridgePortRef.current}/player/heartbeat`, { method: 'POST' }).catch(() => {})
        }
    }, 10000)
    return () => clearInterval(interval)
  }, [])

  return null // Renderless logic component
}
