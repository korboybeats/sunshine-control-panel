<template>
  <DesktopWindow :title="appTitle" :icon="sunshineIcon" :has-sidebar="true" :show-title-bar="false" :class="{ 'gamepad-active': gamepadActive }">
    <template #sidebar>
      <DesktopSidebar
        :items="navItems"
        :bottom-items="bottomNavItems"
        :active-item="activeNav"
        @item-click="handleNavClick"
        @update:active-item="activeNav = $event"
      />
    </template>

    <template #default>
      <component :is="currentView" @openThemeEditor="themeEditorOpen = true" />
    </template>
  </DesktopWindow>

  <ThemeEditor
    :open="themeEditorOpen"
    :vars="themeVars"
    :activePreset="activePreset"
    :presets="presets"
    :wallpaper="wallpaper"
    :wallpaperColors="wallpaperColors"
    @close="themeEditorOpen = false"
    @setVar="setVar"
    @applyPreset="applyPreset"
    @export="handleThemeExport"
    @import="handleThemeImport"
    @setWallpaper="setWallpaper"
    @removeWallpaper="removeWallpaper"
  />

  <SplashScreen
    :visible="showSplash"
    @done="showSplash = false"
  />
</template>

<script setup>
import { ref, computed, onMounted, watch } from 'vue'

// Desktop UI components
import DesktopWindow from './components/DesktopWindow.vue'
import DesktopSidebar from './components/DesktopSidebar.vue'
import ThemeEditor from './components/ThemeEditor.vue'
import SplashScreen from './components/SplashScreen.vue'

// Gamepad support
import { useGamepad, navigateFocus, confirmFocused } from './composables/useGamepad.js'
import { useTheme } from './composables/useTheme.js'
import { useLaunchHelpers } from './composables/useLaunchHelpers.js'
import { useI18n } from './i18n/index.js'

// Icon components
import IconApps from './icons/IconApps.vue'
import IconDashboard from './icons/IconDashboard.vue'
import IconDevices from './icons/IconDevices.vue'
import IconStream from './icons/IconStream.vue'
import IconTools from './icons/IconTools.vue'
import IconSettings from './icons/IconSettings.vue'
import IconPower from './icons/IconPower.vue'
import IconPalette from './icons/IconPalette.vue'
import IconLang from './icons/IconLang.vue'

// View components
import AppsView from './views/AppsView.vue'
import DashboardView from './views/DashboardView.vue'
import DevicesView from './views/DevicesView.vue'
import StreamView from './views/StreamView.vue'
import ToolsView from './views/ToolsView.vue'
import SettingsView from './views/SettingsView.vue'

// Import icon resources
import sunshineIcon from '../../assets/sunshine.ico'

// i18n
const { t, locale, toggleLocale } = useI18n()

// App configuration
const appTitle = 'FOUNDATION DESKTOP'

// Theme
const { themeVars, activePreset, presets, setVar, applyPreset, exportTheme, importTheme, wallpaper, wallpaperColors, setWallpaper, removeWallpaper } = useTheme()
const themeEditorOpen = ref(false)
const showSplash = ref(true)
const { helperPanelOpen } = useLaunchHelpers(t)

function handleThemeExport() {
  const json = exportTheme()
  navigator.clipboard.writeText(json).catch(() => {})
}

function handleThemeImport() {
  const json = prompt(t.value.nav.theme + ' JSON:')
  if (json) importTheme(json)
}

// Navigation state — default to the Apps Library page
const activeNav = ref('apps')

// Primary navigation items
const navItems = computed(() => [
  { id: 'apps', label: t.value.nav.apps, icon: IconApps, disabled: false },
  { id: 'dashboard', label: t.value.nav.dashboard, icon: IconDashboard, disabled: false },
  { id: 'devices', label: t.value.nav.devices, icon: IconDevices, disabled: false },
  { id: 'stream', label: t.value.nav.stream, icon: IconStream, disabled: false },
  { id: 'tools', label: t.value.nav.tools, icon: IconTools, disabled: false },
])

// Bottom navigation items
const bottomNavItems = computed(() => [
  // 'lang' button label shows the OTHER language's name — keep native names for a language picker
  { id: 'lang', label: locale.value === 'zh' ? 'EN' : '中文', icon: IconLang, disabled: false },
  { id: 'theme', label: t.value.nav.theme, icon: IconPalette, disabled: false },
  { id: 'settings', label: t.value.nav.settings, icon: IconSettings, disabled: false },
  { id: 'exit', label: t.value.nav.exit, icon: IconPower, disabled: false },
])

// View map
const viewMap = {
  apps: AppsView,
  dashboard: DashboardView,
  devices: DevicesView,
  stream: StreamView,
  tools: ToolsView,
  settings: SettingsView,
}

const currentView = computed(() => viewMap[activeNav.value] || DashboardView)

// Tauri invoke
const invoke = ref(null)

onMounted(async () => {
  try {
    const tauri = await import('@tauri-apps/api/core')
    invoke.value = tauri.invoke
  } catch (e) {
    console.log('Tauri invoke not available:', e)
  }
})

// Handle navigation click
async function handleNavClick(item) {
  if (item.disabled) return
  if (item.id === 'exit') {
    if (invoke.value) {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window')
        await getCurrentWindow().close()
      } catch (e) {
        window.close()
      }
    }
    return
  }
  if (item.id === 'theme') {
    themeEditorOpen.value = !themeEditorOpen.value
    return
  }
  if (item.id === 'lang') {
    toggleLocale()
    return
  }
  activeNav.value = item.id
}

// Gamepad navigation
const allNavIds = computed(() => [
  ...navItems.filter(i => !i.disabled).map(i => i.id),
  ...bottomNavItems.filter(i => !i.disabled).map(i => i.id),
])

const { gamepadActive } = useGamepad({
  onNavigate(direction) {
    navigateFocus(direction)
  },
  onConfirm() {
    confirmFocused()
  },
  onBack() {
    // B button: prefer closing an open drawer/panel, otherwise go back to the Apps Library home
    if (helperPanelOpen.value) {
      helperPanelOpen.value = false
    } else if (themeEditorOpen.value) {
      themeEditorOpen.value = false
    } else {
      activeNav.value = 'apps'
    }
  },
  onTabPrev() {
    // LB: switch to previous tab
    const ids = allNavIds.value
    const idx = ids.indexOf(activeNav.value)
    if (idx > 0) {
      const prevId = ids[idx - 1]
      if (prevId === 'exit') return
      activeNav.value = prevId
    }
  },
  onTabNext() {
    // RB: switch to next tab
    const ids = allNavIds.value
    const idx = ids.indexOf(activeNav.value)
    if (idx < ids.length - 1) {
      const nextId = ids[idx + 1]
      if (nextId === 'exit') return
      activeNav.value = nextId
    }
  },
})
</script>

<style lang="less" scoped>
// Component styles are managed in their respective component files
</style>
