use std::{ffi::CString, fs, os::unix::net::UnixStream};

use anyhow::{Context, Result};
use libc::MS_BIND;
use log::{error, info, warn};

use crate::companion::{
    CompanionRequest, CompanionResponse, send_companion_command, write_companion_response,
};
use crate::config::MergedAppConfig;
use zygisk_api::api::{V4, ZygiskApi};

const CPU_SPOOF_STATE_DIR: &str = "/data/local/tmp/device_faker_cpu";
const PROC_CPUINFO: &str = "/proc/cpuinfo";

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
/// 只负责 bind mount；不阻塞等待，也不手动 umount。
/// app 进程退出后，其 mount namespace 会自动销毁，bind mount 随之释放。
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

    // 使用 /data/local/tmp/ 作为源文件位置，避免 selinux 对 /data/adb/ 下文件的访问限制。
    let internal_path = format!("{CPU_SPOOF_STATE_DIR}/cpu_{pid}");
    fs::write(&internal_path, content)
        .with_context(|| format!("Failed to write internal cpuinfo file {internal_path}"))?;

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
            let _ = fs::remove_file(&internal_path);
            anyhow::bail!("mount failed: {err}");
        }
    }

    info!("Successfully mounted fake cpuinfo to {PROC_CPUINFO} for pid {pid}");

    Ok(())
}

fn ensure_dir(path: &str) -> Result<()> {
    fs::create_dir_all(path).with_context(|| format!("Failed to create directory {path}"))?;
    Ok(())
}
