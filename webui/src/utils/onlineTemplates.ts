import { parse as parseToml } from 'smol-toml'
import type { Template } from '../types'
import { execCommand } from './ksu'

// Gitee 配置
const GITEE_OWNER = 'Seyud'
const GITEE_REPO = 'device_faker_config_mirror'
const GITEE_API_BASE = 'https://gitee.com/api/v5'
const GITEE_RAW_BASE = `https://gitee.com/${GITEE_OWNER}/${GITEE_REPO}/raw/main`

// 模板分类
export const TEMPLATE_CATEGORIES = {
  common: '通用设备',
  gaming: '游戏设备',
  transcend: '破限设备',
} as const

export type TemplateCategory = keyof typeof TEMPLATE_CATEGORIES

// 在线模板信息
export interface OnlineTemplate {
  name: string // 文件名（不含扩展名）
  displayName: string // 显示名称
  category: TemplateCategory
  path: string // 文件路径
  downloadUrl: string // 下载地址
  template?: Template // 解析后的模板内容
}

/**
 * 解析 Gitee 网页 HTML 获取文件列表（不受 API 限流影响）
 */
async function getTomlFilesFromHTML(
  path: string,
  category: TemplateCategory
): Promise<OnlineTemplate[]> {
  const templates: OnlineTemplate[] = []
  const url = `https://gitee.com/${GITEE_OWNER}/${GITEE_REPO}/tree/main/${path}`

  try {
    // 使用 curl 命令获取 HTML
    const html = await execCommand(`curl -s "${url}"`)

    // 解析 HTML 找到文件和目录链接
    // Gitee 的文件链接格式: /Seyud/device_faker_config_mirror/blob/main/templates/...
    // 目录链接格式: /Seyud/device_faker_config_mirror/tree/main/templates/...

    // 需要转义路径中的特殊字符
    const escapedPath = path.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')

    const fileRegex = new RegExp(
      `/${GITEE_OWNER}/${GITEE_REPO}/blob/main/(${escapedPath}/[^"]+\\.toml)`,
      'g'
    )
    const dirRegex = new RegExp(
      `/${GITEE_OWNER}/${GITEE_REPO}/tree/main/(${escapedPath}/[^"/]+)(?=")`,
      'g'
    )

    // 提取文件
    let match
    const foundFiles = new Set<string>()
    while ((match = fileRegex.exec(html)) !== null) {
      foundFiles.add(match[1])
    }

    // 提取子目录并递归
    const foundDirs = new Set<string>()
    const dirMatches = html.matchAll(dirRegex)
    for (const dirMatch of dirMatches) {
      const dirPath = dirMatch[1]
      if (!foundDirs.has(dirPath) && dirPath !== path) {
        foundDirs.add(dirPath)
      }
    }

    // 处理找到的文件
    for (const filePath of foundFiles) {
      const fileName = filePath.split('/').pop()!.replace('.toml', '')
      templates.push({
        name: fileName,
        displayName: fileName.replace(/_/g, ' '),
        category,
        path: filePath,
        downloadUrl: `${GITEE_RAW_BASE}/${filePath}`,
      })
    }

    // 递归处理子目录
    for (const dirPath of foundDirs) {
      const subTemplates = await getTomlFilesFromHTML(dirPath, category)
      templates.push(...subTemplates)
    }

    return templates
  } catch (error) {
    console.error(`Failed to parse HTML from ${path}:`, error)
    throw error
  }
}

/**
 * 使用 curl 命令获取 API 数据（避免 CORS 问题）
 */
async function getTomlFilesFromAPI(
  path: string,
  category: TemplateCategory
): Promise<OnlineTemplate[]> {
  const url = `${GITEE_API_BASE}/repos/${GITEE_OWNER}/${GITEE_REPO}/contents/${path}?ref=main`
  const templates: OnlineTemplate[] = []

  try {
    // 使用 curl 命令获取数据
    const jsonStr = await execCommand(`curl -s "${url}"`)
    const files = JSON.parse(jsonStr)

    if (!Array.isArray(files)) {
      throw new Error('API response is not an array')
    }

    for (const file of files) {
      if (file.type === 'file' && file.name.endsWith('.toml')) {
        templates.push({
          name: file.name.replace('.toml', ''),
          displayName: file.name.replace('.toml', '').replace(/_/g, ' '),
          category,
          path: file.path,
          downloadUrl: `${GITEE_RAW_BASE}/${file.path}`,
        })
      } else if (file.type === 'dir') {
        const subTemplates = await getTomlFilesFromAPI(file.path, category)
        templates.push(...subTemplates)
      }
    }

    return templates
  } catch (error) {
    console.error(`Failed to fetch from API ${path}:`, error)
    throw error
  }
}

/**
 * 获取指定分类下的所有 .toml 文件
 * 优先使用 API，失败则解析 HTML
 */
async function getTomlFiles(category: TemplateCategory): Promise<OnlineTemplate[]> {
  const path = `templates/${category}`

  // 方式1：使用 Gitee API（快速但可能被限流）
  try {
    const templates = await getTomlFilesFromAPI(path, category)
    if (templates.length > 0) {
      return templates
    }
  } catch (apiError) {
    console.warn(`API method failed for ${category}:`, apiError)
  }

  // 方式2：解析 HTML 页面（慢但不受限流影响）
  try {
    const templates = await getTomlFilesFromHTML(path, category)
    if (templates.length > 0) {
      return templates
    }
  } catch (htmlError) {
    console.error(`HTML method failed for ${category}:`, htmlError)
  }

  return []
}

/**
 * 获取所有分类的模板列表
 */
export async function fetchOnlineTemplates(): Promise<OnlineTemplate[]> {
  const categories = Object.keys(TEMPLATE_CATEGORIES) as TemplateCategory[]
  const results = await Promise.all(categories.map((cat) => getTomlFiles(cat)))
  return results.flat()
}

/**
 * 下载并解析模板内容
 */
export async function downloadTemplate(onlineTemplate: OnlineTemplate): Promise<Template | null> {
  try {
    // 使用 curl 命令下载文件
    const content = await execCommand(`curl -s "${onlineTemplate.downloadUrl}"`)
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const parsed = parseToml(content) as any

    // 提取模板内容（通常在 templates 对象下）
    if (parsed.templates) {
      const templateKey = Object.keys(parsed.templates)[0]
      if (templateKey) {
        return parsed.templates[templateKey] as Template
      }
    }

    return null
  } catch (error) {
    console.error(`Failed to download template ${onlineTemplate.name}:`, error)
    return null
  }
}

/**
 * 批量下载模板内容
 */
export async function downloadTemplates(
  onlineTemplates: OnlineTemplate[]
): Promise<OnlineTemplate[]> {
  const results = await Promise.all(
    onlineTemplates.map(async (t) => {
      const template = await downloadTemplate(t)
      return { ...t, template: template || undefined }
    })
  )
  return results.filter((t) => t.template !== undefined)
}
