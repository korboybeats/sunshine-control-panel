import { createApp } from 'vue'
import './styles/global.less'
import './styles/dialog.less'  // Import dialog styles
import App from './App.vue'
// Import Tauri polyfill to support global APIs
import './tauri-polyfill.js'

const app = createApp(App);

app.mount('#app');
