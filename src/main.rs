#![windows_subsystem = "windows"]

use anyhow::Result;
use log::{error, info, LevelFilter};
use simplelog::*;
use std::fs::File;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use trayicon::{Icon, MenuBuilder, TrayIcon, TrayIconBuilder};
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
};

#[derive(Clone, Eq, PartialEq, Debug)]
enum Events {
    DoubleClickTrayIcon,
    Exit,
}

static ENABLED: AtomicBool = AtomicBool::new(true);
static DESIRED_SPEED: AtomicU8 = AtomicU8::new(0);

fn main() -> Result<()> {
    let mut logfile = std::env::current_exe().unwrap();
    logfile = logfile.parent().unwrap().to_path_buf();
    logfile.push("cursor-speed-pin.log");
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Debug, Config::default(), TerminalMode::Mixed),
        WriteLogger::new(
            LevelFilter::Debug,
            Config::default(),
            File::create(logfile).unwrap(),
        ),
    ])
    .unwrap();
    info!("Initialized logger");

    let event_loop = EventLoop::<Events>::with_user_event();
    let proxy = event_loop.create_proxy();

    let enabled = include_bytes!("../assets/green.ico");
    let disabled = include_bytes!("../assets/gray.ico");
    let enabled_icon = Icon::from_buffer(enabled, None, None).unwrap();
    let disabled_icon = Icon::from_buffer(disabled, None, None).unwrap();

    let mut tray_icon = TrayIconBuilder::new()
        .sender_winit(proxy)
        .icon_from_buffer(disabled)
        .tooltip("Disabled")
        .menu(MenuBuilder::new().item("Exit", Events::Exit))
        .on_double_click(Events::DoubleClickTrayIcon)
        .build()
        .unwrap();

    info!("Initialized tray icon");
    info!("Fetching initial mouse speed");
    match winapi::get_mouse_speed() {
        Some(speed) => {
            DESIRED_SPEED.store(speed, Ordering::SeqCst);
            info!("Initial status update");
            update_status(&mut tray_icon, &enabled_icon, &disabled_icon);
        }
        None => {
            info!("No initial mouse speed, disabling");
            ENABLED.store(false, Ordering::SeqCst);
        }
    }

    info!("Spawning speed checking thread");
    std::thread::spawn(|| {
        info!("Speed checking thread running");

        loop {
            std::thread::sleep(std::time::Duration::from_millis(50));
            if ENABLED.load(Ordering::Relaxed) {
                if let Some(current) = winapi::get_mouse_speed() {
                    let desired = DESIRED_SPEED.load(Ordering::Relaxed);
                    if desired != current {
                        info!("Found speed of {}, resetting to {}", current, desired);
                        match winapi::set_mouse_speed(desired) {
                            Ok(_) => {
                                winapi::notify(format!(
                                    "Speed was set to {}, resetting to desired speed {}",
                                    current, desired
                                ));
                            }
                            Err(_) => {
                                winapi::notify(
                                    "Failed to set cursor speed, see the log file for more."
                                        .to_string(),
                                );
                            }
                        };
                    }
                }
            }
        }
    });

    info!("Starting winit event loop");
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Event::UserEvent(e) = event {
            match e {
                Events::Exit => *control_flow = ControlFlow::Exit,
                Events::DoubleClickTrayIcon => {
                    if ENABLED.load(Ordering::Relaxed) {
                        info!("Disabling");
                        ENABLED.store(false, Ordering::SeqCst);
                    } else {
                        match winapi::get_mouse_speed() {
                            Some(speed) => {
                                info!("Enabling with speed {}", speed);
                                DESIRED_SPEED.store(speed, Ordering::SeqCst);
                                ENABLED.store(true, Ordering::SeqCst);
                            }
                            None => {
                                winapi::notify(
                                    "Failed to get current cursor speed, see the logfile for more."
                                        .to_string(),
                                );
                                return;
                            }
                        };
                    }
                    update_status(&mut tray_icon, &enabled_icon, &disabled_icon);
                }
            }
        }
    });
}

fn update_status<T>(tray_icon: &mut TrayIcon<T>, enabled_icon: &Icon, disabled_icon: &Icon)
where
    T: PartialEq + Clone + 'static,
{
    let enabled = ENABLED.load(Ordering::Relaxed);
    let (notification, tooltip, icon) = if enabled {
        (
            format!(
                "Pinning cursor speed of {}",
                DESIRED_SPEED.load(Ordering::Relaxed)
            ),
            format!("Enabled, Speed: {}", DESIRED_SPEED.load(Ordering::Relaxed)),
            enabled_icon,
        )
    } else {
        (
            "Unpinned cursor speed".to_string(),
            "Disabled".to_string(),
            disabled_icon,
        )
    };

    winapi::notify(notification);

    if let Err(e) = tray_icon.set_tooltip(&tooltip) {
        error!("Failed to set tray tooltip to {}: {:#?}", tooltip, e);
    };

    if let Err(e) = tray_icon.set_icon(icon) {
        error!("Failed to set tray icon: {:#?}", e);
    };
}

mod winapi {
    use anyhow::anyhow;
    use anyhow::Result;
    use log::{error, info};
    use std::os::windows::ffi::OsStrExt;
    use winapi::_core::borrow::BorrowMut;
    use winapi::shared::guiddef::GUID;
    use winapi::shared::winerror;
    use winapi::um::combaseapi::CoCreateGuid;
    use winapi::um::shellapi::{self, Shell_NotifyIconW, NOTIFYICONDATAW};
    use winapi::um::winnt::PVOID;
    use winapi::um::winuser::{SystemParametersInfoW, SPI_GETMOUSESPEED, SPI_SETMOUSESPEED};

    macro_rules! encode_string_wide {
        ($string:expr, $length:expr) => {{
            let codepoints = std::ffi::OsString::from($string)
                .as_os_str()
                .encode_wide()
                .take(64)
                .collect::<Vec<u16>>();
            let mut array = [0u16; $length];

            unsafe {
                std::ptr::copy_nonoverlapping(
                    codepoints.as_ptr(),
                    array.as_mut_ptr(),
                    codepoints.len().min($length),
                );
            }
            array
        }};
    }

    fn generate_guid() -> Option<GUID> {
        unsafe {
            let mut gen_guid: GUID = Default::default();
            let result = CoCreateGuid(&mut gen_guid);
            if result == winerror::S_OK {
                Some(gen_guid)
            } else {
                error!("Failed to create GUID, return code was {}", result);
                None
            }
        }
    }

    pub fn notify(message: String) {
        info!("Sending desktop notification with content '{}'", &message);
        std::thread::spawn(move || {
            let mut notification_descriptor = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                uFlags: shellapi::NIF_INFO | shellapi::NIF_REALTIME | shellapi::NIF_GUID,
                dwInfoFlags: shellapi::NIIF_NOSOUND,
                guidItem: generate_guid().unwrap(), // If we failed to create one it is already logged; exit this thread
                szInfoTitle: encode_string_wide!("CursorSpeedPin", 64),
                szInfo: encode_string_wide!(&message, 256),
                ..Default::default()
            };

            let desc = notification_descriptor.borrow_mut();
            let status = unsafe { Shell_NotifyIconW(shellapi::NIM_ADD, &mut *desc) };
            if status == 0 {
                error!(
                    "Failed to send desktop notification, szInfo was '{}'",
                    &message
                );
                return;
            };

            std::thread::sleep(std::time::Duration::from_secs(3));
            let status = unsafe { Shell_NotifyIconW(shellapi::NIM_DELETE, &mut *desc) };
            if status == 0 {
                error!(
                    "Failed to remove desktop notification, szInfo was '{}'",
                    &message
                );
            };
        });
    }

    // Retrieves the currently set mouse speed. Returns a `u8` ranging from 1 to 20.
    pub fn get_mouse_speed() -> Option<u8> {
        let mut res = 0;
        let status =
            unsafe { SystemParametersInfoW(SPI_GETMOUSESPEED, 0, &mut res as *mut _ as PVOID, 0) };
        if status == 0 {
            error!("Failed to get mouse speed, return code was {}", status);
            None
        } else {
            Some(res)
        }
    }

    // Sets the user's mouse speed. Accepts a `u8` ranging from 1 to 20.
    pub fn set_mouse_speed(speed: u8) -> Result<()> {
        let status = unsafe { SystemParametersInfoW(SPI_SETMOUSESPEED, 0, speed as PVOID, 0) };
        if status == 0 {
            Err(anyhow!(
                "Failed to set mouse speed, return code was {}",
                status
            ))
        } else {
            Ok(())
        }
    }
}
