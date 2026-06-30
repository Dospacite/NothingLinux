use adw::prelude::*;
use gtk::glib;
use nothing_core::{ChargeLevel, DeviceCommand, Gesture};
use std::sync::mpsc;

pub(super) fn page_box() -> gtk::Box {
    gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(18)
        .margin_top(28)
        .margin_bottom(36)
        .margin_start(28)
        .margin_end(28)
        .build()
}

pub(super) fn scrolled(child: gtk::Box) -> gtk::ScrolledWindow {
    let clamp = adw::Clamp::builder()
        .maximum_size(860)
        .tightening_threshold(650)
        .child(&child)
        .build();
    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&clamp)
        .build()
}

pub(super) fn page_title(title: &str, subtitle: &str) -> gtk::Box {
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 6);
    box_.append(
        &gtk::Label::builder()
            .label(title)
            .halign(gtk::Align::Start)
            .css_classes(["title-1", "dot-title"])
            .build(),
    );
    box_.append(
        &gtk::Label::builder()
            .label(subtitle)
            .halign(gtk::Align::Start)
            .wrap(true)
            .css_classes(["dim-label"])
            .build(),
    );
    box_
}

pub(super) fn section(title: &str, subtitle: &str) -> gtk::Box {
    let box_ = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(10)
        .css_classes(["tonal-card"])
        .build();
    box_.append(
        &gtk::Label::builder()
            .label(title)
            .halign(gtk::Align::Start)
            .css_classes(["heading"])
            .build(),
    );
    box_.append(
        &gtk::Label::builder()
            .label(subtitle)
            .halign(gtk::Align::Start)
            .wrap(true)
            .css_classes(["dim-label", "caption"])
            .build(),
    );
    box_
}

pub(super) fn unsupported(title: &str, reason: &str) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let label = gtk::Label::builder()
        .label(format!("{title}\n{reason}"))
        .wrap(true)
        .halign(gtk::Align::Start)
        .hexpand(true)
        .css_classes(["dim-label"])
        .build();
    let icon = gtk::Image::from_icon_name("action-unavailable-symbolic");
    icon.set_tooltip_text(Some(reason));
    row.append(&label);
    row.append(&icon);
    row.set_sensitive(false);
    row
}

pub(super) fn command_button(
    label: &str,
    command: DeviceCommand,
    sender: &mpsc::Sender<DeviceCommand>,
) -> gtk::Button {
    let button = gtk::Button::with_label(label);
    button.set_hexpand(true);
    let sender = sender.clone();
    button.connect_clicked(move |button| {
        button.set_sensitive(false);
        let _ = sender.send(command.clone());
        let button = button.clone();
        glib::timeout_add_local_once(std::time::Duration::from_secs(1), move || {
            button.set_sensitive(true)
        });
    });
    button
}

pub(super) fn battery_label(text: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(text)
        .css_classes(["battery-value"])
        .build()
}

pub(super) fn battery_text(name: &str, value: Option<ChargeLevel>) -> String {
    value.map_or_else(
        || format!("{name} —"),
        |value| {
            format!(
                "{name} {}%{}",
                value.percent,
                if value.charging { " ⚡" } else { "" }
            )
        },
    )
}

pub(super) fn gesture_name(value: Gesture) -> &'static str {
    match value {
        Gesture::SinglePinch => "Single pinch",
        Gesture::DoublePinch => "Double pinch",
        Gesture::TriplePinch => "Triple pinch",
        Gesture::PinchAndHold => "Pinch and hold",
        Gesture::DoublePinchAndHold => "Double pinch and hold",
    }
}
