<template>
  <div v-loading="loading" class="apps-list">
    <div
      v-for="app in apps"
      :key="app.packageName"
      class="app-card glass-effect"
      @click="emit('select', app)"
    >
      <div class="app-icon-container" :data-package="app.packageName">
        <div
          v-if="
            !appIcons[app.packageName] ||
            (appIcons[app.packageName] !== 'fallback' && !iconLoaded[app.packageName])
          "
          class="icon-loader"
        ></div>
        <img
          v-if="appIcons[app.packageName] && appIcons[app.packageName] !== 'fallback'"
          :src="appIcons[app.packageName]"
          class="app-icon-img"
          :class="{ loaded: iconLoaded[app.packageName] }"
          alt=""
          loading="lazy"
          @load="onIconLoad(app.packageName)"
          @error="onIconError(app.packageName)"
        />
        <Smartphone
          v-if="appIcons[app.packageName] === 'fallback'"
          :size="40"
          class="app-icon-fallback"
        />
      </div>
      <div class="app-info">
        <h3 class="app-name">{{ app.appName }}</h3>
        <p class="app-package">{{ app.packageName }}</p>
        <div class="app-status-group">
          <p v-if="isConfigured(app.packageName)" class="app-status configured">
            <Check :size="14" />
            {{ t('apps.status.configured') }}
          </p>
          <p v-else class="app-status unconfigured">{{ t('apps.status.unconfigured') }}</p>
          <p v-if="!isInstalled(app)" class="app-status not-installed">
            {{ t('apps.status.not_installed') }}
          </p>
        </div>
      </div>
      <div class="app-actions">
        <ChevronRight :size="20" />
      </div>
    </div>

    <div v-if="!loading && apps.length === 0" class="empty-state">
      <Smartphone :size="64" class="empty-icon" />
      <p class="empty-text">{{ emptyText }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, watch } from 'vue'
import { Smartphone, ChevronRight, Check } from 'lucide-vue-next'
import { useConfigStore } from '../../stores/config'
import { useI18n } from '../../utils/i18n'
import { useAppIcons } from '../../composables/useAppIcons'
import type { InstalledApp } from '../../types'

const props = defineProps<{
  apps: InstalledApp[]
  emptyText: string
  loading: boolean
}>()

const emit = defineEmits<{ select: [InstalledApp] }>()

const { t } = useI18n()
const configStore = useConfigStore()
const { appIcons, iconLoaded, onIconLoad, onIconError, setupIconObserver, teardownIconObserver } =
  useAppIcons()

const isConfigured = (packageName: string) => configStore.isPackageConfigured(packageName)
const isInstalled = (app: InstalledApp) => app.installed !== false

const loading = computed(() => props.loading)

watch(
  () => props.apps,
  () => {
    nextTick(() => setupIconObserver())
  }
)

onMounted(() => {
  setupIconObserver()
})

onUnmounted(() => {
  teardownIconObserver()
})
</script>

<style scoped>
.apps-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.app-card {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 1rem;
  border-radius: 0.75rem;
  box-shadow: 0 2px 8px var(--shadow);
  transition: all 0.2s ease;
  -webkit-tap-highlight-color: transparent;
  user-select: none;
  -webkit-user-select: none;
}

.app-card:active {
  transform: scale(0.98);
  box-shadow: 0 1px 4px var(--shadow);
}

.app-icon-container {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 48px;
  height: 48px;
  flex-shrink: 0;
  border-radius: 0.75rem;
  overflow: hidden;
}

.icon-loader {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: linear-gradient(90deg, var(--card-bg), var(--border), var(--card-bg));
  background-size: 200% 100%;
  animation: shimmer 1.2s infinite linear;
  border-radius: 0.75rem;
}

@keyframes shimmer {
  0% {
    background-position: -200% 0;
  }
  100% {
    background-position: 200% 0;
  }
}

.app-icon-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  border-radius: 0.75rem;
  opacity: 0;
  transition: opacity 0.3s ease;
}

.app-icon-img.loaded {
  opacity: 1;
}

.app-icon-fallback {
  color: var(--primary);
}

.app-info {
  flex: 1;
  min-width: 0;
}

.app-name {
  font-size: 1rem;
  font-weight: 600;
  color: var(--text);
  margin-bottom: 0.25rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.app-package {
  font-size: 0.75rem;
  color: var(--text-secondary);
  margin-bottom: 0.25rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.app-status-group {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  flex-wrap: wrap;
}

.app-status {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  font-size: 0.75rem;
  padding: 0.125rem 0.5rem;
  border-radius: 0.25rem;
}

.app-status.configured {
  background: rgba(34, 197, 94, 0.1);
  color: #22c55e;
}

.app-status.unconfigured {
  background: rgba(156, 163, 175, 0.1);
  color: var(--text-secondary);
}

.app-status.not-installed {
  background: rgba(239, 68, 68, 0.12);
  color: #ef4444;
}

.app-actions {
  display: flex;
  align-items: center;
  color: var(--text-secondary);
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
}
</style>
