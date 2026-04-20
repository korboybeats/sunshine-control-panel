import { ref, computed, onMounted, onUnmounted } from 'vue'
import { ElMessage } from 'element-plus'
import { useRouter, ROUTES } from './useRouter.js'

const STORAGE_KEYS = {
  SKIPPED_VERSION: 'sunshine-skipped-version',
  INCLUDE_PRERELEASE: 'sunshine-include-prerelease',
  THEME: 'sunshine-theme',
}

const THEME = {
  DARK: 'dark',
  LIGHT: 'light',
}

/**
 * Normalize a version string (strip a leading v/V)
 */
const normalizeVersion = (version) => version?.replace(/^[vV]/, '') || ''

/**
 * Send a postMessage to all iframes
 */
const postMessageToIframes = (message) => {
  document.querySelectorAll('iframe').forEach((iframe) => {
    try {
      iframe.contentWindow?.postMessage(message, '*')
    } catch {
      // Cross-origin restrictions — ignore errors
    }
  })
}

/**
 * Get the Tauri invoke function
 */
const getInvoke = async () => {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke
}

/**
 * Safely read a boolean from localStorage
 */
const getStoredBoolean = (key, defaultValue = false) => {
  const value = localStorage.getItem(key)
  return value !== null ? value === 'true' : defaultValue
}

/**
 * Sidebar state-management Composable
 */
export function useSidebarState() {
  const router = useRouter()

  // State definitions
  const isCollapsed = ref(false)
  const isDark = ref(true)
  const isMaximized = ref(false)
  const isAdmin = ref(true)
  const showUpdateDialog = ref(false)
  const updateInfo = ref(null)
  const currentVersion = ref('0.0.0')
  const skippedVersion = ref(localStorage.getItem(STORAGE_KEYS.SKIPPED_VERSION) || '')
  const includePrerelease = ref(false)

  // Cleanup function registry
  const cleanupFns = []

  // Computed properties
  const showVddSettings = computed(() => router.isRoute(ROUTES.VDD_SETTINGS))
  const showWelcome = computed(() => router.isRoute(ROUTES.WELCOME))
  const currentTheme = computed(() => (isDark.value ? THEME.DARK : THEME.LIGHT))

  /**
   * Sync the theme to DOM and localStorage
   */
  const syncTheme = () => {
    document.documentElement?.setAttribute('data-bs-theme', currentTheme.value)
    localStorage.setItem(STORAGE_KEYS.THEME, currentTheme.value)
  }

  /**
   * Toggle the theme
   */
  const toggleTheme = () => {
    isDark.value = !isDark.value
    syncTheme()
    postMessageToIframes({ type: 'theme-sync', theme: currentTheme.value })
    ElMessage.success(isDark.value ? 'Switched to dark mode' : 'Switched to light mode')
  }

  /**
   * Toggle collapsed state
   */
  const toggleCollapse = () => {
    isCollapsed.value = !isCollapsed.value
  }

  // Navigation methods
  const openVddSettings = () => router.navigate(ROUTES.VDD_SETTINGS)
  const openWelcome = () => router.navigate(ROUTES.WELCOME)
  const openWebStream = () => router.navigate(ROUTES.WEB_STREAM)
  const openAiAssistant = () => router.navigate(ROUTES.AI_ASSISTANT)
  const goHome = () => router.goHome()

  /**
   * Ignore the specified version's update
   */
  const skipVersion = (version) => {
    if (!version) return
    const normalized = normalizeVersion(version)
    skippedVersion.value = normalized
    localStorage.setItem(STORAGE_KEYS.SKIPPED_VERSION, normalized)
    ElMessage.info(`Version ${version} ignored — automatic update checks will skip it next time`)
  }

  /**
   * Check whether a version has been ignored
   */
  const isVersionSkipped = (version) => {
    if (!version || !skippedVersion.value) return false
    return normalizeVersion(version) === skippedVersion.value
  }

  /**
   * Initialize the beta-update preference
   */
  const initIncludePrerelease = async () => {
    try {
      const invoke = await getInvoke()
      const savedPreference = localStorage.getItem(STORAGE_KEYS.INCLUDE_PRERELEASE)

      if (savedPreference !== null) {
        const value = savedPreference === 'true'
        includePrerelease.value = value
        await invoke('set_include_prerelease_preference', { include: value })
      } else {
        includePrerelease.value = await invoke('get_include_prerelease_preference')
        localStorage.setItem(STORAGE_KEYS.INCLUDE_PRERELEASE, includePrerelease.value.toString())
      }
    } catch (error) {
      console.error('Failed to load beta preference:', error)
      includePrerelease.value = getStoredBoolean(STORAGE_KEYS.INCLUDE_PRERELEASE)
    }
  }

  /**
   * Set the include-prerelease preference
   */
  const setIncludePrerelease = async (value) => {
    includePrerelease.value = value
    localStorage.setItem(STORAGE_KEYS.INCLUDE_PRERELEASE, value.toString())

    try {
      const invoke = await getInvoke()
      await invoke('set_include_prerelease_preference', { include: value })
    } catch (error) {
      console.error('Failed to save beta preference:', error)
    }
  }

  /**
   * Initialize admin-privilege status
   */
  const initAdminStatus = async () => {
    try {
      const invoke = await getInvoke()
      isAdmin.value = await invoke('is_running_as_admin')
      console.log(isAdmin.value ? '✅ Running with admin privileges' : '⚠️ Not running as admin')
    } catch (error) {
      console.error('Failed to detect admin privileges:', error)
    }
  }

  /**
   * Initialize theme
   */
  const initTheme = () => {
    const savedTheme = localStorage.getItem(STORAGE_KEYS.THEME)
    isDark.value = savedTheme
      ? savedTheme === THEME.DARK
      : window.matchMedia('(prefers-color-scheme: dark)').matches
    syncTheme()
  }

  /**
   * Initialize window state
   */
  const initWindowState = async () => {
    try {
      const { getCurrentWebviewWindow } = await import('@tauri-apps/api/webviewWindow')
      isMaximized.value = await getCurrentWebviewWindow().isMaximized()
    } catch (error) {
      console.error('Failed to detect window state:', error)
    }
  }

  /**
   * Initialize version info
   */
  const initVersion = async () => {
    try {
      const invoke = await getInvoke()
      currentVersion.value = (await invoke('get_sunshine_version')) || 'Unknown'
    } catch (error) {
      console.error('Failed to get Sunshine version:', error)
      currentVersion.value = 'Unknown'
    }
  }

  /**
   * Initialize event listeners
   */
  const initEventListeners = async () => {
    // Listen for iframe theme requests
    const handleMessage = ({ origin, data }) => {
      const isLocalhost = origin.includes('localhost') || origin.includes('127.0.0.1')
      if (isLocalhost && data.type === 'request-theme') {
        postMessageToIframes({ type: 'theme-sync', theme: currentTheme.value })
      }
    }
    window.addEventListener('message', handleMessage)
    cleanupFns.push(() => window.removeEventListener('message', handleMessage))

    // Tauri event listeners
    const { listen } = await import('@tauri-apps/api/event')

    const unlistenUpdate = await listen('update-available', ({ payload }) => {
      // When is_latest, always pop up — bypass the skipped-version check
      if (payload?.is_latest || !isVersionSkipped(payload?.version)) {
        updateInfo.value = payload
        showUpdateDialog.value = true
      }
    })
    cleanupFns.push(unlistenUpdate)

    const unlistenCheckResult = await listen('update-check-result', ({ payload }) => {
      const { error } = payload
      if (error) {
        ElMessage.error(`Update check failed: ${error}`)
      }
    })
    cleanupFns.push(unlistenCheckResult)
  }

  /**
   * Initialize state
   */
  const initState = async () => {
    initTheme()
    // Initialize preferences first, so the backend has the right preference before checking for updates
    await initIncludePrerelease()
    // Then initialize the rest in parallel
    await Promise.all([
      initAdminStatus(),
      initWindowState(),
      initVersion(),
      initEventListeners(),
    ])
  }

  onMounted(initState)

  onUnmounted(() => {
    cleanupFns.forEach((fn) => fn?.())
    cleanupFns.length = 0
  })

  return {
    // State
    isCollapsed,
    isDark,
    isMaximized,
    isAdmin,
    showVddSettings,
    showWelcome,
    showUpdateDialog,
    updateInfo,
    currentVersion,
    skippedVersion,
    includePrerelease,
    router,

    // Methods
    toggleTheme,
    toggleCollapse,
    openVddSettings,
    openWelcome,
    openWebStream,
    openAiAssistant,
    goHome,
    skipVersion,
    isVersionSkipped,
    setIncludePrerelease,
  }
}
