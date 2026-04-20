import { ref, computed } from 'vue'

/**
 * Route definitions
 */
export const ROUTES = {
  HOME: 'home',           // Default content (slot)
  VDD_SETTINGS: 'vdd-settings',
  WELCOME: 'welcome',
  WEB_STREAM: 'web-stream',
  AI_ASSISTANT: 'ai-assistant',
}

/**
 * Route configuration
 */
const routeConfig = {
  [ROUTES.HOME]: {
    name: ROUTES.HOME,
    component: null, // Uses slot
    title: 'Advanced Settings',
  },
  [ROUTES.VDD_SETTINGS]: {
    name: ROUTES.VDD_SETTINGS,
    component: 'VddSettings',
    title: 'Virtual Display',
  },
  [ROUTES.WELCOME]: {
    name: ROUTES.WELCOME,
    component: 'Welcome',
    title: 'Welcome',
  },
  [ROUTES.WEB_STREAM]: {
    name: ROUTES.WEB_STREAM,
    component: 'WebStreamSettings',
    title: 'Web Streaming',
  },
  [ROUTES.AI_ASSISTANT]: {
    name: ROUTES.AI_ASSISTANT,
    component: 'AiAssistant',
    title: 'Mita',
  },
}

/**
 * Router management Composable
 */
export function useRouter() {
  const currentRoute = ref(ROUTES.HOME)
  const routeHistory = ref([ROUTES.HOME])

  /**
   * Navigate to a specific route
   * @param {string} routeName - route name
   * @param {object} options - navigation options
   */
  const navigate = (routeName, options = {}) => {
    if (!routeConfig[routeName]) {
      console.warn(`Route ${routeName} does not exist`)
      return
    }

    // Replace the current history entry instead of pushing a new one
    if (options.replace) {
      routeHistory.value[routeHistory.value.length - 1] = routeName
    } else {
      routeHistory.value.push(routeName)
      // Cap history length
      if (routeHistory.value.length > 10) {
        routeHistory.value.shift()
      }
    }

    currentRoute.value = routeName
  }

  /**
   * Go back to the previous page
   */
  const goBack = () => {
    if (routeHistory.value.length > 1) {
      routeHistory.value.pop() // Remove current route
      currentRoute.value = routeHistory.value[routeHistory.value.length - 1]
    }
  }

  /**
   * Go back to home
   */
  const goHome = () => {
    navigate(ROUTES.HOME, { replace: true })
  }

  /**
   * Get the current route config
   */
  const getCurrentRouteConfig = computed(() => {
    return routeConfig[currentRoute.value] || routeConfig[ROUTES.HOME]
  })

  /**
   * Check whether we are on a specific route
   */
  const isRoute = (routeName) => {
    return currentRoute.value === routeName
  }

  return {
    currentRoute,
    routeHistory,
    navigate,
    goBack,
    goHome,
    getCurrentRouteConfig,
    isRoute,
    ROUTES,
  }
}

