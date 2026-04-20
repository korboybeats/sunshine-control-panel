/**
 * AI action parsing and execution
 * Parses JSON action instructions returned by AI and applies changes to Sunshine.
 */

import { ElMessage } from 'element-plus'

/**
 * Get the Sunshine API proxy URL
 */
async function getProxyUrl() {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    return await invoke('get_proxy_url_command')
  } catch {
    return 'https://localhost:47990'
  }
}

/**
 * Fetch the current apps list
 */
async function fetchApps() {
  const proxyUrl = await getProxyUrl()
  const resp = await fetch(`${proxyUrl}/api/apps`)
  if (!resp.ok) throw new Error(`Failed to fetch apps list: ${resp.status}`)
  const data = await resp.json()
  return { apps: data.apps || data || [], proxyUrl }
}

/**
 * Save a single app (matches Sunshine API format)
 */
async function saveApp(proxyUrl, apps, appIndex, app) {
  const editApp = { index: appIndex, ...app }
  const resp = await fetch(`${proxyUrl}/api/apps`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ apps, editApp }),
  })
  if (!resp.ok) throw new Error(`Save failed: ${resp.status}`)
}

/**
 * Generate a random ID (10 alphanumeric chars)
 */
function generateId() {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789'
  return Array.from({ length: 10 }, () => chars[Math.floor(Math.random() * chars.length)]).join('')
}

/**
 * Fetch recent Sunshine logs for the AI to analyze.
 * Only includes the last N lines to keep token usage reasonable.
 */
export async function getLogsContext(maxLines = 150) {
  try {
    const proxyUrl = await getProxyUrl()
    const resp = await fetch(`${proxyUrl}/api/logs`, { headers: { 'X-Log-Offset': '0' } })
    if (!resp.ok) return ''
    const text = await resp.text()
    const lines = text.split('\n')
    const recent = lines.slice(-maxLines).join('\n')
    return `\n\nRecent Sunshine logs (last ${Math.min(lines.length, maxLines)} lines):\n\`\`\`\n${recent}\n\`\`\``
  } catch {
    return ''
  }
}

/**
 * Get a summary of the currently configured apps, for AI context
 */
export async function getAppsContext() {
  try {
    const { apps } = await fetchApps()
    const summary = apps
      .map((a) => {
        const menuCmds = (a['menu-cmd'] || []).map((c) => `  - ${c.name}: ${c.cmd}`).join('\n')
        return `- ${a.name}${a.cmd ? ` (cmd: ${a.cmd})` : ''}${menuCmds ? `\n  Existing menu commands:\n${menuCmds}` : ''}`
      })
      .join('\n')
    return `\n\nCurrently configured apps:\n${summary}`
  } catch {
    return ''
  }
}

/**
 * Parse a JSON action instruction from an AI reply
 * @returns {object|null} parsed action object, or null
 */
export function parseAction(message) {
  try {
    const jsonMatch = message.match(/```json\s*([\s\S]*?)\s*```/) || message.match(/(\{[\s\S]*"action"[\s\S]*\})/)
    if (!jsonMatch) return null

    const action = JSON.parse(jsonMatch[1])
    const validActions = ['add_menu_cmd', 'add_prep_cmd', 'modify_config', 'enhance_apps']
    if (validActions.includes(action.action)) return action
    return null
  } catch {
    return null
  }
}

/**
 * Execute an AI-suggested action
 * @returns {string} description of the result
 */
export async function executeAction(action) {
  if (!action) throw new Error('Invalid action')

  switch (action.action) {
    case 'add_menu_cmd':
      return applyMenuCmd(action)
    case 'add_prep_cmd':
      return applyPrepCmd(action)
    case 'enhance_apps':
      return applyEnhanceApps(action)
    case 'modify_config':
      return applyConfigChange(action)
    default:
      throw new Error(`Unknown action type: ${action.action}`)
  }
}

/**
 * Add menu commands
 */
async function applyMenuCmd(action) {
  const { apps, proxyUrl } = await fetchApps()
  const targetName = action.app_name || 'Desktop'
  const appIndex = apps.findIndex((a) => a.name === targetName)
  if (appIndex === -1) throw new Error(`App "${targetName}" not found`)

  const app = apps[appIndex]
  if (!app['menu-cmd']) app['menu-cmd'] = []

  for (const cmd of action.commands) {
    const existIdx = app['menu-cmd'].findIndex((c) => c.name === cmd.name)
    const newCmd = {
      id: generateId(),
      name: cmd.name,
      cmd: cmd.cmd,
      elevated: cmd.elevated || 'false',
    }
    if (existIdx >= 0) {
      app['menu-cmd'][existIdx] = { ...app['menu-cmd'][existIdx], ...newCmd, id: app['menu-cmd'][existIdx].id }
    } else {
      app['menu-cmd'].push(newCmd)
    }
  }

  await saveApp(proxyUrl, apps, appIndex, app)

  const cmdNames = action.commands.map((c) => c.name).join(', ')
  ElMessage.success(`Added menu commands: ${cmdNames}`)
  return `✅ Added ${action.commands.length} menu command(s) to "${targetName}": ${cmdNames}\n\n${action.explanation || ''}`
}

/**
 * Add prep (pre/post) commands
 */
async function applyPrepCmd(action) {
  const { apps, proxyUrl } = await fetchApps()
  const targetName = action.app_name || 'Desktop'
  const appIndex = apps.findIndex((a) => a.name === targetName)
  if (appIndex === -1) throw new Error(`App "${targetName}" not found`)

  const app = apps[appIndex]
  if (!app['prep-cmd']) app['prep-cmd'] = []

  for (const cmd of action.commands) {
    app['prep-cmd'].push({
      do: cmd.do || '',
      undo: cmd.undo || '',
      elevated: cmd.elevated || 'false',
    })
  }

  await saveApp(proxyUrl, apps, appIndex, app)

  ElMessage.success(`Added ${action.commands.length} prep command(s)`)
  return `✅ Added ${action.commands.length} prep command(s) to "${targetName}"\n\n${action.explanation || ''}`
}

/**
 * Modify Sunshine config
 */
async function applyConfigChange(action) {
  if (!action?.changes) throw new Error('No config changes provided')

  const proxyUrl = await getProxyUrl()

  // Fetch current config
  const getResp = await fetch(`${proxyUrl}/api/config`)
  if (!getResp.ok) throw new Error(`Failed to fetch config: ${getResp.status}`)
  const currentConfig = await getResp.json()

  // Merge updates
  const updates = {}
  for (const change of action.changes) {
    updates[change.key] = change.value
  }

  // Save config
  const saveResp = await fetch(`${proxyUrl}/api/config`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ ...currentConfig, ...updates }),
  })
  if (!saveResp.ok) throw new Error(`Failed to save config: ${saveResp.status}`)

  ElMessage.success(action.explanation || 'Config updated')
  return `✅ Applied changes: ${action.explanation}`
}

/**
 * Bulk-enhance app configs
 */
async function applyEnhanceApps(action) {
  if (!action?.apps?.length) throw new Error('No apps to enhance')

  const { apps, proxyUrl } = await fetchApps()
  let updatedCount = 0

  for (const enhancedApp of action.apps) {
    const appIndex = apps.findIndex((a) => a.name === enhancedApp.name)
    if (appIndex === -1) continue

    const app = apps[appIndex]

    if (enhancedApp['prep-cmd']?.length) {
      if (!app['prep-cmd']) app['prep-cmd'] = []
      for (const cmd of enhancedApp['prep-cmd']) {
        app['prep-cmd'].push({
          do: cmd.do || '',
          undo: cmd.undo || '',
          elevated: cmd.elevated || 'false',
        })
      }
    }

    if (enhancedApp['menu-cmd']?.length) {
      if (!app['menu-cmd']) app['menu-cmd'] = []
      for (const cmd of enhancedApp['menu-cmd']) {
        const existIdx = app['menu-cmd'].findIndex((c) => c.name === cmd.name)
        const newCmd = {
          id: generateId(),
          name: cmd.name,
          cmd: cmd.cmd,
          elevated: cmd.elevated || 'false',
        }
        if (existIdx >= 0) {
          app['menu-cmd'][existIdx] = { ...app['menu-cmd'][existIdx], ...newCmd, id: app['menu-cmd'][existIdx].id }
        } else {
          app['menu-cmd'].push(newCmd)
        }
      }
    }

    const editApp = { index: appIndex, ...app }
    const saveResp = await fetch(`${proxyUrl}/api/apps`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ apps, editApp }),
    })
    if (saveResp.ok) updatedCount++
  }

  ElMessage.success(`Enhanced configs for ${updatedCount} app(s)`)
  return `✅ Enhanced configs for ${updatedCount}/${action.apps.length} app(s)\n\n${action.explanation || ''}`
}
