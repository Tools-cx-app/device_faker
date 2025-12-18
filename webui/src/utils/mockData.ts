// 开发环境的模拟数据
export const mockConfig = `
# 机型伪装配置文件
default_mode = "lite"

[templates.redmagic_9_pro]
packages = [
    "com.mobilelegends.mi",
    "com.supercell.brawlstars",
]
manufacturer = "ZTE"
brand = "nubia"
model = "NX769J"
device = "REDMAGIC 9 Pro"
product = "NX769J"
name = "NX769J"
fingerprint = "nubia/NX769J/NX769J:14/UKQ1.230917.001/20240813.173312:user/release-keys"

[templates.xiaomi_13_pro]
packages = [
    "com.levelinfinite.sgameGlobal",
]
manufacturer = "Xiaomi"
brand = "Xiaomi"
model = "2210132G"
device = "Xiaomi 13 Pro"
product = "2210132G"
name = "fuxi"
fingerprint = "Xiaomi/fuxi_eea/fuxi:13/TKQ1.221114.001/OS2.0.102.0.VMCEUXM:user/release-keys"

[[apps]]
package = "com.omarea.vtools"
manufacturer = "Xiaomi"
model = "2509FPN0BC"
brand = "Xiaomi"
device = "Xiaomi 15 Pro"
fingerprint = "Xiaomi/2509FPN0BC/Xiaomi15Pro:14/UP1A.231005.007/V816.0.3.0.VNBCNXM:user/release-keys"
`

export const mockModuleProp = `
id=device_faker
name=Device Faker
version=1.0.0
versionCode=100
author=Seyud
description=机型伪装模块
`

export const mockInstalledApps = [
  {
    packageName: 'com.android.chrome',
    appName: 'Chrome',
    versionName: '120.0.6099.144',
    versionCode: 609914400,
    installed: true,
  },
  {
    packageName: 'com.tencent.mm',
    appName: '微信',
    versionName: '8.0.40',
    versionCode: 2340,
    installed: true,
  },
  {
    packageName: 'com.tencent.mobileqq',
    appName: 'QQ',
    versionName: '9.0.0',
    versionCode: 9000,
    installed: true,
  },
  {
    packageName: 'com.mobilelegends.mi',
    appName: 'Mobile Legends',
    versionName: '1.8.20',
    versionCode: 18200,
    installed: true,
  },
  {
    packageName: 'com.supercell.brawlstars',
    appName: 'Brawl Stars',
    versionName: '51.170',
    versionCode: 51170,
    installed: true,
  },
  {
    packageName: 'com.levelinfinite.sgameGlobal',
    appName: 'PUBG Mobile',
    versionName: '2.9.0',
    versionCode: 29000,
    installed: true,
  },
  {
    packageName: 'com.omarea.vtools',
    appName: 'Scene',
    versionName: '5.6.1',
    versionCode: 561,
    installed: true,
  },
]
