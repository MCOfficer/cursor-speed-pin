use anyhow::Result;
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
    let event_loop = EventLoop::<Events>::with_user_event();
    let proxy = event_loop.create_proxy();

    let green = include_bytes!("../green.ico");
    let red = include_bytes!("../red.ico");
    let green_icon = Icon::from_buffer(green, None, None).unwrap();
    let red_icon = Icon::from_buffer(red, None, None).unwrap();

    let mut tray_icon = TrayIconBuilder::new()
        .sender_winit(proxy)
        .icon_from_buffer(green)
        .tooltip("Enabled")
        .menu(MenuBuilder::new().item("Exit", Events::Exit))
        .on_double_click(Events::DoubleClickTrayIcon)
        .build()
        .unwrap();

    DESIRED_SPEED.store(winapi::get_mouse_speed()?, Ordering::SeqCst);
    update_status(&mut tray_icon, &green_icon, &red_icon);

    let checking_thread = std::thread::spawn(|| loop {
        std::thread::sleep_ms(50);
        if ENABLED.load(Ordering::Relaxed) {
            let current = winapi::get_mouse_speed().unwrap();
            let desired = DESIRED_SPEED.load(Ordering::Relaxed);
            if desired != current {
                winapi::notify(format!(
                    "Speed was set to {}, resetting to desired speed {}",
                    current, desired
                ));
                winapi::set_mouse_speed(desired).unwrap();
            }
        }
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            // User events
            Event::UserEvent(e) => match e {
                Events::Exit => *control_flow = ControlFlow::Exit,
                Events::DoubleClickTrayIcon => {
                    if ENABLED.load(Ordering::Relaxed) {
                        ENABLED.store(false, Ordering::SeqCst);
                    } else {
                        DESIRED_SPEED.store(winapi::get_mouse_speed().unwrap(), Ordering::SeqCst);
                        ENABLED.store(true, Ordering::SeqCst);
                    }
                    update_status(&mut tray_icon, &green_icon, &red_icon);
                }
            },
            _ => (),
        }
    });
}

fn update_status<T>(tray_icon: &mut TrayIcon<T>, green: &Icon, red: &Icon)
where
    T: PartialEq + Clone + 'static,
{
    if ENABLED.load(Ordering::Relaxed) {
        winapi::notify(format!(
            "Pinning cursor speed of {}",
            DESIRED_SPEED.load(Ordering::Relaxed)
        ));
        tray_icon.set_tooltip(&format!(
            "Enabled, Speed: {}",
            DESIRED_SPEED.load(Ordering::Relaxed)
        ));
        tray_icon.set_icon(green);
    } else {
        winapi::notify("Unpinned cursor speed".to_string());
        tray_icon.set_tooltip("Disabled");
        tray_icon.set_icon(red);
    }
}

mod winapi {
    use anyhow::anyhow;
    use anyhow::Result;
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

    fn generate_guid() -> Result<GUID, i32> {
        unsafe {
            let mut gen_guid: GUID = Default::default();
            let result = CoCreateGuid(&mut gen_guid);
            if result == winerror::S_OK {
                Ok(gen_guid)
            } else {
                Err(result)
            }
        }
    }

    pub fn notify(message: String) {
        std::thread::spawn(move || {
            let mut notification_descriptor = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                uFlags: shellapi::NIF_INFO | shellapi::NIF_REALTIME | shellapi::NIF_GUID,
                dwInfoFlags: shellapi::NIIF_NOSOUND,
                guidItem: generate_guid().expect("Unable to generate a new GUID"),
                szInfoTitle: encode_string_wide!("CursorSpeedPin", 64),
                szInfo: encode_string_wide!(message, 256),
                ..Default::default()
            };
            let desc = notification_descriptor.borrow_mut();
            unsafe { Shell_NotifyIconW(shellapi::NIM_ADD, &mut *desc) };

            std::thread::sleep(std::time::Duration::from_secs(3));
            unsafe { Shell_NotifyIconW(shellapi::NIM_DELETE, &mut *desc) };
        });
    }

    // Retrieves the currently set mouse speed. Returns a `u8` ranging from 1 to 20.
    pub fn get_mouse_speed() -> Result<u8> {
        let mut res = 0;
        let status =
            unsafe { SystemParametersInfoW(SPI_GETMOUSESPEED, 0, &mut res as *mut _ as PVOID, 0) };
        if status == 1 {
            Ok(res)
        } else {
            Err(anyhow!(
                "Failed to get mouse speed, return code was {}",
                status
            ))
        }
    }

    // Sets the user's mouse speed. Accepts a `u8` ranging from 1 to 20.
    pub fn set_mouse_speed(speed: u8) -> Result<()> {
        let status = unsafe { SystemParametersInfoW(SPI_SETMOUSESPEED, 0, speed as PVOID, 0) };
        if status == 1 {
            Ok(())
        } else {
            Err(anyhow!(
                "Failed to set mouse speed, return code was {}",
                status
            ))
        }
    }
}
