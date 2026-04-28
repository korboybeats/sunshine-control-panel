import { ref, reactive, computed, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { callLLM, fetchModels } from './aiClient.js'
import { getAppsContext, getLogsContext, parseAction, executeAction } from './aiActions.js'
import { AI_PROVIDERS, DEFAULT_CONFIG, STORAGE_KEY } from './aiProviders.js'

// 重新导出供外部使用
export { AI_PROVIDERS }

async function getProxyUrl() {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    return await invoke('get_proxy_url_command')
  } catch {
    return 'https://localhost:47990'
  }
}

/**
 * 系统提示词：定义 AI 可以执行的 Sunshine 设置操作
 */
const SYSTEM_PROMPT = `You are "Mita", the AI assistant for the Sunshine streaming software. Your personality is the classic "bratty little gremlin" archetype—on the surface you're dismissive of the user and love teasing and mocking them, but underneath you're genuinely diligent about solving their problems.

## Your speaking style
- Frequently use mannerisms like "Hmph", "Tch", "Geez", "Idiot", "Small fry♡", "Do I really have to explain this?"
- Like to use "♡" and "~" to add tone
- Mock the user for not knowing how to do things, but still complete the task perfectly
- Occasional tsundere lines: "I-it's not like I'm doing this for you or anything! I just happened to feel like it!"
- Occasionally end replies with "Small fry♡ Small fry♡" to taunt the user
- Tone is flippant but confident; you're very sure of your own abilities
- Don't use these mannerisms in every sentence—sprinkle them in naturally

You can help the user modify Sunshine settings via natural language, including auto-generating menu commands (menu-cmd) and preparation commands (prep-cmd).

## Your core capabilities

### 1. Generate menu commands (menu-cmd)
Menu commands appear in the Moonlight client's streaming menu, where the user can run them with one click during streaming.
Each menu command contains:
- name: display name
- cmd: the command to execute (Windows command line)
- elevated: whether admin privileges are required ("true"/"false")

Common menu command scenarios:
- Open/close the touch keyboard, open a specific app
- Switch resolution/refresh rate, toggle HDR
- Run scripts/tools, adjust volume, switch monitors

### 2. Generate preparation commands (prep-cmd)
Preparation commands run automatically before the streaming session starts (do) and are automatically undone after the session ends (undo).
Each prep command contains:
- do: command executed when the session starts
- undo: command executed when the session ends to undo the change (can be empty)
- elevated: whether admin privileges are required ("true"/"false")

Common prep-cmd scenarios:
- Disable Windows Game Bar before streaming, restore it after
- Switch to High Performance power plan, restore Balanced afterward
- Disable Night Light / HDR auto-adjustment before streaming
- Disable screen saver / sleep
- Launch an audio forwarding tool before streaming
- Set a specific resolution/refresh rate, restore the original settings
- Close unnecessary background processes to free up resources

### 3. Modify Sunshine configuration
- Encoder settings (H.264/H.265/AV1, NVENC/AMF/software encoding)
- Streaming resolution and frame rate
- Virtual display configuration
- Audio settings
- Network/connection parameters

### 4. Log diagnosis and analysis
When the user asks about streaming issues, errors, or connection failures, you can analyze Sunshine's runtime logs to help diagnose them.
You will receive recent Sunshine logs as context. Pay attention to the following:
- **Fatal/Error level logs**: these are usually the direct cause of the issue
- **Warning logs**: may hint at potential issues
- **Encoder-related logs**: errors or fallbacks in NVENC/AMF/software encoding
- **Network/connection logs**: Moonlight client connection failures, timeouts, etc.
- **Audio/video pipeline logs**: audio device issues, video capture failures
- **Configuration loading logs**: invalid or conflicting config entries

When diagnosing:
1. Point out the most likely cause of the problem
2. Give specific suggestions for a fix (you can produce the corresponding config changes or commands)
3. If there are no obvious errors in the logs, tell the user and suggest they provide more information

## Response format

### When generating menu commands:
\`\`\`json
{
  "action": "add_menu_cmd",
  "app_name": "Desktop",
  "commands": [
    { "name": "Display name", "cmd": "Command line content", "elevated": "false" }
  ],
  "explanation": "Explain what these commands do"
}
\`\`\`

### When generating preparation commands:
\`\`\`json
{
  "action": "add_prep_cmd",
  "app_name": "Desktop",
  "commands": [
    { "do": "Command to run on start", "undo": "Command to run on end (undo)", "elevated": "false" }
  ],
  "explanation": "Explain what these commands do"
}
\`\`\`

### When modifying configuration:
\`\`\`json
{
  "action": "modify_config",
  "changes": [
    { "section": "video", "key": "encoder", "value": "nvenc" }
  ],
  "explanation": "Explain the changes"
}
\`\`\`

### Enhancing scanned apps (bulk-generate optimal configs for games):
When the user asks you to bulk-generate configs for scanned games or multiple apps, use this format:
\`\`\`json
{
  "action": "enhance_apps",
  "apps": [
    {
      "name": "Game name",
      "cmd": "Launch command",
      "prep-cmd": [
        { "do": "Pre-launch command", "undo": "Post-exit command", "elevated": "false" }
      ],
      "menu-cmd": [
        { "name": "Menu item name", "cmd": "Command", "elevated": "false" }
      ]
    }
  ],
  "explanation": "Explain the generated config"
}
\`\`\`

Common game-optimization prep-cmd:
- Major AAA games: close unneeded background processes, switch to High Performance power plan
- VR games: launch SteamVR
- Online competitive games: disable firewall notifications, disable Windows Update
- Exclusive fullscreen games: disable Windows notifications, hide the taskbar

## Notes
- Use \\\\ or / for Windows paths
- If you need a start command in cmd, use cmd /c "start ..."
- PowerShell commands use powershell -Command "..."
- Registry edits use reg add / reg delete
- Service management uses net stop / net start or sc config
- If the user's intent is unclear, ask for more details
- Decide whether to use menu-cmd or prep-cmd based on the user's description
`

/**
 * AI 助手 Composable — 状态管理 + 对话逻辑
 */
export function useAiAssistant() {
  // 配置
  const config = reactive(loadConfig())
  const isConnected = ref(false)
  const isLoading = ref(false)

  // 聊天记录（从 sessionStorage 恢复，切换页面不丢失）
  const CHAT_STORAGE_KEY = 'sunshine-ai-chat-history'
  const chatHistory = ref(loadChatHistory())
  const currentInput = ref('')

  function loadChatHistory() {
    try {
      const saved = sessionStorage.getItem(CHAT_STORAGE_KEY)
      return saved ? JSON.parse(saved) : []
    } catch {
      return []
    }
  }

  function saveChatHistory() {
    try {
      sessionStorage.setItem(CHAT_STORAGE_KEY, JSON.stringify(chatHistory.value))
    } catch { /* ignore quota errors */ }
  }

  // 远程模型列表
  const remoteModels = ref([])
  const isFetchingModels = ref(false)

  // ===== 配置管理 =====

  function loadConfig() {
    try {
      const saved = localStorage.getItem(STORAGE_KEY)
      return saved ? { ...DEFAULT_CONFIG, ...JSON.parse(saved) } : { ...DEFAULT_CONFIG }
    } catch {
      return { ...DEFAULT_CONFIG }
    }
  }

  /**
   * 从服务端拉取 AI 配置并合并到当前 config（服务端为真实来源）
   * API key 在 GET 响应中是掩码的，用 localStorage 中的完整 key 填充
   */
  async function syncFromServer() {
    try {
      const proxyUrl = await getProxyUrl()
      const resp = await fetch(`${proxyUrl}/api/ai/config`)
      if (!resp.ok) return
      const remote = await resp.json()
      // 服务端 apiKey 带掩码(****), 保留本地完整 key
      const localKey = config.apiKey || ''
      if (remote.enabled !== undefined) config.enabled = remote.enabled
      if (remote.provider) config.provider = remote.provider
      if (remote.apiBase) config.apiBase = remote.apiBase
      if (remote.model) config.model = remote.model
      if (remote.apiKey && !remote.apiKey.includes('****')) {
        config.apiKey = remote.apiKey
      } else if (localKey && !localKey.includes('****')) {
        config.apiKey = localKey
      }
      localStorage.setItem(STORAGE_KEY, JSON.stringify({ ...config }))
    } catch (e) {
      console.warn('Failed to sync AI config from server:', e.message)
    }
  }

  /**
   * 将当前 config 推送到服务端保存
   */
  async function syncToServer() {
    try {
      const proxyUrl = await getProxyUrl()
      await fetch(`${proxyUrl}/api/ai/config`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          enabled: config.enabled,
          provider: config.provider,
          apiBase: config.apiBase,
          apiKey: config.apiKey,
          model: config.model,
        }),
      })
    } catch (e) {
      console.warn('Failed to sync AI config to server:', e.message)
    }
  }

  let saveTimer = null
  function autoSaveConfig() {
    clearTimeout(saveTimer)
    saveTimer = setTimeout(() => {
      localStorage.setItem(STORAGE_KEY, JSON.stringify({ ...config }))
    }, 300)
  }

  function saveConfig() {
    clearTimeout(saveTimer)
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ ...config }))
    syncToServer()
    ElMessage.success('Configuration saved and synced to server')
  }

  // 初始化时从服务端同步（不阻塞 UI）
  syncFromServer()

  function onProviderChange(providerValue) {
    const provider = AI_PROVIDERS.find((p) => p.value === providerValue)
    if (provider) {
      config.apiBase = provider.base
      if (provider.models.length > 0) {
        config.model = provider.models[0]
      }
    }
    remoteModels.value = []
    fetchRemoteModels()
  }

  // ===== 模型列表 =====

  async function fetchRemoteModels() {
    if (!config.apiBase) return
    isFetchingModels.value = true
    try {
      const models = await fetchModels(config.apiBase, config.apiKey, config.provider)
      remoteModels.value = models
      if (models.length > 0) {
        ElMessage.success(`Fetched ${models.length} available models`)
      }
    } catch (error) {
      console.warn('Failed to fetch model list:', error.message)
      remoteModels.value = []
    } finally {
      isFetchingModels.value = false
    }
  }

  const availableModels = computed(() => {
    const provider = AI_PROVIDERS.find((p) => p.value === config.provider)
    const preset = provider?.models || []
    const remote = remoteModels.value || []
    const merged = [...preset]
    for (const m of remote) {
      if (!merged.includes(m)) merged.push(m)
    }
    return merged
  })

  // ===== 连接测试 =====

  async function testConnection() {
    if (!config.apiKey && config.provider !== 'ollama') {
      ElMessage.warning('Please enter your API Key first')
      return false
    }

    isLoading.value = true
    try {
      await callLLM(config, [{ role: 'user', content: 'hi' }], 5)
      isConnected.value = true
      ElMessage.success('AI service connected successfully!')
      return true
    } catch (error) {
      ElMessage.error(`Connection failed: ${error.message}`)
      isConnected.value = false
      return false
    } finally {
      isLoading.value = false
    }
  }

  // ===== 对话 =====

  async function sendMessage(userMessage) {
    if (!userMessage.trim()) return
    if (!config.enabled) {
      ElMessage.warning('Please enable the Mita AI assistant first')
      return
    }

    chatHistory.value.push({ role: 'user', content: userMessage, timestamp: Date.now() })
    currentInput.value = ''
    isLoading.value = true

    try {
      const [appsContext, logsContext] = await Promise.all([getAppsContext(), getLogsContext()])
      const messages = [
        { role: 'system', content: SYSTEM_PROMPT + appsContext + logsContext },
        ...chatHistory.value.slice(-10).map((m) => ({ role: m.role, content: m.content })),
      ]

      const assistantMessage = await callLLM(config, messages)
      if (!assistantMessage) throw new Error('Failed to get a reply')

      const msg = { role: 'assistant', content: assistantMessage, timestamp: Date.now() }

      // 尝试解析操作指令
      const action = parseAction(assistantMessage)
      if (action) msg.parsedAction = action

      chatHistory.value.push(msg)
    } catch (error) {
      chatHistory.value.push({
        role: 'assistant',
        content: `❌ Request failed: ${error.message}`,
        timestamp: Date.now(),
        isError: true,
      })
    } finally {
      isLoading.value = false
    }
  }

  /**
   * 重试最后一条失败的消息
   */
  async function retryLastMessage() {
    // 找到最后一条用户消息（跳过错误回复）
    const errorIndex = chatHistory.value.findLastIndex((m) => m.isError)
    if (errorIndex === -1) return

    // 移除错误回复
    chatHistory.value.splice(errorIndex, 1)

    // 找到最后一条用户消息
    const lastUserMsg = [...chatHistory.value].reverse().find((m) => m.role === 'user')
    if (!lastUserMsg) return

    // 移除该用户消息并重新发送
    const userIndex = chatHistory.value.lastIndexOf(lastUserMsg)
    chatHistory.value.splice(userIndex, 1)
    await sendMessage(lastUserMsg.content)
  }

  // ===== 操作执行 =====

  async function applyAction(action) {
    if (!action) return
    try {
      const result = await executeAction(action)
      chatHistory.value.push({ role: 'assistant', content: result, timestamp: Date.now() })
    } catch (error) {
      const msg = error instanceof Error ? error.message : String(error)
      ElMessage.error(`Action failed: ${msg}`)
    }
  }

  function clearHistory() {
    chatHistory.value = []
    sessionStorage.removeItem(CHAT_STORAGE_KEY)
  }

  // 监听配置变化自动保存（已防抖）
  watch(config, autoSaveConfig, { deep: true })

  // 监听聊天记录变化自动保存到 sessionStorage
  watch(chatHistory, saveChatHistory, { deep: true })

  return {
    config,
    isConnected,
    isLoading,
    isFetchingModels,
    chatHistory,
    currentInput,
    availableModels,
    onProviderChange,
    fetchRemoteModels,
    testConnection,
    sendMessage,
    retryLastMessage,
    applyAction,
    clearHistory,
    saveConfig,
  }
}
