use std::{
    ffi::CString,
    fs,
    os::unix::net::UnixStream,
    path::Path,
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use libc::{MNT_DETACH, MS_BIND};
use log::{error, info, warn};

use crate::companion::{
    CompanionRequest, CompanionResponse, send_companion_command, write_companion_response,
};
use crate::config::MergedAppConfig;
use zygisk_api::api::{V4, ZygiskApi};

// bind mount 的源文件放在 /data/adb/device_faker/cpu/ 下。
// 之所以不放 /data/local/tmp/ 是为了规避检测：部分检测器（如 Duck-Detector 的
// ShellTmpConcealmentProbe）会扫描 /proc/self/mountinfo，对挂载点落在
// /data/local/tmp 及其子路径下的挂载报 "Shell tmp dedicated mount" 风险。
// 放到 /data/adb/ 下不会触发该检测（参考 cpuwz 模块的实现）。
//
// SELinux 关键点：bind mount 之后 app 读 /proc/cpuinfo 时，内核在 VFS 层把
// 路径解析到源文件的 inode，SELinux 检查的是**源文件 inode 的 label**，
// 而非 mount point 的 label。/data/adb/device_faker/ 目录的默认 label
// （adb_data_file:s0 等）untrusted_app 无权读取，会导致 app open(/proc/cpuinfo)
// 返回 EACCES。因此 companion 创建目录和源文件后必须把它们的 label 改成
// app 可读的 system_file:s0（与 customize.sh 对 config 文件的处理一致）。
// cpuwz 之所以不需要这一步，是因为它的源文件是模块安装时的静态文件，
// 已被 Magisk/KSU 框架的 set_perm_recursive 赋予了可读 label。
const CPU_SPOOF_STATE_DIR: &str = "/data/adb/device_faker/cpu";
const PROC_CPUINFO: &str = "/proc/cpuinfo";
// app 可读的 SELinux label，与 customize.sh 对 config 文件设置的一致。
const SELINUX_CONTEXT: &str = "u:object_r:system_file:s0";

/// 在 app specialize 时触发 CPU 伪装。
/// 通过 companion 进程在目标应用的 mount namespace 中执行 bind mount。
/// Zygisk 框架保证 companion 进程已经位于目标进程的 mount namespace 中，
/// 因此不需要手动 setns。
pub fn apply_cpu_spoof(
    api: &mut ZygiskApi<V4>,
    merged: &MergedAppConfig,
    package_name: &str,
    debug: bool,
) -> anyhow::Result<()> {
    let Some(content) = &merged.cpuinfo_content else {
        return Ok(());
    };

    if content.is_empty() {
        return Ok(());
    }

    if debug {
        info!("Applying CPU spoof for {package_name}");
    }

    let request = CompanionRequest::CpuSpoof(crate::companion::CpuSpoofRequest {
        pid: std::process::id(),
        content: content.clone(),
    });

    let response = send_companion_command(api, &request)?;
    if response.status != 0 {
        anyhow::bail!(
            response
                .message
                .unwrap_or_else(|| "companion cpu spoof failed".to_string())
        );
    }

    if debug {
        info!("CPU spoof applied successfully for {package_name}");
    }

    Ok(())
}

/// Companion 进程入口：处理 CPU 伪装请求。
/// 负责 bind mount，并 fork 一个独立子进程在 app 退出后 umount + 清理源文件。
/// companion 本身不阻塞等待 app 退出——socket 在 pre_app_specialize 后同步关闭，
/// umount 逻辑由独立 daemon 子进程承担。
pub fn handle_companion_cpu_spoof(
    stream: &mut UnixStream,
    request: crate::companion::CpuSpoofRequest,
) {
    // companion 进程不会调用 ZygiskModule::on_load，因此需要自行初始化日志。
    #[cfg(target_os = "android")]
    crate::file_logger::init();

    let response = match do_cpu_spoof_setup(request.pid, &request.content) {
        Ok(()) => CompanionResponse::ok(),
        Err(e) => {
            error!("CPU spoof setup failed: {e}");
            CompanionResponse::err(e.to_string())
        }
    };

    if let Err(e) = write_companion_response(stream, &response) {
        warn!("Failed to write CPU spoof response: {e}");
    }
}

fn do_cpu_spoof_setup(pid: u32, content: &str) -> Result<()> {
    ensure_dir(CPU_SPOOF_STATE_DIR)?;
    // 目录必须 app 可读，否则其下新建文件的 label 继承也可能受影响。
    set_selinux_context(CPU_SPOOF_STATE_DIR);

    let internal_path = format!("{CPU_SPOOF_STATE_DIR}/cpu_{pid}");
    fs::write(&internal_path, content)
        .with_context(|| format!("Failed to write internal cpuinfo file {internal_path}"))?;
    // 源文件的 label 决定 app 读 /proc/cpuinfo（bind 后解析到此 inode）的 SELinux 判定，
    // 必须在 mount 之前设好。
    set_selinux_context(&internal_path);

    unsafe {
        let source = CString::new(internal_path.as_str())?;
        let target = CString::new(PROC_CPUINFO)?;
        let ret = libc::mount(
            source.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            MS_BIND,
            std::ptr::null(),
        );

        if ret != 0 {
            let err = std::io::Error::last_os_error();
            // 挂载失败时立即清理源文件，避免残留。
            let _ = fs::remove_file(&internal_path);
            anyhow::bail!("mount failed: {err}");
        }
    }

    info!("Successfully mounted fake cpuinfo to {PROC_CPUINFO} for pid {pid}");

    // 挂载成功后 fork 一个独立子进程：app 退出时 umount 并删除源文件。
    // 这样 /proc/self/mountinfo 中的 bind mount 痕迹会随 app 退出立即消失，
    // 同时清理 cpu_<pid> 源文件。fork 在 companion 写响应之前完成，
    // 故 companion 仍能同步返回，不阻塞 socket。
    if let Err(e) = spawn_cpu_umount_watcher(pid, internal_path.clone()) {
        warn!("Failed to spawn CPU umount watcher for pid {pid}: {e}");
        warn!(
            "Bind mount will be released by namespace destruction on app exit, \
             but the source file will not be cleaned up automatically"
        );
    }

    Ok(())
}

/// Fork 一个独立 daemon 子进程，轮询 `/proc/<pid>` 是否存在。
/// 进程消失（app 退出）后 umount /proc/cpuinfo 并删除源文件。
///
/// 复用 resetprop watcher（`companion.rs::spawn_restore_watcher`）的
/// fork + setsid 模式：watcher 脱离 companion 进程组独立运行，
/// 不持有 companion socket，不影响 companion 同步返回。
fn spawn_cpu_umount_watcher(pid: u32, internal_path: String) -> Result<()> {
    unsafe {
        match libc::fork() {
            -1 => anyhow::bail!("fork failed: {}", std::io::Error::last_os_error()),
            0 => {
                if libc::setsid() == -1 {
                    libc::_exit(1);
                }
                if let Err(e) = watch_and_cleanup(pid, &internal_path) {
                    error!("CPU umount watcher failed for pid {pid}: {e}");
                }
                libc::_exit(0);
            }
            _ => Ok(()),
        }
    }
}

fn watch_and_cleanup(pid: u32, internal_path: &str) -> Result<()> {
    const POLL_INTERVAL: Duration = Duration::from_millis(200);
    // 设置一个上限，避免异常情况下 watcher 永久存活（如 /proc 被遮挡导致误判）。
    // 1 小时后强制清理退出。
    const MAX_LIFETIME: Duration = Duration::from_secs(3600);
    let proc_path = format!("/proc/{pid}");
    let deadline = Instant::now() + MAX_LIFETIME;

    loop {
        if !Path::new(&proc_path).exists() {
            break;
        }
        if Instant::now() > deadline {
            warn!("CPU umount watcher reached max lifetime for pid {pid}, forcing cleanup");
            break;
        }
        thread::sleep(POLL_INTERVAL);
    }

    // app 已退出（或超时）：umount 并清理源文件。
    // umount2 + MNT_DETACH：lazy detach，立即从挂载层级移除，
    // 已有 fd 引用仍可继续读直到关闭。
    let target = CString::new(PROC_CPUINFO)?;
    let ret = unsafe { libc::umount2(target.as_ptr(), MNT_DETACH) };
    if ret != 0 {
        let err = std::io::Error::last_os_error();
        // umount 失败通常意味着 mount 已随 namespace 销毁而消失，仅记录。
        warn!("umount2 /proc/cpuinfo failed (may already be gone): {err}");
    } else {
        info!("umounted fake cpuinfo for pid {pid}");
    }

    if let Err(e) = fs::remove_file(internal_path) {
        warn!("Failed to remove cpuinfo source {internal_path}: {e}");
    }

    Ok(())
}

fn ensure_dir(path: &str) -> Result<()> {
    fs::create_dir_all(path).with_context(|| format!("Failed to create directory {path}"))?;
    Ok(())
}

/// 把给定路径的 SELinux label 设为 app 可读的 system_file:s0。
///
/// bind mount 后 app 读 /proc/cpuinfo 时，内核在 VFS 层把路径解析到源文件 inode，
/// SELinux 检查的是**源文件 inode 的 label**。/data/adb/device_faker/ 下的文件默认
/// label（adb_data_file:s0 等）untrusted_app 无权读取，会返回 EACCES，导致 app 读
/// 不到伪装后的 cpuinfo。必须把目录和源文件都改成 system_file:s0（与 customize.sh
/// 对 config 文件的处理一致）。
///
/// 失败时仅记录警告而非中断：在某些 root 实现下 lsetxattr 可能被策略限制，此时退回
/// 默认 label；最坏情况是 app 读不到 cpuinfo（与不修复无异），但不影响 mount 本身。
fn set_selinux_context(path: &str) {
    let result = (|| -> std::io::Result<()> {
        let p = CString::new(path).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "path contained nul")
        })?;
        let ctx = CString::new(SELINUX_CONTEXT).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "context contained nul")
        })?;
        // flags = 0：若属性已存在则覆盖，不存在则创建（create-or-replace）。
        let ret = unsafe {
            libc::lsetxattr(
                p.as_ptr(),
                c"security.selinux".as_ptr() as *const _,
                ctx.as_ptr() as *const libc::c_void,
                SELINUX_CONTEXT.len(), // 不含末尾 nul
                0,
            )
        };
        if ret != 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    })();

    match result {
        Ok(()) => {
            #[cfg(target_os = "android")]
            info!("Set SELinux context {SELINUX_CONTEXT} on {path}");
        }
        Err(e) => {
            // 不致命：记录后继续，mount 仍可完成；最坏 app 读不到 cpuinfo。
            warn!("Failed to set SELinux context on {path}: {e} (app may not read cpuinfo)");
        }
    }
}
