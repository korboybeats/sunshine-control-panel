import { ref, onMounted, onUnmounted } from 'vue'

/**
 * Window-controls composable
 * Provides minimize/maximize/close and related window operations
 */
export function useWindowControls() {
  const tauriWindow = ref(null)
  const isMaximized = ref(false)
  const isMinimized = ref(false)
  const isFocused = ref(true)

  let unlistenResize = null
  let unlistenFocus = null

  // Initialize the Tauri window
  async function initWindow() {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      tauriWindow.value = getCurrentWindow()

      if (tauriWindow.value) {
        isMaximized.value = await tauriWindow.value.isMaximized()
        isMinimized.value = await tauriWindow.value.isMinimized()
        isFocused.value = await tauriWindow.value.isFocused()

        // Listen for window resize
        unlistenResize = await tauriWindow.value.onResized(async () => {
          if (tauriWindow.value) {
            isMaximized.value = await tauriWindow.value.isMaximized()
            isMinimized.value = await tauriWindow.value.isMinimized()
          }
        })

        // Listen for window focus changes
        unlistenFocus = await tauriWindow.value.onWindowEvent((event) => {
          if (event.event === 'tauri://focus') {
            isFocused.value = true
          } else if (event.event === 'tauri://blur') {
            isFocused.value = false
          }
        })
      }
    } catch (e) {
      console.log('Tauri API not available, running in browser mode:', e)
    }
  }

  // Minimize the window
  async function minimize() {
    if (tauriWindow.value) {
      await tauriWindow.value.minimize()
      isMinimized.value = true
    }
  }

  // Maximize / restore the window
  async function maximize() {
    if (tauriWindow.value) {
      await tauriWindow.value.maximize()
      isMaximized.value = true
    }
  }

  // Restore the window
  async function unmaximize() {
    if (tauriWindow.value) {
      await tauriWindow.value.unmaximize()
      isMaximized.value = false
    }
  }

  // Toggle maximize state
  async function toggleMaximize() {
    if (tauriWindow.value) {
      await tauriWindow.value.toggleMaximize()
      isMaximized.value = await tauriWindow.value.isMaximized()
    }
  }

  // Close the window
  async function close() {
    if (tauriWindow.value) {
      await tauriWindow.value.close()
    }
  }

  // Show the window
  async function show() {
    if (tauriWindow.value) {
      await tauriWindow.value.show()
      isMinimized.value = false
    }
  }

  // Hide the window
  async function hide() {
    if (tauriWindow.value) {
      await tauriWindow.value.hide()
    }
  }

  // Focus the window
  async function setFocus() {
    if (tauriWindow.value) {
      await tauriWindow.value.setFocus()
      isFocused.value = true
    }
  }

  // Center the window
  async function center() {
    if (tauriWindow.value) {
      await tauriWindow.value.center()
    }
  }

  // Set window size
  async function setSize(width, height) {
    if (tauriWindow.value) {
      await tauriWindow.value.setSize({ width, height })
    }
  }

  // Get window size
  async function getSize() {
    if (tauriWindow.value) {
      return await tauriWindow.value.innerSize()
    }
    return null
  }

  onMounted(() => {
    initWindow()
  })

  onUnmounted(() => {
    if (unlistenResize) {
      unlistenResize()
    }
    if (unlistenFocus) {
      unlistenFocus()
    }
  })

  return {
    tauriWindow,
    isMaximized,
    isMinimized,
    isFocused,
    minimize,
    maximize,
    unmaximize,
    toggleMaximize,
    close,
    show,
    hide,
    setFocus,
    center,
    setSize,
    getSize,
  }
}
