import { exec, listPackages, getPackagesInfo } from 'kernelsu-alt'

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

// 写入文件
export async function writeFile(path: string, content: string): Promise<void> {
  const escapedContent = content.replace(/'/g, "'\\''")
  await execCommand(`echo '${escapedContent}' > ${path}`)
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

    // 使用 kernelsu-alt 的 listPackages API
    try {
      const [userPkgs, systemPkgs] = await Promise.all([
        listPackages('user').catch(() => []),
        listPackages('system').catch(() => []),
      ])
      packageList = [...userPkgs, ...systemPkgs]
    } catch {
      // Fallback to pm list packages
    }

    // Fallback: 使用 pm list packages
    if (packageList.length === 0) {
      try {
        const packages = await execCommand('pm list packages | cut -d: -f2')
        packageList = packages.split('\n').filter((p) => p.trim())
      } catch {
        return []
      }
    }

    // 去重
    packageList = Array.from(new Set(packageList))

    const apps = []

    // 批量获取应用信息
    for (const pkg of packageList) {
      try {
        let appName = pkg
        let versionName = ''
        let versionCode = 0

        // 使用 kernelsu-alt 的 getPackagesInfo API
        try {
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          if (typeof (globalThis as any).ksu?.getPackagesInfo !== 'undefined') {
            const info = await getPackagesInfo(pkg)
            appName = info.appLabel || pkg
            versionName = info.versionName || ''
            versionCode = info.versionCode || 0
          } else {
            throw new Error('No ksu api')
          }
        } catch {
          // 尝试使用 WebUI-X packageManager API
          if (typeof window.$packageManager !== 'undefined') {
            try {
              const info = window.$packageManager.getApplicationInfo(pkg, 0, 0)
              appName = info.getLabel() || pkg
            } catch {
              // 静默失败，使用包名
            }
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
      } catch {
        // 即使获取信息失败，也添加基本信息
        apps.push({
          packageName: pkg,
          appName: pkg,
          icon: '',
          versionName: '',
          versionCode: 0,
          installed: true,
        })
      }
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
