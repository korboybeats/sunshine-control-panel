import { ElMessage } from 'element-plus'

/**
 * Window controls Composable
 */
export function useWindowControls(isMaximized) {
  /**
   * Perform a window action
   * @param {string} action - action name (minimize/hide)
   * @param {string} actionName - action display name
   */
  const performWindowAction = async (action, actionName) => {
    try {
      const { getCurrentWebviewWindow } = await import('@tauri-apps/api/webviewWindow')
      const currentWindow = getCurrentWebviewWindow()
      await currentWindow[action]()
      console.log(`✅ Window ${actionName}`)
    } catch (error) {
      console.error(`Failed to ${actionName} window:`, error)
      ElMessage.error(`Failed to ${actionName}: ${error.message}`)
    }
  }

  /**
   * Minimize the window
   */
  const minimizeWindow = async () => {
    await performWindowAction('minimize', 'minimize')
  }

  /**
   * Toggle maximized state
   */
  const toggleMaximize = async () => {
    try {
      const { getCurrentWebviewWindow } = await import('@tauri-apps/api/webviewWindow')
      const window = getCurrentWebviewWindow()

      const maximized = await window.isMaximized()

      if (maximized) {
        await window.unmaximize()
        isMaximized.value = false
        console.log('✅ Window restored')
      } else {
        await window.maximize()
        isMaximized.value = true
        console.log('✅ Window maximized')
      }
    } catch (error) {
      console.error('❌ Failed to toggle maximize:', error)
      ElMessage.error(`Failed to toggle maximize: ${error}`)
    }
  }

  /**
   * Close window (hide)
   */
  const closeWindow = async () => {
    await performWindowAction('hide', 'hide')
  }

  return {
    minimizeWindow,
    toggleMaximize,
    closeWindow,
  }
}


