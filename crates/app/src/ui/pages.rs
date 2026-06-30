use super::components::{
    battery_label, command_button, gesture_name, page_box, page_title, scrolled, section,
    unsupported,
};
use adw::prelude::*;
use nothing_core::{
    AdvancedEqProfile, AncLevel, AncMode, AppConfig, AudioCodec, DeviceCommand, DeviceSnapshot,
    EarbudSide, EqBand, EqPreset, Gesture, GestureAction, Paths,
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

pub(super) struct MoreRefs {
    pub(super) bass_switch: gtk::Switch,
    pub(super) bass_scale: gtk::Scale,
    pub(super) in_ear_switch: gtk::Switch,
    pub(super) low_lag_switch: gtk::Switch,
    pub(super) dual_switch: gtk::Switch,
    pub(super) codec: gtk::DropDown,
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
) -> gtk::ScrolledWindow {
    let page = page_box();
    page.append(&page_title(
        "NOISE CONTROL",
        "Control how much of the outside world you hear.",
    ));
    let modes = section(
        "MODE",
        "Changes are shown only after the earbuds confirm them.",
    );
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
    }
    page.append(&modes);
    let strength = section(
        "ANC STRENGTH",
        "Adaptive responds to changing ambient sound.",
    );
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
    }
    page.append(&strength);
    page.append(&unsupported(
        "Personalized ANC",
        "Not exposed by the verified B171 firmware profile.",
    ));
    scrolled(page)
}

pub(super) fn equalizer_page(
    commands: &mpsc::Sender<DeviceCommand>,
    writes: &mut Vec<gtk::Widget>,
) -> gtk::ScrolledWindow {
    let page = page_box();
    page.append(&page_title(
        "EQUALIZER",
        "Shape the sound. Profiles stay on this computer.",
    ));
    let presets = section("PRESETS", "Select one profile at a time.");
    let preset_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    preset_row.append(
        &gtk::Label::builder()
            .label("Profile")
            .hexpand(true)
            .halign(gtk::Align::Start)
            .build(),
    );
    let preset_dropdown =
        gtk::DropDown::from_strings(&["Balanced", "More bass", "More treble", "Voice"]);
    let sender = commands.clone();
    preset_dropdown.connect_selected_notify(move |dropdown| {
        let preset = match dropdown.selected() {
            0 => EqPreset::Balanced,
            1 => EqPreset::MoreBass,
            2 => EqPreset::MoreTreble,
            3 => EqPreset::Voice,
            _ => return,
        };
        let _ = sender.send(DeviceCommand::SetAdvancedEqEnabled(false));
        let _ = sender.send(DeviceCommand::SetEqPreset(preset));
    });
    writes.push(preset_dropdown.clone().upcast());
    preset_row.append(&preset_dropdown);
    presets.append(&preset_row);
    page.append(&presets);
    let custom = section(
        "THREE-BAND CUSTOM",
        "Bass · mids · treble, from −6 to +6 dB.",
    );
    let mut scales = Vec::new();
    for label in ["BASS", "MIDS", "TREBLE"] {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row.append(
            &gtk::Label::builder()
                .label(label)
                .width_chars(8)
                .halign(gtk::Align::Start)
                .build(),
        );
        let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, -6.0, 6.0, 0.5);
        scale.set_hexpand(true);
        scale.set_draw_value(true);
        row.append(&scale);
        custom.append(&row);
        writes.push(scale.clone().upcast());
        scales.push(scale);
    }
    let apply = gtk::Button::with_label("Apply custom EQ");
    apply.add_css_class("suggested-action");
    let sender = commands.clone();
    apply.connect_clicked(move |_| {
        let gains = [
            scales[0].value() as f32,
            scales[1].value() as f32,
            scales[2].value() as f32,
        ];
        let _ = sender.send(DeviceCommand::SetAdvancedEqEnabled(false));
        let _ = sender.send(DeviceCommand::SetEqPreset(EqPreset::Custom));
        let _ = sender.send(DeviceCommand::SetCustomEq(gains));
    });
    writes.push(apply.clone().upcast());
    custom.append(&apply);
    page.append(&custom);
    let advanced = section(
        "EIGHT-BAND ADVANCED",
        "55 Hz · 110 Hz · 220 Hz · 440 Hz · 1.32 kHz · 3.3 kHz · 6.6 kHz · 13.2 kHz",
    );
    let mut advanced_gains = Vec::new();
    let mut advanced_q = Vec::new();
    for frequency in AdvancedEqProfile::FREQUENCIES {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        row.append(
            &gtk::Label::builder()
                .label(format_frequency(frequency))
                .width_chars(6)
                .build(),
        );
        let gain = gtk::Scale::with_range(gtk::Orientation::Horizontal, -12.0, 12.0, 0.5);
        gain.set_hexpand(true);
        gain.set_draw_value(true);
        gain.set_tooltip_text(Some("Gain in decibels"));
        writes.push(gain.clone().upcast());
        row.append(&gain);
        let q = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.1, 10.0, 0.1);
        q.set_width_request(96);
        q.set_value(1.0);
        q.set_draw_value(true);
        q.set_digits(1);
        q.set_tooltip_text(Some("Q factor"));
        row.append(&q);
        writes.push(q.clone().upcast());
        advanced_gains.push(gain);
        advanced_q.push(q);
        advanced.append(&row);
    }
    let advanced_switch_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    advanced_switch_row.append(
        &gtk::Label::builder()
            .label("Advanced EQ")
            .hexpand(true)
            .halign(gtk::Align::Start)
            .build(),
    );
    let advanced_switch = gtk::Switch::builder()
        .halign(gtk::Align::End)
        .valign(gtk::Align::Center)
        .build();
    let sender = commands.clone();
    advanced_switch.connect_active_notify(move |switch| {
        let _ = sender.send(DeviceCommand::SetAdvancedEqEnabled(switch.is_active()));
    });
    writes.push(advanced_switch.clone().upcast());
    advanced_switch_row.append(&advanced_switch);
    advanced.append(&advanced_switch_row);
    let apply_advanced = gtk::Button::with_label("Apply advanced EQ");
    apply_advanced.add_css_class("suggested-action");
    let sender = commands.clone();
    apply_advanced.connect_clicked(move |_| {
        let bands = AdvancedEqProfile::FREQUENCIES
            .into_iter()
            .enumerate()
            .map(|(index, frequency_hz)| EqBand {
                frequency_hz,
                gain_db: advanced_gains[index].value() as f32,
                q: advanced_q[index].value() as f32,
            })
            .collect();
        let profile = AdvancedEqProfile {
            name: "Nothing Linux".into(),
            bands,
        };
        let _ = sender.send(DeviceCommand::SetAdvancedEqEnabled(true));
        let _ = sender.send(DeviceCommand::SetAdvancedEqProfile(Box::new(profile)));
    });
    writes.push(apply_advanced.clone().upcast());
    advanced.append(&apply_advanced);
    page.append(&advanced);
    scrolled(page)
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
    let (dual, dual_switch) = switch_control(
        "Dual connection",
        current.dual_connection.unwrap_or(false),
        writes,
        {
            let sender = commands.clone();
            let updating = updating_controls.clone();
            move |enabled| {
                if updating.get() {
                    return;
                }
                let _ = sender.send(DeviceCommand::SetDualConnection(enabled));
            }
        },
    );
    sound.append(&dual);
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
            codec,
        },
    )
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
