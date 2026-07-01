use super::components::{
    battery_label, command_button, gesture_name, page_box, page_title, scrolled, section,
    unsupported,
};
use adw::prelude::*;
use nothing_core::{
    AdvancedEqProfile, AncLevel, AncMode, AppConfig, AudioCodec, DeviceCommand, DeviceSnapshot,
    DualConnectionDevice, EarbudSide, EqBand, EqPreset, EqProfileStore, Gesture, GestureAction,
    Paths,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::mpsc,
};

pub(super) struct OverviewRefs {
    pub(super) left: gtk::Label,
    pub(super) right: gtk::Label,
    pub(super) case: gtk::Label,
    pub(super) wear: gtk::Label,
    pub(super) firmware: gtk::Label,
}

pub(super) struct NoiseRefs {
    mode_buttons: Vec<(AncMode, gtk::Button)>,
    level_buttons: Vec<(AncLevel, gtk::Button)>,
}

impl NoiseRefs {
    pub(super) fn refresh(&self, snapshot: &DeviceSnapshot) {
        for (mode, button) in &self.mode_buttons {
            if *mode == snapshot.anc_mode {
                button.add_css_class("control-selected");
            } else {
                button.remove_css_class("control-selected");
            }
        }
        for (level, button) in &self.level_buttons {
            if snapshot.anc_mode == AncMode::NoiseCancellation && *level == snapshot.anc_level {
                button.add_css_class("control-selected");
            } else {
                button.remove_css_class("control-selected");
            }
        }
    }
}

pub(super) struct MoreRefs {
    pub(super) bass_switch: gtk::Switch,
    pub(super) bass_scale: gtk::Scale,
    pub(super) in_ear_switch: gtk::Switch,
    pub(super) low_lag_switch: gtk::Switch,
    pub(super) dual_switch: gtk::Switch,
    dual_devices_revealer: gtk::Revealer,
    dual_count: gtk::Label,
    dual_devices_list: gtk::Box,
    commands: mpsc::Sender<DeviceCommand>,
    pub(super) codec: gtk::DropDown,
}

impl MoreRefs {
    pub(super) fn refresh_dual_connection(&self, snapshot: &DeviceSnapshot) {
        let enabled = snapshot.dual_connection.unwrap_or(false);
        self.dual_devices_revealer.set_reveal_child(enabled);
        populate_dual_devices(
            &self.dual_devices_list,
            &self.dual_count,
            &snapshot.dual_devices,
            &self.commands,
        );
    }
}

pub(super) struct MorePageDeps {
    pub(super) snapshot: Rc<RefCell<DeviceSnapshot>>,
    pub(super) updating_controls: Rc<Cell<bool>>,
    pub(super) toast: adw::ToastOverlay,
    pub(super) config: Rc<RefCell<AppConfig>>,
    pub(super) paths: Option<Paths>,
    pub(super) firmware: gtk::Label,
}

pub(super) fn overview_page(
    commands: &mpsc::Sender<DeviceCommand>,
    writes: &mut Vec<gtk::Widget>,
) -> (gtk::ScrolledWindow, OverviewRefs) {
    let page = page_box();
    let hero = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .css_classes(["hero-card"])
        .build();
    let batteries = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .homogeneous(true)
        .spacing(12)
        .build();
    let left = battery_label("LEFT —");
    let case = battery_label("CASE —");
    let right = battery_label("RIGHT —");
    batteries.append(&left);
    batteries.append(&case);
    batteries.append(&right);
    hero.append(&batteries);
    page.append(&hero);
    let wear = gtk::Label::builder()
        .label("Left: unknown  ·  Right: unknown")
        .halign(gtk::Align::Start)
        .css_classes(["dim-label"])
        .build();
    page.append(&wear);
    let quick = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .build();
    let anc = section("QUICK NOISE CONTROL", "Choose a listening mode");
    let anc_button = command_button(
        "ANC · HIGH",
        DeviceCommand::SetAnc {
            mode: AncMode::NoiseCancellation,
            level: AncLevel::High,
        },
        commands,
    );
    writes.push(anc_button.clone().upcast());
    anc.append(&anc_button);
    quick.append(&anc);
    let eq = section("QUICK EQUALIZER", "Return to a neutral profile");
    let eq_button = command_button(
        "BALANCED",
        DeviceCommand::SetEqPreset(EqPreset::Balanced),
        commands,
    );
    writes.push(eq_button.clone().upcast());
    eq.append(&eq_button);
    quick.append(&eq);
    page.append(&quick);
    let firmware = gtk::Label::new(Some("Waiting for device…"));
    (
        scrolled(page),
        OverviewRefs {
            left,
            right,
            case,
            wear,
            firmware,
        },
    )
}

pub(super) fn noise_page(
    commands: &mpsc::Sender<DeviceCommand>,
    writes: &mut Vec<gtk::Widget>,
) -> (gtk::ScrolledWindow, NoiseRefs) {
    let page = page_box();
    page.append(&page_title(
        "NOISE CONTROL",
        "Control how much of the outside world you hear.",
    ));
    let modes = section("MODE", "Choose a listening mode.");
    let mut mode_buttons = Vec::new();
    for (label, mode) in [
        ("Off", AncMode::Off),
        ("Transparency", AncMode::Transparency),
        ("Noise cancellation", AncMode::NoiseCancellation),
    ] {
        let button = command_button(
            label,
            DeviceCommand::SetAnc {
                mode,
                level: AncLevel::High,
            },
            commands,
        );
        writes.push(button.clone().upcast());
        modes.append(&button);
        mode_buttons.push((mode, button));
    }
    page.append(&modes);
    let strength = section(
        "ANC STRENGTH",
        "Adaptive responds to changing ambient sound.",
    );
    let mut level_buttons = Vec::new();
    for (label, level) in [
        ("High", AncLevel::High),
        ("Mid", AncLevel::Mid),
        ("Low", AncLevel::Low),
        ("Adaptive", AncLevel::Adaptive),
    ] {
        let button = command_button(
            label,
            DeviceCommand::SetAnc {
                mode: AncMode::NoiseCancellation,
                level,
            },
            commands,
        );
        writes.push(button.clone().upcast());
        strength.append(&button);
        level_buttons.push((level, button));
    }
    page.append(&strength);
    page.append(&unsupported(
        "Personalized ANC",
        "Not exposed by the verified B171 firmware profile.",
    ));
    (
        scrolled(page),
        NoiseRefs {
            mode_buttons,
            level_buttons,
        },
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SimpleEqState {
    gains: [f32; 3],
}

impl Default for SimpleEqState {
    fn default() -> Self {
        Self { gains: [0.0; 3] }
    }
}

impl SimpleEqState {
    fn from_gains(gains: [f32; 3]) -> Self {
        Self {
            gains: gains.map(|gain| gain.clamp(-6.0, 6.0)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct AdvancedUiState {
    name: String,
    gains: [f32; 8],
    frequencies: [f32; 8],
    qs: [f32; 8],
    selected: usize,
}

impl AdvancedUiState {
    fn neutral(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            gains: [0.0; 8],
            frequencies: AdvancedEqProfile::FREQUENCIES,
            qs: [1.0; 8],
            selected: 0,
        }
    }

    fn from_profile(profile: &AdvancedEqProfile) -> Self {
        let mut state = Self::neutral(profile.name.clone());
        for (index, band) in profile.bands.iter().take(8).enumerate() {
            let (min_frequency, max_frequency) = AdvancedEqProfile::FREQUENCY_RANGES[index];
            state.gains[index] = band.gain_db.clamp(-12.0, 12.0);
            state.frequencies[index] = band.frequency_hz.clamp(min_frequency, max_frequency);
            state.qs[index] = band.q.clamp(0.1, 10.0);
        }
        state
    }

    fn profile(&self) -> AdvancedEqProfile {
        AdvancedEqProfile {
            name: self.name.clone(),
            bands: self
                .frequencies
                .iter()
                .enumerate()
                .map(|(index, frequency_hz)| EqBand {
                    frequency_hz: *frequency_hz,
                    gain_db: self.gains[index],
                    q: self.qs[index],
                })
                .collect(),
        }
    }
}

pub(super) struct EqualizerRefs {
    simple_tab: gtk::Button,
    advanced_tab: gtk::Button,
    stack: gtk::Stack,
    simple_state: Rc<RefCell<SimpleEqState>>,
    simple_graph: gtk::DrawingArea,
    simple_scales: Vec<gtk::Scale>,
    preset_buttons: Vec<(EqPreset, gtk::Button)>,
    advanced_state: Rc<RefCell<AdvancedUiState>>,
    advanced_graph: gtk::DrawingArea,
    advanced_gain_scales: Vec<gtk::Scale>,
    advanced_gain_labels: Vec<gtk::Label>,
    advanced_frequency_labels: Vec<gtk::Label>,
    frequency_scale: gtk::Scale,
    q_scale: gtk::Scale,
}

impl EqualizerRefs {
    pub(super) fn refresh(&self, snapshot: &DeviceSnapshot) {
        let simple = SimpleEqState::from_gains(snapshot.custom_eq);
        *self.simple_state.borrow_mut() = simple;
        set_simple_controls(simple, &self.simple_scales, &self.simple_graph);

        let advanced_active = snapshot.advanced_eq_enabled.unwrap_or(false);
        let active_preset = if advanced_active {
            EqPreset::Advanced
        } else {
            snapshot.eq_preset
        };
        update_preset_buttons(&self.preset_buttons, active_preset);
        if advanced_active {
            set_eq_tab(
                &self.stack,
                &self.simple_tab,
                &self.advanced_tab,
                "advanced",
            );
        }

        if let Some(profile) = &snapshot.advanced_eq_profile {
            let selected = self.advanced_state.borrow().selected.min(7);
            let name = self.advanced_state.borrow().name.clone();
            let mut state = AdvancedUiState::from_profile(profile);
            state.name = name;
            state.selected = selected;
            *self.advanced_state.borrow_mut() = state;
            set_advanced_controls(
                &self.advanced_state.borrow(),
                &self.advanced_gain_scales,
                &self.advanced_gain_labels,
                &self.advanced_frequency_labels,
                &self.frequency_scale,
                &self.q_scale,
                &self.advanced_graph,
            );
        }
    }
}

pub(super) fn equalizer_page(
    commands: &mpsc::Sender<DeviceCommand>,
    writes: &mut Vec<gtk::Widget>,
    paths: Option<Paths>,
    updating_controls: Rc<Cell<bool>>,
) -> (gtk::ScrolledWindow, EqualizerRefs) {
    let page = page_box();
    page.append(&page_title(
        "EQUALIZER",
        "Shape the sound. Profiles stay on this computer.",
    ));

    let simple_state = Rc::new(RefCell::new(SimpleEqState::default()));
    let profile_store = Rc::new(RefCell::new(load_profile_store(paths.as_ref())));
    let initial_profile = profile_store
        .borrow()
        .0
        .first()
        .cloned()
        .unwrap_or_else(|| neutral_profile("Custom 1"));
    let advanced_state = Rc::new(RefCell::new(AdvancedUiState::from_profile(
        &initial_profile,
    )));
    let current_profile = Rc::new(Cell::new(0_usize));
    let undo_stack: Rc<RefCell<Vec<AdvancedUiState>>> = Rc::new(RefCell::new(Vec::new()));
    let redo_stack: Rc<RefCell<Vec<AdvancedUiState>>> = Rc::new(RefCell::new(Vec::new()));

    let tab_row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .homogeneous(true)
        .spacing(0)
        .css_classes(["eq-tab-row"])
        .build();
    let simple_tab = gtk::Button::with_label("Simple");
    simple_tab.add_css_class("eq-tab-button");
    simple_tab.add_css_class("eq-tab-active");
    let advanced_tab = gtk::Button::with_label("Advanced");
    advanced_tab.add_css_class("eq-tab-button");
    tab_row.append(&simple_tab);
    tab_row.append(&advanced_tab);
    page.append(&tab_row);

    let stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::SlideLeftRight)
        .build();
    let simple_tab_for_click = simple_tab.clone();
    let advanced_tab_for_click = advanced_tab.clone();
    let stack_for_click = stack.clone();
    simple_tab.connect_clicked(move |_| {
        set_eq_tab(
            &stack_for_click,
            &simple_tab_for_click,
            &advanced_tab_for_click,
            "simple",
        );
    });
    let simple_tab_for_click = simple_tab.clone();
    let advanced_tab_for_click = advanced_tab.clone();
    let stack_for_click = stack.clone();
    advanced_tab.connect_clicked(move |_| {
        set_eq_tab(
            &stack_for_click,
            &simple_tab_for_click,
            &advanced_tab_for_click,
            "advanced",
        );
    });

    let simple_panel = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(18)
        .css_classes(["eq-panel"])
        .build();
    let simple_graph = gtk::DrawingArea::builder()
        .content_width(328)
        .content_height(328)
        .hexpand(false)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .css_classes(["eq-graph", "eq-radar"])
        .build();
    simple_graph.set_size_request(328, 328);
    {
        let state = simple_state.clone();
        simple_graph.set_draw_func(move |_, cr, width, height| {
            draw_simple_radar(cr, width, height, &state.borrow());
        });
    }
    simple_panel.append(&simple_graph);

    let simple_controls = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(10)
        .css_classes(["eq-control-group"])
        .build();
    let mut simple_scales = Vec::new();
    for (index, label) in ["BASS", "MID", "TREBLE"].into_iter().enumerate() {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row.append(
            &gtk::Label::builder()
                .label(label)
                .width_chars(7)
                .halign(gtk::Align::Start)
                .css_classes(["caption", "dim-label"])
                .build(),
        );
        let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, -6.0, 6.0, 0.5);
        scale.set_hexpand(true);
        scale.set_draw_value(true);
        scale.set_digits(1);
        let sender = commands.clone();
        let state = simple_state.clone();
        let graph = simple_graph.clone();
        let updating = updating_controls.clone();
        scale.connect_value_changed(move |scale| {
            if updating.get() {
                return;
            }
            state.borrow_mut().gains[index] = scale.value() as f32;
            graph.queue_draw();
            send_simple_custom(&sender, &state.borrow());
        });
        writes.push(scale.clone().upcast());
        row.append(&scale);
        simple_controls.append(&row);
        simple_scales.push(scale);
    }
    simple_panel.append(&simple_controls);

    for controller in simple_graph_controllers(
        &simple_graph,
        &simple_state,
        &simple_scales,
        commands,
        &updating_controls,
    ) {
        simple_graph.add_controller(controller);
    }

    let preset_grid = gtk::Grid::builder()
        .column_spacing(10)
        .row_spacing(10)
        .css_classes(["eq-preset-grid"])
        .build();
    let mut preset_buttons = Vec::new();
    for (index, (label, preset)) in [
        ("Balanced", EqPreset::Balanced),
        ("More bass", EqPreset::MoreBass),
        ("More treble", EqPreset::MoreTreble),
        ("Voice", EqPreset::Voice),
        ("Custom", EqPreset::Custom),
    ]
    .into_iter()
    .enumerate()
    {
        let button = gtk::Button::with_label(label);
        button.add_css_class("eq-preset-button");
        button.set_hexpand(true);
        let sender = commands.clone();
        let state = simple_state.clone();
        let updating = updating_controls.clone();
        button.connect_clicked(move |_| {
            if updating.get() {
                return;
            }
            let _ = sender.send(DeviceCommand::SetAdvancedEqEnabled(false));
            let _ = sender.send(DeviceCommand::SetEqPreset(preset));
            if preset == EqPreset::Custom {
                let _ = sender.send(DeviceCommand::SetCustomEq(state.borrow().gains));
            }
        });
        writes.push(button.clone().upcast());
        preset_grid.attach(&button, (index % 2) as i32, (index / 2) as i32, 1, 1);
        preset_buttons.push((preset, button));
    }
    update_preset_buttons(&preset_buttons, EqPreset::Balanced);
    simple_panel.append(&preset_grid);
    stack.add_titled(&simple_panel, Some("simple"), "Simple");

    let advanced_panel = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(16)
        .css_classes(["eq-panel"])
        .build();
    let profile_header = gtk::Box::new(gtk::Orientation::Vertical, 8);
    profile_header.append(
        &gtk::Label::builder()
            .label("PROFILES")
            .halign(gtk::Align::Start)
            .css_classes(["caption", "dim-label"])
            .build(),
    );
    let profile_row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .css_classes(["eq-profile-row"])
        .build();
    let add_profile = icon_button("list-add-symbolic", "Add profile");
    let delete_profile = icon_button("user-trash-symbolic", "Delete profile");
    let profile_names: Vec<String> = profile_store
        .borrow()
        .0
        .iter()
        .map(|profile| profile.name.clone())
        .collect();
    let profile_name_refs: Vec<&str> = profile_names.iter().map(String::as_str).collect();
    let profile_model = gtk::StringList::new(&profile_name_refs);
    let profile_dropdown = gtk::DropDown::new(Some(profile_model.clone()), None::<gtk::Expression>);
    profile_dropdown.set_hexpand(true);
    profile_row.append(&add_profile);
    profile_row.append(&profile_dropdown);
    profile_row.append(&delete_profile);
    profile_header.append(&profile_row);
    advanced_panel.append(&profile_header);
    writes.push(add_profile.clone().upcast());
    writes.push(delete_profile.clone().upcast());
    writes.push(profile_dropdown.clone().upcast());

    let advanced_graph = gtk::DrawingArea::builder()
        .content_width(328)
        .content_height(126)
        .hexpand(true)
        .css_classes(["eq-graph", "eq-waveform"])
        .build();
    {
        let state = advanced_state.clone();
        advanced_graph.set_draw_func(move |_, cr, width, height| {
            draw_advanced_waveform(cr, width, height, &state.borrow());
        });
    }
    advanced_panel.append(&advanced_graph);

    let gain_area = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .css_classes(["eq-gain-area"])
        .build();
    let gain_axis = gtk::Box::new(gtk::Orientation::Vertical, 0);
    gain_axis.append(
        &gtk::Label::builder()
            .label("+")
            .css_classes(["caption", "dim-label"])
            .build(),
    );
    gain_axis.append(&gtk::Box::builder().vexpand(true).build());
    gain_axis.append(
        &gtk::Label::builder()
            .label("0")
            .css_classes(["caption", "dim-label"])
            .build(),
    );
    gain_axis.append(&gtk::Box::builder().vexpand(true).build());
    gain_axis.append(
        &gtk::Label::builder()
            .label("-")
            .css_classes(["caption", "dim-label"])
            .build(),
    );
    gain_area.append(&gain_axis);
    let gain_columns = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .homogeneous(true)
        .spacing(8)
        .hexpand(true)
        .build();
    let mut advanced_gain_scales = Vec::new();
    let mut advanced_gain_labels = Vec::new();
    let mut advanced_frequency_labels = Vec::new();
    for index in 0..8 {
        let column = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .css_classes(["eq-gain-column"])
            .build();
        let value_label = gtk::Label::builder()
            .label("0")
            .css_classes(["caption", "eq-gain-value"])
            .build();
        let gain = gtk::Scale::with_range(gtk::Orientation::Vertical, -12.0, 12.0, 0.5);
        gain.add_css_class("eq-gain-scale");
        gain.set_size_request(28, 192);
        gain.set_draw_value(false);
        gain.set_vexpand(false);
        gain.set_tooltip_text(Some("Gain in decibels"));
        if index == 0 {
            gain.add_css_class("eq-selected-band");
        }
        let frequency_label = gtk::Label::builder()
            .label(format_frequency(AdvancedEqProfile::FREQUENCIES[index]))
            .css_classes(["caption", "dim-label"])
            .build();
        column.append(&value_label);
        column.append(&gain);
        column.append(&frequency_label);
        gain_columns.append(&column);
        advanced_gain_labels.push(value_label);
        advanced_frequency_labels.push(frequency_label);
        advanced_gain_scales.push(gain);
    }
    gain_area.append(&gain_columns);
    advanced_panel.append(&gain_area);

    let selected_controls = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .css_classes(["eq-control-group"])
        .build();
    let frequency_scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 20.0, 99.0, 1.0);
    frequency_scale.set_hexpand(true);
    frequency_scale.set_draw_value(true);
    frequency_scale.set_digits(0);
    frequency_scale.set_tooltip_text(Some("Selected band frequency"));
    selected_controls.append(&labeled_control("FREQUENCY", &frequency_scale));
    let q_scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.1, 10.0, 0.1);
    q_scale.set_hexpand(true);
    q_scale.set_draw_value(true);
    q_scale.set_digits(1);
    q_scale.set_tooltip_text(Some("Selected band Q factor"));
    selected_controls.append(&labeled_control("Q FACTOR", &q_scale));
    advanced_panel.append(&selected_controls);
    writes.push(frequency_scale.clone().upcast());
    writes.push(q_scale.clone().upcast());

    for (index, gain) in advanced_gain_scales.iter().enumerate() {
        let state = advanced_state.clone();
        let graph = advanced_graph.clone();
        let labels = advanced_gain_labels.clone();
        let gain_scales = advanced_gain_scales.clone();
        let frequency_scale_for_gain = frequency_scale.clone();
        let q_scale_for_gain = q_scale.clone();
        let sender = commands.clone();
        let store = profile_store.clone();
        let profile_index = current_profile.clone();
        let paths_for_save = paths.clone();
        let updating = updating_controls.clone();
        let undo = undo_stack.clone();
        let redo = redo_stack.clone();
        gain.connect_value_changed(move |scale| {
            if updating.get() {
                return;
            }
            remember_advanced_change(&state, &undo, &redo);
            {
                let mut state = state.borrow_mut();
                state.selected = index;
                state.gains[index] = scale.value() as f32;
            }
            set_selected_band(&gain_scales, index);
            let current = state.borrow().clone();
            with_control_update(&updating, || {
                set_advanced_controls(
                    &current,
                    &gain_scales,
                    &labels,
                    &[],
                    &frequency_scale_for_gain,
                    &q_scale_for_gain,
                    &graph,
                );
            });
            send_advanced_state(&sender, &state, &store, &profile_index, &paths_for_save);
        });
        writes.push(gain.clone().upcast());
    }

    {
        let state = advanced_state.clone();
        let graph = advanced_graph.clone();
        let gain_scales = advanced_gain_scales.clone();
        let gain_labels = advanced_gain_labels.clone();
        let frequency_labels = advanced_frequency_labels.clone();
        let q_scale_for_frequency = q_scale.clone();
        let sender = commands.clone();
        let store = profile_store.clone();
        let profile_index = current_profile.clone();
        let paths_for_save = paths.clone();
        let updating = updating_controls.clone();
        let undo = undo_stack.clone();
        let redo = redo_stack.clone();
        frequency_scale.connect_value_changed(move |scale| {
            if updating.get() {
                return;
            }
            remember_advanced_change(&state, &undo, &redo);
            {
                let mut state = state.borrow_mut();
                let selected = state.selected.min(7);
                state.frequencies[selected] = scale.value() as f32;
            }
            let current = state.borrow().clone();
            with_control_update(&updating, || {
                set_advanced_controls(
                    &current,
                    &gain_scales,
                    &gain_labels,
                    &frequency_labels,
                    scale,
                    &q_scale_for_frequency,
                    &graph,
                );
            });
            send_advanced_state(&sender, &state, &store, &profile_index, &paths_for_save);
        });
    }
    {
        let state = advanced_state.clone();
        let graph = advanced_graph.clone();
        let gain_scales = advanced_gain_scales.clone();
        let gain_labels = advanced_gain_labels.clone();
        let frequency_labels = advanced_frequency_labels.clone();
        let frequency_scale_for_q = frequency_scale.clone();
        let sender = commands.clone();
        let store = profile_store.clone();
        let profile_index = current_profile.clone();
        let paths_for_save = paths.clone();
        let updating = updating_controls.clone();
        let undo = undo_stack.clone();
        let redo = redo_stack.clone();
        q_scale.connect_value_changed(move |scale| {
            if updating.get() {
                return;
            }
            remember_advanced_change(&state, &undo, &redo);
            {
                let mut state = state.borrow_mut();
                let selected = state.selected.min(7);
                state.qs[selected] = scale.value() as f32;
            }
            let current = state.borrow().clone();
            with_control_update(&updating, || {
                set_advanced_controls(
                    &current,
                    &gain_scales,
                    &gain_labels,
                    &frequency_labels,
                    &frequency_scale_for_q,
                    scale,
                    &graph,
                );
            });
            send_advanced_state(&sender, &state, &store, &profile_index, &paths_for_save);
        });
    }

    let action_row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .homogeneous(true)
        .spacing(10)
        .build();
    let reset = gtk::Button::with_label("Reset");
    reset.add_css_class("eq-action-button");
    let undo = icon_button("edit-undo-symbolic", "Undo");
    let redo = icon_button("edit-redo-symbolic", "Redo");
    action_row.append(&reset);
    action_row.append(&undo);
    action_row.append(&redo);
    advanced_panel.append(&action_row);
    writes.push(reset.clone().upcast());
    writes.push(undo.clone().upcast());
    writes.push(redo.clone().upcast());

    {
        let state = advanced_state.clone();
        let graph = advanced_graph.clone();
        let gain_scales = advanced_gain_scales.clone();
        let gain_labels = advanced_gain_labels.clone();
        let frequency_labels = advanced_frequency_labels.clone();
        let frequency_scale = frequency_scale.clone();
        let q_scale = q_scale.clone();
        let sender = commands.clone();
        let store = profile_store.clone();
        let profile_index = current_profile.clone();
        let paths_for_save = paths.clone();
        let updating = updating_controls.clone();
        let undo_stack = undo_stack.clone();
        let redo_stack = redo_stack.clone();
        reset.connect_clicked(move |_| {
            if updating.get() {
                return;
            }
            remember_advanced_change(&state, &undo_stack, &redo_stack);
            let name = state.borrow().name.clone();
            *state.borrow_mut() = AdvancedUiState::neutral(name);
            let current = state.borrow().clone();
            with_control_update(&updating, || {
                set_advanced_controls(
                    &current,
                    &gain_scales,
                    &gain_labels,
                    &frequency_labels,
                    &frequency_scale,
                    &q_scale,
                    &graph,
                );
            });
            send_advanced_state(&sender, &state, &store, &profile_index, &paths_for_save);
        });
    }
    {
        let state = advanced_state.clone();
        let graph = advanced_graph.clone();
        let gain_scales = advanced_gain_scales.clone();
        let gain_labels = advanced_gain_labels.clone();
        let frequency_labels = advanced_frequency_labels.clone();
        let frequency_scale = frequency_scale.clone();
        let q_scale = q_scale.clone();
        let sender = commands.clone();
        let store = profile_store.clone();
        let profile_index = current_profile.clone();
        let paths_for_save = paths.clone();
        let undo_stack = undo_stack.clone();
        let redo_stack = redo_stack.clone();
        let updating = updating_controls.clone();
        undo.connect_clicked(move |_| {
            with_control_update(&updating, || {
                apply_history_step(
                    &state,
                    &undo_stack,
                    &redo_stack,
                    AdvancedControlWidgets {
                        gain_scales: &gain_scales,
                        gain_labels: &gain_labels,
                        frequency_labels: &frequency_labels,
                        frequency_scale: &frequency_scale,
                        q_scale: &q_scale,
                        graph: &graph,
                    },
                );
            });
            send_advanced_state(&sender, &state, &store, &profile_index, &paths_for_save);
        });
    }
    {
        let state = advanced_state.clone();
        let graph = advanced_graph.clone();
        let gain_scales = advanced_gain_scales.clone();
        let gain_labels = advanced_gain_labels.clone();
        let frequency_labels = advanced_frequency_labels.clone();
        let frequency_scale = frequency_scale.clone();
        let q_scale = q_scale.clone();
        let sender = commands.clone();
        let store = profile_store.clone();
        let profile_index = current_profile.clone();
        let paths_for_save = paths.clone();
        let undo_stack = undo_stack.clone();
        let redo_stack = redo_stack.clone();
        let updating = updating_controls.clone();
        redo.connect_clicked(move |_| {
            with_control_update(&updating, || {
                apply_history_step(
                    &state,
                    &redo_stack,
                    &undo_stack,
                    AdvancedControlWidgets {
                        gain_scales: &gain_scales,
                        gain_labels: &gain_labels,
                        frequency_labels: &frequency_labels,
                        frequency_scale: &frequency_scale,
                        q_scale: &q_scale,
                        graph: &graph,
                    },
                );
            });
            send_advanced_state(&sender, &state, &store, &profile_index, &paths_for_save);
        });
    }

    {
        let state = advanced_state.clone();
        let graph = advanced_graph.clone();
        let gain_scales = advanced_gain_scales.clone();
        let gain_labels = advanced_gain_labels.clone();
        let frequency_labels = advanced_frequency_labels.clone();
        let frequency_scale = frequency_scale.clone();
        let q_scale = q_scale.clone();
        let sender = commands.clone();
        let store = profile_store.clone();
        let profile_index = current_profile.clone();
        let paths_for_save = paths.clone();
        let updating = updating_controls.clone();
        profile_dropdown.connect_selected_notify(move |dropdown| {
            if updating.get() {
                return;
            }
            let index = dropdown.selected() as usize;
            let Some(profile) = store.borrow().0.get(index).cloned() else {
                return;
            };
            profile_index.set(index);
            *state.borrow_mut() = AdvancedUiState::from_profile(&profile);
            let current = state.borrow().clone();
            with_control_update(&updating, || {
                set_advanced_controls(
                    &current,
                    &gain_scales,
                    &gain_labels,
                    &frequency_labels,
                    &frequency_scale,
                    &q_scale,
                    &graph,
                );
            });
            send_advanced_state(&sender, &state, &store, &profile_index, &paths_for_save);
        });
    }
    {
        let state = advanced_state.clone();
        let model = profile_model.clone();
        let dropdown = profile_dropdown.clone();
        let store = profile_store.clone();
        let profile_index = current_profile.clone();
        let paths_for_save = paths.clone();
        add_profile.connect_clicked(move |_| {
            if store.borrow().0.len() >= 20 {
                return;
            }
            let name = next_profile_name(&store.borrow());
            let mut profile = state.borrow().profile();
            profile.name = name.clone();
            if store.borrow_mut().push(profile).is_ok() {
                model.append(&name);
                let index = store.borrow().0.len().saturating_sub(1);
                profile_index.set(index);
                dropdown.set_selected(index as u32);
                save_profile_store(&store, paths_for_save.as_ref());
            }
        });
    }
    {
        let state = advanced_state.clone();
        let model = profile_model.clone();
        let dropdown = profile_dropdown.clone();
        let store = profile_store.clone();
        let profile_index = current_profile.clone();
        let paths_for_save = paths.clone();
        let graph = advanced_graph.clone();
        let gain_scales = advanced_gain_scales.clone();
        let gain_labels = advanced_gain_labels.clone();
        let frequency_labels = advanced_frequency_labels.clone();
        let frequency_scale = frequency_scale.clone();
        let q_scale = q_scale.clone();
        let updating = updating_controls.clone();
        delete_profile.connect_clicked(move |_| {
            if store.borrow().0.len() <= 1 {
                return;
            }
            let index = profile_index.get().min(store.borrow().0.len() - 1);
            store.borrow_mut().0.remove(index);
            model.remove(index as u32);
            let next_index = index.min(store.borrow().0.len() - 1);
            profile_index.set(next_index);
            dropdown.set_selected(next_index as u32);
            if let Some(profile) = store.borrow().0.get(next_index).cloned() {
                *state.borrow_mut() = AdvancedUiState::from_profile(&profile);
                let current = state.borrow().clone();
                with_control_update(&updating, || {
                    set_advanced_controls(
                        &current,
                        &gain_scales,
                        &gain_labels,
                        &frequency_labels,
                        &frequency_scale,
                        &q_scale,
                        &graph,
                    );
                });
            }
            save_profile_store(&store, paths_for_save.as_ref());
        });
    }

    with_control_update(&updating_controls, || {
        set_advanced_controls(
            &advanced_state.borrow(),
            &advanced_gain_scales,
            &advanced_gain_labels,
            &advanced_frequency_labels,
            &frequency_scale,
            &q_scale,
            &advanced_graph,
        );
    });
    stack.add_titled(&advanced_panel, Some("advanced"), "Advanced");
    page.append(&stack);

    (
        scrolled(page),
        EqualizerRefs {
            simple_tab,
            advanced_tab,
            stack,
            simple_state,
            simple_graph,
            simple_scales,
            preset_buttons,
            advanced_state,
            advanced_graph,
            advanced_gain_scales,
            advanced_gain_labels,
            advanced_frequency_labels,
            frequency_scale,
            q_scale,
        },
    )
}

fn load_profile_store(paths: Option<&Paths>) -> EqProfileStore {
    let mut store = paths
        .and_then(|paths| EqProfileStore::load(paths).ok())
        .unwrap_or_default();
    if store.0.is_empty() {
        let _ = store.push(neutral_profile("Custom 1"));
    }
    store
}

fn neutral_profile(name: &str) -> AdvancedEqProfile {
    AdvancedUiState::neutral(name).profile()
}

fn set_eq_tab(
    stack: &gtk::Stack,
    simple_tab: &gtk::Button,
    advanced_tab: &gtk::Button,
    name: &str,
) {
    stack.set_visible_child_name(name);
    if name == "advanced" {
        simple_tab.remove_css_class("eq-tab-active");
        advanced_tab.add_css_class("eq-tab-active");
    } else {
        advanced_tab.remove_css_class("eq-tab-active");
        simple_tab.add_css_class("eq-tab-active");
    }
}

fn set_simple_controls(state: SimpleEqState, scales: &[gtk::Scale], graph: &gtk::DrawingArea) {
    for (index, scale) in scales.iter().enumerate() {
        scale.set_value(f64::from(state.gains[index]));
    }
    graph.queue_draw();
}

fn update_preset_buttons(buttons: &[(EqPreset, gtk::Button)], selected: EqPreset) {
    for (preset, button) in buttons {
        if *preset == selected {
            button.add_css_class("eq-preset-selected");
        } else {
            button.remove_css_class("eq-preset-selected");
        }
    }
}

fn set_advanced_controls(
    state: &AdvancedUiState,
    gain_scales: &[gtk::Scale],
    gain_labels: &[gtk::Label],
    frequency_labels: &[gtk::Label],
    frequency_scale: &gtk::Scale,
    q_scale: &gtk::Scale,
    graph: &gtk::DrawingArea,
) {
    let selected = state.selected.min(7);
    for (index, scale) in gain_scales.iter().enumerate() {
        scale.set_value(f64::from(state.gains[index]));
        if index == selected {
            scale.add_css_class("eq-selected-band");
        } else {
            scale.remove_css_class("eq-selected-band");
        }
    }
    for (index, label) in gain_labels.iter().enumerate() {
        label.set_label(&format_gain(state.gains[index]));
    }
    for (index, label) in frequency_labels.iter().enumerate() {
        label.set_label(&format_frequency(state.frequencies[index]));
    }
    let (min_frequency, max_frequency) = AdvancedEqProfile::FREQUENCY_RANGES[selected];
    frequency_scale.set_range(f64::from(min_frequency), f64::from(max_frequency));
    frequency_scale.set_value(f64::from(state.frequencies[selected]));
    q_scale.set_value(f64::from(state.qs[selected]));
    graph.queue_draw();
}

fn with_control_update<F>(updating_controls: &Rc<Cell<bool>>, update: F)
where
    F: FnOnce(),
{
    let was_updating = updating_controls.replace(true);
    update();
    updating_controls.set(was_updating);
}

fn set_selected_band(gain_scales: &[gtk::Scale], selected: usize) {
    for (index, scale) in gain_scales.iter().enumerate() {
        if index == selected {
            scale.add_css_class("eq-selected-band");
        } else {
            scale.remove_css_class("eq-selected-band");
        }
    }
}

fn send_simple_custom(sender: &mpsc::Sender<DeviceCommand>, state: &SimpleEqState) {
    let _ = sender.send(DeviceCommand::SetCustomEq(state.gains));
    let _ = sender.send(DeviceCommand::SetAdvancedEqEnabled(false));
    let _ = sender.send(DeviceCommand::SetEqPreset(EqPreset::Custom));
}

fn send_advanced_state(
    sender: &mpsc::Sender<DeviceCommand>,
    state: &Rc<RefCell<AdvancedUiState>>,
    store: &Rc<RefCell<EqProfileStore>>,
    profile_index: &Rc<Cell<usize>>,
    paths: &Option<Paths>,
) {
    let index = profile_index.get();
    let mut profile = state.borrow().profile();
    if let Some(stored) = store.borrow().0.get(index) {
        profile.name = stored.name.clone();
    }
    if let Some(slot) = store.borrow_mut().0.get_mut(index) {
        *slot = profile.clone();
    }
    save_profile_store(store, paths.as_ref());
    let _ = sender.send(DeviceCommand::SetAdvancedEqProfile(Box::new(profile)));
    let _ = sender.send(DeviceCommand::SetAdvancedEqEnabled(true));
}

fn save_profile_store(store: &Rc<RefCell<EqProfileStore>>, paths: Option<&Paths>) {
    if let Some(paths) = paths {
        let _ = store.borrow().save(paths);
    }
}

fn remember_advanced_change(
    state: &Rc<RefCell<AdvancedUiState>>,
    undo_stack: &Rc<RefCell<Vec<AdvancedUiState>>>,
    redo_stack: &Rc<RefCell<Vec<AdvancedUiState>>>,
) {
    let snapshot = state.borrow().clone();
    let mut undo = undo_stack.borrow_mut();
    if undo.last() != Some(&snapshot) {
        undo.push(snapshot);
        if undo.len() > 64 {
            undo.remove(0);
        }
    }
    redo_stack.borrow_mut().clear();
}

struct AdvancedControlWidgets<'a> {
    gain_scales: &'a [gtk::Scale],
    gain_labels: &'a [gtk::Label],
    frequency_labels: &'a [gtk::Label],
    frequency_scale: &'a gtk::Scale,
    q_scale: &'a gtk::Scale,
    graph: &'a gtk::DrawingArea,
}

fn apply_history_step(
    state: &Rc<RefCell<AdvancedUiState>>,
    source: &Rc<RefCell<Vec<AdvancedUiState>>>,
    target: &Rc<RefCell<Vec<AdvancedUiState>>>,
    controls: AdvancedControlWidgets<'_>,
) {
    let Some(next) = source.borrow_mut().pop() else {
        return;
    };
    target.borrow_mut().push(state.borrow().clone());
    *state.borrow_mut() = next;
    set_advanced_controls(
        &state.borrow(),
        controls.gain_scales,
        controls.gain_labels,
        controls.frequency_labels,
        controls.frequency_scale,
        controls.q_scale,
        controls.graph,
    );
}

fn next_profile_name(store: &EqProfileStore) -> String {
    for index in 1..=20 {
        let candidate = format!("Custom {index}");
        if !store.0.iter().any(|profile| profile.name == candidate) {
            return candidate;
        }
    }
    "Custom".into()
}

fn icon_button(icon_name: &str, tooltip: &str) -> gtk::Button {
    let button = gtk::Button::new();
    button.add_css_class("eq-icon-button");
    button.set_tooltip_text(Some(tooltip));
    button.set_child(Some(&gtk::Image::from_icon_name(icon_name)));
    button
}

fn labeled_control(label: &str, scale: &gtk::Scale) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.append(
        &gtk::Label::builder()
            .label(label)
            .width_chars(10)
            .halign(gtk::Align::Start)
            .css_classes(["caption", "dim-label"])
            .build(),
    );
    row.append(scale);
    row
}

fn simple_graph_controllers(
    graph: &gtk::DrawingArea,
    state: &Rc<RefCell<SimpleEqState>>,
    scales: &[gtk::Scale],
    commands: &mpsc::Sender<DeviceCommand>,
    updating_controls: &Rc<Cell<bool>>,
) -> [gtk::EventController; 2] {
    let click = gtk::GestureClick::new();
    {
        let graph = graph.clone();
        let state = state.clone();
        let scales = scales.to_vec();
        let sender = commands.clone();
        let updating = updating_controls.clone();
        click.connect_pressed(move |_, _, x, y| {
            if updating.get() {
                return;
            }
            update_simple_from_point(&graph, &state, &scales, &sender, &updating, x, y);
        });
    }
    let drag = gtk::GestureDrag::new();
    let drag_origin = Rc::new(Cell::new((0.0, 0.0)));
    {
        let origin = drag_origin.clone();
        drag.connect_drag_begin(move |_, x, y| origin.set((x, y)));
    }
    {
        let graph = graph.clone();
        let state = state.clone();
        let scales = scales.to_vec();
        let sender = commands.clone();
        let updating = updating_controls.clone();
        drag.connect_drag_update(move |_, offset_x, offset_y| {
            if updating.get() {
                return;
            }
            let (start_x, start_y) = drag_origin.get();
            update_simple_from_point(
                &graph,
                &state,
                &scales,
                &sender,
                &updating,
                start_x + offset_x,
                start_y + offset_y,
            );
        });
    }
    [click.upcast(), drag.upcast()]
}

fn update_simple_from_point(
    graph: &gtk::DrawingArea,
    state: &Rc<RefCell<SimpleEqState>>,
    scales: &[gtk::Scale],
    sender: &mpsc::Sender<DeviceCommand>,
    updating_controls: &Rc<Cell<bool>>,
    x: f64,
    y: f64,
) {
    let width = f64::from(graph.width()).max(1.0);
    let height = f64::from(graph.height()).max(1.0);
    let center_x = width / 2.0;
    let center_y = height / 2.0;
    let radius = simple_eq_outer_radius(width, height);
    let axes = simple_eq_axes();
    let dx = x - center_x;
    let dy = y - center_y;
    let mut best = 0;
    let mut best_projection = f64::MIN;
    for (index, (_, angle)) in axes.iter().enumerate() {
        let projection = dx * angle.cos() + dy * angle.sin();
        if projection > best_projection {
            best_projection = projection;
            best = index;
        }
    }
    let value = (simple_eq_gain_for_radius(best_projection, radius) * 2.0).round() / 2.0;
    if (f64::from(state.borrow().gains[best]) - value).abs() < f64::EPSILON {
        return;
    }
    state.borrow_mut().gains[best] = value as f32;
    with_control_update(updating_controls, || {
        if let Some(scale) = scales.get(best) {
            scale.set_value(value);
        }
    });
    graph.queue_draw();
    send_simple_custom(sender, &state.borrow());
}

fn simple_eq_axes() -> [(&'static str, f64); 3] {
    [
        ("BASS", 150.0_f64.to_radians()),
        ("MID", (-90.0_f64).to_radians()),
        ("TREBLE", 30.0_f64.to_radians()),
    ]
}

fn simple_eq_outer_radius(width: f64, height: f64) -> f64 {
    width.min(height) * 0.34
}

fn simple_eq_inner_radius(outer_radius: f64) -> f64 {
    outer_radius * 0.25
}

fn simple_eq_gain_for_radius(projected_radius: f64, outer_radius: f64) -> f64 {
    let inner_radius = simple_eq_inner_radius(outer_radius);
    let usable_radius = (outer_radius - inner_radius).max(1.0);
    let normalized = ((projected_radius - inner_radius) / usable_radius).clamp(0.0, 1.0);
    normalized * 12.0 - 6.0
}

fn simple_eq_radius_for_gain(gain: f32, outer_radius: f64) -> f64 {
    let inner_radius = simple_eq_inner_radius(outer_radius);
    let normalized = ((f64::from(gain.clamp(-6.0, 6.0)) + 6.0) / 12.0).clamp(0.0, 1.0);
    inner_radius + normalized * (outer_radius - inner_radius)
}

fn simple_eq_knob_points(
    center_x: f64,
    center_y: f64,
    outer_radius: f64,
    state: &SimpleEqState,
) -> [(f64, f64); 3] {
    let axes = simple_eq_axes();
    std::array::from_fn(|index| {
        let radius = simple_eq_radius_for_gain(state.gains[index], outer_radius);
        let angle = axes[index].1;
        (
            center_x + angle.cos() * radius,
            center_y + angle.sin() * radius,
        )
    })
}

fn draw_simple_radar(cr: &gtk::cairo::Context, width: i32, height: i32, state: &SimpleEqState) {
    let width = f64::from(width);
    let height = f64::from(height);
    let center_x = width / 2.0;
    let center_y = height / 2.0;
    let radius = simple_eq_outer_radius(width, height);
    let inner_radius = simple_eq_inner_radius(radius);
    cr.set_line_width(1.0);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.08);
    for step in 1..=4 {
        cr.arc(
            center_x,
            center_y,
            radius * f64::from(step) / 4.0,
            0.0,
            std::f64::consts::TAU,
        );
        let _ = cr.stroke();
    }
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.16);
    cr.arc(center_x, center_y, inner_radius, 0.0, std::f64::consts::TAU);
    let _ = cr.stroke();

    let axes = simple_eq_axes();
    for (label, angle) in axes {
        let end_x = center_x + angle.cos() * radius;
        let end_y = center_y + angle.sin() * radius;
        cr.move_to(center_x, center_y);
        cr.line_to(end_x, end_y);
        let _ = cr.stroke();
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.62);
        let label_x_offset = if angle.cos() < -0.2 {
            -40.0
        } else if angle.cos() > 0.2 {
            4.0
        } else {
            -18.0
        };
        let label_y_offset = if angle.sin() < -0.5 { -6.0 } else { 10.0 };
        cr.move_to(
            center_x + angle.cos() * (radius + 24.0) + label_x_offset,
            center_y + angle.sin() * (radius + 24.0) + label_y_offset,
        );
        let _ = cr.show_text(label);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.08);
    }

    let knob_points = simple_eq_knob_points(center_x, center_y, radius, state);
    let balance_radius = knob_points
        .iter()
        .map(|(x, y)| ((x - center_x).powi(2) + (y - center_y).powi(2)).sqrt())
        .sum::<f64>()
        / knob_points.len() as f64;
    let balance_radius = balance_radius.clamp(inner_radius, radius);
    if balance_radius.is_finite() {
        let gradient = gtk::cairo::RadialGradient::new(
            center_x,
            center_y,
            inner_radius * 0.35,
            center_x,
            center_y,
            balance_radius,
        );
        gradient.add_color_stop_rgba(0.0, 0.9, 0.1, 0.12, 0.22);
        gradient.add_color_stop_rgba(0.7, 0.9, 0.1, 0.12, 0.08);
        gradient.add_color_stop_rgba(1.0, 0.9, 0.1, 0.12, 0.02);
        let _ = cr.set_source(&gradient);
        cr.arc(
            center_x,
            center_y,
            balance_radius,
            0.0,
            std::f64::consts::TAU,
        );
        let _ = cr.fill();
    }

    cr.set_source_rgba(0.9, 0.1, 0.12, 0.95);
    for (x, y) in knob_points {
        cr.arc(x, y, 9.0, 0.0, std::f64::consts::TAU);
        let _ = cr.fill_preserve();
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.82);
        cr.set_line_width(2.0);
        let _ = cr.stroke();
        cr.set_source_rgba(0.9, 0.1, 0.12, 0.95);
    }
}

fn draw_advanced_waveform(
    cr: &gtk::cairo::Context,
    width: i32,
    height: i32,
    state: &AdvancedUiState,
) {
    let width = f64::from(width);
    let height = f64::from(height);
    let padding_x = 12.0;
    let center_y = height / 2.0;
    let usable_width = (width - padding_x * 2.0).max(1.0);
    cr.set_line_width(1.0);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.08);
    for fraction in [0.25, 0.5, 0.75] {
        let y = height * fraction;
        cr.move_to(padding_x, y);
        cr.line_to(width - padding_x, y);
        let _ = cr.stroke();
    }
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.18);
    cr.move_to(padding_x, center_y);
    cr.line_to(width - padding_x, center_y);
    let _ = cr.stroke();

    let log_min = 20.0_f64.log10();
    let log_max = 20_000.0_f64.log10();
    cr.set_source_rgba(0.9, 0.1, 0.12, 0.95);
    cr.set_line_width(2.4);
    for sample in 0..=180 {
        let t = f64::from(sample) / 180.0;
        let log_frequency = log_min + t * (log_max - log_min);
        let mut gain = 0.0;
        for index in 0..8 {
            let band_log = f64::from(state.frequencies[index]).log10();
            let distance = log_frequency - band_log;
            let width_factor = (0.5 / f64::from(state.qs[index]).sqrt()).clamp(0.08, 0.55);
            let weight = (-(distance * distance) / (2.0 * width_factor * width_factor)).exp();
            gain += f64::from(state.gains[index]) * weight;
        }
        let gain = gain.clamp(-12.0, 12.0);
        let x = padding_x + t * usable_width;
        let y = center_y - (gain / 12.0) * (height * 0.38);
        if sample == 0 {
            cr.move_to(x, y);
        } else {
            cr.line_to(x, y);
        }
    }
    let _ = cr.stroke();
}

pub(super) fn controls_page(
    commands: &mpsc::Sender<DeviceCommand>,
    writes: &mut Vec<gtk::Widget>,
) -> gtk::ScrolledWindow {
    let page = page_box();
    page.append(&page_title(
        "CONTROLS",
        "Set each stem independently. Single pinch remains fixed.",
    ));
    for side in [EarbudSide::Left, EarbudSide::Right] {
        let group = section(
            if side == EarbudSide::Left {
                "LEFT EARBUD"
            } else {
                "RIGHT EARBUD"
            },
            "Single pinch · Play / pause (fixed)",
        );
        for gesture in [
            Gesture::DoublePinch,
            Gesture::TriplePinch,
            Gesture::PinchAndHold,
            Gesture::DoublePinchAndHold,
        ] {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            row.append(
                &gtk::Label::builder()
                    .label(gesture_name(gesture))
                    .hexpand(true)
                    .halign(gtk::Align::Start)
                    .build(),
            );
            let actions: &[&str] = if matches!(gesture, Gesture::DoublePinch | Gesture::TriplePinch)
            {
                &["Skip back", "Skip forward", "Voice assistant", "None"]
            } else {
                &[
                    "Noise control",
                    "Volume up",
                    "Volume down",
                    "Voice assistant",
                    "None",
                ]
            };
            let dropdown = gtk::DropDown::from_strings(actions);
            let sender = commands.clone();
            dropdown.connect_selected_notify(move |dropdown| {
                let action = if matches!(gesture, Gesture::DoublePinch | Gesture::TriplePinch) {
                    [
                        GestureAction::SkipBack,
                        GestureAction::SkipForward,
                        GestureAction::VoiceAssistant,
                        GestureAction::None,
                    ][dropdown.selected() as usize]
                } else {
                    [
                        GestureAction::NoiseControl,
                        GestureAction::VolumeUp,
                        GestureAction::VolumeDown,
                        GestureAction::VoiceAssistant,
                        GestureAction::None,
                    ][dropdown.selected() as usize]
                };
                let _ = sender.send(DeviceCommand::SetGesture {
                    side,
                    gesture,
                    action,
                });
            });
            writes.push(dropdown.clone().upcast());
            row.append(&dropdown);
            group.append(&row);
        }
        page.append(&group);
    }
    scrolled(page)
}

pub(super) fn more_page(
    commands: &mpsc::Sender<DeviceCommand>,
    writes: &mut Vec<gtk::Widget>,
    deps: MorePageDeps,
) -> (gtk::ScrolledWindow, MoreRefs) {
    let MorePageDeps {
        snapshot,
        updating_controls,
        toast,
        config,
        paths,
        firmware,
    } = deps;
    let page = page_box();
    page.append(&page_title(
        "MORE",
        "Device tools and local application settings.",
    ));
    let sound = section(
        "SOUND & DETECTION",
        "Optional features confirmed for B171. Dual connection may require an earbud reboot.",
    );
    let current = snapshot.borrow().clone();
    let bass_enabled = current.bass_enhance.is_some();
    let bass_level = f64::from(current.bass_enhance.unwrap_or(3).clamp(1, 5));
    let bass_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let bass_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    bass_row.append(
        &gtk::Label::builder()
            .label("Bass Enhance")
            .hexpand(true)
            .halign(gtk::Align::Start)
            .build(),
    );
    let bass_switch = gtk::Switch::builder()
        .active(bass_enabled)
        .halign(gtk::Align::End)
        .valign(gtk::Align::Center)
        .build();
    let bass_scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 1.0, 5.0, 1.0);
    bass_scale.set_value(bass_level);
    bass_scale.set_digits(0);
    bass_scale.set_draw_value(true);
    bass_scale.set_hexpand(true);
    let sender = commands.clone();
    let scale_for_switch = bass_scale.clone();
    let updating = updating_controls.clone();
    bass_switch.connect_active_notify(move |switch| {
        if updating.get() {
            return;
        }
        let command = if switch.is_active() {
            DeviceCommand::SetBassEnhance(Some(scale_for_switch.value().round() as u8))
        } else {
            DeviceCommand::SetBassEnhance(None)
        };
        let _ = sender.send(command);
    });
    let sender = commands.clone();
    let switch_for_scale = bass_switch.clone();
    let updating = updating_controls.clone();
    bass_scale.connect_value_changed(move |scale| {
        if updating.get() {
            return;
        }
        if switch_for_scale.is_active() {
            let _ = sender.send(DeviceCommand::SetBassEnhance(Some(
                scale.value().round() as u8
            )));
        }
    });
    writes.push(bass_switch.clone().upcast());
    writes.push(bass_scale.clone().upcast());
    bass_row.append(&bass_switch);
    bass_box.append(&bass_row);
    bass_box.append(&bass_scale);
    sound.append(&bass_box);

    let (in_ear, in_ear_switch) = switch_control(
        "In-ear detection",
        current.in_ear_detection.unwrap_or(false),
        writes,
        {
            let sender = commands.clone();
            let updating = updating_controls.clone();
            move |enabled| {
                if updating.get() {
                    return;
                }
                let _ = sender.send(DeviceCommand::SetInEarDetection(enabled));
            }
        },
    );
    sound.append(&in_ear);
    let (low_lag, low_lag_switch) =
        switch_control("Low-lag mode", current.low_lag.unwrap_or(false), writes, {
            let sender = commands.clone();
            let updating = updating_controls.clone();
            move |enabled| {
                if updating.get() {
                    return;
                }
                let _ = sender.send(DeviceCommand::SetLowLag(enabled));
            }
        });
    sound.append(&low_lag);
    let dual_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(14)
        .build();
    let dual_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    dual_row.append(
        &gtk::Label::builder()
            .label("Dual connection")
            .hexpand(true)
            .halign(gtk::Align::Start)
            .build(),
    );
    let dual_switch = gtk::Switch::builder()
        .active(current.dual_connection.unwrap_or(false))
        .halign(gtk::Align::End)
        .valign(gtk::Align::Center)
        .build();
    dual_row.append(&dual_switch);
    dual_box.append(&dual_row);

    let dual_devices_revealer = gtk::Revealer::builder()
        .reveal_child(current.dual_connection.unwrap_or(false))
        .transition_type(gtk::RevealerTransitionType::SlideDown)
        .build();
    let dual_devices_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(10)
        .build();
    let dual_header = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(6)
        .css_classes(["dual-device-header"])
        .build();
    dual_header.append(
        &gtk::Label::builder()
            .label("My devices")
            .halign(gtk::Align::Start)
            .hexpand(false)
            .build(),
    );
    let dual_count = gtk::Label::builder()
        .halign(gtk::Align::Start)
        .css_classes(["dim-label"])
        .build();
    dual_header.append(&dual_count);
    let dual_devices_list = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .css_classes(["dual-device-list"])
        .build();
    populate_dual_devices(
        &dual_devices_list,
        &dual_count,
        &current.dual_devices,
        commands,
    );
    dual_devices_box.append(&dual_header);
    dual_devices_box.append(&dual_devices_list);
    dual_devices_revealer.set_child(Some(&dual_devices_box));
    dual_box.append(&dual_devices_revealer);
    writes.push(dual_switch.clone().upcast());
    let sender = commands.clone();
    let updating = updating_controls.clone();
    let devices_for_switch = dual_devices_revealer.clone();
    dual_switch.connect_active_notify(move |switch| {
        let enabled = switch.is_active();
        devices_for_switch.set_reveal_child(enabled);
        if updating.get() {
            return;
        }
        let _ = sender.send(DeviceCommand::SetDualConnection(enabled));
    });
    sound.append(&dual_box);
    let codec_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    codec_row.append(
        &gtk::Label::builder()
            .label("High-quality audio codec")
            .hexpand(true)
            .halign(gtk::Align::Start)
            .build(),
    );
    let codec = gtk::DropDown::from_strings(&["Default", "LHDC", "LDAC"]);
    codec.set_selected(match current.audio_codec.unwrap_or_default() {
        AudioCodec::Default => 0,
        AudioCodec::Lhdc => 1,
        AudioCodec::Ldac => 2,
    });
    let sender = commands.clone();
    let updating = updating_controls.clone();
    codec.connect_selected_notify(move |dropdown| {
        if updating.get() {
            return;
        }
        let codec = match dropdown.selected() {
            0 => AudioCodec::Default,
            1 => AudioCodec::Lhdc,
            2 => AudioCodec::Ldac,
            _ => return,
        };
        let _ = sender.send(DeviceCommand::SetAudioCodec(codec));
    });
    writes.push(codec.clone().upcast());
    codec_row.append(&codec);
    sound.append(&codec_row);
    page.append(&sound);
    let find = section(
        "FIND EARBUDS",
        "Remove earbuds before playing the locator sound. Each side toggles between play and pause.",
    );
    for (label, side) in [("Left", EarbudSide::Left), ("Right", EarbudSide::Right)] {
        let button = gtk::Button::with_label(&format!("{label} · Play"));
        let sender = commands.clone();
        let snapshot = snapshot.clone();
        let toast = toast.clone();
        let ringing = Rc::new(Cell::new(false));
        let ringing_for_click = ringing.clone();
        button.connect_clicked(move |button| {
            if ringing_for_click.get() {
                ringing_for_click.set(false);
                button.set_label(&format!("{label} · Play"));
                let _ = sender.send(DeviceCommand::FindBud {
                    side,
                    ringing: false,
                });
                return;
            }
            let worn = if side == EarbudSide::Left {
                snapshot.borrow().wear.left
            } else {
                snapshot.borrow().wear.right
            };
            if worn {
                toast.add_toast(adw::Toast::new(
                    "This earbud is reported as worn. Remove it before playing a loud locator sound.",
                ));
            } else {
                ringing_for_click.set(true);
                button.set_label(&format!("{label} · Pause"));
                let _ = sender.send(DeviceCommand::FindBud {
                    side,
                    ringing: true,
                });
            }
        });
        writes.push(button.clone().upcast());
        find.append(&button);
    }
    page.append(&find);
    let fit = section(
        "EAR-TIP FIT TEST",
        "Both earbuds must be connected and worn. The test can be cancelled.",
    );
    let start = command_button("Start fit test", DeviceCommand::StartFitTest, commands);
    let cancel = command_button("Cancel test", DeviceCommand::CancelFitTest, commands);
    writes.push(start.clone().upcast());
    writes.push(cancel.clone().upcast());
    fit.append(&start);
    fit.append(&cancel);
    page.append(&fit);
    let information = section("DEVICE INFORMATION", "Firmware is read-only.");
    information.append(&firmware);
    page.append(&information);
    let application = section(
        "APPLICATION",
        "Nothing Linux is fully local and sends no telemetry.",
    );
    let autostart = gtk::CheckButton::with_label("Start at login");
    autostart.set_active(config.borrow().start_at_login);
    autostart.connect_toggled(move |toggle| {
        let enabled = toggle.is_active();
        if config.borrow_mut().set_autostart(enabled).is_ok()
            && let Some(paths) = &paths
        {
            let _ = config.borrow().save(paths);
        }
    });
    application.append(&autostart);
    page.append(&application);
    (
        scrolled(page),
        MoreRefs {
            bass_switch,
            bass_scale,
            in_ear_switch,
            low_lag_switch,
            dual_switch,
            dual_devices_revealer,
            dual_count,
            dual_devices_list,
            commands: commands.clone(),
            codec,
        },
    )
}

fn populate_dual_devices(
    list: &gtk::Box,
    count: &gtk::Label,
    devices: &[DualConnectionDevice],
    commands: &mpsc::Sender<DeviceCommand>,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    let connected = devices
        .iter()
        .filter(|device| device.connected)
        .count()
        .min(2);
    count.set_label(&format!("({connected}/2)"));

    if devices.is_empty() {
        let row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .css_classes(["dual-device-row"])
            .build();
        row.append(
            &gtk::Label::builder()
                .label("No paired Nothing devices")
                .xalign(0.0)
                .hexpand(true)
                .css_classes(["dim-label"])
                .build(),
        );
        list.append(&row);
        return;
    }

    for (index, device) in devices.iter().enumerate() {
        let row = dual_device_row(device, commands);
        if index > 0 {
            row.add_css_class("dual-device-row-separated");
        }
        list.append(&row);
    }
}

fn dual_device_row(
    device: &DualConnectionDevice,
    commands: &mpsc::Sender<DeviceCommand>,
) -> gtk::Box {
    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(14)
        .css_classes(["dual-device-row"])
        .build();
    let labels = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .valign(gtk::Align::Center)
        .build();
    labels.append(
        &gtk::Label::builder()
            .label(dual_device_name(device))
            .xalign(0.0)
            .halign(gtk::Align::Fill)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build(),
    );
    if let Some(subtitle) = dual_device_subtitle(device) {
        labels.append(
            &gtk::Label::builder()
                .label(subtitle)
                .xalign(0.0)
                .halign(gtk::Align::Fill)
                .css_classes(["dim-label"])
                .build(),
        );
    }
    let check = gtk::CheckButton::new();
    check.set_active(device.connected);
    check.set_can_target(false);
    check.set_focusable(false);
    check.set_valign(gtk::Align::Center);
    let click = gtk::GestureClick::new();
    let sender = commands.clone();
    let address = device.address_bytes;
    let connect = !device.connected;
    click.connect_released(move |_, _, _, _| {
        let _ = sender.send(DeviceCommand::SetDualConnectionDevice { connect, address });
    });
    row.append(&labels);
    row.append(&check);
    row.add_controller(click);
    row
}

fn dual_device_name(device: &DualConnectionDevice) -> &str {
    let name = device.name.trim();
    if name.is_empty() {
        "Nothing audio device"
    } else {
        name
    }
}

fn dual_device_subtitle(device: &DualConnectionDevice) -> Option<&'static str> {
    if device.owner_device {
        Some("Current")
    } else {
        None
    }
}

fn switch_control<F>(
    label: &str,
    active: bool,
    writes: &mut Vec<gtk::Widget>,
    on_change: F,
) -> (gtk::Box, gtk::Switch)
where
    F: Fn(bool) + 'static,
{
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.append(
        &gtk::Label::builder()
            .label(label)
            .hexpand(true)
            .halign(gtk::Align::Start)
            .build(),
    );
    let switch = gtk::Switch::builder()
        .active(active)
        .halign(gtk::Align::End)
        .valign(gtk::Align::Center)
        .build();
    switch.connect_active_notify(move |switch| on_change(switch.is_active()));
    writes.push(switch.clone().upcast());
    row.append(&switch);
    (row, switch)
}

fn format_frequency(value: f32) -> String {
    if value >= 1_000.0 {
        let khz = value / 1_000.0;
        if khz.fract() == 0.0 {
            format!("{khz:.0}K")
        } else {
            format!("{khz:.1}K")
        }
    } else {
        format!("{value:.0}")
    }
}

fn format_gain(value: f32) -> String {
    if value.abs() < f32::EPSILON {
        "0".into()
    } else if value > 0.0 {
        format!("+{value:.1}")
    } else {
        format!("{value:.1}")
    }
}
