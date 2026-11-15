//! Системные предпочтения интерфейса.

/// Система просит меньше анимаций. Читает мост Settings → Settings.systemReducedMotion → Theme.reducedMotion
/// при Settings.animations == "system". Windows — SPI_GETCLIENTAREAANIMATION == FALSE; Linux — GNOME
/// `enable-animations` == false; macOS — «Уменьшить движение». Не удалось узнать — false.
pub fn reduced_motion() -> bool {
    imp::reduced_motion()
}

/// Тема панели задач (для значка трея): Some(true) — тёмная. Знает только Windows
/// (HKCU\…\Themes\Personalize, SystemUsesLightTheme); на других ОС — None, QML берёт тему Qt.
pub fn taskbar_dark() -> Option<bool> {
    imp::taskbar_dark()
}

/// Windows: the app keeps full speed in the power-saving modes. The process opts out of power throttling
/// (EcoQoS: Windows otherwise puts the app on the slow, efficient cores and coarsens its timers when the power mode
/// is set to battery saving), so playback, scrolling and animations do not stutter on battery. Elsewhere: nothing.
pub fn prefer_full_speed() {
    #[cfg(target_os = "windows")]
    imp::prefer_full_speed();
}

/// Windows: the graphics card the system runs the app on — Windows' own per-app setting (Settings → System →
/// Display → Graphics, `HKCU\Software\Microsoft\DirectX\UserGpuPreferences`): "discrete" — high performance,
/// "integrated" — power saving, anything else — the system's choice (the entry is removed). Elsewhere: nothing.
pub fn windows_gpu_preference(choice: &str) {
    #[cfg(target_os = "windows")]
    imp::windows_gpu_preference(choice);
    #[cfg(not(target_os = "windows"))]
    let _ = choice;
}

#[cfg(target_os = "windows")]
mod imp {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SPI_GETCLIENTAREAANIMATION, SystemParametersInfoW};

    pub fn prefer_full_speed() {
        use windows_sys::Win32::System::Threading::{
            GetCurrentProcess, PROCESS_POWER_THROTTLING_CURRENT_VERSION, PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
            PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION, PROCESS_POWER_THROTTLING_STATE, ProcessPowerThrottling,
            SetProcessInformation,
        };
        // both kinds of throttling under our control, and turned off (state 0) for this process
        let state = PROCESS_POWER_THROTTLING_STATE {
            Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED | PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION,
            StateMask: 0,
        };
        // SAFETY: the pointer and size describe `state`, the PROCESS_POWER_THROTTLING_STATE this class asks for
        let ok = unsafe {
            SetProcessInformation(
                GetCurrentProcess(),
                ProcessPowerThrottling,
                (&raw const state).cast(),
                size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
            )
        };
        if ok == 0 {
            tracing::debug!("power throttling opt-out refused: {}", std::io::Error::last_os_error());
        }
    }

    pub fn windows_gpu_preference(choice: &str) {
        let Ok(exe) = std::env::current_exe() else { return };
        let exe = exe.to_string_lossy().trim_start_matches(r"\\?\").to_owned();
        let Ok((key, _)) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
            .create_subkey(r"Software\Microsoft\DirectX\UserGpuPreferences")
        else {
            return;
        };
        let result = match choice {
            "discrete" => key.set_value(&exe, &"GpuPreference=2;"),
            "integrated" => key.set_value(&exe, &"GpuPreference=1;"),
            _ => match key.delete_value(&exe) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                other => other,
            },
        };
        if let Err(error) = result {
            tracing::debug!("graphics preference not saved: {error}");
        }
    }

    pub fn taskbar_dark() -> Option<bool> {
        let key = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
            .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
            .ok()?;
        // нет значения — Windows 10 до 1903: панель тёмная
        Some(key.get_value::<u32, _>("SystemUsesLightTheme").unwrap_or(0) == 0)
    }

    pub fn reduced_motion() -> bool {
        let mut enabled: i32 = 1;
        // SAFETY: pvParam указывает на BOOL, как требует SPI_GETCLIENTAREAANIMATION
        let ok = unsafe { SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, (&raw mut enabled).cast(), 0) };
        ok != 0 && enabled == 0
    }
}

#[cfg(target_os = "linux")]
mod imp {
    pub fn taskbar_dark() -> Option<bool> {
        None
    }

    pub fn reduced_motion() -> bool {
        super::command_output("gsettings", &["get", "org.gnome.desktop.interface", "enable-animations"])
            .is_some_and(|out| out == "false")
    }
}

#[cfg(target_os = "macos")]
mod imp {
    pub fn taskbar_dark() -> Option<bool> {
        None
    }

    pub fn reduced_motion() -> bool {
        super::command_output("defaults", &["read", "com.apple.universalaccess", "reduceMotion"])
            .is_some_and(|out| out == "1")
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
mod imp {
    pub fn taskbar_dark() -> Option<bool> {
        None
    }

    pub fn reduced_motion() -> bool {
        false
    }
}

/// Вывод команды без пробелов по краям; None — нет команды или код ошибки.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

#[cfg(test)]
mod tests {
    #[test]
    fn answers_without_panic() {
        // значение зависит от машины — проверяем только, что вызов работает
        let _ = super::reduced_motion();
        let _ = super::taskbar_dark();
    }
}
