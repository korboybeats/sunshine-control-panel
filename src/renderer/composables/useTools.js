import { ElMessage, ElMessageBox } from 'element-plus'
import { openExternalUrl, tools, vmouse } from '@/tauri-adapter.js'

/**
 * Tool-actions Composable
 */
export function useTools() {
  /**
   * Common confirm-dialog action helper
   * @param {string} message - confirm message
   * @param {string} title - dialog title
   * @param {function} action - action to execute
   * @param {string} successMsg - success message
   */
  const confirmAction = async (message, title, action, successMsg) => {
    try {
      await ElMessageBox.confirm(message, title, {
        confirmButtonText: 'OK',
        cancelButtonText: 'Cancel',
        type: 'warning',
      })
      await action()
      ElMessage.success(successMsg)
    } catch (error) {
      if (error !== 'cancel') {
        ElMessage.error(`Operation failed: ${error}`)
      }
    }
  }

  /**
   * Uninstall VDD
   */
  const uninstallVdd = async () => {
    await confirmAction(
      'Uninstall the virtual display driver? This requires administrator privileges.',
      'Confirm uninstall',
      tools.uninstallVddDriver,
      'Uninstall request sent'
    )
  }

  /**
   * Restart the graphics driver
   */
  const restartDriver = async () => {
    await confirmAction(
      'Restart the graphics driver? The screen will briefly go black.',
      'Confirm restart',
      tools.restartGraphicsDriver,
      'Restart request sent'
    )
  }

  /**
   * Restart the Sunshine service
   */
  const restartSunshine = async () => {
    await confirmAction(
      'Restart the Sunshine service? All active connections will be dropped.\n\nIf a UAC prompt appears, click "Yes" to confirm.\nSunshine should come back up within a few seconds.',
      'Confirm restart',
      tools.restartSunshineService,
      'Restart request sent'
    )
  }

  /**
   * Restart Sunshine in user mode (not service mode)
   */
  const restartSunshineInUserMode = async () => {
    await confirmAction(
      'Restart Sunshine in user mode?\n\nThis will:\n1. Stop the Sunshine service\n2. Kill all Sunshine processes\n3. Start Sunshine in user mode\n\nAll active connections will be dropped.',
      'Confirm restart',
      tools.restartSunshineInUserMode,
      'User-mode restart request sent'
    )
  }

  /**
   * Open the stream timer window
   */
  const openTimer = async () => {
    await createWindow('/stop-clock-canvas/index.html', 'Stream Timer', {
      prefix: 'timer',
      width: 1080,
      height: 600,
    })
  }

  /**
   * Open an external URL
   * @param {string} url - URL to open
   */
  const openUrl = async (url) => {
    await openExternalUrl(url)
  }

  /**
   * Clean up unused cover images and temp files
   */
  const cleanupCovers = async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core')

      // First check whether we're running as admin
      const isRunningAsAdmin = await invoke('is_running_as_admin')

      if (!isRunningAsAdmin) {
        // Not admin — prompt for restart
        await ElMessageBox.confirm('Cleaning temp files requires administrator privileges.\n\nRestart the app as administrator?', 'Admin privileges required', {
          confirmButtonText: 'Restart as admin',
          cancelButtonText: 'Cancel',
          type: 'warning',
        })

        // User confirmed — restart as admin
        await restartAsAdmin()
        return
      }

      // Already admin — continue with cleanup
      await ElMessageBox.confirm(
        'This will delete:\n1. Cover images no longer referenced by any app\n2. temp_ files under the config directory\n\nContinue?',
        'Clean up unused files',
        {
          confirmButtonText: 'OK',
          cancelButtonText: 'Cancel',
          type: 'warning',
        }
      )

      // Show a loading toast
      const loading = ElMessage({
        message: 'Cleaning up unused files...',
        type: 'info',
        duration: 0,
      })

      // Call the Tauri command
      const result = await invoke('cleanup_unused_covers')

      loading.close()

      // Show result
      if (result.success) {
        if (result.deleted_count > 0) {
          ElMessageBox.alert(
            `${result.message}\n\nFiles deleted: ${result.deleted_count}\nSpace freed: ${(
              result.freed_space / 1024
            ).toFixed(2)} KB`,
            'Cleanup complete',
            {
              confirmButtonText: 'OK',
              type: 'success',
            }
          )
        } else {
          ElMessage.success(result.message)
        }
      } else {
        ElMessage.error('Cleanup failed: ' + result.message)
      }
    } catch (error) {
      if (error !== 'cancel') {
        console.error('Cleanup failed:', error)
        ElMessage.error('Cleanup failed: ' + error)
      }
    }
  }

  /**
   * Restart the GUI with administrator privileges
   */
  const restartAsAdmin = async () => {
    try {
      // Confirm dialog
      await ElMessageBox.confirm('The app will restart with administrator privileges; the current window will close. Continue?', 'Elevate privileges', {
        confirmButtonText: 'OK',
        cancelButtonText: 'Cancel',
        type: 'warning',
      })

      // Show prompt
      ElMessage.info('Requesting administrator privileges...')

      // Call the Tauri command
      const { invoke } = await import('@tauri-apps/api/core')
      await invoke('restart_as_admin')

      // If we reached here, the restart was successfully requested
      ElMessage.success('Restarting as administrator...')
    } catch (error) {
      if (error !== 'cancel') {
        console.error('Restart failed:', error)
        ElMessage.error('Restart failed: ' + error)
      }
    }
  }

  /**
   * Check for updates. Returns UpdateInfo (includes `is_latest`) — the caller decides how to display.
   */
  const checkForUpdates = async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core')

      ElMessage.info('Checking for updates...')

      const result = await invoke('check_for_updates')

      if (result) {
        return result // Return update info (with `is_latest`) so the caller handles display
      }
      return null
    } catch (error) {
      console.error('Update check failed:', error)
      ElMessage.error('Update check failed: ' + error)
      return null
    }
  }

  /**
   * Shared window-creation helper
   * @param {string} url - URL path for the window
   * @param {string} title - window title
   * @param {object} options - window config options
   */
  const createWindow = async (url, title, options = {}) => {
    try {
      const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow')
      const baseUrl = window.location.origin
      const windowId = `${options.prefix || 'window'}_${Date.now()}`

      const newWindow = new WebviewWindow(windowId, {
        url: `${baseUrl}${url}`,
        title,
        width: options.width || 1080,
        height: options.height || 800,
        decorations: options.decorations !== false,
        center: true,
      })

      // Wait for the window to be created, then show it
      newWindow.once('tauri://created', async () => {
        console.log(`✅ ${title} window created`)
        await newWindow.show()
        await newWindow.setFocus()
        console.log(`✅ ${title} window shown`)
      })

      newWindow.once('tauri://error', (e) => {
        console.error(`❌ Failed to create ${title} window:`, e)
        ElMessage.error(`Failed to create ${title} window`)
      })
    } catch (error) {
      console.error(`❌ Failed to open ${title}:`, error)
      ElMessage.error(`Failed to open ${title}: ${error.message}`)
    }
  }

  /**
   * Install the virtual-mouse driver
   */
  const installVmouse = async () => {
    await confirmAction(
      'Install the virtual mouse driver. This requires administrator privileges.\nA system restart may be needed for the driver to take effect.',
      'Confirm install',
      vmouse.install,
      'Install request sent'
    )
  }

  /**
   * Uninstall the virtual-mouse driver
   */
  const uninstallVmouse = async () => {
    await confirmAction(
      'Uninstall the virtual mouse driver? This requires administrator privileges.\nSunshine will fall back to SendInput-based mouse input.',
      'Confirm uninstall',
      vmouse.uninstall,
      'Uninstall request sent'
    )
  }

  return {
    confirmAction,
    uninstallVdd,
    restartDriver,
    restartSunshine,
    restartSunshineInUserMode,
    openTimer,
    openUrl,
    cleanupCovers,
    restartAsAdmin,
    checkForUpdates,
    createWindow,
    installVmouse,
    uninstallVmouse,
  }
}

