/// <reference types="vite/client" />

declare module '*.vue' {
  import type { DefineComponent } from 'vue'
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const component: DefineComponent<Record<string, never>, Record<string, never>, any>
  export default component
}

interface ImportMetaEnv {
  readonly DEV: boolean
  readonly PROD: boolean
  readonly MODE: string
  readonly VITE_DEBUG?: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}

// WebUI-X 应用信息接口
interface WXApplicationInfo {
  getPackageName(): string
  getName(): string | null
  getLabel(): string | null
  getVersionName(): string | null
  getVersionCode(): number
  getNonLocalizedLabel(): string | null
  getDataDir(): string | null
  getSourceDir(): string | null
  getTargetSdkVersion(): number
  getMinSdkVersion(): number
  getUid(): number
}

// WebUI-X 文件输入流接口
interface FileInputInterfaceStream {
  read(): number
  readChunk(chunkSize: number): string
  close(): void
  skip(n: number): number
}

// WebUI-X PackageManager 接口
interface PackageManagerInterface {
  getPackageUid(packageName: string, flags: number, userId: number): number
  getApplicationIcon(
    packageName: string,
    flags: number,
    userId: number
  ): FileInputInterfaceStream | null
  getInstalledPackages(flags: number, userId: number): string
  getApplicationInfo(packageName: string, flags: number, userId: number): WXApplicationInfo
}

// 扩展 Window 接口
interface Window {
  $packageManager?: PackageManagerInterface
  wrapInputStream?: (stream: FileInputInterfaceStream) => Promise<{
    arrayBuffer: () => Promise<ArrayBuffer>
  }>
  ksu?: unknown
}

interface ObjectConstructor {
  hasOwn(o: object, v: PropertyKey): boolean
}
