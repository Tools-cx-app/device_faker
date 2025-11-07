<template>
  <div class="template-page">
    <div class="page-header">
      <h2 class="page-title">机型模板</h2>
      <button class="add-btn" @click="showAddDialog">
        <Plus :size="20" />
        新建模板
      </button>
    </div>

    <div class="template-list">
      <div v-for="(template, name) in templates" :key="name" class="template-card glass-effect">
        <div class="template-header">
          <div class="template-info">
            <h3 class="template-name">{{ name }}</h3>
            <p class="template-device">{{ template.brand }} {{ template.model }}</p>
          </div>
          <div class="template-actions">
            <button class="icon-btn" @click="editTemplate(name, template)">
              <Edit2 :size="18" />
            </button>
            <button class="icon-btn danger" @click="deleteTemplateConfirm(name)">
              <Trash2 :size="18" />
            </button>
          </div>
        </div>

        <div class="template-details">
          <div class="detail-item">
            <span class="detail-label">Manufacturer:</span>
            <span class="detail-value">{{ template.manufacturer }}</span>
          </div>
          <div class="detail-item">
            <span class="detail-label">Device:</span>
            <span class="detail-value">{{ template.device }}</span>
          </div>
          <div class="detail-item">
            <span class="detail-label">Fingerprint:</span>
            <span class="detail-value fingerprint">{{ template.fingerprint }}</span>
          </div>
          <div v-if="template.packages && template.packages.length > 0" class="detail-item">
            <span class="detail-label">应用包名:</span>
            <span class="detail-value">{{ template.packages.length }} 个</span>
          </div>
        </div>
      </div>

      <div v-if="Object.keys(templates).length === 0" class="empty-state">
        <FileText :size="64" class="empty-icon" />
        <p class="empty-text">暂无机型模板</p>
        <p class="empty-hint">点击上方按钮创建新模板</p>
      </div>
    </div>

    <!-- 编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="isEditing ? '编辑模板' : '新建模板'"
      width="90%"
      :close-on-click-modal="false"
      :append-to-body="true"
      :destroy-on-close="true"
      :z-index="2001"
      class="template-dialog"
    >
      <el-form :model="formData" label-width="120px" label-position="top">
        <el-form-item label="模板名称">
          <el-input
            v-model="formData.name"
            :disabled="isEditing"
            placeholder="例如：redmagic_9_pro"
          />
        </el-form-item>

        <el-form-item label="Manufacturer">
          <el-input v-model="formData.manufacturer" placeholder="例如：ZTE" />
        </el-form-item>

        <el-form-item label="Brand">
          <el-input v-model="formData.brand" placeholder="例如：nubia" />
        </el-form-item>

        <el-form-item label="Model">
          <el-input v-model="formData.model" placeholder="例如：NX769J" />
        </el-form-item>

        <el-form-item label="Device">
          <el-input v-model="formData.device" placeholder="例如：REDMAGIC 9 Pro" />
        </el-form-item>

        <el-form-item label="Product">
          <el-input v-model="formData.product" placeholder="例如：NX769J" />
        </el-form-item>

        <el-form-item label="Name (可选，仅 full 模式)">
          <el-input v-model="formData.name_field" placeholder="例如：" />
        </el-form-item>

        <el-form-item label="Market Name (可选，仅 full 模式)">
          <el-input v-model="formData.marketname" placeholder="例如：REDMAGIC 9 Pro" />
        </el-form-item>

        <el-form-item label="Fingerprint">
          <el-input
            v-model="formData.fingerprint"
            type="textarea"
            :rows="3"
            placeholder="例如：nubia/NX769J/NX769J:14/UKQ1.230917.001/20240813.173312:user/release-keys"
          />
        </el-form-item>

        <el-form-item label="应用包名列表 (可选)">
          <div class="package-manager">
            <div class="package-input-wrapper">
              <el-autocomplete
                v-model="packageInput"
                :fetch-suggestions="searchPackages"
                placeholder="输入或搜索应用包名"
                style="width: 100%"
                clearable
                @keyup.enter="addPackage"
              >
                <template #default="{ item }">
                  <div class="package-suggestion">
                    <div class="app-info">
                      <div class="app-name">{{ item.appName }}</div>
                      <div class="package-name">{{ item.packageName }}</div>
                    </div>
                  </div>
                </template>
              </el-autocomplete>
              <el-button type="primary" :disabled="!packageInput" @click="addPackage"
                >添加</el-button
              >
              >
            </div>

            <div v-if="formData.packages.length > 0" class="package-list">
              <div v-for="(pkg, index) in formData.packages" :key="index" class="package-item">
                <div class="package-info">
                  <span class="package-name-text">{{ pkg }}</span>
                  <span v-if="getAppName(pkg)" class="app-name-text">{{ getAppName(pkg) }}</span>
                </div>
                <el-button type="danger" size="small" circle @click="removePackage(index)">
                  ×
                </el-button>
              </div>
            </div>
            <div v-else class="package-list-empty">暂无包名，可以输入包名或从已安装应用中选择</div>
          </div>
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="saveTemplate">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onActivated } from 'vue'
import { Plus, Edit2, Trash2, FileText } from 'lucide-vue-next'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useConfigStore } from '../stores/config'
import { useAppsStore } from '../stores/apps'
import type { Template } from '../types'

const configStore = useConfigStore()
const appsStore = useAppsStore()

const templates = computed(() => configStore.getTemplates())
const installedApps = computed(() => appsStore.installedApps)

const dialogVisible = ref(false)
const isEditing = ref(false)
const packageInput = ref('')
const formData = ref({
  name: '',
  manufacturer: '',
  brand: '',
  model: '',
  device: '',
  product: '',
  name_field: '',
  marketname: '',
  fingerprint: '',
  packages: [] as string[],
})

function showAddDialog() {
  isEditing.value = false
  packageInput.value = ''
  formData.value = {
    name: '',
    manufacturer: '',
    brand: '',
    model: '',
    device: '',
    product: '',
    name_field: '',
    marketname: '',
    fingerprint: '',
    packages: [],
  }
  dialogVisible.value = true
}

function editTemplate(name: string, template: Template) {
  isEditing.value = true
  packageInput.value = ''
  formData.value = {
    name,
    manufacturer: template.manufacturer || '',
    brand: template.brand || '',
    model: template.model || '',
    device: template.device || '',
    product: template.product || '',
    name_field: template.name || '',
    marketname: template.marketname || '',
    fingerprint: template.fingerprint || '',
    packages: template.packages || [],
  }
  dialogVisible.value = true
}

// 搜索包名建议
function searchPackages(
  queryString: string,
  cb: (suggestions: Array<{ value: string; appName: string; packageName: string }>) => void
) {
  const suggestions = installedApps.value
    .filter((app) => {
      const query = queryString.toLowerCase()
      return (
        app.packageName.toLowerCase().includes(query) || app.appName.toLowerCase().includes(query)
      )
    })
    .map((app) => ({
      value: app.packageName,
      appName: app.appName,
      packageName: app.packageName,
    }))
  cb(suggestions)
}

// 添加包名
function addPackage() {
  const pkgName = packageInput.value.trim()
  if (!pkgName) {
    return
  }

  if (formData.value.packages.includes(pkgName)) {
    ElMessage.warning('该包名已添加')
    return
  }

  formData.value.packages.push(pkgName)
  packageInput.value = ''
}

// 删除包名
function removePackage(index: number) {
  formData.value.packages.splice(index, 1)
}

// 获取应用名称
function getAppName(packageName: string): string {
  const app = installedApps.value.find((a) => a.packageName === packageName)
  return app ? app.appName : ''
}

async function saveTemplate() {
  if (!formData.value.name) {
    ElMessage.error('请输入模板名称')
    return
  }

  const template: Template = {
    manufacturer: formData.value.manufacturer,
    brand: formData.value.brand,
    model: formData.value.model,
    device: formData.value.device,
    product: formData.value.product,
    fingerprint: formData.value.fingerprint,
  }

  if (formData.value.name_field) {
    template.name = formData.value.name_field
  }

  if (formData.value.marketname) {
    template.marketname = formData.value.marketname
  }

  if (formData.value.packages.length > 0) {
    template.packages = formData.value.packages
  }

  configStore.setTemplate(formData.value.name, template)

  try {
    await configStore.saveConfig()
    ElMessage.success('模板已保存')
    dialogVisible.value = false
  } catch {
    ElMessage.error('保存失败')
  }
}

async function deleteTemplateConfirm(name: string) {
  try {
    await ElMessageBox.confirm(`确定要删除模板 "${name}" 吗？`, '确认删除', {
      confirmButtonText: '删除',
      cancelButtonText: '取消',
      type: 'warning',
    })

    configStore.deleteTemplate(name)
    await configStore.saveConfig()
    ElMessage.success('模板已删除')
  } catch {
    // 用户取消
  }
}

onMounted(() => {
  // 初始加载
  if (appsStore.installedApps.length === 0) {
    appsStore.loadInstalledApps()
  }
})

onActivated(() => {
  // KeepAlive 激活时，不重新加载应用列表，避免重复请求
})
</script>

<style scoped>
.template-page {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.5rem;
}

.page-title {
  font-size: 1.5rem;
  font-weight: 600;
  color: var(--text);
}

.add-btn {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  background: var(--primary);
  color: white;
  border: none;
  border-radius: 0.5rem;
  font-size: 0.875rem;
  font-weight: 500;
  transition: all 0.2s ease;
  -webkit-tap-highlight-color: transparent;
  user-select: none;
  -webkit-user-select: none;
}

.add-btn:active {
  opacity: 0.8;
  transform: scale(0.98);
}

.template-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.template-card {
  padding: 1.25rem;
  border-radius: 1rem;
  box-shadow: 0 2px 8px var(--shadow);
}

.template-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 1rem;
}

.template-info {
  flex: 1;
}

.template-name {
  font-size: 1.125rem;
  font-weight: 600;
  color: var(--text);
  margin-bottom: 0.25rem;
}

.template-device {
  font-size: 0.875rem;
  color: var(--text-secondary);
}

.template-actions {
  display: flex;
  gap: 0.5rem;
}

.icon-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  background: var(--background);
  border: none;
  border-radius: 0.5rem;
  color: var(--text);
  transition: all 0.2s ease;
  -webkit-tap-highlight-color: transparent;
  user-select: none;
  -webkit-user-select: none;
}

.icon-btn:active {
  background: var(--primary);
  color: white;
  transform: scale(0.95);
}

.icon-btn.danger:active {
  background: #ef4444;
}

.template-details {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.detail-item {
  display: flex;
  gap: 0.5rem;
  font-size: 0.875rem;
}

.detail-label {
  color: var(--text-secondary);
  min-width: 100px;
}

.detail-value {
  color: var(--text);
  flex: 1;
  word-break: break-all;
}

.detail-value.fingerprint {
  font-family: monospace;
  font-size: 0.75rem;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 3rem 1rem;
  text-align: center;
}

.empty-icon {
  color: var(--text-secondary);
  opacity: 0.3;
  margin-bottom: 1rem;
}

.empty-text {
  font-size: 1.125rem;
  font-weight: 500;
  color: var(--text);
  margin-bottom: 0.5rem;
}

.empty-hint {
  font-size: 0.875rem;
  color: var(--text-secondary);
}

/* 包名管理器样式 */
.package-manager {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.package-input-wrapper {
  display: flex;
  gap: 0.5rem;
  align-items: flex-start;
}

.package-suggestion {
  padding: 0.25rem 0;
}

.app-info {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.app-name {
  font-size: 0.875rem;
  color: var(--text);
}

.package-name {
  font-size: 0.75rem;
  color: var(--text-secondary);
  font-family: monospace;
}

.package-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 0.75rem;
  background: var(--background);
  border-radius: 0.5rem;
  max-height: 300px;
  overflow-y: auto;
}

.package-list-empty {
  padding: 1.5rem;
  text-align: center;
  color: var(--text-secondary);
  font-size: 0.875rem;
  background: var(--background);
  border-radius: 0.5rem;
}

.package-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.625rem 0.75rem;
  background: var(--card);
  border-radius: 0.375rem;
  transition: all 0.2s ease;
}

.package-item:hover {
  background: var(--hover);
}

.package-info {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
  flex: 1;
  min-width: 0;
}

.package-name-text {
  font-size: 0.875rem;
  font-family: monospace;
  color: var(--text);
  word-break: break-all;
}

.app-name-text {
  font-size: 0.75rem;
  color: var(--text-secondary);
}

/* Dialog 样式优化 */
.template-dialog :deep(.el-dialog) {
  margin-top: 5vh !important;
  margin-bottom: 80px !important; /* 为底栏留出足够空间 */
  max-height: calc(100vh - 80px - 10vh) !important; /* 减去底栏高度和顶部边距 */
  display: flex;
  flex-direction: column;
}

.template-dialog :deep(.el-dialog__body) {
  flex: 1;
  overflow-y: auto;
  padding: 1.5rem;
  padding-bottom: 2rem; /* 增加底部内边距 */
  max-height: calc(100vh - 200px); /* 确保有足够的滚动空间 */
}

.template-dialog :deep(.el-dialog__footer) {
  padding: 1rem 1.5rem;
  border-top: 1px solid var(--border);
  background: var(--card-bg);
  flex-shrink: 0; /* 防止底部按钮区域被压缩 */
}

/* 确保 Dialog 遮罩层在底栏之上 */
:deep(.el-overlay) {
  z-index: 2000 !important;
}
</style>
