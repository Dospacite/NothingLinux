mod artwork;
mod components;
mod navigation;
mod pages;
mod style;

use crate::ui_state;
use adw::prelude::*;
use components::battery_text;
use navigation::bottom_navigation;
use nothing_core::{
    AppConfig, DeviceCapabilities, DeviceCommand, DeviceEvent, DeviceSnapshot, Paths,
};
use pages::{
    EqualizerRefs, MorePageDeps, MoreRefs, NoiseRefs, controls_page, equalizer_page, more_page,
    noise_page, overview_page,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::mpsc,
};
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
    confirmed_snapshot: RefCell<DeviceSnapshot>,
    snapshot: Rc<RefCell<DeviceSnapshot>>,
    pending_commands: RefCell<Vec<PendingCommand>>,
    status: gtk::Label,
    spinner: gtk::Spinner,
    left_battery: gtk::Label,
    right_battery: gtk::Label,
    case_battery: gtk::Label,
    wear: gtk::Label,
    firmware: gtk::Label,
    write_widgets: Vec<gtk::Widget>,
    updating_controls: Rc<Cell<bool>>,
    noise_refs: NoiseRefs,
    equalizer_refs: EqualizerRefs,
    more_refs: MoreRefs,
}

#[derive(Debug, Clone)]
struct PendingCommand {
    sequence: u8,
    command: DeviceCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefreshScope {
    All,
    Header,
    Battery,
    Wear,
    Anc,
    Equalizer,
    Bass,
    InEar,
    LowLag,
    DualConnection,
    Codec,
    Firmware,
}

impl Shell {
    pub fn window(&self) -> &adw::ApplicationWindow {
        &self.0.window
    }

    pub fn apply_event(&self, event: DeviceEvent) {
        if let Some(message) = ui_state::failure_message(&event) {
            self.0.toast.add_toast(adw::Toast::new(&message));
        }
        let scope = match &event {
            DeviceEvent::CommandStarted { sequence, command } => {
                self.0.pending_commands.borrow_mut().push(PendingCommand {
                    sequence: *sequence,
                    command: command.clone(),
                });
                scope_for_command(command)
            }
            DeviceEvent::CommandConfirmed { sequence, command } => {
                remove_pending(&self.0.pending_commands, Some(*sequence), command);
                scope_for_command(command)
            }
            DeviceEvent::CommandFailed {
                sequence, command, ..
            } => {
                remove_pending(&self.0.pending_commands, *sequence, command);
                scope_for_command(command)
            }
            _ => {
                let scope = scope_for_event(&event);
                apply_confirmed_event(&mut self.0.confirmed_snapshot.borrow_mut(), &event);
                if matches!(
                    event,
                    DeviceEvent::ConnectionChanged(value)
                        if value != nothing_core::ConnectionState::Ready
                ) {
                    self.0.pending_commands.borrow_mut().clear();
                }
                scope
            }
        };
        let mut displayed = self.0.confirmed_snapshot.borrow().clone();
        for pending in self.0.pending_commands.borrow().iter() {
            apply_optimistic_command(&mut displayed, &pending.command);
        }
        *self.0.snapshot.borrow_mut() = displayed;
        self.refresh(scope);
    }

    fn refresh(&self, scope: RefreshScope) {
        let snapshot = self.0.snapshot.borrow();
        let view = ui_state::map_snapshot(&snapshot, &DeviceCapabilities::b171());
        self.0.status.set_label(&view.status);
        let updating = !self.0.pending_commands.borrow().is_empty();
        self.0.spinner.set_spinning(view.spinner || updating);
        self.0.spinner.set_visible(view.spinner || updating);
        for widget in &self.0.write_widgets {
            widget.set_sensitive(view.controls_enabled);
        }
        if matches!(scope, RefreshScope::All | RefreshScope::Battery) {
            self.0
                .left_battery
                .set_label(&battery_text("LEFT", snapshot.battery.left));
            self.0
                .right_battery
                .set_label(&battery_text("RIGHT", snapshot.battery.right));
            self.0
                .case_battery
                .set_label(&battery_text("CASE", snapshot.battery.case));
        }
        if matches!(scope, RefreshScope::All | RefreshScope::Wear) {
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
        }
        if matches!(scope, RefreshScope::All | RefreshScope::Anc) {
            self.0.noise_refs.refresh(&snapshot);
        }
        if matches!(scope, RefreshScope::All | RefreshScope::Firmware) {
            self.0.firmware.set_label(
                snapshot
                    .firmware
                    .as_deref()
                    .unwrap_or("Waiting for device…"),
            );
        }
        self.0.updating_controls.set(true);
        if matches!(scope, RefreshScope::All | RefreshScope::Equalizer) {
            self.0.equalizer_refs.refresh(&snapshot);
        }
        if matches!(scope, RefreshScope::All | RefreshScope::Bass) {
            if let Some(level) = snapshot.bass_enhance {
                self.0.more_refs.bass_switch.set_active(true);
                self.0.more_refs.bass_scale.set_value(f64::from(level));
            } else {
                self.0.more_refs.bass_switch.set_active(false);
            }
        }
        if matches!(scope, RefreshScope::All | RefreshScope::InEar) {
            self.0
                .more_refs
                .in_ear_switch
                .set_active(snapshot.in_ear_detection.unwrap_or(false));
        }
        if matches!(scope, RefreshScope::All | RefreshScope::LowLag) {
            self.0
                .more_refs
                .low_lag_switch
                .set_active(snapshot.low_lag.unwrap_or(false));
        }
        if matches!(scope, RefreshScope::All | RefreshScope::DualConnection) {
            self.0
                .more_refs
                .dual_switch
                .set_active(snapshot.dual_connection.unwrap_or(false));
            self.0.more_refs.refresh_dual_connection(&snapshot);
        }
        if matches!(scope, RefreshScope::All | RefreshScope::Codec) {
            self.0
                .more_refs
                .codec
                .set_selected(match snapshot.audio_codec.unwrap_or_default() {
                    nothing_core::AudioCodec::Default => 0,
                    nothing_core::AudioCodec::Lhdc => 1,
                    nothing_core::AudioCodec::Ldac => 2,
                });
        }
        self.0.updating_controls.set(false);
    }
}

fn remove_pending(
    pending: &RefCell<Vec<PendingCommand>>,
    sequence: Option<u8>,
    command: &DeviceCommand,
) {
    let mut pending = pending.borrow_mut();
    let index = sequence
        .and_then(|sequence| pending.iter().position(|item| item.sequence == sequence))
        .or_else(|| pending.iter().position(|item| item.command == *command));
    if let Some(index) = index {
        pending.remove(index);
    }
}

fn apply_confirmed_event(snapshot: &mut DeviceSnapshot, event: &DeviceEvent) {
    match event {
        DeviceEvent::ConnectionChanged(value) => snapshot.connection = *value,
        DeviceEvent::Snapshot(value) => {
            let dual_devices = snapshot.dual_devices.clone();
            *snapshot = value.clone();
            if snapshot.dual_devices.is_empty() {
                snapshot.dual_devices = dual_devices;
            }
        }
        DeviceEvent::Battery(value) => snapshot.battery = value.clone(),
        DeviceEvent::Wear(value) => snapshot.wear = *value,
        DeviceEvent::Anc { mode, level } => {
            snapshot.anc_mode = *mode;
            snapshot.anc_level = *level;
        }
        DeviceEvent::Eq(value) => snapshot.eq_preset = *value,
        DeviceEvent::CustomEq(value) => snapshot.custom_eq = *value,
        DeviceEvent::AdvancedEqEnabled(value) => snapshot.advanced_eq_enabled = Some(*value),
        DeviceEvent::AdvancedEqProfile(value) => {
            snapshot.advanced_eq_profile = Some(value.clone());
        }
        DeviceEvent::Gestures(value) => snapshot.gestures = value.clone(),
        DeviceEvent::BassEnhance(value) => snapshot.bass_enhance = *value,
        DeviceEvent::InEarDetection(value) => snapshot.in_ear_detection = Some(*value),
        DeviceEvent::LowLag(value) => snapshot.low_lag = Some(*value),
        DeviceEvent::AudioCodec(value) => snapshot.audio_codec = Some(*value),
        DeviceEvent::DualConnection(value) => snapshot.dual_connection = Some(*value),
        DeviceEvent::DualConnectionDevices(value) => snapshot.dual_devices = value.clone(),
        DeviceEvent::Firmware(value) => snapshot.firmware = Some(value.clone()),
        _ => {}
    }
}

fn apply_optimistic_command(snapshot: &mut DeviceSnapshot, command: &DeviceCommand) {
    match command {
        DeviceCommand::SetAnc { mode, level } => {
            snapshot.anc_mode = *mode;
            snapshot.anc_level = *level;
        }
        DeviceCommand::SetEqPreset(value) => snapshot.eq_preset = *value,
        DeviceCommand::SetCustomEq(value) => snapshot.custom_eq = *value,
        DeviceCommand::SetAdvancedEqEnabled(value) => snapshot.advanced_eq_enabled = Some(*value),
        DeviceCommand::SetAdvancedEqProfile(value) => {
            snapshot.advanced_eq_profile = Some((**value).clone());
        }
        DeviceCommand::SetGesture {
            side,
            gesture,
            action,
        } => {
            snapshot.gestures.insert((*side, *gesture), *action);
        }
        DeviceCommand::SetBassEnhance(value) => snapshot.bass_enhance = *value,
        DeviceCommand::SetInEarDetection(value) => snapshot.in_ear_detection = Some(*value),
        DeviceCommand::SetLowLag(value) => snapshot.low_lag = Some(*value),
        DeviceCommand::SetDualConnection(value) => snapshot.dual_connection = Some(*value),
        DeviceCommand::SetDualConnectionDevice { connect, address } => {
            let address = address
                .iter()
                .map(|byte| format!("{byte:02X}"))
                .collect::<Vec<_>>()
                .join(":");
            if let Some(device) = snapshot
                .dual_devices
                .iter_mut()
                .find(|device| device.address == address)
            {
                device.connected = *connect;
            }
        }
        DeviceCommand::SetAudioCodec(value) => snapshot.audio_codec = Some(*value),
        _ => {}
    }
}

fn scope_for_event(event: &DeviceEvent) -> RefreshScope {
    match event {
        DeviceEvent::ConnectionChanged(_) | DeviceEvent::Snapshot(_) => RefreshScope::All,
        DeviceEvent::Battery(_) => RefreshScope::Battery,
        DeviceEvent::Wear(_) => RefreshScope::Wear,
        DeviceEvent::Anc { .. } => RefreshScope::Anc,
        DeviceEvent::Eq(_)
        | DeviceEvent::CustomEq(_)
        | DeviceEvent::AdvancedEqEnabled(_)
        | DeviceEvent::AdvancedEqProfile(_) => RefreshScope::Equalizer,
        DeviceEvent::BassEnhance(_) => RefreshScope::Bass,
        DeviceEvent::InEarDetection(_) => RefreshScope::InEar,
        DeviceEvent::LowLag(_) => RefreshScope::LowLag,
        DeviceEvent::DualConnection(_) | DeviceEvent::DualConnectionDevices(_) => {
            RefreshScope::DualConnection
        }
        DeviceEvent::AudioCodec(_) => RefreshScope::Codec,
        DeviceEvent::Firmware(_) => RefreshScope::Firmware,
        _ => RefreshScope::Header,
    }
}

fn scope_for_command(command: &DeviceCommand) -> RefreshScope {
    match command {
        DeviceCommand::SetAnc { .. } => RefreshScope::Anc,
        DeviceCommand::SetEqPreset(_)
        | DeviceCommand::SetCustomEq(_)
        | DeviceCommand::SetAdvancedEqEnabled(_)
        | DeviceCommand::SetAdvancedEqProfile(_) => RefreshScope::Equalizer,
        DeviceCommand::SetBassEnhance(_) => RefreshScope::Bass,
        DeviceCommand::SetInEarDetection(_) => RefreshScope::InEar,
        DeviceCommand::SetLowLag(_) => RefreshScope::LowLag,
        DeviceCommand::SetDualConnection(_) | DeviceCommand::SetDualConnectionDevice { .. } => {
            RefreshScope::DualConnection
        }
        DeviceCommand::SetAudioCodec(_) => RefreshScope::Codec,
        _ => RefreshScope::Header,
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
        .default_width(440)
        .default_height(780)
        .width_request(390)
        .height_request(560)
        .build();
    window.set_resizable(false);
    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&artwork::logo_label()));
    let status = gtk::Label::builder()
        .label("Disconnected")
        .css_classes(["status-pill"])
        .build();
    let spinner = gtk::Spinner::new();
    spinner.set_visible(false);
    let spinner_slot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spinner_slot.set_size_request(24, 24);
    spinner_slot.append(&spinner);
    let status_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    status_box.append(&status);
    header.pack_start(&spinner_slot);
    header.pack_end(&status_box);
    toolbar.add_top_bar(&header);

    let toast = adw::ToastOverlay::new();
    let layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
    toast.set_child(Some(&layout));
    toolbar.set_content(Some(&toast));
    window.set_content(Some(&toolbar));
    let stack = gtk::Stack::builder()
        .hexpand(true)
        .vexpand(true)
        .transition_type(gtk::StackTransitionType::Crossfade)
        .build();
    layout.append(&stack);
    layout.append(&bottom_navigation(&stack));

    let snapshot = Rc::new(RefCell::new(DeviceSnapshot::default()));
    let updating_controls = Rc::new(Cell::new(false));
    let mut write_widgets = Vec::new();
    let overview = overview_page(&commands, &mut write_widgets);
    let overview_refs = overview.1;
    stack.add_titled(&overview.0, Some("overview"), "Overview");
    let noise = noise_page(&commands, &mut write_widgets);
    stack.add_titled(&noise.0, Some("noise"), "Noise control");
    let equalizer = equalizer_page(
        &commands,
        &mut write_widgets,
        paths.clone(),
        updating_controls.clone(),
    );
    stack.add_titled(&equalizer.0, Some("equalizer"), "Equalizer");
    stack.add_titled(
        &controls_page(&commands, &mut write_widgets),
        Some("controls"),
        "Controls",
    );
    let more = more_page(
        &commands,
        &mut write_widgets,
        MorePageDeps {
            snapshot: snapshot.clone(),
            updating_controls: updating_controls.clone(),
            toast: toast.clone(),
            config,
            paths,
            firmware: overview_refs.firmware.clone(),
        },
    );
    stack.add_titled(&more.0, Some("more"), "More");
    let more_refs = more.1;

    let shell = Shell(Rc::new(ShellInner {
        window,
        toast,
        confirmed_snapshot: RefCell::new(DeviceSnapshot::default()),
        snapshot,
        pending_commands: RefCell::new(Vec::new()),
        status,
        spinner,
        left_battery: overview_refs.left,
        right_battery: overview_refs.right,
        case_battery: overview_refs.case,
        wear: overview_refs.wear,
        firmware: overview_refs.firmware,
        write_widgets,
        updating_controls,
        noise_refs: noise.1,
        equalizer_refs: equalizer.1,
        more_refs,
    }));
    for widget in &shell.0.write_widgets {
        widget.set_sensitive(false);
    }
    shell
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_commands_overlay_and_rollback_to_confirmed_state() {
        let confirmed = DeviceSnapshot {
            bass_enhance: Some(2),
            ..DeviceSnapshot::default()
        };
        let pending = RefCell::new(vec![PendingCommand {
            sequence: 7,
            command: DeviceCommand::SetBassEnhance(Some(5)),
        }]);
        let mut displayed = confirmed.clone();
        for command in pending.borrow().iter() {
            apply_optimistic_command(&mut displayed, &command.command);
        }
        assert_eq!(displayed.bass_enhance, Some(5));

        remove_pending(&pending, Some(7), &DeviceCommand::SetBassEnhance(Some(5)));
        let mut displayed = confirmed;
        for command in pending.borrow().iter() {
            apply_optimistic_command(&mut displayed, &command.command);
        }
        assert_eq!(displayed.bass_enhance, Some(2));
    }
}
