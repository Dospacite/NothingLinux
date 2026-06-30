use ksni::blocking::{Handle, TrayMethods};
use nothing_core::{BatteryState, ChargeLevel};
use std::sync::mpsc;

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

pub fn start() -> (Option<Handle<TrayItem>>, mpsc::Receiver<TrayAction>) {
    let (actions, receiver) = mpsc::channel();
    let item = TrayItem {
        battery: "Not connected".into(),
        actions,
    };
    let handle = item.assume_sni_available(true).spawn().ok();
    (handle, receiver)
}
