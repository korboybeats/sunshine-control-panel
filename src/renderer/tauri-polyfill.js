/**
 * Tauri compatibility polyfill
 * Exposes Electron-like globals so the existing code keeps working.
 */

import { darkMode, openExternalUrl, vdd, sunshine, tools } from './tauri-adapter.js'

// IPC channel mapping
const IPC_HANDLERS = {
  'vdd:loadSettings': vdd.loadSettings,
  'vdd:saveSettings': vdd.saveSettings,
  'vdd:getGPUs': vdd.getGPUs,
  'vdd:execPipeCmd': vdd.execPipeCmd,
  'dark-mode:toggle': darkMode.toggle,
  'dark-mode:system': darkMode.system,
  openExternalUrl,
}

/**
 * Get the local path for a File (Electron-compatible API)
 * @param {File} file - File object
 * @returns {string} the file's Object URL or path
 */
const getPathForFile = (file) => {
  if (!file) {
    console.error('❌ getPathForFile: file argument is empty')
    return ''
  }

  // Prefer File.path (non-standard, but supported in some environments)
  if (file.path) {
    return file.path
  }

  // Create an Object URL for immediate use
  const objectUrl = URL.createObjectURL(file)

  // Asynchronously convert to a Data URL (more durable)
  const reader = new FileReader()
  reader.onload = ({ target }) => {
    window.dispatchEvent(
      new CustomEvent('file-converted', {
        detail: { name: file.name, dataUrl: target.result },
      })
    )
  }
  reader.readAsDataURL(file)

  return objectUrl
}

// Emulate Electron's window.electron API
if (typeof window !== 'undefined') {
  window.electron = {
    ipcRenderer: {
      invoke: async (channel, data) => {
        const handler = IPC_HANDLERS[channel]
        if (handler) {
          return handler(data)
        }
        console.error(`Unknown IPC channel: ${channel}`)
        return { success: false, message: 'Feature not implemented' }
      },
    },
    webUtils: { getPathForFile },
  }

  window.darkMode = darkMode
}

// Detect whether we're in production
const isProductionEnv = () => {
  if (typeof __PROD__ !== 'undefined') return __PROD__ === true
  if (typeof __DEV__ !== 'undefined') return __DEV__ === false
  try {
    return import.meta.env?.PROD === true
  } catch {
    return false
  }
}

// Detect whether we're in a Tauri environment
const isTauriEnv = () => 
  typeof window !== 'undefined' && (window.__TAURI__ || window.isTauri)

// Function to disable the right-click menu (production only)
let contextMenuHandler = null
let keydownHandler = null

const disableContextMenu = () => {
  if (typeof document === 'undefined' || !isProductionEnv() || !isTauriEnv()) {
    return
  }
  
  // Remove old event listeners
  if (contextMenuHandler) {
    document.removeEventListener('contextmenu', contextMenuHandler, true)
  }
  if (keydownHandler) {
    document.removeEventListener('keydown', keydownHandler, true)
  }
  
  // Disable the right-click menu
  contextMenuHandler = (e) => {
    e.preventDefault()
    return false
  }

  // Disable DevTools shortcuts
  const blockedKeys = new Set([
    123,  // F12
  ])
  const blockedCtrlShiftKeys = new Set([73, 74])  // I, J
  const blockedCtrlKeys = new Set([85])  // U
  
  keydownHandler = (e) => {
    if (blockedKeys.has(e.keyCode) ||
        (e.ctrlKey && e.shiftKey && blockedCtrlShiftKeys.has(e.keyCode)) ||
        (e.ctrlKey && !e.shiftKey && blockedCtrlKeys.has(e.keyCode))) {
      e.preventDefault()
      return false
    }
  }
  
  document.addEventListener('contextmenu', contextMenuHandler, true)
  document.addEventListener('keydown', keydownHandler, true)
}

// Theme switching
export function initTheme() {
  if (typeof document === 'undefined') return

  const html = document.documentElement
  const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')

  const updateTheme = (isDark) => {
    const theme = isDark ? 'dark' : 'light'
    html.setAttribute('data-bs-theme', theme)
  }

  updateTheme(mediaQuery.matches)
  mediaQuery.addEventListener('change', (e) => updateTheme(e.matches))
}

// Listen for navigation events (SPA apps)
const initNavigationListener = () => {
  if (typeof window === 'undefined' || !window.navigation) return
  
  window.navigation.addEventListener('navigate', (e) => {
    if (!e.canIntercept || e.hashChange || e.downloadRequest) return
    setTimeout(disableContextMenu, 200)
  })
}

// Auto-initialize
if (typeof document !== 'undefined') {
  const init = () => {
    initTheme()
    disableContextMenu()
    initNavigationListener()
  }
  
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init)
    window.addEventListener('load', disableContextMenu)
  } else {
    init()
  }
}

export default {
  initTheme,
  disableContextMenu,
  darkMode,
  vdd,
  sunshine,
  tools,
  openExternalUrl,
}
