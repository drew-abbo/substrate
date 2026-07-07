import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

// Singleton state — shared across all component instances
const isPlaying = ref(true) // engine plays by default on startup

export function usePlayback() {
  async function play() {
    try {
      await invoke('play_streams')
      isPlaying.value = true
    } catch (err) {
      console.warn('play_streams failed:', err)
    }
  }

  async function pause() {
    try {
      await invoke('pause_streams')
      isPlaying.value = false
    } catch (err) {
      console.warn('pause_streams failed:', err)
    }
  }

  async function togglePlay() {
    if (isPlaying.value) await pause()
    else await play()
  }

  async function syncState() {
    try {
      const playing = await invoke<boolean>('get_playback_state')
      isPlaying.value = playing
    } catch {
      // engine not ready yet; leave default (true)
    }
  }

  return { isPlaying, togglePlay, syncState }
}
