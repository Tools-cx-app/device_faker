use std::{
    ffi::CString,
    fs,
    io::{Read, Write},
    os::unix::io::AsRawFd,
    os::unix::net::UnixStream,
    sync::atomic::{AtomicBool, AtomicI32, Ordering},
};

use anyhow::{Context, Result};
use libc::MS_BIND;
use log::{error, info, warn};

use crate::companion::{CompanionRequest, CompanionResponse, write_companion_response};
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
///
/// **Mount namespace 策略**：Zygisk companion（zygiskd）运行在 root mount namespace
/// （从 magiskd fork，不执行 setns）。/proc 在 Android 上是 MS_PRIVATE 传播，
/// root namespace 的 bind mount 不会传播到 app 的独立 namespace。
///
/// 因为 companion 可能是**多线程**的（线程池），直接调用 `setns(CLONE_NEWNS)`
/// 会返回 `EINVAL`（Linux 对多线程进程拒绝 CLONE_NEWNS）。
/// 所以 mount/unmount 操作通过 **fork 子进程** 完成：子进程在 fork 后是单线程的，
/// 可以安全调用 `setns` 切换到 app 的 mount namespace。
///
/// app 退出检测由 companion 侧的 `pidfd_open` + `poll()` 完成，
/// 与 socket fd 完全独立，无 fd 继承问题。
static LEAKED_FD: AtomicI32 = AtomicI32::new(-1);

// ---------------------------------------------------------------------------
// unshare PLT hook：检测 NeoZygisk 的 namespace 切换并即时 remount
// ---------------------------------------------------------------------------
//
// NeoZygisk 在 app specialize 时会 hook `unshare(CLONE_NEWNS)`，在真正的
// unshare 系统调用前执行 setns 切换到缓存 namespace，然后 old_unshare 创建新 namespace。
// 我们通过 Zygisk V4 的 plt_hook_register hook `unshare`，在 old_unshare 返回后
// （此时 app 已在新 namespace 中）直接执行 umount + bind mount，无需轮询。
//
// 对于不调用 unshare(CLONE_NEWNS) 的标准 Zygisk 实现，mount 子进程的
// monitor_and_remount 轮询作为 fallback 继续工作。
// ---------------------------------------------------------------------------

/// unshare hook 是否已初始化（源文件路径已设置）。
static UNSHARE_HOOK_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// unshare PLT hook 是否已注册（GOT entry 需要在 DlClose 前恢复）。
static HOOK_REGISTERED: AtomicBool = AtomicBool::new(false);

/// 原始 unshare 函数指针（通过 PLT hook 保存，用于恢复 GOT entry）。
static mut ORIGINAL_UNSHARE: Option<unsafe extern "C" fn(libc::c_int) -> libc::c_int> = None;

/// 泄漏的 CString，持有 fake cpuinfo 源文件路径。
/// 在 init_unshare_hook_state 中设置，永不 drop（hook 需要 'static 生命周期）。
static mut UNSHARE_SOURCE_CSTRING: Option<CString> = None;

/// fake cpuinfo 源文件的 C 字符串指针，由 UNSHARE_SOURCE_CSTRING 持有。
/// 仅在 UNSHARE_HOOK_INITIALIZED 为 true 后访问。
static mut UNSHARE_SOURCE_PTR: *const libc::c_char = std::ptr::null();

/// **Socket 生命周期**：`with_companion` 内部的 `companion_sock` 是局部变量，
/// 闭包返回后自动 drop 关闭 fd。因此我们在闭包内调用 `libc::dup()` 复制 fd，
/// 将副本存入 `LEAKED_FD`。原始 fd 随闭包结束关闭，副本保持打开。
/// 但注意：副本在 `apply_cpu_spoof` 中被**立即关闭**，不会泄漏到 app 进程。
/// app 退出检测由 companion 侧的 `pidfd_open` + `poll()` 完成。
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

    // 初始化 unshare hook 状态（源文件路径），供 hook 函数使用。
    init_unshare_hook_state(content);

    let request = CompanionRequest::CpuSpoof(crate::companion::CpuSpoofRequest {
        pid: std::process::id(),
        content: content.clone(),
    });

    // 发送请求并获取响应。
    // 现在 app 退出检测由 companion 侧的 pidfd + poll 完成，
    // 不再需要泄漏 socket fd。dup'd fd 在 companion 返回后立即关闭。
    let response = send_companion_command_leak_fd(api, &request)?;

    // 立即关闭泄漏的 fd，避免它被 fork 到 app 进程。
    // pidfd 方案完全独立于 socket，不需要这个 fd 存活。
    let leaked = LEAKED_FD.swap(-1, Ordering::SeqCst);
    if leaked >= 0 {
        unsafe { libc::close(leaked) };
        info!("Closed leaked companion fd {leaked} (pidfd handles monitoring)");
    }

    if response.status != 0 {
        anyhow::bail!(
            response
                .message
                .unwrap_or_else(|| "companion cpu spoof failed".to_string())
        );
    }

    // 注册 unshare PLT hook，在 specialize 期间检测 namespace 切换并即时 remount。
    register_unshare_hook(api)?;

    if debug {
        info!("CPU spoof applied successfully for {package_name}");
    }

    Ok(())
}

/// 与 `send_companion_command` 相同，但通过 `libc::dup()` 复制 socket fd。
///
/// `with_companion` 闭包返回后 `companion_sock`（局部变量）自动 drop 关闭原始 fd，
/// 因此必须用 `libc::dup()` 复制一份 fd 以保持 socket 打开直到响应读取完成。
/// 副本存入 `LEAKED_FD`，在 `apply_cpu_spoof` 中被立即关闭。
/// app 退出检测由 companion 侧的 pidfd + poll 完成，不依赖 socket 状态。
fn send_companion_command_leak_fd(
    api: &mut ZygiskApi<V4>,
    request: &CompanionRequest,
) -> anyhow::Result<CompanionResponse> {
    let payload = serde_json::to_vec(request)?;
    let response = api
        .with_companion(|stream| -> anyhow::Result<CompanionResponse> {
            stream.write_all(&(payload.len() as u32).to_le_bytes())?;
            stream.write_all(&payload)?;
            stream.flush()?;

            let mut len_buf = [0u8; 4];
            stream.read_exact(&mut len_buf)?;
            let resp_len = u32::from_le_bytes(len_buf) as usize;
            let mut resp_buf = vec![0u8; resp_len];
            stream.read_exact(&mut resp_buf)?;

            let resp = serde_json::from_slice::<CompanionResponse>(&resp_buf)?;

            // dup'd fd 仅用于保持 socket 打开直到响应读取完成，之后立即关闭。
            // app 退出检测由 companion 侧的 pidfd + poll 完成。
            let dup_fd = unsafe { libc::dup(stream.as_raw_fd()) };
            if dup_fd < 0 {
                anyhow::bail!("dup(fd) failed: {}", std::io::Error::last_os_error());
            }
            LEAKED_FD.store(dup_fd, Ordering::SeqCst);
            info!("Dup'd companion fd {dup_fd} (will close after response)");

            Ok(resp)
        })
        .map_err(|e| anyhow::anyhow!("Failed to talk to companion: {e}"))??;

    Ok(response)
}

// ---------------------------------------------------------------------------
// unshare PLT hook 实现
// ---------------------------------------------------------------------------

/// 初始化 unshare hook 的全局状态（源文件路径）。
/// 必须在 `register_unshare_hook` 之前调用，且在 app 进程中（非 companion）。
fn init_unshare_hook_state(source_path: &str) {
    // 安全：单线程环境（pre_app_specialize），写入后不再修改。
    unsafe {
        let cs = CString::new(source_path).expect("source_path contained NUL");
        UNSHARE_SOURCE_PTR = cs.as_ptr();
        UNSHARE_SOURCE_CSTRING = Some(cs);
        UNSHARE_HOOK_INITIALIZED.store(true, Ordering::Release);
    }
    info!("unshare hook state initialized: {source_path}");
}

/// 通过 Zygisk V4 PLT hook 注册 unshare 替身。
/// hook 在 `unshare(CLONE_NEWNS)` 返回后触发，在新 namespace 中执行 remount。
fn register_unshare_hook(api: &mut ZygiskApi<V4>) -> anyhow::Result<()> {
    let symbol = c"unshare";

    // NeoZygisk v4 API 拒绝 dev=0, inode=0（静默返回，不注册 hook）。
    // 必须找到调用 unshare 的库（libandroid_runtime.so）的真实 device 和 inode。
    // 注意：不能 hook libc.so（unshare 的定义者），因为 libc 的 PLT 没有 unshare 的
    // GOT entry。NeoZygisk 自己也是 hook libandroid_runtime.so 中的 unshare PLT entry。
    let (dev, ino) = find_lib_dev_ino("libandroid_runtime.so")
        .or_else(|| find_lib_dev_ino("libc.so"))
        .ok_or_else(|| anyhow::anyhow!("libandroid_runtime.so/libc.so not found in maps"))?;

    // 安全：plt_hook_register 要求 &mut *const () 作为 original 输出。
    // register 完成后 ORIGINAL_UNSHARE 被设置，之后仅在单线程 hook 中读取。
    unsafe {
        let mut original: *const () = std::ptr::null();
        api.plt_hook_register(dev, ino, symbol, unshare_hook as *const (), &mut original);
        api.plt_hook_commit()
            .map_err(|_| anyhow::anyhow!("plt_hook_commit failed for unshare"))?;

        ORIGINAL_UNSHARE = Some(std::mem::transmute::<
            *const (),
            unsafe extern "C" fn(libc::c_int) -> libc::c_int,
        >(original));
    }

    HOOK_REGISTERED.store(true, Ordering::Release);
    info!("unshare PLT hook registered (dev={dev:#x}, ino={ino})");
    Ok(())
}

/// 从 /proc/self/maps 解析指定库的 device 和 inode。
/// NeoZygisk v4 plt_hook_register 需要真实的 dev/ino，(0, 0) 会被静默拒绝。
fn find_lib_dev_ino(lib_name: &str) -> Option<(libc::dev_t, libc::ino_t)> {
    let maps = fs::read_to_string("/proc/self/maps").ok()?;
    for line in maps.lines() {
        if !line.contains(lib_name) {
            continue;
        }
        // 格式: "addr-addr perms offset dev inode path"
        // dev 是 "XX:XX" 格式的十六进制
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 6 {
            continue;
        }
        let dev_str = parts[3];
        let ino_str = parts[4];
        let mut dev_parts = dev_str.split(':');
        let major = u64::from_str_radix(dev_parts.next()?, 16).ok()?;
        let minor = u64::from_str_radix(dev_parts.next()?, 16).ok()?;
        let ino: u64 = ino_str.parse().ok()?;
        return Some(((major << 8 | minor) as libc::dev_t, ino as libc::ino_t));
    }
    None
}

/// unshare PLT 替身：调用原始 unshare，然后在新 namespace 中 remount。
///
/// NeoZygisk 的 unshare hook 会在 old_unshare 前执行 setns 到缓存 namespace，
/// 然后 old_unshare(CLONE_NEWNS) 创建新 namespace。我们的 hook 在 old_unshare
/// 返回后触发——此时 app 已在新 namespace 中，直接 umount + bind mount 即可。
///
/// 安全：此函数在 app specialize 的单线程环境中执行，libc 调用均安全。
unsafe extern "C" fn unshare_hook(flags: libc::c_int) -> libc::c_int {
    // 调用原始 unshare（链式调用 NeoZygisk 的 hook）
    let result = unsafe { ORIGINAL_UNSHARE.unwrap_unchecked()(flags) };

    // 仅在 CLONE_NEWNS 且 hook 状态已初始化时执行 remount
    if (flags & libc::CLONE_NEWNS) != 0
        && UNSHARE_HOOK_INITIALIZED.load(Ordering::Acquire)
        && result == 0
    {
        // 安全：unshare_hook 在 app specialize 的单线程环境中执行，
        // UNSHARE_SOURCE_PTR 由 init_unshare_hook_state 设置且持有 'static 引用。
        unsafe {
            let target = c"/proc/cpuinfo";

            // 防御性卸载（新 namespace 中可能有继承的挂载）
            libc::umount2(target.as_ptr(), libc::MNT_DETACH);

            // bind mount fake cpuinfo
            let ret = libc::mount(
                UNSHARE_SOURCE_PTR,
                target.as_ptr(),
                std::ptr::null(),
                MS_BIND,
                std::ptr::null(),
            );

            if ret == 0 {
                info!("unshare hook: remounted fake cpuinfo in new namespace");
            } else {
                warn!(
                    "unshare hook: remount failed: {}",
                    std::io::Error::last_os_error()
                );
            }
        }
    }

    result
}

// ---------------------------------------------------------------------------
// GOT entry 恢复：DlClose 前必须将 unshare 的 GOT entry 指回 libc 原始地址
// ---------------------------------------------------------------------------
//
// Zygisk 通过 PLT hook 将 unshare 的 GOT entry 从 libc 原始地址改为指向我们的
// hook 函数。DlCloseModuleLibrary 会 unmap 模块内存，但不会恢复 GOT entry，
// 导致 GOT 悬空指针 → app 后续调用 unshare 时 crash。
//
// 解决方案：在 post_app_specialize（DlClose 之前）解析 app 的 ELF，找到 unshare
// 的 JMP_SLOT relocation，将 GOT entry 恢复为 libc 中的真实地址。
// 使用 getauxval(AT_PHDR) 定位 ELF header，避免依赖 _DYNAMIC 符号。

// ELF 类型定义（aarch64 = Elf64_*，arm = Elf32_*）
#[cfg(target_arch = "aarch64")]
type ElfAddr = u64;
#[cfg(target_arch = "aarch64")]
type ElfWord = u32;
#[cfg(target_arch = "aarch64")]
type ElfXword = u64;
#[cfg(target_arch = "aarch64")]
type ElfSxword = i64;

#[cfg(target_arch = "arm")]
type ElfAddr = u32;
#[cfg(target_arch = "arm")]
type ElfWord = u32;
#[cfg(target_arch = "arm")]
type ElfXword = u32;
#[cfg(target_arch = "arm")]
type ElfSxword = i32;

#[repr(C)]
struct ElfDyn {
    d_tag: ElfSxword,
    d_val: ElfXword,
}

#[repr(C)]
struct ElfSym {
    st_name: ElfWord,
    #[cfg(target_arch = "aarch64")]
    st_info: u8,
    #[cfg(target_arch = "aarch64")]
    st_other: u8,
    #[cfg(target_arch = "aarch64")]
    st_shndx: u16,
    #[cfg(target_arch = "arm")]
    st_value: ElfAddr,
    #[cfg(target_arch = "arm")]
    st_size: ElfXword,
    #[cfg(target_arch = "aarch64")]
    st_value: ElfAddr,
    #[cfg(target_arch = "aarch64")]
    st_size: ElfXword,
    #[cfg(target_arch = "arm")]
    st_info: u8,
    #[cfg(target_arch = "arm")]
    st_other: u8,
    #[cfg(target_arch = "arm")]
    st_shndx: u16,
}

#[repr(C)]
struct ElfRela {
    r_offset: ElfAddr,
    r_info: ElfXword,
    r_addend: ElfSxword,
}

const PT_DYNAMIC: ElfWord = 2;

#[cfg(target_arch = "aarch64")]
const R_AARCH64_JUMP_SLOT: ElfXword = 1026;
#[cfg(target_arch = "arm")]
const R_ARM_JUMP_SLOT: ElfXword = 22;

#[cfg(target_arch = "aarch64")]
fn is_jump_slot(r_type: ElfXword) -> bool {
    r_type == R_AARCH64_JUMP_SLOT
}

#[cfg(target_arch = "arm")]
fn is_jump_slot(r_type: ElfXword) -> bool {
    r_type == R_ARM_JUMP_SLOT
}

/// 恢复 unshare 的 GOT entry 到 libc 原始地址。
///
/// 使用 dl_iterate_phdr 定位 libandroid_runtime.so 的 ELF，解析其 DT_JMPREL
/// relocation 表找到 unshare 的条目，将 GOT entry 写回 ORIGINAL_UNSHARE。
/// 必须在 DlCloseModuleLibrary 之前调用，否则 GOT 悬空指针导致 crash。
fn restore_unshare_got() {
    let Some(original_fn) = (unsafe { ORIGINAL_UNSHARE }) else {
        return;
    };

    // 1. 使用 dl_iterate_phdr 找到 libandroid_runtime.so 的 ELF base address
    let mut phdr_info: *const libc::dl_phdr_info = std::ptr::null();
    unsafe {
        libc::dl_iterate_phdr(
            Some(find_libandroid_runtime_cb),
            &mut phdr_info as *mut _ as *mut _,
        );
    }
    let Some(info) = (unsafe { phdr_info.as_ref() }) else {
        warn!("restore_unshare_got: libandroid_runtime.so not found via dl_iterate_phdr");
        return;
    };

    let base = info.dlpi_addr as usize;
    let phdr_ptr = info.dlpi_phdr;
    let phnum = info.dlpi_phnum as usize;

    // 2. 从 PT_DYNAMIC 获取 dynamic section 地址和大小
    let mut dyn_addr: *const ElfDyn = std::ptr::null();
    let mut dyn_size: usize = 0;
    for i in 0..phnum {
        let ph = unsafe { &*phdr_ptr.add(i) };
        if ph.p_type == PT_DYNAMIC {
            dyn_addr = (base + ph.p_vaddr as usize) as *const ElfDyn;
            dyn_size = ph.p_filesz as usize / std::mem::size_of::<ElfDyn>();
            break;
        }
    }
    if dyn_addr.is_null() {
        warn!("restore_unshare_got: PT_DYNAMIC not found in libandroid_runtime.so");
        return;
    }

    // 3. 解析 dynamic entries
    let mut symtab: *const ElfSym = std::ptr::null();
    let mut strtab: *const u8 = std::ptr::null();
    let mut jmprel: *const ElfRela = std::ptr::null();
    let mut jmprelsz: usize = 0;

    for i in 0..dyn_size {
        let entry = unsafe { &*dyn_addr.add(i) };
        match entry.d_tag {
            6 => symtab = (base + entry.d_val as usize) as *const ElfSym,
            5 => strtab = (base + entry.d_val as usize) as *const u8,
            23 => jmprel = (base + entry.d_val as usize) as *const ElfRela,
            2 => jmprelsz = entry.d_val as usize,
            0 => break,
            _ => {}
        }
    }

    if symtab.is_null() || strtab.is_null() || jmprel.is_null() || jmprelsz == 0 {
        warn!("restore_unshare_got: missing ELF dynamic entries in libandroid_runtime.so");
        return;
    }

    // 4. 遍历 JMPREL relocation 表，找到 unshare 的条目并恢复 GOT entry
    let num_relas = jmprelsz / std::mem::size_of::<ElfRela>();
    let original_ptr = original_fn as *const ();
    let mut restored = false;

    for i in 0..num_relas {
        let rela = unsafe { &*jmprel.add(i) };
        let r_type = rela.r_info & 0xffffffff;

        if !is_jump_slot(r_type) {
            continue;
        }

        let sym_idx = (rela.r_info >> 32) as usize;
        let sym = unsafe { &*symtab.add(sym_idx) };
        let name = unsafe {
            std::ffi::CStr::from_ptr(strtab.add(sym.st_name as usize) as *const libc::c_char)
        };

        if name.to_bytes() == b"unshare" {
            let got_entry = (base + rela.r_offset as usize) as *mut *const ();
            // 安全：GOT entry 是可写的（pre-app-specialize 时 lsplt 已修改过它）。
            unsafe {
                libc::mprotect(
                    ((base + rela.r_offset as usize) & !0xfff) as *mut libc::c_void,
                    0x1000,
                    libc::PROT_READ | libc::PROT_WRITE,
                );
                got_entry.write(original_ptr);
            }
            restored = true;
            info!("restore_unshare_got: restored GOT entry for unshare");
            break;
        }
    }

    if !restored {
        warn!("restore_unshare_got: unshare relocation not found in JMPREL");
    }
}

/// dl_iterate_phdr 回调：查找 libandroid_runtime.so 的 dl_phdr_info。
///
/// 使用 /proc/self/maps 中解析到的 device/inode 进行匹配（存储在
/// find_lib_dev_ino 的返回值中，通过 closure 捕获）。
/// 匹配成功后将 dl_phdr_info 指针写入 user_data。
///
/// # Safety
/// user_data 必须是指向 `*const libc::dl_phdr_info` 的有效指针。
unsafe extern "C" fn find_libandroid_runtime_cb(
    info: *mut libc::dl_phdr_info,
    _size: usize,
    user_data: *mut libc::c_void,
) -> libc::c_int {
    let info_ref = unsafe { &*info };
    // dlpi_name 为库的路径，检查是否包含 libandroid_runtime.so
    if !info_ref.dlpi_name.is_null() {
        let name = unsafe { std::ffi::CStr::from_ptr(info_ref.dlpi_name) };
        if let Ok(s) = name.to_str()
            && s.contains("libandroid_runtime.so")
        {
            unsafe {
                *(user_data as *mut *const libc::dl_phdr_info) = info;
            }
            return 1; // 找到，停止迭代
        }
    }
    0 // 继续迭代
}

/// CPU 伪装 unshare hook 的 DlClose 前清理。
///
/// 在 `post_app_specialize` 中、`DlCloseModuleLibrary` 之前调用。
/// 恢复 unshare 的 GOT entry 到 libc 原始地址，防止 DlClose 后 GOT 悬空指针。
pub fn cleanup_cpu_spoof_hook() {
    if HOOK_REGISTERED.load(Ordering::Acquire) {
        restore_unshare_got();
    }
}

/// Companion 进程入口：处理 CPU 伪装请求。
///
/// **进程退出检测方案：`pidfd_open` + `poll()`**
///
/// Companion 直接在当前线程阻塞等待 app 退出。
/// 每个 companion 连接是独立的，阻塞不影响其他 app 的 companion 请求。
/// 这彻底消除了 fork 模型中旧 watcher 与新 mount 之间的竞态条件。
///
/// `pidfd_open` 打开目标进程的 fd（Linux 5.3+），与 socket 完全独立。
/// `poll(pidfd)` 在进程退出时被内核唤醒（POLLIN），是真正的事件驱动。
/// 对旧内核回退到 /proc/<pid> 存在性检查。
pub fn handle_companion_cpu_spoof(
    stream: &mut UnixStream,
    request: crate::companion::CpuSpoofRequest,
) {
    // companion 进程不会调用 ZygiskModule::on_load，因此需要自行初始化日志。
    #[cfg(target_os = "android")]
    crate::file_logger::init();

    let pid = request.pid;
    info!(
        "Companion cpu_spoof handler entered, pid={pid}, self_pid={}",
        std::process::id()
    );

    let (setup_ok, mount_child_pid) = match do_cpu_spoof_setup(pid, &request.content) {
        Ok(child_pid) => (true, child_pid),
        Err(e) => {
            error!("CPU spoof setup failed for pid {pid}: {e}");
            let response = CompanionResponse::err(e.to_string());
            if let Err(e) = write_companion_response(stream, &response) {
                warn!("Failed to write CPU spoof response: {e}");
            }
            (false, -1)
        }
    };

    if setup_ok {
        // 发送 OK 给 module，让 app 继续启动。
        if let Err(e) = write_companion_response(stream, &CompanionResponse::ok()) {
            warn!("Failed to write CPU spoof response: {e}");
        }

        // 阻塞等待 app 退出。
        wait_for_app_exit(pid);

        // 通知 mount 子进程执行 umount 清理并退出。
        signal_mount_child_cleanup(mount_child_pid, pid);

        // 清理源文件
        let internal_path = format!("{CPU_SPOOF_STATE_DIR}/cpu_{pid}");
        if let Err(e) = fs::remove_file(&internal_path) {
            warn!("Failed to remove cpuinfo source {internal_path}: {e}");
        }
    }
}

/// 通知 mount 子进程执行 umount 清理：发送 SIGTERM 并等待其退出。
fn signal_mount_child_cleanup(child_pid: i32, app_pid: u32) {
    if child_pid <= 0 {
        return;
    }
    info!("Sending SIGTERM to mount child {child_pid} for app pid {app_pid}");
    unsafe { libc::kill(child_pid, libc::SIGTERM) };
    // 等待子进程退出，回收僵尸进程
    let mut status = 0i32;
    unsafe { libc::waitpid(child_pid, &mut status, 0) };
    info!("Mount child {child_pid} exited (app pid {app_pid})");
}

/// 使用 `pidfd_open` + `poll()` 阻塞等待目标进程退出。
///
/// `pidfd_open` (Linux 5.3+, Android API 31 所需内核版本) 返回一个专门的文件描述符，
/// 当目标进程退出时内核将其标记为可读。`poll()` 阻塞直到可读，是纯事件驱动。
/// 与 socket EOF 方案不同，pidfd 不涉及任何 fd 继承问题。
///
/// 对不支持 `pidfd_open` 的旧内核，自动回退到 `/proc/<pid>` 存在性检查。
fn wait_for_app_exit(pid: u32) {
    // 尝试 pidfd_open（syscall 434 on aarch64）
    let pidfd =
        unsafe { libc::syscall(libc::SYS_pidfd_open, pid as libc::pid_t, 0 as libc::c_uint) };

    if pidfd >= 0 {
        let pidfd = pidfd as i32;
        info!("Monitoring app pid {pid} via pidfd {pidfd} (event-driven)");

        loop {
            let mut pfd = libc::pollfd {
                fd: pidfd,
                events: libc::POLLIN,
                revents: 0,
            };
            let ret = unsafe { libc::poll(&mut pfd, 1, -1) };
            if ret > 0 {
                info!("pidfd signaled — app (pid {pid}) has exited");
                break;
            }
            if ret < 0 {
                let err = std::io::Error::last_os_error();
                if err.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                warn!("poll(pidfd) error for pid {pid}: {err}");
                break;
            }
        }

        unsafe { libc::close(pidfd) };
        return;
    }

    // Fallback：pidfd_open 不可用（旧内核），使用 /proc/<pid> 检查。
    let err = std::io::Error::last_os_error();
    warn!("pidfd_open failed for pid {pid}: {err}, falling back to procfs check");

    let proc_path = format!("/proc/{pid}");
    loop {
        std::thread::sleep(std::time::Duration::from_secs(2));
        if !std::path::Path::new(&proc_path).exists() {
            info!("App (pid {pid}) exited (procfs check)");
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// Fork + setns 架构
// ---------------------------------------------------------------------------
//
// Companion（zygiskd）可能是多线程的（线程池模型），直接调用 setns(CLONE_NEWNS)
// 在多线程进程中会返回 EINVAL（Linux 内核限制）。解决方案是 fork 子进程——
// fork 后子进程是单线程的，可以安全调用 setns。
//
// 子进程进入 app 的 mount namespace 后**常驻等待**（不立即退出），以保持
// namespace 引用并确保 bind mount 在 app 整个生命周期内有效。
// Companion 线程等待 app 退出后通过 SIGTERM 通知子进程执行 umount 清理。
// ---------------------------------------------------------------------------

/// 执行 CPU 伪装的 setup：写入源文件、fork 子进程进入 app namespace 并挂载。
/// 返回子进程 pid，调用者在 app 退出后应通知子进程清理。
fn do_cpu_spoof_setup(pid: u32, content: &str) -> Result<i32> {
    ensure_dir(CPU_SPOOF_STATE_DIR)?;
    set_selinux_context(CPU_SPOOF_STATE_DIR);

    let internal_path = format!("{CPU_SPOOF_STATE_DIR}/cpu_{pid}");
    fs::write(&internal_path, content)
        .with_context(|| format!("Failed to write internal cpuinfo file {internal_path}"))?;
    set_selinux_context(&internal_path);

    // 通过 fork+pipe 将 mount 操作委派给子进程（子进程是单线程，可安全 setns）。
    let result = fork_mount_child(pid, &internal_path);

    match &result {
        Ok(child_pid) => {
            info!("Successfully mounted fake cpuinfo for pid {pid} (child_pid={child_pid})")
        }
        Err(e) => {
            error!("Mount operation failed for pid {pid}: {e}");
            let _ = fs::remove_file(&internal_path);
        }
    }

    result
}

/// fork 子进程：setns 进入 app namespace → bind mount → 常驻等待。
///
/// 子进程通过 pipe 报告挂载结果后**不退出**，保持 namespace 引用。
/// 父进程（companion 线程）返回子进程 pid；app 退出后发送 SIGTERM 通知清理。
///
/// Pipe 协议：
/// - 成功：写 4 字节 `0i32`
/// - 失败：写 4 字节 `-1i32` + 4 字节 msg_len + UTF-8 错误消息
fn fork_mount_child(pid: u32, source_path: &str) -> Result<i32> {
    let mut pipe_fds = [0i32; 2];
    if unsafe { libc::pipe(pipe_fds.as_mut_ptr()) } != 0 {
        anyhow::bail!("pipe failed: {}", std::io::Error::last_os_error());
    }
    let read_fd = pipe_fds[0];
    let write_fd = pipe_fds[1];

    match unsafe { libc::fork() } {
        -1 => {
            unsafe {
                libc::close(read_fd);
                libc::close(write_fd);
            }
            anyhow::bail!("fork failed: {}", std::io::Error::last_os_error());
        }
        0 => {
            // === 子进程（单线程，可安全 setns）===
            unsafe { libc::close(read_fd) };
            let status = do_mount_in_child(pid, source_path);
            match status {
                Ok(()) => {
                    let code: i32 = 0;
                    unsafe {
                        libc::write(
                            write_fd,
                            &code as *const i32 as *const libc::c_void,
                            std::mem::size_of::<i32>(),
                        )
                    };
                }
                Err(e) => {
                    let msg = e.to_string();
                    let code: i32 = -1;
                    let msg_bytes = msg.as_bytes();
                    let msg_len = msg_bytes.len() as i32;
                    unsafe {
                        libc::write(
                            write_fd,
                            &code as *const i32 as *const libc::c_void,
                            std::mem::size_of::<i32>(),
                        );
                        libc::write(
                            write_fd,
                            &msg_len as *const i32 as *const libc::c_void,
                            std::mem::size_of::<i32>(),
                        );
                        libc::write(
                            write_fd,
                            msg_bytes.as_ptr() as *const libc::c_void,
                            msg_bytes.len(),
                        );
                        libc::close(write_fd);
                        libc::_exit(1);
                    }
                }
            }
            // 挂载成功：关闭 pipe 写端，然后监控 namespace 变化
            unsafe { libc::close(write_fd) };

            // 注册 SIGTERM handler
            unsafe {
                libc::signal(
                    libc::SIGTERM,
                    child_sigterm_handler as *const () as libc::sighandler_t,
                );
            }

            // 监控 namespace 变化：某些 Zygisk 实现（NeoZygisk）会在 unshare hook
            // 后切换 app 的 namespace，此时需要在新 namespace 中重新 mount。
            monitor_and_remount(pid, source_path);

            // 不可达（monitor_and_remount 内循环，只在 SIGTERM 时通过 handler 退出）
            unsafe { libc::_exit(0) }
        }
        child_pid => {
            // === 父进程（companion 线程）===
            unsafe { libc::close(write_fd) };

            // 读取结果码
            let mut code: i32 = -1;
            let n = unsafe {
                libc::read(
                    read_fd,
                    &mut code as *mut i32 as *mut libc::c_void,
                    std::mem::size_of::<i32>(),
                )
            };
            if n != std::mem::size_of::<i32>() as isize {
                unsafe { libc::close(read_fd) };
                let mut status = 0i32;
                unsafe { libc::waitpid(child_pid, &mut status, 0) };
                anyhow::bail!("Failed to read mount result from child (read {n} bytes)");
            }

            if code != 0 {
                // 读取错误消息
                let mut msg_len: i32 = 0;
                let n = unsafe {
                    libc::read(
                        read_fd,
                        &mut msg_len as *mut i32 as *mut libc::c_void,
                        std::mem::size_of::<i32>(),
                    )
                };
                let err_msg = if n == std::mem::size_of::<i32>() as isize && msg_len > 0 {
                    let mut buf = vec![0u8; msg_len as usize];
                    unsafe {
                        libc::read(
                            read_fd,
                            buf.as_mut_ptr() as *mut libc::c_void,
                            msg_len as usize,
                        )
                    };
                    String::from_utf8_lossy(&buf).to_string()
                } else {
                    format!("error code {code}")
                };
                unsafe { libc::close(read_fd) };
                let mut status = 0i32;
                unsafe { libc::waitpid(child_pid, &mut status, 0) };
                anyhow::bail!("Mount child failed: {err_msg}");
            }

            unsafe { libc::close(read_fd) };
            // 子进程仍在运行（等待 SIGTERM），返回其 pid
            Ok(child_pid)
        }
    }
}

/// 子进程的 SIGTERM handler：收到信号后 umount 并退出。
extern "C" fn child_sigterm_handler(_sig: libc::c_int) {
    let _ = CString::new(PROC_CPUINFO).map(|target| {
        unsafe { libc::umount2(target.as_ptr(), libc::MNT_DETACH) };
    });
    unsafe { libc::_exit(0) };
}

/// 在 fork 子进程中执行 setns + bind mount。
/// namespace 变化由 monitor_and_remount 处理。
fn do_mount_in_child(pid: u32, source_path: &str) -> Result<()> {
    let ns_path = format!("/proc/{pid}/ns/mnt");
    let ns_path_c = CString::new(ns_path.as_str())?;

    let initial_ino = read_ns_ino(pid)?;

    // setns 到 app 当前的 namespace
    let ns_fd = unsafe { libc::open(ns_path_c.as_ptr(), libc::O_RDONLY) };
    if ns_fd < 0 {
        anyhow::bail!(
            "Failed to open {}: {}",
            ns_path,
            std::io::Error::last_os_error()
        );
    }

    let ret = unsafe {
        libc::syscall(
            libc::SYS_setns,
            ns_fd as libc::c_long,
            libc::CLONE_NEWNS as libc::c_long,
        )
    };
    if ret != 0 {
        let err = std::io::Error::last_os_error();
        unsafe { libc::close(ns_fd) };
        anyhow::bail!("setns failed for pid {pid}: {err}");
    }
    info!("[child] Entered NS of pid {pid} (ino={initial_ino})");

    // 防御性卸载
    let target = CString::new(PROC_CPUINFO)?;
    unsafe {
        libc::umount2(target.as_ptr(), libc::MNT_DETACH);
    }

    // bind mount
    let source = CString::new(source_path)?;
    let ret = unsafe {
        libc::mount(
            source.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            MS_BIND,
            std::ptr::null(),
        )
    };
    if ret != 0 {
        let err = std::io::Error::last_os_error();
        anyhow::bail!("bind mount failed: {err}");
    }

    info!("[child] Mounted fake cpuinfo for pid {pid} (ns_ino={initial_ino})");

    // 验证
    match fs::read_to_string(PROC_CPUINFO) {
        Ok(actual) if !actual.is_empty() => {
            info!("[child] Verified for pid {pid} ({} bytes)", actual.len());
        }
        Ok(_) => warn!("[child] /proc/cpuinfo empty for pid {pid}"),
        Err(e) => warn!("[child] Read failed for pid {pid}: {e}"),
    }

    Ok(())
}

/// 监控 app 的 mount namespace 变化，如果 namespace 被切换（如 NeoZygisk 的
/// unshare hook），在新 namespace 中重新执行 bind mount。
/// 循环直到 SIGTERM 或 app 退出。
fn monitor_and_remount(pid: u32, source_path: &str) {
    let mut current_ino = match read_ns_ino(pid) {
        Ok(ino) => ino,
        Err(_) => return,
    };

    info!("[child] Monitoring NS for pid {pid} (initial ino={current_ino})");

    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));

        // 检查 app 是否还活着
        let proc_path = format!("/proc/{pid}");
        if !std::path::Path::new(&proc_path).exists() {
            info!("[child] App pid {pid} exited, stopping monitor");
            return;
        }

        // 检查 namespace 是否变化
        match read_ns_ino(pid) {
            Ok(new_ino) if new_ino != current_ino => {
                info!("[child] NS changed for pid {pid}: {current_ino} -> {new_ino}");

                // 进入新 namespace
                let ns_path = format!("/proc/{pid}/ns/mnt");
                let Ok(ns_path_c) = CString::new(ns_path.as_str()) else {
                    continue;
                };
                let ns_fd = unsafe { libc::open(ns_path_c.as_ptr(), libc::O_RDONLY) };
                if ns_fd < 0 {
                    warn!("[child] Cannot open new NS for pid {pid}");
                    continue;
                }

                let ret = unsafe {
                    libc::syscall(
                        libc::SYS_setns,
                        ns_fd as libc::c_long,
                        libc::CLONE_NEWNS as libc::c_long,
                    )
                };
                if ret != 0 {
                    warn!(
                        "[child] setns to new NS failed for pid {pid}: {}",
                        std::io::Error::last_os_error()
                    );
                    unsafe { libc::close(ns_fd) };
                    continue;
                }
                // 不关闭 ns_fd，保持 namespace 引用

                // 防御性卸载 + 重新 mount
                let target = match CString::new(PROC_CPUINFO) {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                unsafe { libc::umount2(target.as_ptr(), libc::MNT_DETACH) };

                let source = match CString::new(source_path) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let ret = unsafe {
                    libc::mount(
                        source.as_ptr(),
                        target.as_ptr(),
                        std::ptr::null(),
                        MS_BIND,
                        std::ptr::null(),
                    )
                };
                if ret == 0 {
                    info!("[child] Re-mounted in new NS for pid {pid} (ino={new_ino})");
                } else {
                    warn!(
                        "[child] Re-mount failed for pid {pid}: {}",
                        std::io::Error::last_os_error()
                    );
                }

                current_ino = new_ino;
            }
            Ok(_) => {}       // namespace 没变，继续监控
            Err(_) => return, // 无法读取 namespace
        }
    }
}

/// 读取 /proc/{pid}/ns/mnt 的 namespace 标识（通过 readlink 获取 mnt:[inode]）。
/// stat() 返回的是 procfs 条目的 inode（固定不变），必须用 readlink 获取真正的 namespace ID。
fn read_ns_ino(pid: u32) -> Result<u64> {
    let ns_path = format!("/proc/{pid}/ns/mnt");
    let ns_path_c = CString::new(ns_path.as_str())?;
    let mut buf = [0u8; 64];
    let len = unsafe {
        libc::readlink(
            ns_path_c.as_ptr(),
            buf.as_mut_ptr() as *mut libc::c_char,
            buf.len() - 1,
        )
    };
    if len < 0 {
        anyhow::bail!(
            "readlink({}) failed: {}",
            ns_path,
            std::io::Error::last_os_error()
        );
    }
    buf[len as usize] = 0;
    let link = std::str::from_utf8(&buf[..len as usize])
        .map_err(|_| anyhow::anyhow!("invalid utf8 in ns link"))?;
    // 格式: "mnt:[4026535831]"
    let ino_str = link
        .strip_prefix("mnt:[")
        .and_then(|s| s.strip_suffix(']'))
        .ok_or_else(|| anyhow::anyhow!("unexpected ns link format: {link}"))?;
    ino_str
        .parse::<u64>()
        .map_err(|_| anyhow::anyhow!("cannot parse ns ino: {ino_str}"))
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
