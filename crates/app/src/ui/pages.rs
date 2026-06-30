use super::{
    artwork::draw_earbuds,
    components::{
        battery_label, command_button, gesture_name, page_box, page_title, scrolled, section,
        unsupported,
    },
};
use adw::prelude::*;
use nothing_core::{
    AncLevel, AncMode, AppConfig, DeviceCommand, DeviceSnapshot, EarbudSide, EqPreset, Gesture,
    GestureAction, Paths,
};
use std::{cell::RefCell, rc::Rc, sync::mpsc};

pub(super) struct OverviewRefs {
    pub(super) left: gtk::Label,
    pub(super) right: gtk::Label,
    pub(super) case: gtk::Label,
    pub(super) wear: gtk::Label,
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
    let illustration = gtk::DrawingArea::builder()
        .height_request(220)
        .hexpand(true)
        .accessible_role(gtk::AccessibleRole::Img)
        .build();
    illustration.update_property(&[gtk::accessible::Property::Label(
        "Original abstract illustration of two earbuds",
    )]);
    illustration.set_draw_func(|_, cr, width, height| {
        draw_earbuds(cr, f64::from(width), f64::from(height))
    });
    hero.append(&illustration);
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
    for (label, preset) in [
        ("Balanced", EqPreset::Balanced),
        ("More bass", EqPreset::MoreBass),
        ("More treble", EqPreset::MoreTreble),
        ("Voice", EqPreset::Voice),
    ] {
        let button = command_button(label, DeviceCommand::SetEqPreset(preset), commands);
        writes.push(button.clone().upcast());
        presets.append(&button);
    }
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
        let _ = sender.send(DeviceCommand::SetCustomEq(gains));
    });
    writes.push(apply.clone().upcast());
    custom.append(&apply);
    page.append(&custom);
    let advanced = section(
        "EIGHT-BAND ADVANCED",
        "20 Hz · 63 Hz · 250 Hz · 1 kHz · 4 kHz · 8 kHz · 12 kHz · 20 kHz",
    );
    for frequency in ["20", "63", "250", "1K", "4K", "8K", "12K", "20K"] {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        row.append(
            &gtk::Label::builder()
                .label(frequency)
                .width_chars(4)
                .build(),
        );
        let gain = gtk::Scale::with_range(gtk::Orientation::Horizontal, -12.0, 12.0, 0.5);
        gain.set_hexpand(true);
        gain.set_tooltip_text(Some("Gain in decibels"));
        row.append(&gain);
        let q = gtk::SpinButton::with_range(0.1, 10.0, 0.1);
        q.set_value(1.0);
        q.set_tooltip_text(Some("Q factor"));
        row.append(&q);
        advanced.append(&row);
    }
    let enable = command_button(
        "Enable advanced EQ",
        DeviceCommand::SetAdvancedEqEnabled(true),
        commands,
    );
    writes.push(enable.clone().upcast());
    advanced.append(&enable);
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
    snapshot: Rc<RefCell<DeviceSnapshot>>,
    toast: adw::ToastOverlay,
    config: Rc<RefCell<AppConfig>>,
    paths: Option<Paths>,
    firmware: gtk::Label,
) -> gtk::ScrolledWindow {
    let page = page_box();
    page.append(&page_title(
        "MORE",
        "Device tools and local application settings.",
    ));
    let sound = section("SOUND & DETECTION", "Optional features confirmed for B171.");
    for (label, command) in [
        (
            "Bass Enhance · Level 3",
            DeviceCommand::SetBassEnhance(Some(3)),
        ),
        (
            "In-ear detection · On",
            DeviceCommand::SetInEarDetection(true),
        ),
        ("Low-lag mode · On", DeviceCommand::SetLowLag(true)),
    ] {
        let button = command_button(label, command, commands);
        writes.push(button.clone().upcast());
        sound.append(&button);
    }
    sound.append(&unsupported(
        "High-quality audio",
        "No verified B171 write command is available.",
    ));
    sound.append(&unsupported(
        "Dual connection",
        "Read/write payloads are not verified for this firmware.",
    ));
    page.append(&sound);
    let find = section(
        "FIND EARBUDS",
        "Remove earbuds before playing the locator sound. Stop is always available.",
    );
    for (label, side) in [
        ("Play left", EarbudSide::Left),
        ("Play right", EarbudSide::Right),
    ] {
        let button = gtk::Button::with_label(label);
        let sender = commands.clone();
        let snapshot = snapshot.clone();
        let toast = toast.clone();
        button.connect_clicked(move |_| {
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
                let _ = sender.send(DeviceCommand::FindBud {
                    side,
                    ringing: true,
                });
            }
        });
        writes.push(button.clone().upcast());
        find.append(&button);
    }
    for (label, side) in [
        ("Stop left", EarbudSide::Left),
        ("Stop right", EarbudSide::Right),
    ] {
        let button = command_button(
            label,
            DeviceCommand::FindBud {
                side,
                ringing: false,
            },
            commands,
        );
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
    scrolled(page)
}
