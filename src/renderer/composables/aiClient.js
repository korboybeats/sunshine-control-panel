/**
 * AI HTTP communication layer
 * Handles communication with AI service APIs; uses the Tauri proxy to bypass CORS.
 */

import { AI_PROVIDERS } from './aiProviders.js'

/**
 * Send an HTTP request through the Tauri backend proxy (bypasses CORS).
 * Falls back to fetch if not running in a Tauri environment.
 */
export async function proxyFetch(url, method, headers, body) {
  const tauri = window.__TAURI__
  if (tauri?.core?.invoke) {
    const result = await tauri.core.invoke('ai_api_proxy', {
      request: {
        url,
        method,
        headers: headers || {},
        body: body ? JSON.stringify(body) : null,
      },
    })
    return JSON.parse(result)
  }
  // Fall back to direct fetch (web environment)
  const resp = await fetch(url, {
    method,
    headers: { ...headers, 'Content-Type': 'application/json' },
    body: body ? JSON.stringify(body) : undefined,
  })
  if (!resp.ok) {
    const error = await resp.text()
    throw new Error(`${resp.status} - ${error.substring(0, 200)}`)
  }
  return resp.json()
}

/**
 * Get a provider's API type
 */
export function getApiType(providerValue) {
  const provider = AI_PROVIDERS.find((p) => p.value === providerValue)
  return provider?.apiType || 'openai'
}

/**
 * Call an OpenAI-compatible API
 */
export async function callOpenAI(apiBase, apiKey, model, messages, maxTokens = 2048) {
  const headers = {}
  if (apiKey) {
    headers['Authorization'] = `Bearer ${apiKey}`
  }

  const base = apiBase.replace(/\/+$/, '')
  const data = await proxyFetch(`${base}/chat/completions`, 'POST', headers, {
    model,
    messages,
    temperature: 0.7,
    max_tokens: maxTokens,
  })
  return data.choices?.[0]?.message?.content || ''
}

/**
 * Call the Anthropic Claude API
 */
export async function callAnthropic(apiBase, apiKey, model, messages, maxTokens = 2048) {
  const systemMsg = messages.find((m) => m.role === 'system')?.content || ''
  const chatMsgs = messages.filter((m) => m.role !== 'system')

  const base = apiBase.replace(/\/+$/, '')
  const data = await proxyFetch(
    `${base}/v1/messages`,
    'POST',
    {
      'x-api-key': apiKey,
      'anthropic-version': '2023-06-01',
    },
    { model, system: systemMsg, messages: chatMsgs, max_tokens: maxTokens }
  )
  return data.content?.map((c) => c.text).join('') || ''
}

/**
 * Unified LLM entry point
 * @param {object} config - AI config
 * @param {Array} messages - message list
 * @param {number} maxTokens - max token count
 */
export async function callLLM(config, messages, maxTokens = 2048) {
  const apiType = getApiType(config.provider)
  if (apiType === 'anthropic') {
    return callAnthropic(config.apiBase, config.apiKey, config.model, messages, maxTokens)
  }
  return callOpenAI(config.apiBase, config.apiKey, config.model, messages, maxTokens)
}

/**
 * Build vision message content with an image (multimodal)
 * @param {string} text - text prompt
 * @param {string} imageDataUrl - image in data:image/jpeg;base64,... format
 * @returns content array suitable for OpenAI / Anthropic vision APIs
 */
export function buildVisionContent(text, imageDataUrl) {
  // Extract base64 and media type from the data URL
  const match = imageDataUrl.match(/^data:(image\/\w+);base64,(.+)$/)
  if (!match) return text // fallback to text-only

  const [, mediaType, base64Data] = match

  return [
    { type: 'text', text },
    {
      type: 'image_url',
      image_url: { url: imageDataUrl, detail: 'low' }, // low detail = fewer tokens
    },
  ]
}

/**
 * Build Anthropic-format vision message content
 */
export function buildAnthropicVisionContent(text, imageDataUrl) {
  const match = imageDataUrl.match(/^data:(image\/\w+);base64,(.+)$/)
  if (!match) return text

  const [, mediaType, base64Data] = match

  return [
    {
      type: 'image',
      source: { type: 'base64', media_type: mediaType, data: base64Data },
    },
    { type: 'text', text },
  ]
}

/**
 * Call a vision-capable LLM (send screenshot + text prompt)
 */
export async function callVisionLLM(config, systemPrompt, userText, imageDataUrl, maxTokens = 512) {
  const apiType = getApiType(config.provider)

  if (apiType === 'anthropic') {
    const content = buildAnthropicVisionContent(userText, imageDataUrl)
    const messages = [{ role: 'user', content }]
    return callAnthropic(config.apiBase, config.apiKey, config.model, [
      { role: 'system', content: systemPrompt },
      ...messages,
    ], maxTokens)
  }

  // OpenAI-compatible (GPT-4o, Qwen-VL, GLM-4V, etc.)
  const content = buildVisionContent(userText, imageDataUrl)
  const messages = [
    { role: 'system', content: systemPrompt },
    { role: 'user', content },
  ]
  return callOpenAI(config.apiBase, config.apiKey, config.model, messages, maxTokens)
}

/**
 * Fetch the list of available models from the API
 */
export async function fetchModels(apiBase, apiKey, providerValue) {
  const apiType = getApiType(providerValue)

  // Anthropic does not provide a /models listing endpoint
  if (apiType === 'anthropic') return []

  if (!apiBase) return []

  const headers = {}
  if (apiKey) {
    headers['Authorization'] = `Bearer ${apiKey}`
  }

  const base = apiBase.replace(/\/+$/, '')
  const data = await proxyFetch(`${base}/models`, 'GET', headers, null)

  return (data.data || data.models || [])
    .map((m) => m.id || m.name || m)
    .filter((m) => typeof m === 'string' && m.length > 0)
    .sort()
}
