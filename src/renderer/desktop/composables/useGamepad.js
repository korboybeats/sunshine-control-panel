import { ref, onMounted, onUnmounted } from 'vue'

// Gamepad button mapping (Xbox standard)
const BUTTON = {
  A: 0,        // Confirm
  B: 1,        // Back
  X: 2,
  Y: 3,
  LB: 4,       // Previous tab / sidebar up
  RB: 5,       // Next tab / sidebar down
  LT: 6,
  RT: 7,
  BACK: 8,
  START: 9,
  L3: 10,
  R3: 11,
  DPAD_UP: 12,
  DPAD_DOWN: 13,
  DPAD_LEFT: 14,
  DPAD_RIGHT: 15,
}

const AXIS = {
  LEFT_X: 0,
  LEFT_Y: 1,
  RIGHT_X: 2,
  RIGHT_Y: 3,
}

const DEADZONE = 0.4
const REPEAT_DELAY = 400    // Initial repeat delay in ms
const REPEAT_INTERVAL = 120 // Repeat interval in ms

export function useGamepad(options = {}) {
  const {
    onNavigate,   // (direction: 'up'|'down'|'left'|'right') => void
    onConfirm,    // () => void
    onBack,       // () => void
    onTabPrev,    // () => void  (LB)
    onTabNext,    // () => void  (RB)
  } = options

  const gamepadActive = ref(false)
  const gamepadConnected = ref(false)

  let rafId = null
  let prevButtons = []
  let prevAxes = []
  let repeatTimers = {}

  function getGamepad() {
    const gamepads = navigator.getGamepads()
    for (let i = 0; i < gamepads.length; i++) {
      if (gamepads[i] && gamepads[i].connected) return gamepads[i]
    }
    return null
  }

  function isButtonPressed(gp, index) {
    return gp.buttons[index] && gp.buttons[index].pressed
  }

  function handleButtonDown(buttonIndex) {
    gamepadActive.value = true

    switch (buttonIndex) {
      case BUTTON.A:
        onConfirm?.()
        break
      case BUTTON.B:
        onBack?.()
        break
      case BUTTON.LB:
        onTabPrev?.()
        break
      case BUTTON.RB:
        onTabNext?.()
        break
      case BUTTON.DPAD_UP:
        startRepeat('dpad_up', () => onNavigate?.('up'))
        break
      case BUTTON.DPAD_DOWN:
        startRepeat('dpad_down', () => onNavigate?.('down'))
        break
      case BUTTON.DPAD_LEFT:
        startRepeat('dpad_left', () => onNavigate?.('left'))
        break
      case BUTTON.DPAD_RIGHT:
        startRepeat('dpad_right', () => onNavigate?.('right'))
        break
    }
  }

  function handleButtonUp(buttonIndex) {
    switch (buttonIndex) {
      case BUTTON.DPAD_UP:    stopRepeat('dpad_up'); break
      case BUTTON.DPAD_DOWN:  stopRepeat('dpad_down'); break
      case BUTTON.DPAD_LEFT:  stopRepeat('dpad_left'); break
      case BUTTON.DPAD_RIGHT: stopRepeat('dpad_right'); break
    }
  }

  function startRepeat(key, action) {
    action()
    stopRepeat(key)
    repeatTimers[key] = {
      timeout: setTimeout(() => {
        repeatTimers[key].interval = setInterval(action, REPEAT_INTERVAL)
      }, REPEAT_DELAY),
    }
  }

  function stopRepeat(key) {
    if (repeatTimers[key]) {
      clearTimeout(repeatTimers[key].timeout)
      clearInterval(repeatTimers[key].interval)
      delete repeatTimers[key]
    }
  }

  function handleAxis(axisIndex, value) {
    const key = `axis_${axisIndex}_${value > 0 ? 'pos' : 'neg'}`
    const oppositeKey = `axis_${axisIndex}_${value > 0 ? 'neg' : 'pos'}`

    stopRepeat(oppositeKey)

    if (Math.abs(value) > DEADZONE) {
      gamepadActive.value = true
      if (!repeatTimers[key]) {
        const direction = axisIndex === AXIS.LEFT_Y
          ? (value > 0 ? 'down' : 'up')
          : (value > 0 ? 'right' : 'left')

        startRepeat(key, () => onNavigate?.(direction))
      }
    } else {
      stopRepeat(key)
    }
  }

  function pollGamepad() {
    const gp = getGamepad()
    if (!gp) {
      gamepadConnected.value = false
      // When no gamepad is present, stop polling — wait for gamepadconnected to wake us up
      rafId = null
      return
    }

    gamepadConnected.value = true

    // Button handling
    for (let i = 0; i < gp.buttons.length; i++) {
      const pressed = isButtonPressed(gp, i)
      const wasPressed = prevButtons[i] || false

      if (pressed && !wasPressed) handleButtonDown(i)
      if (!pressed && wasPressed) handleButtonUp(i)

      prevButtons[i] = pressed
    }

    // Left stick
    if (gp.axes.length >= 2) {
      const lx = gp.axes[AXIS.LEFT_X]
      const ly = gp.axes[AXIS.LEFT_Y]

      handleAxis(AXIS.LEFT_X, lx)
      handleAxis(AXIS.LEFT_Y, ly)
    }

    rafId = requestAnimationFrame(pollGamepad)
  }

  function startPolling() {
    if (!rafId) {
      rafId = requestAnimationFrame(pollGamepad)
    }
  }

  function onGamepadConnected() {
    gamepadConnected.value = true
    startPolling()
  }

  function onGamepadDisconnected() {
    const gp = getGamepad()
    gamepadConnected.value = !!gp
    if (!gp) {
      gamepadActive.value = false
      // Clear all repeats
      Object.keys(repeatTimers).forEach(stopRepeat)
    }
  }

  // Exit gamepad mode when the mouse moves
  function onMouseMove() {
    if (gamepadActive.value) {
      gamepadActive.value = false
    }
  }

  onMounted(() => {
    window.addEventListener('gamepadconnected', onGamepadConnected)
    window.addEventListener('gamepaddisconnected', onGamepadDisconnected)
    window.addEventListener('mousemove', onMouseMove)
    rafId = requestAnimationFrame(pollGamepad)
  })

  onUnmounted(() => {
    window.removeEventListener('gamepadconnected', onGamepadConnected)
    window.removeEventListener('gamepaddisconnected', onGamepadDisconnected)
    window.removeEventListener('mousemove', onMouseMove)
    if (rafId) cancelAnimationFrame(rafId)
    Object.keys(repeatTimers).forEach(stopRepeat)
  })

  return {
    gamepadActive,
    gamepadConnected,
  }
}

/**
 * Focus navigation helper: move focus between focusable elements inside a container
 */
export function navigateFocus(direction, container) {
  const root = container || document
  const focusables = Array.from(root.querySelectorAll(
    '[data-focusable]:not([disabled]):not([data-focusable="false"]), .app-tile, .nav-item, .desktop-card.action-card, button:not([disabled]), a[href], input:not([disabled]), select:not([disabled])'
  )).filter(el => {
    // Exclude invisible elements
    const rect = el.getBoundingClientRect()
    return rect.width > 0 && rect.height > 0
  })

  if (focusables.length === 0) return

  const current = document.activeElement
  const currentIndex = focusables.indexOf(current)

  if (currentIndex === -1) {
    // No current focus — select the first
    focusables[0].focus()
    return
  }

  const currentRect = current.getBoundingClientRect()
  let best = null
  let bestScore = Infinity

  for (let i = 0; i < focusables.length; i++) {
    if (i === currentIndex) continue
    const el = focusables[i]
    const rect = el.getBoundingClientRect()
    const cx = rect.left + rect.width / 2
    const cy = rect.top + rect.height / 2
    const curCx = currentRect.left + currentRect.width / 2
    const curCy = currentRect.top + currentRect.height / 2

    const dx = cx - curCx
    const dy = cy - curCy

    let isValid = false
    let primaryDist = 0
    let crossDist = 0

    switch (direction) {
      case 'up':
        isValid = dy < -5
        primaryDist = Math.abs(dy)
        crossDist = Math.abs(dx)
        break
      case 'down':
        isValid = dy > 5
        primaryDist = Math.abs(dy)
        crossDist = Math.abs(dx)
        break
      case 'left':
        isValid = dx < -5
        primaryDist = Math.abs(dx)
        crossDist = Math.abs(dy)
        break
      case 'right':
        isValid = dx > 5
        primaryDist = Math.abs(dx)
        crossDist = Math.abs(dy)
        break
    }

    if (!isValid) continue

    // Prefer candidates close along the primary axis; cross-axis distance is secondary
    const score = primaryDist + crossDist * 2
    if (score < bestScore) {
      bestScore = score
      best = el
    }
  }

  if (best) {
    best.focus()
    // Make sure it scrolls into view
    best.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
  }
}

/**
 * Confirm the currently focused element (simulate click)
 */
export function confirmFocused() {
  const el = document.activeElement
  if (el && el !== document.body) {
    el.click()
  }
}
