use std::{collections::HashMap, time::Duration};

use anyhow::{Result, anyhow};
use log::{debug, error};
use prop_rs_android::{resetprop::ResetProp, sys_prop};

use crate::watcher;

struct Props {
    backup: HashMap<String, String>,
    sys: ResetProp,
}

impl Drop for Props {
    fn drop(&mut self) {
        for (k, v) in &self.backup {
            if v == &String::from("deleted") {
                self.sys.delete(k).unwrap();
            } else {
                self.sys.set(k, v).unwrap();
            }
        }
    }
}

impl Props {
    fn new() -> Result<Self> {
        sys_prop::init()?;

        Ok(Self {
            backup: HashMap::new(),
            sys: ResetProp {
                skip_svc: true,
                persistent: false,
                persist_only: false,
                verbose: false,
                show_context: false,
                rebuild: false,
            },
        })
    }

    fn delete_prop<S>(&mut self, k: S)
    where
        S: AsRef<str>,
    {
        let orig_v = self.sys.get(k.as_ref());

        if let Some(v) = orig_v {
            match self.sys.delete(&k.as_ref()) {
                Ok(s) if s => {
                    self.backup.insert(k.as_ref().to_string(), v);
                    debug!("set prop successful!!")
                }
                Err(e) => {
                    error!("failed to set prop, k: {} v: {v}, err: {e}", k.as_ref());
                }
                _ => {}
            }
        }
    }

    fn set_prop<S>(&mut self, k: S, v: S)
    where
        S: AsRef<str>,
    {
        let orig_v = self.sys.get(k.as_ref());

        if let Some(orig_v) = orig_v {
            match self.sys.set(&k.as_ref(), &v.as_ref()) {
                Ok(()) => {
                    self.backup.insert(k.as_ref().to_string(), orig_v);
                    debug!("set prop successful!!")
                }
                Err(e) => {
                    error!(
                        "failed to set prop, k: {} v: {}, err: {e}",
                        k.as_ref(),
                        v.as_ref()
                    );
                }
            }
        } else {
            match self.sys.set(&k.as_ref(), &v.as_ref()) {
                Ok(()) => {
                    self.backup
                        .insert(k.as_ref().to_string(), "deleted".to_string());
                    debug!("set prop successful!!")
                }
                Err(e) => {
                    error!(
                        "failed to set prop, k: {} v: {}, err: {e}",
                        k.as_ref(),
                        v.as_ref()
                    );
                }
            }
        }
    }
}

pub fn spwan_props(
    props_map: &HashMap<String, String>,
    deleted_props_map: Vec<String>,
) -> Result<()> {
    let ppid = std::process::id();

    unsafe {
        let pid = libc::fork();
        if pid < 0 {
            error!("fork error {}", std::io::Error::last_os_error());
            return Err(anyhow!("fork fialed"));
        } else if pid > 0 {
            let sid = libc::setsid();
            if sid == -1 {
                return Err(anyhow!("sid failed"));
            }
        }
    }
    let mut props = Props::new()?;
    let mut watcher = watcher::WindowsDumper::new();

    for k in deleted_props_map {
        props.delete_prop(&k);
    }

    for (k, v) in props_map {
        props.set_prop(k, v);
    }

    loop {
        let cache = watcher.cache();

        if cache.pid.is_some_and(|p| p as u32 != ppid) || !cache.visible_freeform_window {
            debug!("dropped sys_init!!");
            drop(props);
            break;
        }

        std::thread::sleep(Duration::from_secs(1));
    }

    Ok(())
}
