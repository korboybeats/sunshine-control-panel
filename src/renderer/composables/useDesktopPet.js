/**
 * Desktop-pet vision observation module
 * Periodically captures a screenshot of the desktop and sends it to a multimodal LLM,
 * letting Mita produce teasing / snarky commentary about what the user is doing.
 */

import { ref } from 'vue'
import { callVisionLLM } from './aiClient.js'
import { STORAGE_KEY, DEFAULT_CONFIG } from './aiProviders.js'

// System prompt for desktop-pet vision observation
const PET_VISION_PROMPT = `You are "Mita", a cute but sharp-tongued desktop pet. You're peeking at the user's screen.

## Your task
Based on the screenshot, assume the user is doing something and tease them. Don't guess with "Are you...?" — just assert "You're slacking off again with..." and tease them directly.

## Your personality
- Bratty-gremlin vibe: mocking without being mean, tsundere but caring
- Favorite phrases: "scrub♡", "hmph", "tsk", "dummy"
- Occasionally show you care — e.g. if the user is working late, say "It's this late and you're still not sleeping... dummy"

## Style examples
- Gaming → "Slacking off with games again♡ Your play is hard to watch, scrub~"
- Code → "Debugging forever and probably added MORE bugs, scrub programmer~"
- Browsing at work → "Scrolling this on company time? Your boss is gonna love that♡"
- Chatting → "Who are you having such a fun time chatting with? Hmph, not that I care~"

## Rules
- Output only a single sentence (roughly 15–40 characters), no explanation
- Assert what the user is doing directly — don't guess
- Don't repeat the same line twice
- Reply in English`

// ===== Module-level shared state (singleton) =====
const petMessage = ref('')
const isObserving = ref(false)
const lastObserveTime = ref(0)
const petEnabled = ref(loadPetEnabled())
const observeInterval = ref(loadObserveInterval())
let timer = null
let initialized = false

function loadPetEnabled() {
  try {
    return localStorage.getItem('sunshine-pet-enabled') === 'true'
  } catch {
    return false
  }
}

function loadObserveInterval() {
  try {
    const saved = localStorage.getItem('sunshine-pet-interval')
    return saved ? parseInt(saved, 10) : 60000
  } catch {
    return 60000
  }
}

function savePetConfig() {
  localStorage.setItem('sunshine-pet-enabled', String(petEnabled.value))
  localStorage.setItem('sunshine-pet-interval', String(observeInterval.value))
}

async function captureScreen() {
  const tauri = window.__TAURI__
  if (!tauri?.core?.invoke) {
    throw new Error('Must run inside a Tauri environment')
  }
  return await tauri.core.invoke('capture_screenshot')
}

function getAiConfig() {
  try {
    const saved = localStorage.getItem(STORAGE_KEY)
    return saved ? { ...DEFAULT_CONFIG, ...JSON.parse(saved) } : { ...DEFAULT_CONFIG }
  } catch {
    return { ...DEFAULT_CONFIG }
  }
}

async function observe() {
  const config = getAiConfig()
  if (!config.enabled || !config.apiKey) {
    console.warn('[pet] AI is not enabled or no API key configured — skipping observation')
    petMessage.value = '(Mita is not connected to an AI service yet. Configure the AI assistant in Settings first~)'
    return
  }

  isObserving.value = true
  try {
    console.log('[pet] Capturing screenshot...')
    const screenshot = await captureScreen()
    console.log('[pet] Screenshot captured — calling Vision LLM...')
    const response = await callVisionLLM(
      config,
      PET_VISION_PROMPT,
      'Take a look at my desktop and say something',
      screenshot,
      150
    )

    if (response && response.trim()) {
      console.log('[pet] LLM reply:', response.trim())
      petMessage.value = response.trim()
      lastObserveTime.value = Date.now()
    }
  } catch (err) {
    const errMsg = typeof err === 'string' ? err : (err?.message || JSON.stringify(err))
    console.warn('[pet] Observation failed:', errMsg, err)
    petMessage.value = `(Observation failed: ${errMsg})`
  } finally {
    isObserving.value = false
  }
}

function startObserving() {
  if (timer) clearInterval(timer)
  petEnabled.value = true
  savePetConfig()

  observe()

  timer = setInterval(() => {
    if (!isObserving.value) {
      observe()
    }
  }, observeInterval.value)
}

function stopObserving() {
  petEnabled.value = false
  savePetConfig()
  if (timer) {
    clearInterval(timer)
    timer = null
  }
}

function setIntervalSeconds(seconds) {
  observeInterval.value = Math.max(15, seconds) * 1000
  savePetConfig()
  if (petEnabled.value) {
    startObserving()
  }
}

async function poke() {
  await observe()
}

function dismissMessage() {
  petMessage.value = ''
}

/**
 * Desktop-pet vision observation Composable (shared singleton state)
 */
export function useDesktopPet() {
  // On first call, auto-restore the previous enabled state
  if (!initialized) {
    initialized = true
    if (petEnabled.value) {
      startObserving()
    }
  }

  return {
    petMessage,
    petEnabled,
    isObserving,
    lastObserveTime,
    observeInterval,
    startObserving,
    stopObserving,
    setIntervalSeconds,
    poke,
    dismissMessage,
  }
}
