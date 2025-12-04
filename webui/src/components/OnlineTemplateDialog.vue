<template>
  <el-dialog
    v-model="visible"
    title="在线模板库"
    width="90%"
    :close-on-click-modal="false"
    :append-to-body="true"
    :destroy-on-close="true"
    :z-index="2001"
    class="online-template-dialog"
    modal-class="online-template-modal"
  >
    <div class="online-template-content">
      <!-- 分类标签 -->
      <div class="category-tabs">
        <button
          v-for="(label, key) in TEMPLATE_CATEGORIES"
          :key="key"
          :class="['category-tab', { active: selectedCategory === key }]"
          @click="selectedCategory = key as TemplateCategory"
        >
          {{ label }}
        </button>
      </div>

      <!-- 加载状态 -->
      <div v-if="loading" class="loading-state">
        <el-icon class="is-loading">
          <Loading />
        </el-icon>
        <p>正在加载在线模板...</p>
      </div>

      <!-- 错误状态 -->
      <div v-else-if="error" class="error-state">
        <el-icon>
          <CircleClose />
        </el-icon>
        <p>{{ error }}</p>
        <el-button @click="loadTemplates">重试</el-button>
      </div>

      <!-- 模板列表 -->
      <div v-else class="template-grid">
        <div
          v-for="template in filteredTemplates"
          :key="template.path"
          class="online-template-card"
        >
          <div class="template-header">
            <h4 class="template-title">{{ template.displayName }}</h4>
            <span class="template-category">{{ TEMPLATE_CATEGORIES[template.category] }}</span>
          </div>

          <div v-if="template.template" class="template-info">
            <p class="info-line">
              <span class="label">品牌:</span>
              <span class="value">{{ template.template.brand }}</span>
            </p>
            <p class="info-line">
              <span class="label">型号:</span>
              <span class="value">{{ template.template.model }}</span>
            </p>
            <p v-if="template.template.marketname" class="info-line">
              <span class="label">市场名:</span>
              <span class="value">{{ template.template.marketname }}</span>
            </p>
          </div>

          <div class="template-actions">
            <el-button
              type="primary"
              size="small"
              :loading="importingTemplates.has(template.path)"
              @click="importTemplate(template)"
            >
              {{ importingTemplates.has(template.path) ? '导入中...' : '导入' }}
            </el-button>
          </div>
        </div>

        <div v-if="filteredTemplates.length === 0" class="empty-state">
          <p>该分类暂无模板</p>
        </div>
      </div>
    </div>

    <template #footer>
      <el-button @click="visible = false">关闭</el-button>
      <el-button type="primary" @click="loadTemplates">刷新列表</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Loading, CircleClose } from '@element-plus/icons-vue'
import { toast } from 'kernelsu-alt'
import {
  fetchOnlineTemplates,
  downloadTemplate,
  TEMPLATE_CATEGORIES,
  type OnlineTemplate,
  type TemplateCategory,
} from '../utils/onlineTemplates'
import { useConfigStore } from '../stores/config'

const props = defineProps<{
  modelValue: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
}>()

const configStore = useConfigStore()

const visible = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value),
})

const loading = ref(false)
const error = ref<string | null>(null)
const templates = ref<OnlineTemplate[]>([])
const selectedCategory = ref<TemplateCategory>('common')
const importingTemplates = ref(new Set<string>())

// 过滤当前分类的模板
const filteredTemplates = computed(() => {
  return templates.value.filter((t) => t.category === selectedCategory.value)
})

// 加载在线模板列表
async function loadTemplates() {
  loading.value = true
  error.value = null

  toast('开始加载在线模板...')

  try {
    toast('正在获取模板列表...')
    const onlineTemplates = await fetchOnlineTemplates()

    toast(`获取到 ${onlineTemplates.length} 个模板`)

    if (onlineTemplates.length === 0) {
      error.value = '未找到任何模板，请检查网络连接或稍后重试'
      toast('未找到任何模板，可能是网络问题或 API 限流')
      return
    }

    // 先显示模板列表（不等待下载）
    templates.value = onlineTemplates
    toast('模板列表已加载，正在后台下载详细内容...')

    // 后台异步下载模板内容
    let successCount = 0
    let failCount = 0

    Promise.all(
      onlineTemplates.map(async (t, index) => {
        try {
          const template = await downloadTemplate(t)
          if (template) {
            templates.value[index] = { ...t, template }
            successCount++
          } else {
            failCount++
          }
        } catch {
          failCount++
        }
      })
    ).then(() => {
      if (successCount > 0) {
        toast(`成功加载 ${successCount} 个模板内容`)
      }
      if (failCount > 0) {
        toast(`${failCount} 个模板内容加载失败`)
      }
    })
  } catch (e) {
    error.value = e instanceof Error ? e.message : '加载失败'
    toast(`加载失败: ${error.value}`)
  } finally {
    loading.value = false
  }
}

// 导入模板
async function importTemplate(onlineTemplate: OnlineTemplate) {
  if (!onlineTemplate.template) {
    ElMessage.error('模板内容为空')
    return
  }

  importingTemplates.value.add(onlineTemplate.path)

  try {
    // 使用文件名作为模板名称
    const templateName = onlineTemplate.name

    // 检查是否已存在
    const existingTemplates = configStore.getTemplates()
    if (existingTemplates[templateName]) {
      await ElMessageBox.confirm(`模板 "${templateName}" 已存在，是否覆盖？`, '确认导入', {
        confirmButtonText: '覆盖',
        cancelButtonText: '取消',
        type: 'warning',
      })
    }

    // 保存模板
    configStore.setTemplate(templateName, onlineTemplate.template)
    await configStore.saveConfig()

    ElMessage.success(`模板 "${templateName}" 导入成功`)
  } catch (e) {
    if (e !== 'cancel') {
      ElMessage.error('导入失败')
    }
  } finally {
    importingTemplates.value.delete(onlineTemplate.path)
  }
}

// 监听对话框打开，自动加载模板
watch(
  visible,
  (newValue) => {
    if (newValue && templates.value.length === 0) {
      loadTemplates().catch((err) => {
        console.error('loadTemplates error:', err)
        toast(`加载模板时发生错误: ${err.message || err}`)
      })
    }
  },
  { immediate: true }
)
</script>

<style scoped>
.online-template-content {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  min-height: 400px;
}

.category-tabs {
  display: flex;
  gap: 0.5rem;
  padding-bottom: 1rem;
  border-bottom: 1px solid var(--border);
  flex-wrap: nowrap;
}

.category-tab {
  padding: 0.5rem 1rem;
  background: var(--background);
  border: 1px solid var(--border);
  border-radius: 0.5rem;
  color: var(--text);
  font-size: 0.875rem;
  transition: all 0.2s ease;
  cursor: pointer;
  white-space: nowrap;
}

.category-tab:hover {
  background: var(--hover);
}

.category-tab.active {
  background: var(--primary);
  color: white;
  border-color: var(--primary);
}

.loading-state,
.error-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 1rem;
  padding: 3rem 1rem;
  color: var(--text-secondary);
}

.loading-state .el-icon {
  font-size: 2rem;
}

.error-state .el-icon {
  font-size: 3rem;
  color: #ef4444;
}

.template-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 1rem;
  max-height: 500px;
  overflow-y: auto;
  padding: 0.5rem;
}

.online-template-card {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  padding: 1rem;
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: 0.75rem;
  transition: all 0.2s ease;
}

.online-template-card:hover {
  border-color: var(--primary);
  box-shadow: 0 4px 12px var(--shadow);
}

.template-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.5rem;
}

.template-title {
  font-size: 1rem;
  font-weight: 600;
  color: var(--text);
  margin: 0;
  flex: 1;
}

.template-category {
  padding: 0.25rem 0.5rem;
  background: var(--primary);
  color: white;
  font-size: 0.75rem;
  border-radius: 0.25rem;
  white-space: nowrap;
}

.template-info {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.info-line {
  display: flex;
  gap: 0.5rem;
  font-size: 0.875rem;
  margin: 0;
}

.info-line .label {
  color: var(--text-secondary);
  min-width: 60px;
}

.info-line .value {
  color: var(--text);
  flex: 1;
  word-break: break-all;
}

.template-actions {
  display: flex;
  justify-content: flex-end;
  padding-top: 0.5rem;
  border-top: 1px solid var(--border);
}

.empty-state {
  grid-column: 1 / -1;
  text-align: center;
  padding: 3rem 1rem;
  color: var(--text-secondary);
}

/* Dialog 样式 */
.online-template-dialog :deep(.el-dialog) {
  background: rgba(255, 255, 255, 0.15) !important;
  backdrop-filter: blur(40px) saturate(150%) brightness(1.1);
  -webkit-backdrop-filter: blur(40px) saturate(150%) brightness(1.1);
  border: 1px solid rgba(255, 255, 255, 0.4);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.15);
}

@media (prefers-color-scheme: dark) {
  .online-template-dialog :deep(.el-dialog) {
    background: rgba(20, 20, 20, 0.6) !important;
    backdrop-filter: blur(40px) saturate(150%) brightness(0.9);
    -webkit-backdrop-filter: blur(40px) saturate(150%) brightness(0.9);
    border: 1px solid rgba(255, 255, 255, 0.15);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }
}
</style>

<style>
.online-template-modal {
  backdrop-filter: blur(12px) saturate(120%) !important;
  -webkit-backdrop-filter: blur(12px) saturate(120%) !important;
  background-color: rgba(0, 0, 0, 0.25) !important;
}

@media (prefers-color-scheme: dark) {
  .online-template-modal {
    backdrop-filter: blur(12px) saturate(120%) !important;
    -webkit-backdrop-filter: blur(12px) saturate(120%) !important;
    background-color: rgba(0, 0, 0, 0.4) !important;
  }
}
</style>
