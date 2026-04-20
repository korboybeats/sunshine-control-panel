import { createApp } from 'vue'
import '../style.css'
import App from './index.vue'
import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'
// Import Tauri polyfill
import '../tauri-polyfill.js'

const app = createApp(App)
app.use(ElementPlus)
app.mount('#app')
