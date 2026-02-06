<template>
  <el-dialog
    v-model="visible"
    :title="dialogTitle"
    width="90%"
    :close-on-click-modal="false"
    :append-to-body="true"
    :destroy-on-close="true"
    :z-index="2001"
    class="app-config-dialog"
    modal-class="app-config-modal"
  >
    <!-- 配置类型标签页 -->
    <div class="config-tabs">
      <div
        class="config-tab"
        :class="{ active: activeTab === 'custom', configured: hasCustomConfig }"
        @click="activeTab = 'custom'"
      >
        <el-checkbox
          :model-value="hasCustomConfig"
          @click.stop
          @change="(val: boolean) => handleConfigToggle('custom', val)"
        />
        <span class="tab-label">{{ t('apps.dialog.mode_custom') }}</span>
      </div>
      <div
        class="config-tab"
        :class="{
          active: activeTab === 'template',
          configured: hasTemplateConfig && !hasCustomConfig,
        }"
        @click="activeTab = 'template'"
      >
        <el-checkbox
          :model-value="hasTemplateConfig"
          @click.stop
          @change="(val: boolean) => handleConfigToggle('template', val)"
        />
        <span class="tab-label">{{ t('apps.dialog.mode_template') }}</span>
      </div>
    </div>

    <!-- 模板配置界面 -->
    <div v-if="activeTab === 'template'" class="template-selector">
      <el-input
        v-model="templateSearch"
        :placeholder="t('apps.dialog.search_template_placeholder')"
        prefix-icon="Search"
        clearable
        class="template-search"
      />
      <el-select
        v-model="selectedTemplate"
        :placeholder="t('apps.dialog.select_template_placeholder')"
        :no-data-text="templateNoDataText"
        :no-match-text="templateNoMatchText"
        filterable
        style="width: 100%"
      >
        <el-option
          v-for="option in filteredTemplateOptions"
          :key="option.name"
          :label="option.label"
          :value="option.name"
        />
      </el-select>

      <!-- 模板配置操作按钮 -->
      <div class="config-actions">
        <el-button v-if="hasTemplateConfig" type="danger" @click="removeTemplateConfig">
          {{ t('apps.dialog.remove_template_config') }}
        </el-button>
      </div>
    </div>

    <!-- 自定义配置界面 -->
    <div v-if="activeTab === 'custom'" class="custom-config">
      <el-form :model="customFormData" label-width="120px" label-position="top">
        <el-form-item :label="t('templates.fields.manufacturer')">
          <el-input
            v-model="customFormData.manufacturer"
            :placeholder="t('templates.placeholders.manufacturer')"
          />
        </el-form-item>
        <el-form-item :label="t('templates.fields.brand')">
          <el-input
            v-model="customFormData.brand"
            :placeholder="t('templates.placeholders.brand')"
          />
        </el-form-item>
        <el-form-item :label="t('templates.fields.model')">
          <el-input
            v-model="customFormData.model"
            :placeholder="t('templates.placeholders.model')"
          />
        </el-form-item>
        <el-form-item :label="t('templates.fields.device')">
          <el-input
            v-model="customFormData.device"
            :placeholder="t('templates.placeholders.device')"
          />
        </el-form-item>
        <el-form-item :label="t('templates.fields.product')">
          <el-input
            v-model="customFormData.product"
            :placeholder="t('templates.placeholders.product')"
          />
        </el-form-item>
        <el-form-item :label="t('templates.fields.name_field')">
          <el-input
            v-model="customFormData.name"
            :placeholder="t('templates.placeholders.name_field')"
          />
        </el-form-item>
        <el-form-item :label="t('templates.fields.market_name')">
          <el-input
            v-model="customFormData.marketname"
            :placeholder="t('templates.placeholders.market_name')"
          />
        </el-form-item>
        <el-form-item :label="t('templates.fields.fingerprint')">
          <el-input
            v-model="customFormData.fingerprint"
            type="textarea"
            :rows="3"
            :placeholder="t('templates.placeholders.fingerprint')"
          />
        </el-form-item>
        <el-form-item :label="t('templates.fields.android_version')">
          <el-input
            v-model="customFormData.android_version"
            :placeholder="t('templates.placeholders.android_version')"
          />
        </el-form-item>
        <el-form-item :label="t('templates.fields.sdk_int')">
          <el-input
            v-model="customFormData.sdk_int"
            type="number"
            :placeholder="t('templates.placeholders.sdk_int')"
          />
        </el-form-item>
        <el-form-item :label="t('templates.fields.mode')">
          <el-select
            v-model="customFormData.mode"
            :placeholder="t('templates.placeholders.mode')"
            clearable
            popper-class="mode-select-popper"
            style="width: 100%"
          >
            <el-option :label="t('templates.options.mode_lite')" value="lite" />
            <el-option :label="t('templates.options.mode_full')" value="full" />
            <el-option :label="t('templates.options.mode_resetprop')" value="resetprop" />
          </el-select>
        </el-form-item>

        <el-form-item
          v-if="
            customFormData.mode === 'full' ||
            (!customFormData.mode && configStore.config.default_mode === 'full')
          "
          :label="t('templates.fields.characteristics')"
        >
          <el-input
            v-model="customFormData.characteristics"
            :placeholder="t('templates.placeholders.characteristics')"
          />
        </el-form-item>

        <el-form-item :label="t('templates.fields.force_denylist_unmount')">
          <el-select
            v-model="customFormData.force_denylist_unmount"
            :placeholder="t('common.default')"
            style="width: 100%"
          >
            <el-option :label="t('common.default')" :value="undefined" />
            <el-option :label="t('common.enabled')" :value="true" />
            <el-option :label="t('common.disabled')" :value="false" />
          </el-select>
        </el-form-item>
      </el-form>

      <!-- 元数据只读显示区域 -->
      <div v-if="hasMetaData" class="config-meta">
        <div v-if="customMetaData.version || customMetaData.version_code" class="meta-item">
          <span class="meta-label">{{ t('templates.labels.version') }}:</span>
          <span class="meta-value">
            {{ customMetaData.version || '' }}
            <span v-if="customMetaData.version_code" class="version-code"
              >({{ customMetaData.version_code }})</span
            >
          </span>
        </div>
        <div v-if="customMetaData.author" class="meta-item">
          <span class="meta-label">{{ t('templates.labels.author') }}:</span>
          <span class="meta-value">{{ customMetaData.author }}</span>
        </div>
        <div v-if="customMetaData.description" class="meta-item meta-description">
          <span class="meta-label">{{ t('templates.labels.description') }}:</span>
          <span class="meta-value">{{ customMetaData.description }}</span>
        </div>
      </div>

      <!-- 自定义配置操作按钮 -->
      <div class="config-actions">
        <el-button v-if="hasCustomConfig" type="danger" @click="removeCustomConfig">
          {{ t('apps.dialog.remove_custom_config') }}
        </el-button>
      </div>
    </div>

    <template #footer>
      <el-button @click="visible = false">{{ t('common.cancel') }}</el-button>
      <el-button type="primary" @click="saveAppConfig">{{ t('common.confirm') }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useConfigStore } from '../../stores/config'
import { useI18n } from '../../utils/i18n'
import { toast } from 'kernelsu-alt'
import type { InstalledApp, AppConfig } from '../../types'

interface TemplateOption {
  name: string
  label: string
  searchable: string
}

const props = defineProps<{
  modelValue: boolean
  app: InstalledApp | null
}>()

const emit = defineEmits<{ 'update:modelValue': [boolean]; saved: [] }>()

const configStore = useConfigStore()
const { t } = useI18n()

const templates = computed(() => configStore.getTemplates())
const visible = computed({
  get: () => props.modelValue,
  set: (val: boolean) => emit('update:modelValue', val),
})

// 当前激活的标签页
const activeTab = ref<'template' | 'custom'>('custom')

// 配置状态追踪
const hasCustomConfig = ref(false)
const hasTemplateConfig = ref(false)

// 原始配置状态（用于检测变更）
const originalCustomConfig = ref<AppConfig | null>(null)
const originalTemplateName = ref<string | null>(null)

// 模板配置数据
const selectedTemplate = ref('')
const templateSearch = ref('')

// 自定义配置数据
const customFormData = ref({
  manufacturer: '',
  brand: '',
  model: '',
  device: '',
  product: '',
  name: '',
  marketname: '',
  fingerprint: '',
  android_version: '',
  sdk_int: '',
  characteristics: '',
  force_denylist_unmount: undefined as boolean | undefined,
  mode: undefined as 'lite' | 'full' | 'resetprop' | undefined,
})

// 元数据字段（只读显示）
const customMetaData = ref({
  version: '' as string | undefined,
  version_code: undefined as number | undefined,
  author: '' as string | undefined,
  description: '' as string | undefined,
})

// 判断是否有元数据需要显示
const hasMetaData = computed(() => {
  return (
    customMetaData.value.version ||
    customMetaData.value.version_code ||
    customMetaData.value.author ||
    customMetaData.value.description
  )
})

const dialogTitle = computed(() =>
  t('apps.dialog.config_title', { name: props.app?.appName || '' })
)

const templateOptions = computed<TemplateOption[]>(() => {
  const allTemplates = templates.value || {}

  return Object.entries(allTemplates).map(([name, template]) => {
    const label = `${name} - ${template.brand || ''} ${template.model || ''}`
      .replace(/\s+/g, ' ')
      .trim()

    const searchable = [
      name,
      template.brand,
      template.model,
      template.marketname,
      template.device,
      template.product,
    ]
      .filter(Boolean)
      .map((part) => String(part).toLowerCase())
      .join(' ')

    return {
      name,
      label: label || name,
      searchable,
    }
  })
})

const filteredTemplateOptions = computed(() => {
  const keyword = templateSearch.value.trim().toLowerCase()
  if (!keyword) return templateOptions.value

  return templateOptions.value.filter((option) => option.searchable.includes(keyword))
})

const templateNoDataText = computed(() =>
  templateSearch.value.trim() ? t('apps.dialog.search_no_result') : t('apps.dialog.no_templates')
)
const templateNoMatchText = computed(() => t('apps.dialog.search_no_result'))

/**
 * 处理配置开关切换
 * @param type - 配置类型
 * @param enabled - 是否启用
 */
function handleConfigToggle(type: 'custom' | 'template', enabled: boolean) {
  if (type === 'custom') {
    hasCustomConfig.value = enabled
    if (enabled) {
      activeTab.value = 'custom'
    }
  } else {
    hasTemplateConfig.value = enabled
    if (enabled) {
      activeTab.value = 'template'
      // 如果没有选择模板，默认选择第一个
      if (!selectedTemplate.value && templateOptions.value.length > 0) {
        selectedTemplate.value = templateOptions.value[0].name
      }
    } else {
      selectedTemplate.value = ''
    }
  }
}

/**
 * 从现有配置同步数据到表单
 */
function syncFromExistingConfig() {
  if (!props.app) return

  templateSearch.value = ''

  // 检查自定义配置
  const appConfig = configStore.getApps().find((a) => a.package === props.app!.packageName)
  if (appConfig) {
    hasCustomConfig.value = true
    originalCustomConfig.value = { ...appConfig }
    customFormData.value = {
      manufacturer: appConfig.manufacturer || '',
      brand: appConfig.brand || '',
      model: appConfig.model || '',
      device: appConfig.device || '',
      product: appConfig.product || '',
      name: appConfig.name || '',
      marketname: appConfig.marketname || '',
      fingerprint: appConfig.fingerprint || '',
      android_version: appConfig.android_version || '',
      sdk_int: appConfig.sdk_int ? String(appConfig.sdk_int) : '',
      characteristics: appConfig.characteristics || '',
      force_denylist_unmount: appConfig.force_denylist_unmount,
      mode: appConfig.mode as 'lite' | 'full' | 'resetprop' | undefined,
    }
    // 同步元数据字段（只读显示）
    customMetaData.value = {
      version: appConfig.version,
      version_code: appConfig.version_code,
      author: appConfig.author,
      description: appConfig.description,
    }
  } else {
    hasCustomConfig.value = false
    originalCustomConfig.value = null
    resetCustomFormData()
  }

  // 检查模板配置
  let foundTemplateName: string | null = null
  for (const [name, template] of Object.entries(templates.value)) {
    if (template.packages?.includes(props.app.packageName)) {
      foundTemplateName = name
      break
    }
  }

  if (foundTemplateName) {
    hasTemplateConfig.value = true
    originalTemplateName.value = foundTemplateName
    selectedTemplate.value = foundTemplateName
  } else {
    hasTemplateConfig.value = false
    originalTemplateName.value = null
    selectedTemplate.value = ''
  }

  // 决定默认激活的标签页：优先显示自定义配置（因为优先级更高）
  if (hasCustomConfig.value) {
    activeTab.value = 'custom'
  } else if (hasTemplateConfig.value) {
    activeTab.value = 'template'
  } else {
    // 都没有配置时，默认显示自定义配置界面
    activeTab.value = 'custom'
  }
}

/**
 * 重置自定义配置表单数据
 */
function resetCustomFormData() {
  customFormData.value = {
    manufacturer: '',
    brand: '',
    model: '',
    device: '',
    product: '',
    name: '',
    marketname: '',
    fingerprint: '',
    android_version: '',
    sdk_int: '',
    characteristics: '',
    force_denylist_unmount: undefined,
    mode: undefined,
  }
  customMetaData.value = {
    version: undefined,
    version_code: undefined,
    author: undefined,
    description: undefined,
  }
}

/**
 * 移除自定义配置
 */
async function removeCustomConfig() {
  if (!props.app) return

  configStore.deleteApp(props.app.packageName)
  hasCustomConfig.value = false
  originalCustomConfig.value = null
  resetCustomFormData()

  try {
    await configStore.saveConfig()
    toast(t('apps.messages.custom_config_removed'))
  } catch {
    toast(t('common.failed'))
  }
}

/**
 * 移除模板配置
 */
async function removeTemplateConfig() {
  if (!props.app) return

  // 从所有模板中移除该应用
  const allTemplates = configStore.getTemplates()
  for (const [name, template] of Object.entries(allTemplates)) {
    if (template.packages?.includes(props.app.packageName)) {
      template.packages = template.packages.filter((p: string) => p !== props.app!.packageName)
      configStore.setTemplate(name, template)
    }
  }

  hasTemplateConfig.value = false
  originalTemplateName.value = null
  selectedTemplate.value = ''

  try {
    await configStore.saveConfig()
    toast(t('apps.messages.template_config_removed'))
  } catch {
    toast(t('common.failed'))
  }
}

/**
 * 保存应用配置
 */
async function saveAppConfig() {
  if (!props.app) return

  // 保存模板配置
  if (hasTemplateConfig.value) {
    if (!selectedTemplate.value) {
      toast(t('apps.messages.select_template'))
      return
    }

    // 如果模板选择发生变化，先从原模板中移除
    if (originalTemplateName.value && originalTemplateName.value !== selectedTemplate.value) {
      const oldTemplate = templates.value[originalTemplateName.value]
      if (oldTemplate && oldTemplate.packages) {
        oldTemplate.packages = oldTemplate.packages.filter(
          (p: string) => p !== props.app!.packageName
        )
        configStore.setTemplate(originalTemplateName.value, oldTemplate)
      }
    }

    // 添加到新模板
    const template = templates.value[selectedTemplate.value]
    if (template) {
      if (!template.packages) {
        template.packages = []
      }
      if (!template.packages.includes(props.app.packageName)) {
        template.packages.push(props.app.packageName)
        configStore.setTemplate(selectedTemplate.value, template)
      }
    }
  } else {
    // 如果取消了模板配置，从所有模板中移除
    const allTemplates = configStore.getTemplates()
    for (const [name, template] of Object.entries(allTemplates)) {
      if (template.packages?.includes(props.app.packageName)) {
        template.packages = template.packages.filter((p: string) => p !== props.app!.packageName)
        configStore.setTemplate(name, template)
      }
    }
  }

  // 保存自定义配置
  if (hasCustomConfig.value) {
    const appConfig: AppConfig = {
      package: props.app.packageName,
      manufacturer: customFormData.value.manufacturer,
      brand: customFormData.value.brand,
      model: customFormData.value.model,
      device: customFormData.value.device,
      product: customFormData.value.product,
      name: customFormData.value.name,
      marketname: customFormData.value.marketname,
      fingerprint: customFormData.value.fingerprint,
      android_version: customFormData.value.android_version,
      sdk_int: customFormData.value.sdk_int ? Number(customFormData.value.sdk_int) : undefined,
      characteristics: customFormData.value.characteristics,
      force_denylist_unmount: customFormData.value.force_denylist_unmount,
      mode: customFormData.value.mode,
    }
    configStore.setApp(appConfig)
  } else {
    // 如果取消了自定义配置，删除它
    configStore.deleteApp(props.app.packageName)
  }

  try {
    await configStore.saveConfig()
    toast(t('apps.messages.saved'))
    visible.value = false
    emit('saved')
  } catch {
    toast(t('common.failed'))
  }
}

watch(
  () => [props.app, visible.value],
  ([, dialogVisible]) => {
    if (dialogVisible) {
      syncFromExistingConfig()
    }
  },
  { immediate: true }
)
</script>

<style scoped>
.config-tabs {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 1.5rem;
  border-bottom: 1px solid var(--border);
  padding-bottom: 0.5rem;
}

.config-tab {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 1rem;
  border-radius: 0.375rem;
  cursor: pointer;
  transition: all 0.2s;
  border: 1px solid transparent;
}

.config-tab:hover {
  background: var(--bg-secondary);
}

.config-tab.active {
  background: var(--primary-color-light, rgba(64, 158, 255, 0.1));
  border-color: var(--primary-color, #409eff);
}

.config-tab.configured {
  position: relative;
}

.config-tab.configured::after {
  content: '';
  position: absolute;
  top: 0.25rem;
  right: 0.25rem;
  width: 6px;
  height: 6px;
  background: var(--success-color, #67c23a);
  border-radius: 50%;
}

.tab-label {
  font-size: 0.875rem;
  color: var(--text);
}

.config-tab.active .tab-label {
  color: var(--primary-color, #409eff);
  font-weight: 500;
}

.template-selector,
.custom-config {
  margin-top: 1rem;
}

.template-search {
  margin-bottom: 0.75rem;
}

.config-actions {
  margin-top: 1.5rem;
  padding-top: 1rem;
  border-top: 1px solid var(--border);
  display: flex;
  justify-content: flex-start;
}

.config-meta {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  margin-top: 1rem;
  padding-top: 0.75rem;
  border-top: 1px solid var(--border);
}

.meta-item {
  display: flex;
  gap: 0.5rem;
  font-size: 0.8125rem;
}

.meta-label {
  color: var(--text-secondary);
  min-width: 50px;
  flex-shrink: 0;
}

.meta-value {
  color: var(--text);
  flex: 1;
  word-break: break-all;
}

.version-code {
  color: var(--text-secondary);
  font-size: 0.75rem;
}

.meta-description .meta-value {
  font-size: 0.75rem;
  color: var(--text-secondary);
  line-height: 1.4;
}

.meta-description .meta-label {
  font-size: 0.75rem;
  line-height: 1.4;
}
</style>

<style scoped>
.app-config-dialog :deep(.el-dialog) {
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
  .app-config-dialog :deep(.el-dialog) {
    background: rgba(20, 20, 20, 0.6) !important;
    backdrop-filter: blur(40px) saturate(150%) brightness(0.9);
    -webkit-backdrop-filter: blur(40px) saturate(150%) brightness(0.9);
    border: 1px solid rgba(255, 255, 255, 0.15);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }
}

.app-config-dialog :deep(.el-dialog__body) {
  flex: 1;
  overflow-y: auto;
  padding: 1.5rem;
  padding-bottom: 2rem;
  max-height: calc(100vh - 200px);
  background: transparent;
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.app-config-dialog :deep(.el-dialog__body::-webkit-scrollbar) {
  display: none;
}

.app-config-dialog :deep(.el-dialog__header) {
  background: rgba(255, 255, 255, 0.1);
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
  border-bottom: 1px solid rgba(255, 255, 255, 0.2);
}

@media (prefers-color-scheme: dark) {
  .app-config-dialog :deep(.el-dialog__header) {
    background: rgba(0, 0, 0, 0.2);
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  }
}

.app-config-dialog :deep(.el-dialog__footer) {
  padding: 1rem 1.5rem;
  border-top: 1px solid rgba(255, 255, 255, 0.2);
  background: rgba(255, 255, 255, 0.15);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  flex-shrink: 0;
}

@media (prefers-color-scheme: dark) {
  .app-config-dialog :deep(.el-dialog__footer) {
    border-top: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(0, 0, 0, 0.3);
  }
}

.app-config-dialog :deep(.el-overlay) {
  z-index: 2000 !important;
  backdrop-filter: blur(8px) brightness(0.7) !important;
  -webkit-backdrop-filter: blur(8px) brightness(0.7) !important;
  background-color: rgba(0, 0, 0, 0.6) !important;
}

@media (prefers-color-scheme: dark) {
  .app-config-dialog :deep(.el-overlay) {
    backdrop-filter: blur(8px) brightness(0.5) !important;
    -webkit-backdrop-filter: blur(8px) brightness(0.5) !important;
    background-color: rgba(0, 0, 0, 0.7) !important;
  }
}
</style>

<style>
.app-config-modal {
  backdrop-filter: blur(12px) saturate(120%) !important;
  -webkit-backdrop-filter: blur(12px) saturate(120%) !important;
  background-color: rgba(0, 0, 0, 0.25) !important;
}

@media (prefers-color-scheme: dark) {
  .app-config-modal {
    backdrop-filter: blur(12px) saturate(120%) !important;
    -webkit-backdrop-filter: blur(12px) saturate(120%) !important;
    background-color: rgba(0, 0, 0, 0.4) !important;
  }
}
</style>
