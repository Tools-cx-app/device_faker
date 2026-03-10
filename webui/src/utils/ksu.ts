import { exec, listPackages, getPackagesInfo } from 'kernelsu-alt'
import { normalizePackageName } from './package'

// 执行命令
export async function execCommand(command: string): Promise<string> {
  // 开发模式下的模拟数据
  if (import.meta.env?.DEV) {
    return new Promise((resolve) => {
      setTimeout(() => resolve(''), 100)
    })
  }

  // 使用 kernelsu-alt 的 exec
  const result = await exec(command)
  if (result.errno === 0) {
    return result.stdout || ''
  } else {
    throw new Error(result.stderr || `Command failed with error code ${result.errno}`)
  }
}

// 读取文件
export async function readFile(path: string): Promise<string> {
  // 开发模式返回模拟数据
  if (import.meta.env?.DEV) {
    const { mockConfig, mockModuleProp } = await import('./mockData')
    if (path.includes('config.toml')) {
      return mockConfig
    }
    if (path.includes('module.prop')) {
      return mockModuleProp
    }
    return ''
  }

  const content = await execCommand(`cat ${path}`)
  return content.trim()
}

function escapeShellPath(path: string): string {
  return path.replace(/'/g, "'\\''")
}

function hasHeredocDelimiter(content: string, delimiter: string): boolean {
  const normalized = content.replace(/\r\n/g, '\n')
  if (normalized.includes(`\n${delimiter}\n`)) return true
  if (normalized.startsWith(`${delimiter}\n`)) return true
  if (normalized.endsWith(`\n${delimiter}`)) return true
  return false
}

function pickHeredocDelimiter(content: string): string {
  const base = 'EOF_DEVICE_FAKER'
  let delimiter = base
  let attempt = 0
  while (hasHeredocDelimiter(content, delimiter)) {
    attempt += 1
    delimiter = `${base}_${attempt}_${Math.random().toString(36).slice(2, 8)}`
  }
  return delimiter
}

// 写入文件
export async function writeFile(path: string, content: string): Promise<void> {
  const delimiter = pickHeredocDelimiter(content)
  const escapedPath = escapeShellPath(path)
  const tempPath = escapeShellPath(`${path}.tmp.${Date.now()}`)
  const script = [
    `cat << '${delimiter}' > '${tempPath}'`,
    content,
    delimiter,
    `sync '${tempPath}' || true`,
    `mv -f '${tempPath}' '${escapedPath}'`,
  ].join('\n')

  try {
    await execCommand(script)
  } catch (err) {
    await execCommand(`rm -f '${tempPath}'`).catch(() => {})
    throw err
  }
}

/**
 * 使用 WebUI-X $packageManager API 获取已安装应用列表
 */
async function getInstalledAppsViaWebUIX(): Promise<string[]> {
  if (typeof window.$packageManager === 'undefined') {
    return []
  }

  try {
    // 获取用户应用 (userId=0)
    const packagesJson = window.$packageManager.getInstalledPackages(0, 0)
    if (!packagesJson) return []

    const packages: string[] = JSON.parse(packagesJson)
    return packages
  } catch {
    return []
  }
}

/**
 * 使用 kernelsu-alt 的 listPackages API 获取已安装应用列表
 */
async function getInstalledAppsViaKernelSU(): Promise<string[]> {
  try {
    const [userPkgs, systemPkgs] = await Promise.all([
      listPackages('user').catch(() => []),
      listPackages('system').catch(() => []),
    ])
    return [...userPkgs, ...systemPkgs]
  } catch {
    return []
  }
}

/**
 * 使用 WebUI-X $packageManager API 获取单个应用信息
 */
function getAppInfoViaWebUIX(
  packageName: string
): { appName: string; versionName: string; versionCode: number } | null {
  if (typeof window.$packageManager === 'undefined') {
    return null
  }

  try {
    const info = window.$packageManager.getApplicationInfo(packageName, 0, 0)
    return {
      appName: info.getLabel() || packageName,
      versionName: info.getVersionName() || '',
      versionCode: info.getVersionCode() || 0,
    }
  } catch {
    return null
  }
}

/**
 * 使用 kernelsu-alt 的 getPackagesInfo API 获取单个应用信息
 */
async function getAppInfoViaKernelSU(
  packageName: string
): Promise<{ appName: string; versionName: string; versionCode: number } | null> {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  if (typeof (globalThis as any).ksu?.getPackagesInfo === 'undefined') {
    return null
  }

  try {
    const info = await getPackagesInfo(packageName)
    return {
      appName: info.appLabel || packageName,
      versionName: info.versionName || '',
      versionCode: info.versionCode || 0,
    }
  } catch {
    return null
  }
}

// 获取已安装应用列表
export async function getInstalledApps() {
  // 开发模式返回模拟数据
  if (import.meta.env?.DEV) {
    const { mockInstalledApps } = await import('./mockData')
    return mockInstalledApps
  }

  try {
    let packageList: string[] = []

    // 优先使用 WebUI-X $packageManager API
    if (typeof window.$packageManager !== 'undefined') {
      packageList = await getInstalledAppsViaWebUIX()
    }

    // 如果 WebUI-X 没有返回结果，回退到 kernelsu-alt API
    if (packageList.length === 0) {
      packageList = await getInstalledAppsViaKernelSU()
    }

    // 如果仍然没有获取到应用列表，返回空列表
    if (packageList.length === 0) {
      return []
    }

    // 去重
    packageList = Array.from(new Set(packageList))

    const apps = []

    // 批量获取应用信息
    for (const pkg of packageList) {
      const normalizedPkg = normalizePackageName(pkg)
      let appName = pkg
      let versionName = ''
      let versionCode = 0

      // 优先使用 WebUI-X $packageManager API 获取应用信息
      const webUIXInfo = getAppInfoViaWebUIX(normalizedPkg)
      if (webUIXInfo) {
        appName = webUIXInfo.appName
        versionName = webUIXInfo.versionName
        versionCode = webUIXInfo.versionCode
      } else {
        // 回退到 kernelsu-alt API
        const ksuInfo = await getAppInfoViaKernelSU(normalizedPkg)
        if (ksuInfo) {
          appName = ksuInfo.appName
          versionName = ksuInfo.versionName
          versionCode = ksuInfo.versionCode
        }
      }

      apps.push({
        packageName: pkg,
        appName,
        icon: '',
        versionName,
        versionCode,
        installed: true,
      })
    }

    return apps
  } catch {
    return []
  }
}

export async function fileExists(path: string): Promise<boolean> {
  try {
    await execCommand(`test -f ${path}`)
    return true
  } catch {
    return false
  }
}

// 创建目录
export async function mkdir(path: string): Promise<void> {
  await execCommand(`mkdir -p ${path}`)
}

export default {
  execCommand,
  readFile,
  writeFile,
  getInstalledApps,
  fileExists,
  mkdir,
}
