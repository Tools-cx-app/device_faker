#!/usr/bin/env python3
"""
打包 Magisk 模块脚本
将 module/ 目录打包为可安装的 ZIP 文件
"""

import os
import zipfile
import datetime

def needs_lf_conversion(file_path):
    """检查文件是否需要转换为 LF 换行符"""
    with open(file_path, 'rb') as f:
        content = f.read()
    return b'\r\n' in content

def convert_to_lf(file_path, rel_path):
    """将文件转换为 LF 换行符（仅当需要时）"""
    if not needs_lf_conversion(file_path):
        print(f"  跳过 (已是 LF): {rel_path}")
        return
    
    with open(file_path, 'rb') as f:
        content = f.read()
    
    content = content.replace(b'\r\n', b'\n')
    
    with open(file_path, 'wb') as f:
        f.write(content)
    
    print(f"  转换: {rel_path}")

def create_magisk_module_zip():
    """创建 Magisk 模块 ZIP 包"""
    project_root = os.path.dirname(os.path.abspath(__file__))
    module_dir = os.path.join(project_root, "module")
    output_dir = os.path.join(project_root, "output")
    
    # 创建 output 目录（如果不存在）
    os.makedirs(output_dir, exist_ok=True)
    
    # 转换所有 .sh 文件为 LF 换行符
    print("正在处理 .sh 文件换行符...")
    for root, dirs, files in os.walk(module_dir):
        for file in files:
            if file.endswith('.sh'):
                file_path = os.path.join(root, file)
                rel_path = os.path.relpath(file_path, module_dir)
                convert_to_lf(file_path, rel_path)
    
    # 生成带时间戳的 ZIP 文件名
    timestamp = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
    zip_filename = f"device_faker_{timestamp}.zip"
    zip_path = os.path.join(output_dir, zip_filename)
    
    print(f"\n开始打包 Magisk 模块...")
    print(f"输出文件: {zip_filename}")
    
    with zipfile.ZipFile(zip_path, 'w', zipfile.ZIP_DEFLATED) as zipf:
        # 遍历 module 目录
        for root, dirs, files in os.walk(module_dir):
            for file in files:
                file_path = os.path.join(root, file)
                # 计算相对路径（相对于 module 目录）
                arcname = os.path.relpath(file_path, module_dir)
                
                zipf.write(file_path, arcname)
                print(f"  添加: {arcname}")
    
    # 获取文件大小
    file_size = os.path.getsize(zip_path)
    size_mb = file_size / (1024 * 1024)
    
    print(f"\n✅ 打包完成！")
    print(f"📦 文件: output/{zip_filename}")
    print(f"📏 大小: {size_mb:.2f} MB")
    print(f"\n请将此 ZIP 文件通过root管理器安装")

if __name__ == "__main__":
    create_magisk_module_zip()
