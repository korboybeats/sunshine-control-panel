/**
 * Tauri API adapter layer
 * Migrates Electron IPC calls over to Tauri invoke calls
 */

import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-shell'

// ─── Helpers ────────────────────────────────────────────

/** Invoke a command, returning { success, data, message } */
async function wrapResult(cmd, args) {
  try {
    const data = await invoke(cmd, args)
    return { success: true, data }
  } catch (error) {
    return { success: false, message: error }
  }
}

/** Invoke a command, returning `fallback` on failure */
async function wrapDefault(cmd, fallback, args) {
  try {
    return await invoke(cmd, args)
  } catch (error) {
    console.warn(`${cmd} failed:`, error)
    return fallback
  }
}

// ─── Theme ───────────────────────────────────────────────

export const darkMode = {
  toggle: () => invoke('toggle_dark_mode'),
  system: async () => true,
}

// ─── External URL ────────────────────────────────────────

export async function openExternalUrl(url) {
  try {
    await open(url)
    return true
  } catch (error) {
    console.error('Failed to open external URL:', error)
    return false
  }
}

// ─── VDD settings ────────────────────────────────────────

export const vdd = {
  getGPUs: () => wrapResult('get_gpus'),
  loadSettings: () => wrapResult('load_vdd_settings'),
  saveSettings: (settings) => wrapResult('save_vdd_settings', { settings }),
  getEdidFilePath: () => wrapResult('get_vdd_edid_file_path'),
  uploadEdidFile: (fileData) => wrapResult('upload_edid_file', { fileData }),
  readEdidFile: () => wrapResult('read_edid_file'),
  deleteEdidFile: () => wrapResult('delete_edid_file'),

  async execPipeCmd(command) {
    try {
      await invoke('exec_pipe_cmd', { command })
      return true
    } catch (error) {
      console.error('Failed to execute pipe command:', error)
      return false
    }
  },
}

// ─── Virtual Mouse ───────────────────────────────────────

export const vmouse = {
  getStatus: () => wrapResult('get_vmouse_status'),
  install: () => wrapResult('install_vmouse_driver'),
  uninstall: () => wrapResult('uninstall_vmouse_driver'),
  setConfig: (enabled) => wrapResult('set_vmouse_config', { enabled }),
}

// ─── Sunshine config ─────────────────────────────────────

export const sunshine = {
  getVersion: () => wrapDefault('get_sunshine_version', 'Unknown'),
  parseConfig: () => wrapDefault('parse_sunshine_config', {}),
  getUrl: () => wrapDefault('get_sunshine_url', 'https://localhost:47990/'),
  getCommandLineUrl: () => wrapDefault('get_command_line_url', null),
  getProxyUrl: () => wrapDefault('get_proxy_url_command', 'http://localhost:48081'),
  getActiveSessions: () => wrapDefault('get_active_sessions', []),
  getLocale: () => wrapDefault('get_sunshine_locale', 'en'),
  setLocale: (locale) => invoke('set_sunshine_locale', { locale }),
  changeBitrate: (clientName, bitrate) => invoke('change_bitrate', { clientName, bitrate }),
}

// ─── System tools ────────────────────────────────────────

export const tools = {
  restartGraphicsDriver: () => invoke('restart_graphics_driver'),
  restartSunshineService: () => invoke('restart_sunshine_service'),
  restartSunshineInUserMode: () => invoke('restart_sunshine_in_user_mode'),
  uninstallVddDriver: () => invoke('uninstall_vdd_driver'),
}

// ─── Moonlight Web ───────────────────────────────────────

export const moonlightWeb = {
  getStatus: () => wrapDefault('moonlight_web_get_status',
    { installed: false, running: false, install_path: '', version: '', access_url: '', port: 8080 }),
  getConfig: () => wrapDefault('moonlight_web_get_config',
    { web_server: { bind_address: '0.0.0.0:8080' }, webrtc: null, default_settings: null }),
  start: () => invoke('moonlight_web_start'),
  stop: () => invoke('moonlight_web_stop'),
  saveConfig: (config) => invoke('moonlight_web_save_config', { config }),
  checkRelease: () => invoke('moonlight_web_check_release'),
  download: (url, version) => invoke('moonlight_web_download', { url, version: version || '' }),
  getInstallPath: () => invoke('moonlight_web_get_install_path'),
  generateCert: () => invoke('moonlight_web_generate_cert'),
}

// ─── File system ─────────────────────────────────────────

export async function readDirectory(path) {
  return []
}

// ─── Exports ─────────────────────────────────────────────

export default {
  darkMode,
  openExternalUrl,
  vdd,
  vmouse,
  sunshine,
  tools,
  moonlightWeb,
  readDirectory,
}
