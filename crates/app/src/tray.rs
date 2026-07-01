use ksni::Icon;
use ksni::blocking::{Handle, TrayMethods};
use nothing_core::{BatteryState, ChargeLevel};
use std::sync::mpsc;

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

fn nxl_tray_icon() -> Icon {
    const SIZE: usize = 64;
    const GLYPHS: [&[&str]; 3] = [
        &[
            "10001", "11001", "10101", "10011", "10001", "10001", "10001",
        ],
        &[
            "10001", "01010", "00100", "00100", "01010", "10001", "10001",
        ],
        &[
            "10000", "10000", "10000", "10000", "10000", "10000", "11111",
        ],
    ];
    let cell_width = 3;
    let cell_height = 7;
    let gap = 3;
    let glyph_width = 5 * cell_width;
    let glyph_height = 7 * cell_height;
    let text_width = glyph_width * GLYPHS.len() + gap * (GLYPHS.len() - 1);
    let start_x = (SIZE - text_width) / 2;
    let start_y = (SIZE - glyph_height) / 2;
    let mut data = vec![0; SIZE * SIZE * 4];
    for (glyph_index, glyph) in GLYPHS.iter().enumerate() {
        let glyph_x = start_x + glyph_index * (glyph_width + gap);
        for (row, pattern) in glyph.iter().enumerate() {
            for (column, value) in pattern.bytes().enumerate() {
                if value != b'1' {
                    continue;
                }
                let block_x = glyph_x + column * cell_width;
                let block_y = start_y + row * cell_height;
                for y in block_y..block_y + cell_height {
                    for x in block_x..block_x + cell_width {
                        let offset = (y * SIZE + x) * 4;
                        data[offset] = 0xff;
                        data[offset + 1] = NXL_RED[0];
                        data[offset + 2] = NXL_RED[1];
                        data[offset + 3] = NXL_RED[2];
                    }
                }
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
