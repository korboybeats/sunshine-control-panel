let _invoke = null

async function ensureInvoke() {
  if (!_invoke) {
    const tauri = await import('@tauri-apps/api/core')
    _invoke = tauri.invoke
  }
  return _invoke
}

/**
 * Invoke a Tauri backend command
 * @param {string} cmd - command name
 * @param {object} params - parameters
 * @returns {Promise<any>}
 */
export async function tauriInvoke(cmd, params = {}) {
  const invoke = await ensureInvoke()
  return invoke(cmd, params)
}
