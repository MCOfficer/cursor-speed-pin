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
        .on_double_click(Events::DoubleClickTrayIcon)
        .build()
        .unwrap();

    DESIRED_SPEED.store(winapi::get_mouse_speed()?, Ordering::SeqCst);
    update_systray(&mut tray_icon, &green_icon, &red_icon);

    let checking_thread = std::thread::spawn(|| loop {
        std::thread::sleep_ms(50);
        if ENABLED.load(Ordering::Relaxed) {
            let current = winapi::get_mouse_speed().unwrap();
            let desired = DESIRED_SPEED.load(Ordering::Relaxed);
            if desired != current {
                println!(
                    "Got value of {}, resetting to desired speed {}",
                    current, desired
                );
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
                    update_systray(&mut tray_icon, &green_icon, &red_icon);
                }
            },
            _ => (),
        }
    });
}

fn update_systray<T>(tray_icon: &mut TrayIcon<T>, green: &Icon, red: &Icon)
where
    T: PartialEq + Clone + 'static,
{
    if ENABLED.load(Ordering::Relaxed) {
        tray_icon.set_tooltip(&format!(
            "Enabled, Speed: {}",
            DESIRED_SPEED.load(Ordering::Relaxed)
        ));
        tray_icon.set_icon(green);
    } else {
        tray_icon.set_tooltip("Disabled");
        tray_icon.set_icon(red);
    }
}

mod winapi {
    use anyhow::anyhow;
    use anyhow::Result;
    use winapi::um::winnt::PVOID;
    use winapi::um::winuser::{SystemParametersInfoW, SPI_GETMOUSESPEED, SPI_SETMOUSESPEED};

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