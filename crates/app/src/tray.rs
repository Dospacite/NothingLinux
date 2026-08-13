use ksni::Icon;
use ksni::blocking::{Handle, TrayMethods};
use nothing_core::{BatteryState, ChargeLevel};
use std::{
    env,
    path::{Path, PathBuf},
    sync::mpsc,
};

const NXL_RED: [u8; 3] = [0xe6, 0x1a, 0x1f];

#[derive(Debug, Clone, Copy)]
pub enum TrayAction {
    Open,
    AncOff,
    AncTransparency,
    AncOn,
    Quit,
}

#[derive(Debug)]
pub struct TrayItem {
    battery: String,
    actions: mpsc::Sender<TrayAction>,
}

impl TrayItem {
    pub fn set_battery(&mut self, battery: &BatteryState) {
        self.battery = format!(
            "L {} · R {} · C {}",
            percent(battery.left),
            percent(battery.right),
            percent(battery.case)
        );
    }
}

impl ksni::Tray for TrayItem {
    fn id(&self) -> String {
        "io_github_nothinglinux_NothingLinux".into()
    }
    fn title(&self) -> String {
        format!("Nothing Linux — {}", self.battery)
    }
    fn icon_name(&self) -> String {
        "io.github.nothinglinux.nothinglinux".into()
    }
    fn icon_theme_path(&self) -> String {
        installed_icon_theme_path()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
    fn icon_pixmap(&self) -> Vec<Icon> {
        vec![nxl_tray_icon()]
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::{MenuItem, StandardItem, SubMenu};
        vec![
            StandardItem {
                label: "Open Nothing Linux".into(),
                icon_name: "window-new-symbolic".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.actions.send(TrayAction::Open);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: self.battery.clone(),
                enabled: false,
                ..Default::default()
            }
            .into(),
            SubMenu {
                label: "Noise control".into(),
                submenu: vec![
                    tray_command("Off", TrayAction::AncOff),
                    tray_command("Transparency", TrayAction::AncTransparency),
                    tray_command("ANC · High", TrayAction::AncOn),
                ],
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".into(),
                icon_name: "application-exit-symbolic".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.actions.send(TrayAction::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

fn tray_command(label: &str, action: TrayAction) -> ksni::MenuItem<TrayItem> {
    ksni::menu::StandardItem {
        label: label.into(),
        activate: Box::new(move |tray: &mut TrayItem| {
            let _ = tray.actions.send(action);
        }),
        ..Default::default()
    }
    .into()
}

fn percent(level: Option<ChargeLevel>) -> String {
    level.map_or_else(|| "—".into(), |level| format!("{}%", level.percent))
}

fn installed_icon_theme_path() -> Option<PathBuf> {
    let app_dir = env::var_os("APPDIR").map(PathBuf::from);
    let executable = env::current_exe().ok();
    icon_theme_path_candidates(app_dir.as_deref(), executable.as_deref())
        .into_iter()
        .find(|path| path.join("hicolor").is_dir())
}

fn icon_theme_path_candidates(app_dir: Option<&Path>, executable: Option<&Path>) -> Vec<PathBuf> {
    let mut candidates = Vec::with_capacity(2);
    if let Some(app_dir) = app_dir {
        candidates.push(app_dir.join("usr/share/icons"));
    }
    if let Some(usr_dir) = executable
        .and_then(Path::parent)
        .filter(|bin_dir| bin_dir.file_name().is_some_and(|name| name == "bin"))
        .and_then(Path::parent)
    {
        candidates.push(usr_dir.join("share/icons"));
    }
    candidates
}

fn nxl_tray_icon() -> Icon {
    const SIZE: usize = 64;
    let mut data = vec![0; SIZE * SIZE * 4];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let offset = (y * SIZE + x) * 4;
            data[offset..offset + 4].copy_from_slice(&[0xff, NXL_RED[0], NXL_RED[1], NXL_RED[2]]);

            let diagonal_x = 18 + (y.saturating_sub(14) * 25 / 36);
            let is_n = (15..=22).contains(&x)
                || (42..=49).contains(&x)
                || ((14..=50).contains(&y) && x.abs_diff(diagonal_x) <= 3);
            if is_n {
                data[offset..offset + 4].copy_from_slice(&[0xff, 0xff, 0xff, 0xff]);
            }
        }
    }
    Icon {
        width: SIZE as i32,
        height: SIZE as i32,
        data,
    }
}

pub fn start() -> (Option<Handle<TrayItem>>, mpsc::Receiver<TrayAction>) {
    let (actions, receiver) = mpsc::channel();
    let item = TrayItem {
        battery: "Not connected".into(),
        actions,
    };
    let handle = item.assume_sni_available(true).spawn().ok();
    (handle, receiver)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appimage_icon_theme_path_precedes_the_system_path() {
        let candidates = icon_theme_path_candidates(
            Some(Path::new("/tmp/AppDir")),
            Some(Path::new("/usr/bin/nothing-linux")),
        );
        assert_eq!(
            candidates,
            vec![
                PathBuf::from("/tmp/AppDir/usr/share/icons"),
                PathBuf::from("/usr/share/icons"),
            ]
        );
    }

    #[test]
    fn tray_fallback_is_a_full_argb_icon() {
        let icon = nxl_tray_icon();
        assert_eq!((icon.width, icon.height), (64, 64));
        assert_eq!(icon.data.len(), 64 * 64 * 4);
        assert_eq!(&icon.data[..4], &[0xff, NXL_RED[0], NXL_RED[1], NXL_RED[2]]);
        let centre = ((32 * 64 + 32) * 4)..((32 * 64 + 32) * 4 + 4);
        assert_eq!(&icon.data[centre], &[0xff, 0xff, 0xff, 0xff]);
    }
}
