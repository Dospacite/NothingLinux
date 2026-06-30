use adw::prelude::*;

pub(super) fn navigation(stack: &gtk::Stack) -> gtk::Box {
    let sidebar = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .width_request(210)
        .css_classes(["navigation"])
        .build();
    let items = [
        ("overview", "Overview", "audio-headphones-symbolic"),
        ("noise", "Noise control", "audio-volume-high-symbolic"),
        (
            "equalizer",
            "Equalizer",
            "multimedia-volume-control-symbolic",
        ),
        ("controls", "Controls", "input-gaming-symbolic"),
        ("more", "More", "preferences-system-symbolic"),
    ];
    for (name, label, icon) in items {
        let button = gtk::Button::builder()
            .label(label)
            .halign(gtk::Align::Fill)
            .css_classes(["flat", "nav-button"])
            .build();
        button.set_icon_name(icon);
        let stack = stack.clone();
        button.connect_clicked(move |_| stack.set_visible_child_name(name));
        sidebar.append(&button);
    }
    sidebar.append(&gtk::Box::builder().vexpand(true).build());
    let note = gtk::Label::builder()
        .label("LOCAL · PRIVATE · UNOFFICIAL")
        .wrap(true)
        .css_classes(["caption", "dim-label"])
        .build();
    sidebar.append(&note);
    sidebar
}

pub(super) fn mobile_navigation(stack: &gtk::Stack) -> gtk::Box {
    let navigation = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .homogeneous(true)
        .css_classes(["mobile-navigation"])
        .build();
    for (name, label, icon) in [
        ("overview", "Overview", "audio-headphones-symbolic"),
        ("noise", "Noise", "audio-volume-high-symbolic"),
        ("equalizer", "EQ", "multimedia-volume-control-symbolic"),
        ("controls", "Controls", "input-gaming-symbolic"),
        ("more", "More", "preferences-system-symbolic"),
    ] {
        let button = gtk::Button::builder()
            .label(label)
            .icon_name(icon)
            .css_classes(["flat"])
            .build();
        let stack = stack.clone();
        button.connect_clicked(move |_| stack.set_visible_child_name(name));
        navigation.append(&button);
    }
    navigation
}

pub(super) fn set_navigation_width(
    width: i32,
    sidebar: &gtk::Box,
    separator: &gtk::Separator,
    mobile: &gtk::Box,
) {
    let narrow = width > 0 && width <= 700;
    sidebar.set_visible(!narrow);
    separator.set_visible(!narrow);
    mobile.set_visible(narrow);
}
