<template>
  <div class="system-tools">
    <!-- This component provides UI interactions for tool utility functions -->
  </div>
</template>

<script setup>
import { ElMessage, ElMessageBox } from 'element-plus'
import { tools } from '@/tauri-adapter.js'
import { useI18n } from '../desktop/i18n/index.js'

const { t } = useI18n()

// Functions exposed for global use
defineExpose({
  async confirmAndUninstallVdd() {
    try {
      await ElMessageBox.confirm(
        t.value.systemTools.vddUninstallConfirm,
        t.value.systemTools.vddUninstallTitle,
        {
          confirmButtonText: t.value.systemTools.confirm,
          cancelButtonText: t.value.systemTools.cancel,
          type: 'warning',
        }
      )
      
      const result = await tools.uninstallVddDriver()
      ElMessage.success(result)
    } catch (error) {
      if (error !== 'cancel') {
        ElMessage.error(t.value.systemTools.vddUninstallFailed.replace('{error}', error))
      }
    }
  },

  async confirmAndRestartDriver() {
    try {
      await ElMessageBox.confirm(
        t.value.systemTools.restartGpuConfirm,
        t.value.systemTools.restartGpuTitle,
        {
          confirmButtonText: t.value.systemTools.confirm,
          cancelButtonText: t.value.systemTools.cancel,
          type: 'warning',
        }
      )
      
      const result = await tools.restartGraphicsDriver()
      ElMessage.success(result)
    } catch (error) {
      if (error !== 'cancel') {
        ElMessage.error(t.value.systemTools.restartGpuFailed?.replace('{error}', error) || String(error))
      }
    }
  },

  async confirmAndRestartSunshine() {
    try {
      await ElMessageBox.confirm(
        t.value.systemTools.restartSunshineConfirm,
        t.value.systemTools.restartSunshineTitle,
        {
          confirmButtonText: t.value.systemTools.confirm,
          cancelButtonText: t.value.systemTools.cancel,
          type: 'warning',
        }
      )
      
      await tools.restartSunshineService()
      
      // Show a detailed success notification
      await ElMessageBox.alert(
        t.value.systemTools.restartSunshineMsg,
        t.value.systemTools.restartSunshineSuccess,
        {
          confirmButtonText: t.value.systemTools.confirm,
          type: 'success',
        }
      )
      
      // Close the window after 3 seconds
      setTimeout(() => {
        if (window.__TAURI__) {
          window.__TAURI__.window.getCurrent().close()
        }
      }, 3000)
    } catch (error) {
      if (error !== 'cancel') {
        ElMessage.error(t.value.systemTools.restartSunshineFailed?.replace('{error}', error) || String(error))
      }
    }
  }
})
</script>

<style scoped>
.system-tools {
  display: none;
}
</style>

