mod config;

use config::Config;
use log::{error, info};
use std::collections::HashMap;
use std::sync::Mutex;
use zygisk_rs::{register_zygisk_module, Api, AppSpecializeArgs, Module, ServerSpecializeArgs};

// 全局状态:存储当前应用的伪装属性
static FAKE_PROPS: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

// 全局状态:标记是否使用了 full 模式（需要保持模块加载）
static IS_FULL_MODE: Mutex<bool> = Mutex::new(false);

// 原始 native_get 函数指针类型
type OriginalNativeGet = unsafe extern "C" fn(
    env: *mut jni_sys::JNIEnv,
    class: jni_sys::jclass,
    key: jni_sys::jstring,
    def: jni_sys::jstring,
) -> jni_sys::jstring;

// 存储原始 native_get 函数指针
static ORIGINAL_NATIVE_GET: Mutex<Option<OriginalNativeGet>> = Mutex::new(None);

struct MyModule {
    api: Api,
    env: *mut jni_sys::JNIEnv, // 直接保存原始 JNI 环境指针
}

impl Module for MyModule {
    fn new(api: Api, env: *mut jni_sys::JNIEnv) -> Self {
        // 默认只记录错误，减少日志输出以提高隐蔽性
        // 可以通过配置文件的 debug = true 启用详细日志
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Error) // 默认只记录错误
                .with_tag("DeviceFaker"),
        );

        Self { api, env }
    }
    fn pre_app_specialize(&mut self, args: &mut AppSpecializeArgs) {
        let mut inner = || -> anyhow::Result<()> {
            // 获取包名 - 使用原始 JNI 函数
            let package_name = unsafe {
                let jni_funcs = (**self.env).v1_4;
                let jstring = *args.nice_name;
                let chars = (jni_funcs.GetStringUTFChars)(self.env, jstring, std::ptr::null_mut());
                if chars.is_null() {
                    return Err(anyhow::anyhow!("Failed to get package name"));
                }
                let pkg = std::ffi::CStr::from_ptr(chars)
                    .to_string_lossy()
                    .to_string();
                (jni_funcs.ReleaseStringUTFChars)(self.env, jstring, chars);
                pkg
            };

            // 读取配置文件
            let config_path = "/data/adb/device_faker/config/config.toml";

            // 检查文件是否存在
            if !std::path::Path::new(config_path).exists() {
                self.unload_module();
                return Ok(());
            }

            let config_content = match std::fs::read_to_string(config_path) {
                Ok(content) => content,
                Err(e) => {
                    error!("Failed to read config: {}", e);
                    self.unload_module();
                    return Ok(());
                }
            };

            let config = Config::from_toml(&config_content)?;

            // 根据配置启用或禁用详细日志
            if config.debug {
                log::set_max_level(log::LevelFilter::Info);
                info!("Debug mode enabled");
                info!(
                    "Config loaded with {} apps and {} templates",
                    config.apps.len(),
                    config.templates.len()
                );
            }

            // 获取合并后的配置（会同时查找直接配置和模板配置）
            let merged_config = match config.get_merged_config(&package_name) {
                Some(cfg) => cfg,
                None => {
                    // 应用不在配置中
                    if config.debug {
                        info!("App {} not in config, unloading module", package_name);
                    }
                    self.unload_module();
                    return Ok(());
                }
            };

            // 获取工作模式
            let mode = &merged_config.mode;
            if config.debug {
                info!("Using mode: {} for app: {}", mode, package_name);
            }

            // Hook Build 类字段（两种模式都需要）
            self.hook_build_fields(&merged_config)?;
            if config.debug {
                info!("Build fields hooked successfully");
            }

            // 根据模式决定是否 Hook SystemProperties
            match mode.as_str() {
                "lite" => {
                    // 轻量模式：只修改 Build 类，完成后卸载模块
                    if config.debug {
                        info!("Lite mode: only Build fields hooked, unloading module");
                    }
                    *IS_FULL_MODE.lock().unwrap() = false;
                    self.unload_module();
                }
                "full" => {
                    // 完整模式：Hook Build + SystemProperties
                    if config.debug {
                        info!("Full mode: hooking SystemProperties");
                    }

                    // 标记为 full 模式，防止模块被卸载
                    *IS_FULL_MODE.lock().unwrap() = true;

                    // 构建属性映射表并存储
                    let prop_map = Config::build_merged_property_map(&merged_config);
                    if config.debug {
                        info!("Property map created with {} entries", prop_map.len());
                    }
                    *FAKE_PROPS.lock().unwrap() = Some(prop_map);

                    // Hook SystemProperties
                    self.hook_system_properties()?;
                    if config.debug {
                        info!("SystemProperties hooked successfully, module will stay loaded");
                    }
                }
                _ => {
                    // 未知模式,回退到默认的 lite 模式
                    error!(
                        "Mode '{}' not fully supported, falling back to 'lite' mode",
                        mode
                    );
                    if config.debug {
                        info!("Lite mode (fallback): only Build fields hooked, unloading module");
                    }
                    *IS_FULL_MODE.lock().unwrap() = false;
                    self.unload_module();
                }
            }

            Ok(())
        };

        if let Err(e) = inner() {
            error!("Error: {:?}", e);
        }
    }

    fn post_app_specialize(&mut self, _args: &AppSpecializeArgs) {
        // 只在非 full 模式下卸载模块
        // full 模式下，Hook 函数需要保持在内存中，否则会导致崩溃
        let is_full_mode = *IS_FULL_MODE.lock().unwrap();
        if !is_full_mode {
            // lite 模式：可以安全卸载
            self.unload_module();
        }
        // full 模式：不卸载，保持 Hook 函数可用
    }

    fn pre_server_specialize(&mut self, _args: &mut ServerSpecializeArgs) {
        self.unload_module();
    }

    fn post_server_specialize(&mut self, _args: &ServerSpecializeArgs) {}
}

impl MyModule {
    /// 卸载模块的辅助函数
    #[inline]
    fn unload_module(&mut self) {
        self.api
            .set_option(zygisk_rs::ModuleOption::DlcloseModuleLibrary);
    }

    /// Hook Build 类的静态字段
    fn hook_build_fields(&mut self, merged_config: &config::MergedAppConfig) -> anyhow::Result<()> {
        unsafe {
            let jni_funcs = (**self.env).v1_4;

            // 查找 android.os.Build 类
            let class_name = std::ffi::CString::new("android/os/Build")?;
            let build_class = (jni_funcs.FindClass)(self.env, class_name.as_ptr());
            if build_class.is_null() {
                return Err(anyhow::anyhow!("Failed to find Build class"));
            }

            // 只修改配置中存在的字段
            if let Some(manufacturer) = &merged_config.manufacturer {
                self.set_build_field_raw(build_class, "MANUFACTURER", manufacturer)?;
            }

            if let Some(brand) = &merged_config.brand {
                self.set_build_field_raw(build_class, "BRAND", brand)?;
            }

            if let Some(model) = &merged_config.model {
                self.set_build_field_raw(build_class, "MODEL", model)?;
            }

            if let Some(device) = &merged_config.device {
                self.set_build_field_raw(build_class, "DEVICE", device)?;
            }

            if let Some(product) = &merged_config.product {
                self.set_build_field_raw(build_class, "PRODUCT", product)?;
            }

            if let Some(fingerprint) = &merged_config.fingerprint {
                self.set_build_field_raw(build_class, "FINGERPRINT", fingerprint)?;
            }
        }

        Ok(())
    }

    /// 设置 Build 类的字段值 - 使用原始 JNI 调用
    fn set_build_field_raw(
        &mut self,
        build_class: jni_sys::jclass,
        field_name: &str,
        value: &str,
    ) -> anyhow::Result<()> {
        unsafe {
            let jni_funcs = (**self.env).v1_4;

            // 获取字段 ID
            let field_name_cstr = std::ffi::CString::new(field_name)?;
            let signature_cstr = std::ffi::CString::new("Ljava/lang/String;")?;
            let field_id = (jni_funcs.GetStaticFieldID)(
                self.env,
                build_class,
                field_name_cstr.as_ptr(),
                signature_cstr.as_ptr(),
            );
            if field_id.is_null() {
                return Err(anyhow::anyhow!("Failed to get field ID for {}", field_name));
            }

            // 创建新的字符串值
            let value_cstr = std::ffi::CString::new(value)?;
            let new_value = (jni_funcs.NewStringUTF)(self.env, value_cstr.as_ptr());
            if new_value.is_null() {
                return Err(anyhow::anyhow!("Failed to create string for {}", value));
            }

            // 设置静态字段
            (jni_funcs.SetStaticObjectField)(self.env, build_class, field_id, new_value);
        }

        Ok(())
    }

    /// Hook SystemProperties.native_get 方法 (传统 JNI Hook)
    fn hook_system_properties(&mut self) -> anyhow::Result<()> {
        // 定义要 Hook 的 JNI 方法
        let mut methods = [jni_sys::JNINativeMethod {
            name: c"native_get".as_ptr() as *mut u8,
            signature: c"(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;".as_ptr()
                as *mut u8,
            fnPtr: native_get_hook as *mut std::ffi::c_void,
        }];

        // Hook SystemProperties 类的 native_get 方法
        self.api
            .hook_jni_native_methods(self.env, "android/os/SystemProperties", &mut methods);

        // 保存原始函数指针
        let original_fn_ptr = unsafe {
            std::mem::transmute::<*mut std::ffi::c_void, OriginalNativeGet>(methods[0].fnPtr)
        };
        *ORIGINAL_NATIVE_GET.lock().unwrap() = Some(original_fn_ptr);

        Ok(())
    }
}

register_zygisk_module!(MyModule);

/// SystemProperties.native_get Hook 函数
unsafe extern "C" fn native_get_hook(
    env: *mut jni_sys::JNIEnv,
    class: jni_sys::jclass,
    key: jni_sys::jstring,
    def: jni_sys::jstring,
) -> jni_sys::jstring {
    // 获取 JNI 函数表
    let jni_funcs = (**env).v1_4;

    // 获取属性名
    let key_cstr = (jni_funcs.GetStringUTFChars)(env, key, std::ptr::null_mut());
    if key_cstr.is_null() {
        return def;
    }

    // 检查是否需要伪装此属性
    let result = {
        let key_str = std::ffi::CStr::from_ptr(key_cstr);
        if let Ok(key) = key_str.to_str() {
            let fake_props = FAKE_PROPS.lock().unwrap();
            if let Some(props) = fake_props.as_ref() {
                props.get(key).map(|fake_value| {
                    let fake_cstr = std::ffi::CString::new(fake_value.as_str()).unwrap();
                    (jni_funcs.NewStringUTF)(env, fake_cstr.as_ptr())
                })
            } else {
                None
            }
        } else {
            None
        }
    };

    (jni_funcs.ReleaseStringUTFChars)(env, key, key_cstr);

    // 如果有伪装值，返回；否则调用原始函数
    if let Some(fake_result) = result {
        return fake_result;
    }

    // 未匹配的属性：调用原始 native_get 函数获取真实值
    let original_native_get = ORIGINAL_NATIVE_GET.lock().unwrap();
    if let Some(orig_fn) = *original_native_get {
        return orig_fn(env, class, key, def);
    }

    // 如果原始函数不可用（不应该发生），返回默认值
    def
}
