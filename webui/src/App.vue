<template>
  <div :class="['app-container', { dark: isDark }]">
    <!-- 顶部标题栏 -->
    <header class="app-header glass-effect">
      <h1 class="header-title">
        Device Faker
        <span class="version">{{ versionDisplay }}</span>
      </h1>
    </header>

    <!-- 主内容区域 -->
    <main class="main-content">
      <div class="page-stage">
        <Transition name="page-switch">
          <KeepAlive>
            <component :is="currentPageComponent" :key="activePage" class="page-view" />
          </KeepAlive>
        </Transition>
      </div>
    </main>

    <!-- 底部导航栏 -->
    <nav class="bottom-nav glass-effect">
      <button
        v-for="page in pages"
        :key="page.id"
        :class="['nav-item', { active: activePage === page.id }]"
        @pointerdown="primePage(page.id)"
        @click.stop="handlePageChange(page.id)"
      >
        <component :is="page.icon" :size="24" />
        <span class="nav-label">{{ page.label }}</span>
      </button>
    </nav>
  </div>
</template>

<script setup lang="ts">
import {
  ref,
  computed,
  defineAsyncComponent,
  defineComponent,
  h,
  onMounted,
  onUnmounted,
  watch,
} from 'vue'
import { Home, FileText, Smartphone, Settings } from 'lucide-vue-next'
import AppsPageSkeleton from './components/apps/AppsPageSkeleton.vue'
import { useAppsStore } from './stores/apps'
import { useConfigStore } from './stores/config'
import { useSettingsStore } from './stores/settings'
import { useI18n } from './utils/i18n'
import StatusPage from './pages/StatusPage.vue'

type AppsPageComponent = (typeof import('./pages/AppsPage.vue'))['default']
type TemplatePageComponent = (typeof import('./pages/TemplatePage.vue'))['default']
type SettingsPageComponent = (typeof import('./pages/SettingsPage.vue'))['default']
type PageId = 'home' | 'templates' | 'apps' | 'settings'

const AsyncPagePlaceholder = defineComponent({
  name: 'AsyncPagePlaceholder',
  setup() {
    return () =>
      h('div', { class: 'page-placeholder glass-effect' }, [
        h('div', { class: 'page-placeholder__line page-placeholder__line--title' }),
        h('div', { class: 'page-placeholder__line' }),
        h('div', { class: 'page-placeholder__line page-placeholder__line--short' }),
      ])
  },
})

let appsPageLoader: Promise<AppsPageComponent> | null = null
let templatePageLoader: Promise<TemplatePageComponent> | null = null
let settingsPageLoader: Promise<SettingsPageComponent> | null = null
let idleWarmupTimer: number | null = null
let idleWarmupId: number | null = null
let appDataWarmupTimer: number | null = null
let appDataWarmupId: number | null = null

function preloadAppsPage() {
  if (!appsPageLoader) {
    appsPageLoader = import('./pages/AppsPage.vue')
      .then((module) => module.default)
      .catch((error) => {
        appsPageLoader = null
        throw error
      })
  }

  return appsPageLoader
}

function preloadTemplatePage() {
  if (!templatePageLoader) {
    templatePageLoader = import('./pages/TemplatePage.vue')
      .then((module) => module.default)
      .catch((error) => {
        templatePageLoader = null
        throw error
      })
  }

  return templatePageLoader
}

function preloadSettingsPage() {
  if (!settingsPageLoader) {
    settingsPageLoader = import('./pages/SettingsPage.vue')
      .then((module) => module.default)
      .catch((error) => {
        settingsPageLoader = null
        throw error
      })
  }

  return settingsPageLoader
}

const AppsPage = defineAsyncComponent({
  loader: preloadAppsPage,
  suspensible: false,
  loadingComponent: AppsPageSkeleton,
  delay: 0,
})
const TemplatePage = defineAsyncComponent<TemplatePageComponent>({
  loader: preloadTemplatePage,
  suspensible: false,
  loadingComponent: AsyncPagePlaceholder,
  delay: 0,
})
const SettingsPage = defineAsyncComponent<SettingsPageComponent>({
  loader: preloadSettingsPage,
  suspensible: false,
  loadingComponent: AsyncPagePlaceholder,
  delay: 0,
})

const configStore = useConfigStore()
const appsStore = useAppsStore()
const settingsStore = useSettingsStore()

const activePage = ref('home')
const systemPrefersDark = ref(window.matchMedia('(prefers-color-scheme: dark)').matches)
let mediaQuery: ReturnType<typeof window.matchMedia> | null = null
let mediaQueryListener: ((event: { matches: boolean }) => void) | null = null

function getMainContentElement() {
  return document.querySelector('.main-content') as HTMLElement | null
}

function resetMainContentScroll() {
  const mainContent = getMainContentElement()
  if (!mainContent) return

  const previousBehavior = mainContent.style.scrollBehavior
  mainContent.style.scrollBehavior = 'auto'
  mainContent.scrollTop = 0
  mainContent.scrollLeft = 0
  mainContent.style.scrollBehavior = previousBehavior
}

function lockMainContentDuringTemplateEntry() {
  const mainContent = getMainContentElement()
  if (!mainContent) return

  mainContent.classList.add('main-content--template-enter-lock')
}

function unlockMainContentDuringTemplateEntry() {
  const mainContent = getMainContentElement()
  if (!mainContent) return

  mainContent.classList.remove('main-content--template-enter-lock')
}

function stabilizeTemplateEntryFromHome() {
  lockMainContentDuringTemplateEntry()
  resetMainContentScroll()

  requestAnimationFrame(() => {
    resetMainContentScroll()
    requestAnimationFrame(() => {
      resetMainContentScroll()
      unlockMainContentDuringTemplateEntry()
    })
  })
}

function warmPage(pageId: PageId, options: { includeAppData?: boolean } = {}) {
  if (pageId === 'apps') {
    void preloadAppsPage().catch(() => {})
    if (options.includeAppData) {
      void appsStore.ensureUserAppsLoaded()
    }
    return
  }

  if (pageId === 'templates') {
    void preloadTemplatePage().catch(() => {})
    return
  }

  if (pageId === 'settings') {
    void preloadSettingsPage().catch(() => {})
  }
}

function primePage(pageId: string) {
  if (pageId === 'home') {
    return
  }

  warmPage(pageId as PageId, { includeAppData: pageId === 'apps' })
}

function handlePageChange(pageId: string) {
  const previousPage = activePage.value

  if (previousPage === pageId) {
    return
  }

  activePage.value = pageId
  const isTemplateFromHome = previousPage === 'home' && pageId === 'templates'

  if (isTemplateFromHome) {
    stabilizeTemplateEntryFromHome()
  }

  primePage(pageId)
}

const versionDisplay = computed(() =>
  configStore.moduleMetaReady ? configStore.moduleVersion : '--'
)
const isDark = computed(() => {
  if (settingsStore.theme === 'system') {
    return systemPrefersDark.value
  }

  return settingsStore.theme === 'dark'
})

const { t } = useI18n()

const pages = computed(() => [
  { id: 'home', label: t('nav.home'), icon: Home, component: StatusPage },
  { id: 'templates', label: t('nav.templates'), icon: FileText, component: TemplatePage },
  { id: 'apps', label: t('nav.apps'), icon: Smartphone, component: AppsPage },
  { id: 'settings', label: t('nav.settings'), icon: Settings, component: SettingsPage },
])

const currentPageComponent = computed(
  () => pages.value.find((page) => page.id === activePage.value)?.component || StatusPage
)

watch(
  isDark,
  (isDarkMode) => {
    document.documentElement.classList.toggle('dark', isDarkMode)
    document
      .getElementById('theme-color')
      ?.setAttribute('content', isDarkMode ? '#1a2538' : '#f2f9ff')
  },
  { immediate: true }
)

function scheduleConfigBootstrap() {
  requestAnimationFrame(() => {
    window.setTimeout(() => {
      void configStore.bootstrap()
    }, 0)
  })
}

function schedulePageWarmup() {
  const runWarmup = () => {
    idleWarmupId = null
    idleWarmupTimer = null
    warmPage('templates')
    warmPage('settings')
    warmPage('apps')
  }

  if (typeof window.requestIdleCallback === 'function') {
    idleWarmupId = window.requestIdleCallback(runWarmup, { timeout: 1500 })
    return
  }

  idleWarmupTimer = window.setTimeout(runWarmup, 800)
}

function scheduleAppDataWarmup() {
  const runWarmup = () => {
    appDataWarmupId = null
    appDataWarmupTimer = null
    void appsStore.ensureUserAppsLoaded()
  }

  if (typeof window.requestIdleCallback === 'function') {
    appDataWarmupId = window.requestIdleCallback(runWarmup, { timeout: 2500 })
    return
  }

  appDataWarmupTimer = window.setTimeout(runWarmup, 1800)
}

onMounted(() => {
  scheduleConfigBootstrap()
  schedulePageWarmup()
  scheduleAppDataWarmup()

  mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
  systemPrefersDark.value = mediaQuery.matches
  mediaQueryListener = (event) => {
    systemPrefersDark.value = event.matches
  }
  mediaQuery.addEventListener('change', mediaQueryListener)
})

onUnmounted(() => {
  if (mediaQuery && mediaQueryListener) {
    mediaQuery.removeEventListener('change', mediaQueryListener)
  }

  if (idleWarmupTimer !== null) {
    window.clearTimeout(idleWarmupTimer)
  }

  if (idleWarmupId !== null && typeof window.cancelIdleCallback === 'function') {
    window.cancelIdleCallback(idleWarmupId)
  }

  if (appDataWarmupTimer !== null) {
    window.clearTimeout(appDataWarmupTimer)
  }

  if (appDataWarmupId !== null && typeof window.cancelIdleCallback === 'function') {
    window.cancelIdleCallback(appDataWarmupId)
  }
})
</script>

<style scoped>
.app-container {
  display: flex;
  flex-direction: column;
  min-height: 100vh; /* 改为最小高度,允许内容超出视口 */
  background: var(--background);
  /* 移除顶部内边距,让顶栏延伸到状态栏 */
  padding: 0 var(--safe-area-inset-right) var(--safe-area-inset-bottom) var(--safe-area-inset-left);
}

.app-header {
  /* 添加顶部内边距以适配状态栏 */
  padding-top: calc(var(--safe-area-inset-top) + 1rem);
  padding-left: 1rem;
  padding-right: 1rem;
  padding-bottom: 1rem;
  border-radius: 0 0 1rem 1rem;
  margin-bottom: 1rem;
  box-shadow: 0 4px 12px var(--shadow);
  position: relative; /* 改为相对定位,不固定在视野 */
  overflow: hidden;
}

.app-header::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: linear-gradient(135deg, var(--gradient-start) 0%, var(--gradient-end) 100%);
  opacity: 0.08;
  z-index: 0;
}

.header-title {
  font-size: 1.5rem;
  font-weight: 600;
  background: linear-gradient(135deg, var(--gradient-start) 0%, var(--gradient-end) 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
  display: flex;
  align-items: flex-end;
  gap: 0.5rem;
  line-height: 1;
  position: relative;
  z-index: 1;
}

.version {
  font-size: 1rem;
  font-weight: 400;
  color: var(--text-secondary);
  line-height: 1;
  padding-bottom: 0.1rem;
}

.main-content {
  flex: 1;
  overflow-y: scroll;
  padding: 0 1rem;
  padding-bottom: 5.5rem; /* 为固定定位的底栏留出空间 */
  /* 优化滚动性能 */
  -webkit-overflow-scrolling: touch;
  scroll-behavior: smooth;
  /* 确保Android WebView正确处理触摸滚动 */
  touch-action: pan-y;
}

.page-stage {
  position: relative;
  min-height: 100%;
}

.main-content--template-enter-lock {
  overflow: hidden;
}

.page-view {
  min-height: 100%;
  width: 100%;
}

.page-placeholder {
  display: flex;
  flex-direction: column;
  gap: 0.875rem;
  padding: 1.5rem;
  border-radius: 1rem;
  min-height: 14rem;
}

.page-placeholder__line {
  height: 0.95rem;
  width: 100%;
  border-radius: 999px;
  background: linear-gradient(90deg, var(--border) 25%, var(--card-bg) 50%, var(--border) 75%);
  background-size: 200% 100%;
  animation: page-placeholder-shimmer 1.3s linear infinite;
  opacity: 0.75;
}

.page-placeholder__line--title {
  width: 42%;
  height: 1.2rem;
}

.page-placeholder__line--short {
  width: 65%;
}

@keyframes page-placeholder-shimmer {
  from {
    background-position: -200% 0;
  }

  to {
    background-position: 200% 0;
  }
}

.page-switch-enter-active {
  position: relative;
  z-index: 2;
  transition:
    opacity 0.18s ease,
    transform 0.18s ease;
  will-change: opacity, transform;
}

.page-switch-leave-active {
  position: absolute;
  inset: 0;
  width: 100%;
  z-index: 1;
  pointer-events: none;
  transition:
    opacity 0.18s ease,
    transform 0.18s ease;
  will-change: opacity, transform;
}

.page-switch-enter-from,
.page-switch-leave-to {
  opacity: 0;
  transform: translateY(12px);
}

.page-switch-enter-to,
.page-switch-leave-from {
  opacity: 1;
  transform: translateY(0);
}

.bottom-nav {
  display: flex;
  justify-content: space-around;
  align-items: center;
  padding: 0.75rem 0;
  border-radius: 1rem 1rem 0 0;
  box-shadow: 0 -4px 12px var(--shadow);
  position: fixed; /* 使用固定定位 */
  bottom: 0;
  left: 0;
  right: 0;
  z-index: 100; /* 正常显示时的优先级 */
  pointer-events: auto;
  /* 添加真正的毛玻璃效果 */
  background: rgba(255, 255, 255, 0.8);
  backdrop-filter: blur(20px) saturate(180%);
  -webkit-backdrop-filter: blur(20px) saturate(180%);
  border-top: 1px solid rgba(255, 255, 255, 0.4);
}

.dark .bottom-nav {
  background: rgba(30, 41, 59, 0.85);
  backdrop-filter: blur(20px) saturate(180%);
  -webkit-backdrop-filter: blur(20px) saturate(180%);
  border-top: 1px solid rgba(51, 65, 85, 0.4);
}

.nav-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.25rem;
  padding: 0.5rem 1rem;
  background: transparent;
  border: none;
  color: var(--text-secondary);
  transition:
    color 0.2s ease,
    background-color 0.2s ease,
    transform 0.2s ease;
  border-radius: 0.5rem;
  -webkit-tap-highlight-color: transparent;
  user-select: none;
  -webkit-user-select: none;
  cursor: pointer;
  touch-action: manipulation;
}

.nav-item:active {
  background: linear-gradient(135deg, rgba(14, 165, 233, 0.15) 0%, rgba(168, 85, 247, 0.15) 100%);
  transform: scale(0.95);
}

.nav-item.active {
  background: linear-gradient(135deg, rgba(14, 165, 233, 0.1) 0%, rgba(168, 85, 247, 0.1) 100%);
  color: var(--primary);
}

.nav-item.active svg {
  filter: drop-shadow(0 0 8px rgba(14, 165, 233, 0.5));
}

.nav-label {
  font-size: 0.75rem;
  font-weight: 500;
}
</style>
