import { parse as parseToml } from 'smol-toml'
import type { Template } from '../types'
import { execCommand } from './ksu'

const GITEE_OWNER = 'Seyud'
const GITEE_REPO = 'device_faker_config_mirror'
const GITEE_API_BASE = 'https://gitee.com/api/v5'
const GITEE_RAW_BASE = `https://gitee.com/${GITEE_OWNER}/${GITEE_REPO}/raw/main`

// GitHub 镜像源配置 (config 子模块)
const GITHUB_OWNER = 'Seyud'
const GITHUB_REPO = 'device_faker_config'
const GITHUB_RAW_BASE = `https://raw.githubusercontent.com/${GITHUB_OWNER}/${GITHUB_REPO}/main`

// CDN 镜像源配置
const JSDELIVR_BASE = `https://cdn.jsdelivr.net/gh/${GITHUB_OWNER}/${GITHUB_REPO}@main`
const STATICALLY_BASE = `https://cdn.statically.io/gh/${GITHUB_OWNER}/${GITHUB_REPO}/main`
const GITHACK_BASE = `https://raw.githack.com/${GITHUB_OWNER}/${GITHUB_REPO}/main`

const TEMP_DIR = '/data/local/tmp'
const MAX_RETRY = 3
const TIMEOUT = 30
const USER_AGENT = 'Mozilla/5.0 (compatible; DeviceFaker/1.0)'

/**
 * 验证下载文件是否完整
 * @param filePath 文件路径
 * @returns 文件是否存在且非空
 */
async function verifyDownload(filePath: string): Promise<boolean> {
  try {
    const result = await execCommand(`test -f "${filePath}" && test -s "${filePath}" && echo "ok"`)
    return result.trim() === 'ok'
  } catch {
    return false
  }
}

/**
 * 获取文件 MD5 校验值
 * @param filePath 文件路径
 * @returns MD5 值或 null
 */
async function getFileMd5(filePath: string): Promise<string | null> {
  try {
    const result = await execCommand(`md5sum "${filePath}" | cut -d' ' -f1`)
    return result.trim() || null
  } catch {
    return null
  }
}

/**
 * 使用 curl 下载文件
 * @param url 下载地址
 * @param outputPath 输出路径
 * @param resume 是否启用断点续传
 * @returns 下载是否成功
 */
async function downloadWithCurl(
  url: string,
  outputPath: string,
  resume: boolean = true
): Promise<boolean> {
  const resumeFlag = resume ? '-C -' : ''
  const command = `curl --progress-bar -L -k --connect-timeout ${TIMEOUT} -A "${USER_AGENT}" ${resumeFlag} -o "${outputPath}" "${url}"`
  try {
    console.log(`[Download] 使用 curl 下载: ${url}`)
    await execCommand(command)
    const verified = await verifyDownload(outputPath)
    if (verified) {
      const md5 = await getFileMd5(outputPath)
      console.log(`[Download] curl 下载成功, MD5: ${md5 || 'unknown'}`)
    }
    return verified
  } catch (error) {
    console.error(`[Download] curl 下载失败: ${error}`)
    return false
  }
}

/**
 * 使用 wget 下载文件
 * @param url 下载地址
 * @param outputPath 输出路径
 * @param resume 是否启用断点续传
 * @returns 下载是否成功
 */
async function downloadWithWget(
  url: string,
  outputPath: string,
  resume: boolean = true
): Promise<boolean> {
  const resumeFlag = resume ? '-c' : ''
  const command = `wget --show-progress --timeout=${TIMEOUT} --user-agent="${USER_AGENT}" ${resumeFlag} -O "${outputPath}" "${url}"`
  try {
    console.log(`[Download] 使用 wget 下载: ${url}`)
    await execCommand(command)
    const verified = await verifyDownload(outputPath)
    if (verified) {
      const md5 = await getFileMd5(outputPath)
      console.log(`[Download] wget 下载成功, MD5: ${md5 || 'unknown'}`)
    }
    return verified
  } catch (error) {
    console.error(`[Download] wget 下载失败: ${error}`)
    return false
  }
}

/**
 * 获取镜像源 URL 列表
 * @param originalUrl 原始 URL
 * @returns 镜像源 URL 数组
 */
function getMirrorUrls(originalUrl: string): string[] {
  const urls: string[] = [originalUrl]

  // 如果原始 URL 是 Gitee，添加多个镜像源
  if (originalUrl.includes('gitee.com')) {
    // GitHub 官方源
    const githubUrl = originalUrl.replace(GITEE_RAW_BASE, GITHUB_RAW_BASE)
    urls.push(githubUrl)

    // jsDelivr CDN（国内较快）
    const jsDelivrUrl = originalUrl.replace(GITEE_RAW_BASE, JSDELIVR_BASE)
    urls.push(jsDelivrUrl)

    // Statically CDN
    const staticallyUrl = originalUrl.replace(GITEE_RAW_BASE, STATICALLY_BASE)
    urls.push(staticallyUrl)

    // Githack CDN
    const githackUrl = originalUrl.replace(GITEE_RAW_BASE, GITHACK_BASE)
    urls.push(githackUrl)
  }

  return urls
}

/**
 * 下载文件（支持多工具、多镜像、重试）
 * @param url 下载地址
 * @param outputPath 输出路径
 * @returns 下载是否成功
 */
async function downloadFile(url: string, outputPath: string): Promise<boolean> {
  const urls = getMirrorUrls(url)

  for (let urlIndex = 0; urlIndex < urls.length; urlIndex++) {
    const currentUrl = urls[urlIndex]
    const isMirror = urlIndex > 0

    if (isMirror) {
      console.log(`[Download] 切换到镜像源: ${currentUrl}`)
    }

    for (let attempt = 1; attempt <= MAX_RETRY; attempt++) {
      console.log(`[Download] 第 ${attempt}/${MAX_RETRY} 次尝试...`)

      // 尝试使用 curl
      if (await downloadWithCurl(currentUrl, outputPath, attempt > 1)) {
        return true
      }

      // 尝试使用 wget
      if (await downloadWithWget(currentUrl, outputPath, attempt > 1)) {
        return true
      }

      if (attempt < MAX_RETRY) {
        console.log(`[Download] 等待 2 秒后重试...`)
        await execCommand(`sleep 2`)
      }
    }
  }

  console.error(`[Download] 所有镜像源均下载失败: ${url}`)
  return false
}

export const TEMPLATE_CATEGORIES = {
  common: '通用设备',
  gaming: '游戏设备',
  transcend: '破限设备',
} as const

export type TemplateCategory = keyof typeof TEMPLATE_CATEGORIES

export interface OnlineTemplate {
  name: string
  displayName: string
  category: TemplateCategory
  brand: string | null
  path: string
  downloadUrl: string
  template?: Template
}

export interface OnlineTemplatesResult {
  templates: OnlineTemplate[]
  brands: string[]
}

const brandCache = new Map<TemplateCategory, string[]>()

/**
 * 使用 curl 执行 HTTP GET 请求
 * @param url 请求地址
 * @param outputPath 输出文件路径（可选）
 * @returns 响应内容或 null
 */
async function httpGetWithCurl(url: string, outputPath?: string): Promise<string | null> {
  const outputFlag = outputPath ? `-o "${outputPath}"` : ''
  const command = `curl -s -L -k --connect-timeout ${TIMEOUT} -A "${USER_AGENT}" ${outputFlag} "${url}"`
  try {
    console.log(`[HTTP] curl GET: ${url}`)
    if (outputPath) {
      await execCommand(command)
      const content = await execCommand(`cat "${outputPath}"`)
      await execCommand(`rm -f "${outputPath}"`).catch(() => {})
      return content
    } else {
      return await execCommand(command)
    }
  } catch (error) {
    console.error(`[HTTP] curl 请求失败: ${error}`)
    return null
  }
}

/**
 * 使用 wget 执行 HTTP GET 请求
 * @param url 请求地址
 * @param outputPath 输出文件路径（可选）
 * @returns 响应内容或 null
 */
async function httpGetWithWget(url: string, outputPath?: string): Promise<string | null> {
  const outputFlag = outputPath ? `-O "${outputPath}"` : '-O -'
  const command = `wget -q --timeout=${TIMEOUT} --user-agent="${USER_AGENT}" ${outputFlag} "${url}"`
  try {
    console.log(`[HTTP] wget GET: ${url}`)
    if (outputPath) {
      await execCommand(command)
      const content = await execCommand(`cat "${outputPath}"`)
      await execCommand(`rm -f "${outputPath}"`).catch(() => {})
      return content
    } else {
      return await execCommand(command)
    }
  } catch (error) {
    console.error(`[HTTP] wget 请求失败: ${error}`)
    return null
  }
}

/**
 * HTTP GET 请求（自动尝试 curl/wget，支持多镜像源）
 * @param url 请求地址
 * @param maxRetries 最大重试次数
 * @returns 响应内容或 null
 */
async function httpGet(url: string, maxRetries: number = MAX_RETRY): Promise<string | null> {
  const urls = getMirrorUrls(url)

  for (let urlIndex = 0; urlIndex < urls.length; urlIndex++) {
    const currentUrl = urls[urlIndex]
    const isMirror = urlIndex > 0

    if (isMirror) {
      console.log(`[HTTP] 切换到镜像源: ${currentUrl}`)
    }

    for (let attempt = 1; attempt <= maxRetries; attempt++) {
      console.log(`[HTTP] 第 ${attempt}/${maxRetries} 次尝试...`)

      // 尝试使用 curl
      const curlResult = await httpGetWithCurl(currentUrl)
      if (curlResult && curlResult.length > 0) {
        console.log(`[HTTP] curl 请求成功，响应大小: ${curlResult.length} bytes`)
        return curlResult
      }

      // 尝试使用 wget
      const wgetResult = await httpGetWithWget(currentUrl)
      if (wgetResult && wgetResult.length > 0) {
        console.log(`[HTTP] wget 请求成功，响应大小: ${wgetResult.length} bytes`)
        return wgetResult
      }

      if (attempt < maxRetries) {
        console.log(`[HTTP] 等待 2 秒后重试...`)
        await execCommand(`sleep 2`)
      }
    }
  }

  console.error(`[HTTP] 所有镜像源均请求失败: ${url}`)
  return null
}

async function fetchDirsFromAPI(path: string): Promise<string[]> {
  const url = `${GITEE_API_BASE}/repos/${GITEE_OWNER}/${GITEE_REPO}/contents/${path}?ref=main`
  try {
    const content = await httpGet(url)
    if (!content) return []
    const files = JSON.parse(content)
    if (!Array.isArray(files)) return []
    return files
      .filter((f: { type: string }) => f.type === 'dir')
      .map((f: { name: string }) => f.name)
  } catch {
    return []
  }
}

async function getBrandCategories(category: TemplateCategory): Promise<string[]> {
  if (brandCache.has(category)) {
    return brandCache.get(category)!
  }

  const path = `templates/${category}`
  const dirs = await fetchDirsFromAPI(path)
  const brands = dirs.filter((dir) => !dir.startsWith('.'))

  brandCache.set(category, brands)
  return brands
}

export async function fetchBrandsByCategory(category: TemplateCategory): Promise<string[]> {
  return getBrandCategories(category)
}

export async function fetchAllBrands(): Promise<string[]> {
  const categories = Object.keys(TEMPLATE_CATEGORIES) as TemplateCategory[]
  const allBrands = new Set<string>()

  await Promise.all(
    categories.map(async (cat) => {
      const brands = await getBrandCategories(cat)
      brands.forEach((b) => allBrands.add(b))
    })
  )

  return Array.from(allBrands).sort()
}

async function getTomlFilesFromHTML(
  path: string,
  category: TemplateCategory,
  brands: string[]
): Promise<OnlineTemplate[]> {
  const templates: OnlineTemplate[] = []
  const url = `https://gitee.com/${GITEE_OWNER}/${GITEE_REPO}/tree/main/${path}`

  try {
    const html = await httpGet(url, MAX_RETRY)
    if (!html) throw new Error(`Failed to fetch after ${MAX_RETRY} retries`)

    const escapedPath = path.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    const fileRegex = new RegExp(
      `/${GITEE_OWNER}/${GITEE_REPO}/blob/main/(${escapedPath}/[^"]+\\.toml)`,
      'g'
    )
    const dirRegex = new RegExp(
      `/${GITEE_OWNER}/${GITEE_REPO}/tree/main/(${escapedPath}/[^"/]+)(?=")`,
      'g'
    )

    let match
    const foundFiles = new Set<string>()
    while ((match = fileRegex.exec(html)) !== null) {
      foundFiles.add(match[1])
    }

    const foundDirs = new Set<string>()
    const dirMatches = html.matchAll(dirRegex)
    for (const dirMatch of dirMatches) {
      const dirPath = dirMatch[1]
      if (!foundDirs.has(dirPath) && dirPath !== path) {
        foundDirs.add(dirPath)
      }
    }

    for (const filePath of foundFiles) {
      const fileName = filePath.split('/').pop()!.replace('.toml', '')
      templates.push({
        name: fileName,
        displayName: fileName.replace(/_/g, ' '),
        category,
        brand: null,
        path: filePath,
        downloadUrl: `${GITEE_RAW_BASE}/${filePath}`,
      })
    }

    const brandDirs: { dirPath: string; brand: string }[] = []
    const normalDirs: string[] = []

    for (const dirPath of foundDirs) {
      const dirName = dirPath.split('/').pop()!
      if (brands.includes(dirName)) {
        brandDirs.push({ dirPath, brand: dirName })
      } else {
        normalDirs.push(dirPath)
      }
    }

    for (const { dirPath, brand } of brandDirs) {
      const subTemplates = await getTomlFilesFromHTML(dirPath, category, brands)
      subTemplates.forEach((t) => {
        t.brand = brand
      })
      templates.push(...subTemplates)
    }

    for (const dirPath of normalDirs) {
      const subTemplates = await getTomlFilesFromHTML(dirPath, category, brands)
      templates.push(...subTemplates)
    }

    return templates
  } catch (error) {
    console.error(`Failed to parse HTML from ${path}:`, error)
    throw error
  }
}

async function getTomlFilesFromAPI(
  path: string,
  category: TemplateCategory,
  brands: string[]
): Promise<OnlineTemplate[]> {
  const templates: OnlineTemplate[] = []
  const url = `${GITEE_API_BASE}/repos/${GITEE_OWNER}/${GITEE_REPO}/contents/${path}?ref=main`

  try {
    const content = await httpGet(url, MAX_RETRY)
    if (!content) throw new Error(`Failed to fetch after ${MAX_RETRY} retries`)
    const files = JSON.parse(content)

    if (!Array.isArray(files)) throw new Error('API response is not an array')

    for (const file of files) {
      if (file.type === 'file' && file.name.endsWith('.toml')) {
        templates.push({
          name: file.name.replace('.toml', ''),
          displayName: file.name.replace('.toml', '').replace(/_/g, ' '),
          category,
          brand: null,
          path: file.path,
          downloadUrl: `${GITEE_RAW_BASE}/${file.path}`,
        })
      } else if (file.type === 'dir') {
        const dirName = file.name
        if (brands.includes(dirName)) {
          const brandTemplates = await getTomlFilesFromAPI(file.path, category, brands)
          brandTemplates.forEach((t) => {
            t.brand = dirName
          })
          templates.push(...brandTemplates)
        } else {
          const subTemplates = await getTomlFilesFromAPI(file.path, category, brands)
          templates.push(...subTemplates)
        }
      }
    }

    return templates
  } catch (error) {
    console.error(`Failed to fetch from API ${path}:`, error)
    throw error
  }
}

async function getTomlFiles(category: TemplateCategory): Promise<OnlineTemplate[]> {
  const path = `templates/${category}`
  const brands = await getBrandCategories(category)

  try {
    const templates = await getTomlFilesFromAPI(path, category, brands)
    if (templates.length > 0) return templates
  } catch (apiError) {
    console.warn(`API method failed for ${category}:`, apiError)
  }

  try {
    const templates = await getTomlFilesFromHTML(path, category, brands)
    if (templates.length > 0) return templates
  } catch (htmlError) {
    console.error(`HTML method failed for ${category}:`, htmlError)
  }

  return []
}

export async function fetchOnlineTemplates(): Promise<OnlineTemplatesResult> {
  const categories = Object.keys(TEMPLATE_CATEGORIES) as TemplateCategory[]
  const results = await Promise.all(categories.map((cat) => getTomlFiles(cat)))
  const templates = results.flat()
  const brands = await fetchAllBrands()

  return { templates, brands }
}

export async function downloadTemplate(onlineTemplate: OnlineTemplate): Promise<Template | null> {
  const tempFile = `${TEMP_DIR}/template_${Date.now()}.toml`

  try {
    const success = await downloadFile(onlineTemplate.downloadUrl, tempFile)
    if (!success) {
      console.error(`Failed to download template ${onlineTemplate.name}`)
      return null
    }

    const content = await execCommand(`cat "${tempFile}"`)
    await execCommand(`rm -f "${tempFile}"`).catch(() => {})

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const parsed = parseToml(content) as any

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
