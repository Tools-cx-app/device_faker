import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Config, Template, AppConfig } from '../types'
import { readFile, writeFile, fileExists, mkdir } from '../utils/ksu'
import { parse as parseToml, stringify as stringifyToml } from 'smol-toml'
import { normalizePackageName } from '../utils/package'
import { toast } from 'kernelsu-alt'
import { useI18n } from '../utils/i18n'

const CONFIG_PATH = '/data/adb/device_faker/config/config.toml'
const MODULE_PROP_PATH = '/data/adb/modules/device_faker/module.prop'
const CONFIG_BACKUP_KEY = 'device_faker_config_backup_v1'

export const useConfigStore = defineStore('config', () => {
  const config = ref<Config>({})
  const moduleVersion = ref('0.0.0')
  const loading = ref(false)
  const error = ref<string | null>(null)
  const { t } = useI18n()

  function saveConfigBackup(content: string) {
    if (typeof localStorage === 'undefined') return
    localStorage.setItem(CONFIG_BACKUP_KEY, content)
  }

  function loadConfigBackup(): string | null {
    if (typeof localStorage === 'undefined') return null
    return localStorage.getItem(CONFIG_BACKUP_KEY)
  }

  // 加载配置文件
  async function loadConfig() {
    loading.value = true
    error.value = null
    try {
      let content: string | null = null

      try {
        content = await readFile(CONFIG_PATH)
      } catch {
        content = null
      }

      if (content && content.trim().length > 0) {
        config.value = parseToml(content) as Config
        saveConfigBackup(content)
        return
      }

      const exists = await fileExists(CONFIG_PATH)
      if (!exists && (!content || content.trim().length === 0)) {
        // 创建默认配置
        await mkdir('/data/adb/device_faker/config')
        config.value = {
          default_mode: 'lite',
          templates: {},
          apps: [],
        }
        await saveConfig()
        return
      }

      throw new Error(t('config.load_failed'))
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
      const backup = loadConfigBackup()
      if (backup) {
        try {
          config.value = parseToml(backup) as Config
          error.value = null
          toast(t('config.backup_used'))
          return
        } catch {
          // fallthrough
        }
      }
      toast(t('config.backup_failed'))
    } finally {
      loading.value = false
    }
  }

  // 保存配置文件
  async function saveConfig() {
    loading.value = true
    error.value = null
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const content = stringifyToml(config.value as any)
      if (!content || content.trim().length === 0) {
        throw new Error(t('config.empty_content'))
      }

      await writeFile(CONFIG_PATH, content)

      const verifyContent = await readFile(CONFIG_PATH)
      if (!verifyContent || verifyContent.trim().length === 0) {
        throw new Error(t('config.save_verify_failed'))
      }

      parseToml(verifyContent)
      saveConfigBackup(verifyContent)
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
      throw e
    } finally {
      loading.value = false
    }
  }

  // 加载模块版本
  async function loadModuleVersion() {
    try {
      const content = await readFile(MODULE_PROP_PATH)
      const match = content.match(/version=(.+)/)
      if (match) {
        moduleVersion.value = match[1].trim()
      }
    } catch {
      // 使用默认版本
    }
  }

  // 获取所有模板（使用 computed 缓存）
  const templates = computed(() => config.value.templates || {})

  // 获取所有应用配置（使用 computed 缓存）
  const apps = computed(() => config.value.apps || [])

  // 获取应用伪装数量（使用 computed 自动缓存）
  const deviceFakerCount = computed(() => {
    let count = 0

    // 统计模板中的包名
    for (const template of Object.values(templates.value)) {
      if (template.packages) {
        count += template.packages.length
      }
    }

    // 统计直接配置的应用
    count += apps.value.length

    return count
  })

  // 获取模板数量（使用 computed 自动缓存）
  const templateCount = computed(() => Object.keys(templates.value).length)

  // 兼容旧的函数式 API
  function getTemplates(): Record<string, Template> {
    return templates.value
  }

  // 兼容旧的函数式 API
  function getApps(): AppConfig[] {
    return apps.value
  }

  // 兼容旧的函数式 API
  function getDeviceFakerCount(): number {
    return deviceFakerCount.value
  }

  // 添加或更新模板
  function setTemplate(name: string, template: Template) {
    if (!config.value.templates) {
      config.value.templates = {}
    }
    config.value.templates[name] = template
  }

  // 删除模板
  function deleteTemplate(name: string) {
    if (config.value.templates) {
      delete config.value.templates[name]
    }
  }

  // 添加或更新应用配置
  function setApp(appConfig: AppConfig) {
    if (!config.value.apps) {
      config.value.apps = []
    }
    const index = config.value.apps.findIndex((a: AppConfig) => a.package === appConfig.package)
    if (index >= 0) {
      config.value.apps[index] = appConfig
    } else {
      config.value.apps.push(appConfig)
    }
  }

  // 删除应用配置
  function deleteApp(packageName: string) {
    if (config.value.apps) {
      config.value.apps = config.value.apps.filter((a: AppConfig) => a.package !== packageName)
    }
  }

  // 检查包名是否已配置
  function isPackageConfigured(packageName: string): boolean {
    const hasUserSuffix = /@\d+$/.test(packageName)

    // 先尝试精确匹配，保留多用户后缀
    for (const template of Object.values(templates.value)) {
      if (template.packages?.includes(packageName)) {
        return true
      }
    }

    if (apps.value.some((a) => a.package === packageName)) {
      return true
    }

    // 仅当调用方带有 userId 后缀时，才做归一化匹配以兼容旧配置
    if (!hasUserSuffix) {
      return false
    }

    const normalized = normalizePackageName(packageName)

    for (const template of Object.values(templates.value)) {
      if (template.packages?.some((pkg) => normalizePackageName(pkg) === normalized)) {
        return true
      }
    }

    return apps.value.some((a) => normalizePackageName(a.package) === normalized)
  }

  // 获取包名的配置
  function getPackageConfig(
    packageName: string
  ): (Template & { source: string }) | AppConfig | null {
    const hasUserSuffix = /@\d+$/.test(packageName)

    // 先精确匹配
    const exactApp = apps.value.find((a) => a.package === packageName)
    if (exactApp) {
      return exactApp
    }

    for (const [name, template] of Object.entries(templates.value)) {
      if (template.packages?.includes(packageName)) {
        return { ...template, source: name }
      }
    }

    // 仅当调用方带有 userId 后缀时，才做归一化匹配（向后兼容旧配置）
    if (!hasUserSuffix) {
      return null
    }

    const normalized = normalizePackageName(packageName)

    const normalizedApp = apps.value.find((a) => normalizePackageName(a.package) === normalized)
    if (normalizedApp) {
      return normalizedApp
    }

    for (const [name, template] of Object.entries(templates.value)) {
      if (template.packages?.some((pkg) => normalizePackageName(pkg) === normalized)) {
        return { ...template, source: name }
      }
    }

    return null
  }

  // 切换工作模式
  async function toggleWorkMode() {
    const currentMode = config.value.default_mode || 'lite'
    config.value.default_mode = currentMode === 'lite' ? 'full' : 'lite'
    await saveConfig()
  }

  return {
    config,
    moduleVersion,
    loading,
    error,
    // Computed 属性（自动缓存）
    templates,
    apps,
    deviceFakerCount,
    templateCount,
    // 函数
    loadConfig,
    saveConfig,
    loadModuleVersion,
    getTemplates,
    setTemplate,
    deleteTemplate,
    getApps,
    setApp,
    deleteApp,
    getDeviceFakerCount,
    isPackageConfigured,
    getPackageConfig,
    toggleWorkMode,
  }
})
