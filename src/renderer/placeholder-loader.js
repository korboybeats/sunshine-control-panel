/**
 * Placeholder page loader
 * After the placeholder page loads, automatically fetches the Sunshine URL and redirects.
 */

import { invoke } from '@tauri-apps/api/core'

// Wait for DOM to finish loading
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initLoader)
} else {
  initLoader()
}

async function initLoader() {
  try {
    // Wait 1 second to show the placeholder animation
    await new Promise((resolve) => setTimeout(resolve, 1000))

    // Check whether a URL was specified on the command line
    let sunshineUrl = await invoke('get_command_line_url')

    if (sunshineUrl) {
      console.log('✅ Detected command-line URL:', sunshineUrl)
      updateLoadingText('Loading the specified URL...')
      // Pass the URL to sunshine-frame.html (preserves the menu)
      window.location.href = `./sunshine-frame.html?url=${encodeURIComponent(sunshineUrl)}`
    } else {
      console.log('✅ Loading Sunshine with custom menu...')
      updateLoadingText('Connecting to server...')
      // Load Sunshine Frame with the custom menu
      window.location.href = './sunshine-frame.html'
    }
  } catch (error) {
    console.error('Failed to load Sunshine:', error)

    // Show an error message on failure
    const container = document.querySelector('.placeholder-container')
    if (container) {
      const errorText = document.createElement('p')
      errorText.style.color = '#ff4d4f'
      errorText.style.marginTop = '20px'
      errorText.textContent = 'Unable to connect to Sunshine. Please make sure Sunshine is running.'
      container.appendChild(errorText)

      // Try the default URL after 5 seconds
      setTimeout(() => {
        window.location.href = 'https://localhost:47990/'
      }, 5000)
    }
  }
}

// Update the loading hint text
function updateLoadingText(text) {
  const textElement = document.querySelector('.placeholder-text p')
  if (textElement) {
    textElement.textContent = text
  }
}

// Check whether the Sunshine URL is reachable
async function checkSunshineAvailability(url) {
  try {
    const response = await fetch(url, {
      method: 'HEAD',
      mode: 'no-cors',
    })
    return true
  } catch (error) {
    return false
  }
}
