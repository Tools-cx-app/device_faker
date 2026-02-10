<template>
  <div class="apps-page">
    <AppFilters
      v-model:search-query="searchQuery"
      v-model:filter-type="filterType"
      :total-count="allApps.length"
      :configured-count="configuredCount"
      :loading="loading"
    />

    <AppList :apps="filteredApps" :empty-text="emptyText" :loading="loading" @select="openConfig" />

    <AppConfigDialog v-model="configDialogVisible" :app="currentApp" @saved="handleConfigSaved" />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import AppConfigDialog from '../components/apps/AppConfigDialog.vue'
import AppFilters from '../components/apps/AppFilters.vue'
import AppList from '../components/apps/AppList.vue'
import { useAppsStore } from '../stores/apps'
import { useConfigStore } from '../stores/config'
import { useI18n } from '../utils/i18n'
import { normalizePackageName } from '../utils/package'
import type { InstalledApp } from '../types'

type FilterType = 'all' | 'configured'

const configStore = useConfigStore()
const appsStore = useAppsStore()
const { t } = useI18n()

const searchQuery = ref('')
const filterType = ref<FilterType>('all')
const configDialogVisible = ref(false)
const currentApp = ref<InstalledApp | null>(null)

// 根据 store 内容初始化加载状态
// 如果没有应用，则假设需要加载，因此以加载状态开始
// 防止出现空/部分内容的"闪烁"
const isInitializing = ref(appsStore.installedApps.length === 0 && !appsStore.loading)

const loading = computed(() => appsStore.loading || isInitializing.value)
const installedApps = computed(() => appsStore.installedApps)

const configuredApps = computed<InstalledApp[]>(() => {
  const map = new Map<string, InstalledApp>()

  for (const appConfig of configStore.getApps()) {
    if (map.has(appConfig.package)) continue

    map.set(appConfig.package, {
      packageName: appConfig.package,
      appName: appConfig.package,
      installed: false,
    })
  }

  const templates = configStore.getTemplates()
  for (const template of Object.values(templates)) {
    if (!template.packages) continue
    for (const pkg of template.packages) {
      if (map.has(pkg)) continue

      map.set(pkg, {
        packageName: pkg,
        appName: pkg,
        installed: false,
      })
    }
  }

  return Array.from(map.values())
})

const allApps = computed<InstalledApp[]>(() => {
  const result: InstalledApp[] = []
  const packageIndex = new Map<string, number>()
  const normalizedIndex = new Map<string, number>()

  // 保留已安装应用的原始顺序
  for (const app of installedApps.value) {
    const normalized = normalizePackageName(app.packageName)
    if (packageIndex.has(app.packageName)) continue

    const entry = {
      ...app,
      installed: app.installed ?? true,
    }

    const idx = result.length
    result.push(entry)
    packageIndex.set(app.packageName, idx)
    if (!normalizedIndex.has(normalized)) {
      normalizedIndex.set(normalized, idx)
    }
  }

  // 合并配置项：如果包名不同（即使归一化后相同），也应显示为不同应用
  for (const app of configuredApps.value) {
    if (packageIndex.has(app.packageName)) continue

    // 查找具有相同归一化包名的已存在应用，复用其展示信息
    const normalized = normalizePackageName(app.packageName)
    const existingIdx = normalizedIndex.get(normalized)

    const entry = {
      // 如果有相同归一化包名的应用，复用其展示信息，否则使用默认信息
      ...(existingIdx !== undefined ? result[existingIdx] : {}),
      packageName: app.packageName,
      appName: existingIdx !== undefined ? result[existingIdx].appName : app.packageName,
      installed:
        existingIdx !== undefined ? result[existingIdx].installed : (app.installed ?? false),
    }

    const idx = result.length
    result.push(entry)
    packageIndex.set(app.packageName, idx)
  }

  return result
})

const configuredCount = computed(
  () => allApps.value.filter((app) => configStore.isPackageConfigured(app.packageName)).length
)

const filteredApps = computed(() => {
  let apps = allApps.value

  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase()
    apps = apps.filter(
      (app) => app.packageName.toLowerCase().includes(q) || app.appName.toLowerCase().includes(q)
    )
  }

  if (filterType.value === 'configured') {
    apps = apps.filter((app) => configStore.isPackageConfigured(app.packageName))
  }

  return apps.slice().sort((a, b) => {
    const aInstalled = a.installed !== false
    const bInstalled = b.installed !== false

    if (aInstalled === bInstalled) return 0
    return aInstalled ? -1 : 1
  })
})

const emptyText = computed(() => {
  if (searchQuery.value) return t('apps.empty.search')
  if (filterType.value === 'configured') return t('apps.empty.configured')
  return t('apps.empty.all')
})

function openConfig(app: InstalledApp) {
  currentApp.value = app
  configDialogVisible.value = true
}

function handleConfigSaved() {
  // 预留钩子，未来可在保存后刷新列表或提示
}

onMounted(async () => {
  if (appsStore.installedApps.length === 0 && !appsStore.loading) {
    // 延迟到下一帧再加载，先完成页面切换体验
    requestAnimationFrame(async () => {
      await appsStore.loadInstalledApps()
      isInitializing.value = false
    })
  } else {
    isInitializing.value = false
  }
})
</script>

<style scoped>
.apps-page {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  box-sizing: border-box;
  overflow: hidden;
}
</style>
