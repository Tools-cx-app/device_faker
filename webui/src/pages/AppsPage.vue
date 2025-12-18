<template>
  <div class="apps-page">
    <AppFilters
      v-model:search-query="searchQuery"
      v-model:filter-type="filterType"
      :total-count="allApps.length"
      :configured-count="configuredCount"
      :unconfigured-count="unconfiguredCount"
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
import type { InstalledApp } from '../types'

type FilterType = 'all' | 'configured' | 'unconfigured'

const configStore = useConfigStore()
const appsStore = useAppsStore()
const { t } = useI18n()

const searchQuery = ref('')
const filterType = ref<FilterType>('all')
const configDialogVisible = ref(false)
const currentApp = ref<InstalledApp | null>(null)

const loading = computed(() => appsStore.loading)
const installedApps = computed(() => appsStore.installedApps)

const configuredApps = computed<InstalledApp[]>(() => {
  const map = new Map<string, InstalledApp>()

  for (const appConfig of configStore.getApps()) {
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
      if (!map.has(pkg)) {
        map.set(pkg, {
          packageName: pkg,
          appName: pkg,
          installed: false,
        })
      }
    }
  }

  return Array.from(map.values())
})

const allApps = computed<InstalledApp[]>(() => {
  const seen = new Set<string>()
  const merged: InstalledApp[] = []

  for (const app of installedApps.value) {
    merged.push({ ...app, installed: app.installed ?? true })
    seen.add(app.packageName)
  }

  for (const app of configuredApps.value) {
    if (!seen.has(app.packageName)) {
      merged.push(app)
    }
  }

  return merged
})

const configuredCount = computed(
  () => allApps.value.filter((app) => configStore.isPackageConfigured(app.packageName)).length
)

const unconfiguredCount = computed(() => allApps.value.length - configuredCount.value)

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
  } else if (filterType.value === 'unconfigured') {
    apps = apps.filter((app) => !configStore.isPackageConfigured(app.packageName))
  }

  return apps
})

const emptyText = computed(() => {
  if (searchQuery.value) return t('apps.empty.search')
  if (filterType.value === 'configured') return t('apps.empty.configured')
  if (filterType.value === 'unconfigured') return t('apps.empty.unconfigured')
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
  if (appsStore.installedApps.length === 0) {
    await appsStore.loadInstalledApps()
  }
})
</script>

<style scoped>
.apps-page {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  width: 100%;
  max-width: 100%;
  box-sizing: border-box;
  overflow: hidden;
}
</style>
