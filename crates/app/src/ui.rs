mod artwork;
mod components;
mod navigation;
mod pages;
mod style;

use crate::ui_state;
use adw::prelude::*;
use components::battery_text;
use navigation::{mobile_navigation, navigation, set_navigation_width};
use nothing_core::{
    AppConfig, DeviceCapabilities, DeviceCommand, DeviceEvent, DeviceSnapshot, Paths,
};
use pages::{controls_page, equalizer_page, more_page, noise_page, overview_page};
use std::{cell::RefCell, rc::Rc, sync::mpsc};
use style::install_css;

pub struct Shell(Rc<ShellInner>);

impl Clone for Shell {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

struct ShellInner {
    pub window: adw::ApplicationWindow,
    toast: adw::ToastOverlay,
    snapshot: Rc<RefCell<DeviceSnapshot>>,
    status: gtk::Label,
    spinner: gtk::Spinner,
    left_battery: gtk::Label,
    right_battery: gtk::Label,
    case_battery: gtk::Label,
    wear: gtk::Label,
    firmware: gtk::Label,
    write_widgets: Vec<gtk::Widget>,
}

impl Shell {
    pub fn window(&self) -> &adw::ApplicationWindow {
        &self.0.window
    }

    pub fn apply_event(&self, event: DeviceEvent) {
        if let Some(message) = ui_state::failure_message(&event) {
            self.0.toast.add_toast(adw::Toast::new(&message));
        }
        {
            let mut snapshot = self.0.snapshot.borrow_mut();
            match &event {
                DeviceEvent::ConnectionChanged(value) => snapshot.connection = *value,
                DeviceEvent::Snapshot(value) => *snapshot = value.clone(),
                DeviceEvent::Battery(value) => snapshot.battery = value.clone(),
                DeviceEvent::Wear(value) => snapshot.wear = *value,
                DeviceEvent::Anc { mode, level } => {
                    snapshot.anc_mode = *mode;
                    snapshot.anc_level = *level;
                }
                DeviceEvent::Eq(value) => snapshot.eq_preset = *value,
                DeviceEvent::CustomEq(value) => snapshot.custom_eq = *value,
                DeviceEvent::Gestures(value) => snapshot.gestures = value.clone(),
                DeviceEvent::BassEnhance(value) => snapshot.bass_enhance = *value,
                DeviceEvent::InEarDetection(value) => snapshot.in_ear_detection = Some(*value),
                DeviceEvent::LowLag(value) => snapshot.low_lag = Some(*value),
                DeviceEvent::Firmware(value) => snapshot.firmware = Some(value.clone()),
                _ => {}
            }
        }
        self.refresh();
    }

    fn refresh(&self) {
        let snapshot = self.0.snapshot.borrow();
        let view = ui_state::map_snapshot(&snapshot, &DeviceCapabilities::b171());
        self.0.status.set_label(&view.status);
        self.0.spinner.set_spinning(view.spinner);
        self.0.spinner.set_visible(view.spinner);
        for widget in &self.0.write_widgets {
            widget.set_sensitive(view.controls_enabled);
        }
        self.0
            .left_battery
            .set_label(&battery_text("LEFT", snapshot.battery.left));
        self.0
            .right_battery
            .set_label(&battery_text("RIGHT", snapshot.battery.right));
        self.0
            .case_battery
            .set_label(&battery_text("CASE", snapshot.battery.case));
        self.0.wear.set_label(&format!(
            "Left: {}  ·  Right: {}",
            if snapshot.wear.left {
                "in ear"
            } else {
                "not worn"
            },
            if snapshot.wear.right {
                "in ear"
            } else {
                "not worn"
            }
        ));
        self.0.firmware.set_label(
            snapshot
                .firmware
                .as_deref()
                .unwrap_or("Waiting for device…"),
        );
    }
}

pub fn build(
    app: &adw::Application,
    commands: mpsc::Sender<DeviceCommand>,
    config: Rc<RefCell<AppConfig>>,
    paths: Option<Paths>,
) -> Shell {
    install_css();
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Nothing Linux")
        .default_width(1080)
        .default_height(760)
        .width_request(390)
        .height_request(560)
        .build();
    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&artwork::dot_heading()));
    let status = gtk::Label::builder()
        .label("Disconnected")
        .css_classes(["status-pill"])
        .build();
    let spinner = gtk::Spinner::new();
    spinner.set_visible(false);
    let status_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    status_box.append(&spinner);
    status_box.append(&status);
    header.pack_end(&status_box);
    toolbar.add_top_bar(&header);

    let toast = adw::ToastOverlay::new();
    let layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    layout.append(&content);
    toast.set_child(Some(&layout));
    toolbar.set_content(Some(&toast));
    window.set_content(Some(&toolbar));
    let stack = gtk::Stack::builder()
        .hexpand(true)
        .vexpand(true)
        .transition_type(gtk::StackTransitionType::Crossfade)
        .build();
    let sidebar = navigation(&stack);
    let separator = gtk::Separator::new(gtk::Orientation::Vertical);
    let mobile_navigation = mobile_navigation(&stack);
    mobile_navigation.set_visible(false);
    content.append(&sidebar);
    content.append(&separator);
    content.append(&stack);
    layout.append(&mobile_navigation);
    if let Ok(condition) = adw::BreakpointCondition::parse("max-width: 700px") {
        let breakpoint = adw::Breakpoint::new(condition);
        let hidden = false.to_value();
        let shown = true.to_value();
        breakpoint.add_setter(&sidebar, "visible", Some(&hidden));
        breakpoint.add_setter(&separator, "visible", Some(&hidden));
        breakpoint.add_setter(&mobile_navigation, "visible", Some(&shown));
        window.add_breakpoint(breakpoint);
    }
    let responsive_sidebar = sidebar.clone();
    let responsive_separator = separator.clone();
    let responsive_mobile = mobile_navigation.clone();
    window.connect_notify_local(Some("width"), move |window, _| {
        set_navigation_width(
            window.width(),
            &responsive_sidebar,
            &responsive_separator,
            &responsive_mobile,
        );
    });

    let snapshot = Rc::new(RefCell::new(DeviceSnapshot::default()));
    let mut write_widgets = Vec::new();
    let overview = overview_page(&commands, &mut write_widgets);
    let overview_refs = overview.1;
    stack.add_titled(&overview.0, Some("overview"), "Overview");
    stack.add_titled(
        &noise_page(&commands, &mut write_widgets),
        Some("noise"),
        "Noise control",
    );
    stack.add_titled(
        &equalizer_page(&commands, &mut write_widgets),
        Some("equalizer"),
        "Equalizer",
    );
    stack.add_titled(
        &controls_page(&commands, &mut write_widgets),
        Some("controls"),
        "Controls",
    );
    stack.add_titled(
        &more_page(
            &commands,
            &mut write_widgets,
            snapshot.clone(),
            toast.clone(),
            config,
            paths,
            overview_refs.firmware.clone(),
        ),
        Some("more"),
        "More",
    );

    let shell = Shell(Rc::new(ShellInner {
        window,
        toast,
        snapshot,
        status,
        spinner,
        left_battery: overview_refs.left,
        right_battery: overview_refs.right,
        case_battery: overview_refs.case,
        wear: overview_refs.wear,
        firmware: overview_refs.firmware,
        write_widgets,
    }));
    for widget in &shell.0.write_widgets {
        widget.set_sensitive(false);
    }
    shell
}
