<template>
  <div class="settings-page">
    <div class="settings-section glass-effect">
      <h2 class="section-title">显示设置</h2>

      <div class="setting-item">
        <div class="setting-info">
          <div class="setting-icon">
            <Moon :size="24" />
          </div>
          <div class="setting-text">
            <h3 class="setting-name">主题</h3>
            <p class="setting-desc">选择界面主题</p>
          </div>
        </div>
        <el-select v-model="currentTheme" class="setting-control" @change="onThemeChange">
          <el-option label="跟随系统" value="system" />
          <el-option label="浅色模式" value="light" />
          <el-option label="深色模式" value="dark" />
        </el-select>
      </div>

      <div class="setting-item">
        <div class="setting-info">
          <div class="setting-icon">
            <Globe :size="24" />
          </div>
          <div class="setting-text">
            <h3 class="setting-name">语言</h3>
            <p class="setting-desc">选择界面语言</p>
          </div>
        </div>
        <el-select v-model="currentLanguage" class="setting-control" @change="onLanguageChange">
          <el-option label="跟随系统" value="system" />
          <el-option label="简体中文" value="zh" />
          <el-option label="English" value="en" />
        </el-select>
      </div>
    </div>

    <div class="settings-section glass-effect">
      <h2 class="section-title">模块设置</h2>

      <div class="setting-item">
        <div class="setting-info">
          <div class="setting-icon">
            <Settings :size="24" />
          </div>
          <div class="setting-text">
            <h3 class="setting-name">默认工作模式</h3>
            <p class="setting-desc">选择模块的默认工作模式</p>
          </div>
        </div>
        <el-select v-model="defaultMode" class="setting-control" @change="onModeChange">
          <el-option label="轻量模式 (推荐)" value="lite" />
          <el-option label="完整模式" value="full" />
        </el-select>
      </div>

      <div class="setting-item setting-item-horizontal">
        <div class="setting-info">
          <div class="setting-icon">
            <Bug :size="24" />
          </div>
          <div class="setting-text">
            <h3 class="setting-name">调试模式</h3>
            <p class="setting-desc">启用后可在 logcat 中查看详细日志</p>
          </div>
        </div>
        <el-switch v-model="debugMode" class="setting-control-switch" @change="onDebugChange" />
      </div>
    </div>

    <div class="settings-section glass-effect">
      <h2 class="section-title">工具</h2>

      <div class="setting-item setting-item-horizontal">
        <div class="setting-info">
          <div class="setting-icon">
            <FileUp :size="24" />
          </div>
          <div class="setting-text">
            <h3 class="setting-name">配置转换</h3>
            <p class="setting-desc">将 system.prop 转换为机型模板</p>
          </div>
        </div>
        <el-button type="primary" @click="showInputDialog">开始转换</el-button>
      </div>
    </div>

    <!-- 转换路径输入对话框 -->
    <el-dialog
      v-model="inputDialogVisible"
      title="配置转换"
      width="90%"
      :close-on-click-modal="false"
      :append-to-body="true"
      class="template-dialog"
      modal-class="template-dialog-modal"
    >
      <el-form label-position="top">
        <el-form-item label="system.prop 文件路径">
          <el-input
            v-model="convertPath"
            placeholder="请输入 system.prop 文件的绝对路径"
            @keyup.enter="startConversion"
          />
          <div class="form-tip">默认路径: /data/adb/device_faker/config/system.prop</div>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="inputDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="converting" @click="startConversion">
          开始转换
        </el-button>
      </template>
    </el-dialog>

    <!-- 转换结果对话框 -->
    <el-dialog
      v-model="convertDialogVisible"
      title="转换结果"
      width="90%"
      :close-on-click-modal="false"
      :append-to-body="true"
      class="template-dialog"
      modal-class="template-dialog-modal"
    >
      <el-form label-width="100px" label-position="top">
        <el-form-item label="模板名称">
          <el-input v-model="convertedTemplateName" placeholder="请输入模板名称" />
        </el-form-item>
        <el-form-item label="预览内容">
          <el-input
            v-model="convertedContent"
            type="textarea"
            :rows="10"
            readonly
            class="code-font"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="convertDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="saveConvertedTemplate">保存为模板</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onActivated } from 'vue'
import { Moon, Globe, Settings, Bug, FileUp } from 'lucide-vue-next'
import { ElMessage } from 'element-plus'
import { useConfigStore } from '../stores/config'
import { useSettingsStore } from '../stores/settings'
import { writeFile, execCommand, readFile } from '../utils/ksu'
import { parse as parseToml } from 'smol-toml'
import type { Template } from '../types'

const configStore = useConfigStore()
const settingsStore = useSettingsStore()

const currentTheme = ref(settingsStore.theme)
const currentLanguage = ref(settingsStore.language)
const defaultMode = ref(configStore.config.default_mode || 'lite')
const debugMode = ref(configStore.config.debug || false)

const convertPath = ref('/data/adb/device_faker/config/system.prop')
const inputDialogVisible = ref(false)
const convertDialogVisible = ref(false)
const converting = ref(false)
const convertedTemplate = ref<Template | null>(null)
const convertedTemplateName = ref('')
const convertedContent = ref('')

function onThemeChange(value: string) {
  settingsStore.setTheme(value as 'system' | 'light' | 'dark')
}

function onLanguageChange(value: string) {
  settingsStore.setLanguage(value as 'system' | 'zh' | 'en')
  ElMessage.info('语言切换功能待实现')
}

async function onModeChange(value: string) {
  configStore.config.default_mode = value as 'lite' | 'full'
  try {
    await configStore.saveConfig()
    ElMessage.success('默认模式已更新')
  } catch {
    ElMessage.error('保存失败')
  }
}

async function onDebugChange(value: boolean) {
  configStore.config.debug = value
  try {
    await configStore.saveConfig()
    ElMessage.success(value ? '调试模式已启用' : '调试模式已关闭')
  } catch {
    ElMessage.error('保存失败')
  }
}

function showInputDialog() {
  inputDialogVisible.value = true
}

async function startConversion() {
  if (!convertPath.value) {
    ElMessage.warning('请输入文件路径')
    return
  }

  converting.value = true
  try {
    const content = await readFile(convertPath.value)
    if (!content) {
      ElMessage.error('读取文件失败或文件为空')
      return
    }

    const tempInputPath = '/data/local/tmp/device_faker_convert_input.prop'
    const tempOutputPath = '/data/local/tmp/device_faker_convert_output.toml'
    const cliPath = '/data/adb/modules/device_faker/bin/device_faker_cli'

    // 1. Write content to temp file
    await writeFile(tempInputPath, content)

    // 2. Run conversion CLI
    await execCommand(`${cliPath} convert -i ${tempInputPath} -o ${tempOutputPath}`)

    // 3. Read output
    const outputContent = await readFile(tempOutputPath)

    // 4. Parse TOML
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let parsed: any
    try {
      parsed = parseToml(outputContent)
    } catch {
      throw new Error('Invalid TOML output from CLI')
    }

    let templateData: Template | null = null
    let defaultName = 'imported_template'

    // Helper to check if an object looks like a template
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const isTemplate = (obj: any): boolean => {
      return obj && typeof obj === 'object' && (obj.manufacturer || obj.model || obj.brand)
    }

    // 1. Check if parsed object itself is a template (flat format)
    if (isTemplate(parsed)) {
      templateData = parsed as Template
    }
    // 2. Check if parsed has 'templates' and look inside
    else if (parsed.templates && typeof parsed.templates === 'object') {
      // Try to find a template in parsed.templates
      // This handles [templates."Name"]
      for (const key of Object.keys(parsed.templates)) {
        const val = parsed.templates[key]
        if (isTemplate(val)) {
          defaultName = key
          templateData = val
          break
        }
        // Handle double nesting [templates.templates."Name"]
        if (val && typeof val === 'object') {
          for (const subKey of Object.keys(val)) {
            const subVal = val[subKey]
            if (isTemplate(subVal)) {
              defaultName = subKey
              templateData = subVal
              break
            }
          }
        }
        if (templateData) break
      }
    }
    // 3. Generic search in top-level keys
    else {
      for (const key of Object.keys(parsed)) {
        const val = parsed[key]
        if (isTemplate(val)) {
          defaultName = key
          templateData = val
          break
        }
      }
    }

    if (!templateData) {
      throw new Error('Could not find valid template data in CLI output')
    }

    convertedTemplate.value = templateData
    convertedTemplateName.value = defaultName
    convertedContent.value = outputContent

    // Close input dialog and show result dialog
    inputDialogVisible.value = false
    convertDialogVisible.value = true

    // Cleanup
    await execCommand(`rm ${tempInputPath} ${tempOutputPath}`)
  } catch (err) {
    ElMessage.error(`转换失败: ${err instanceof Error ? err.message : String(err)}`)
    console.error(err)
  } finally {
    converting.value = false
  }
}

async function saveConvertedTemplate() {
  if (!convertedTemplateName.value) {
    ElMessage.warning('请输入模板名称')
    return
  }
  if (!convertedTemplate.value) return

  try {
    configStore.setTemplate(convertedTemplateName.value, convertedTemplate.value)
    await configStore.saveConfig()
    ElMessage.success('模板已保存')
    convertDialogVisible.value = false
  } catch (err) {
    ElMessage.error(`保存失败: ${err instanceof Error ? err.message : String(err)}`)
  }
}

// 监听配置变化（只创建一次监听器）
watch(
  () => configStore.config.default_mode,
  (newMode: 'lite' | 'full' | undefined) => {
    if (newMode && defaultMode.value !== newMode) {
      defaultMode.value = newMode
    }
  }
)

watch(
  () => configStore.config.debug,
  (newDebug: boolean | undefined) => {
    const newValue = newDebug || false
    if (debugMode.value !== newValue) {
      debugMode.value = newValue
    }
  }
)

// KeepAlive 激活时同步最新配置
onActivated(() => {
  currentTheme.value = settingsStore.theme
  currentLanguage.value = settingsStore.language
  defaultMode.value = configStore.config.default_mode || 'lite'
  debugMode.value = configStore.config.debug || false
})
</script>

<style scoped>
.settings-page {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.settings-section {
  padding: 1.5rem;
  border-radius: 1rem;
  box-shadow: 0 2px 8px var(--shadow);
}

.section-title {
  font-size: 1.125rem;
  font-weight: 600;
  color: var(--text);
  margin-bottom: 1rem;
}

.setting-item {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  padding: 1rem 0;
  border-bottom: 1px solid var(--border);
}

.setting-item:last-child {
  border-bottom: none;
}

.setting-info {
  display: flex;
  align-items: center;
  gap: 1rem;
  flex: 1;
}

.setting-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  background: var(--background);
  border-radius: 0.5rem;
  color: var(--primary);
  flex-shrink: 0;
}

.setting-text {
  flex: 1;
  overflow: hidden;
}

.setting-name {
  font-size: 1rem;
  font-weight: 500;
  color: var(--text);
  margin: 0 0 0.25rem 0;
  line-height: 1.5;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.setting-desc {
  font-size: 0.875rem;
  color: var(--text-secondary);
  margin: 0;
  line-height: 1.5;
}

.setting-control {
  width: 100%;
}

.setting-item-horizontal {
  flex-direction: row;
  align-items: center;
  justify-content: space-between;
}

.setting-control-switch {
  flex-shrink: 0;
  margin-left: 1rem;
}

/* Code font for preview */
.code-font {
  font-family: monospace;
}

/* Dialog styles (copied from TemplatePage for consistency) */
.template-dialog :deep(.el-dialog) {
  margin-top: 5vh !important;
  margin-bottom: 80px !important;
  max-height: calc(100vh - 80px - 10vh) !important;
  display: flex;
  flex-direction: column;
  background: rgba(255, 255, 255, 0.15) !important;
  backdrop-filter: blur(40px) saturate(150%) brightness(1.1);
  -webkit-backdrop-filter: blur(40px) saturate(150%) brightness(1.1);
  border: 1px solid rgba(255, 255, 255, 0.4);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.15);
}

@media (prefers-color-scheme: dark) {
  .template-dialog :deep(.el-dialog) {
    background: rgba(20, 20, 20, 0.6) !important;
    backdrop-filter: blur(40px) saturate(150%) brightness(0.9);
    -webkit-backdrop-filter: blur(40px) saturate(150%) brightness(0.9);
    border: 1px solid rgba(255, 255, 255, 0.15);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }
}

.template-dialog :deep(.el-dialog__body) {
  flex: 1;
  overflow-y: auto;
  padding: 1.5rem;
  background: transparent;
}

.template-dialog :deep(.el-dialog__header) {
  background: rgba(255, 255, 255, 0.1);
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
  border-bottom: 1px solid rgba(255, 255, 255, 0.2);
}

@media (prefers-color-scheme: dark) {
  .template-dialog :deep(.el-dialog__header) {
    background: rgba(0, 0, 0, 0.2);
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  }
}

.template-dialog :deep(.el-dialog__footer) {
  padding: 1rem 1.5rem;
  border-top: 1px solid rgba(255, 255, 255, 0.2);
  background: rgba(255, 255, 255, 0.15);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  flex-shrink: 0;
}

@media (prefers-color-scheme: dark) {
  .template-dialog :deep(.el-dialog__footer) {
    border-top: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(0, 0, 0, 0.3);
  }
}

.template-dialog :deep(.el-overlay) {
  z-index: 2000 !important;
  backdrop-filter: blur(12px) saturate(120%) !important;
  -webkit-backdrop-filter: blur(12px) saturate(120%) !important;
  background-color: rgba(0, 0, 0, 0.25) !important;
}

@media (prefers-color-scheme: dark) {
  .template-dialog :deep(.el-overlay) {
    backdrop-filter: blur(12px) saturate(120%) !important;
    -webkit-backdrop-filter: blur(12px) saturate(120%) !important;
    background-color: rgba(0, 0, 0, 0.4) !important;
  }
}

.form-tip {
  font-size: 0.75rem;
  color: var(--text-secondary);
  margin-top: 0.25rem;
}
</style>
