import { ref, computed } from 'vue'
import { zh } from './zh.js'
import { en } from './en.js'

const messages = { zh, en }

const currentLocale = ref(localStorage.getItem('language') || 'en')

// Sync language setting from Sunshine config (called once during init)
let syncInitialized = false
async function syncLocaleFromSunshine() {
  if (syncInitialized) return
  syncInitialized = true
  try {
    // Prefer the tray's current language (avoids new windows overriding the tray language on init)
    const { invoke } = await import('@tauri-apps/api/core')
    const trayLocale = await invoke('get_tray_locale')
    if (trayLocale && (trayLocale === 'zh' || trayLocale === 'en')) {
      if (trayLocale !== currentLocale.value) {
        currentLocale.value = trayLocale
        localStorage.setItem('language', trayLocale)
      }
      return // Tray already has a language state; no need to read from Sunshine config
    }
  } catch {
    // invoke is unavailable; fall through to Sunshine config
  }
  try {
    const { sunshine } = await import('../../tauri-adapter.js')
    const sunshineLocale = await sunshine.getLocale()
    // Sunshine uses 'zh'/'zh_TW' etc.; desktop GUI only has 'zh'/'en'
    const guiLocale = sunshineLocale.startsWith('zh') ? 'zh' : 'en'
    if (guiLocale !== currentLocale.value) {
      currentLocale.value = guiLocale
      localStorage.setItem('language', guiLocale)
    }
    // Sync the current language to the tray
    syncLocaleToTray(guiLocale)
  } catch {
    // Not running under Tauri or API unavailable; ignore
  }
}
syncLocaleFromSunshine()

// Listen for tray language-switch events
async function listenTrayLocaleChanged() {
  try {
    const { listen } = await import('@tauri-apps/api/event')
    listen('tray-locale-changed', (event) => {
      const newLocale = event.payload
      if (newLocale && newLocale !== currentLocale.value) {
        currentLocale.value = newLocale
        localStorage.setItem('language', newLocale)
        // Sync to Sunshine config
        syncLocaleToSunshine(newLocale)
      }
    })
  } catch {
    // Not running under Tauri; ignore
  }
}
listenTrayLocaleChanged()

// Sync language to tray
async function syncLocaleToTray(locale) {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    await invoke('set_tray_locale', { locale })
  } catch {
    // ignore
  }
}

export function useI18n() {
  const t = computed(() => messages[currentLocale.value] || messages.en)
  const locale = computed({
    get: () => currentLocale.value,
    set: (val) => {
      currentLocale.value = val
      localStorage.setItem('language', val)
    },
  })
  const toggleLocale = () => {
    const newLocale = locale.value === 'zh' ? 'en' : 'zh'
    locale.value = newLocale
    // Asynchronously sync to Sunshine config
    syncLocaleToSunshine(newLocale)
    // Sync to tray
    syncLocaleToTray(newLocale)
  }
  return { t, locale, toggleLocale }
}

async function syncLocaleToSunshine(locale) {
  try {
    const { sunshine } = await import('../../tauri-adapter.js')
    await sunshine.setLocale(locale)
    // Notify SunshineFrame to refresh the iframe so the new language applies
    window.dispatchEvent(new CustomEvent('locale-changed', { detail: { locale } }))
  } catch (e) {
    console.warn('Failed to sync locale to Sunshine:', e)
  }
}
