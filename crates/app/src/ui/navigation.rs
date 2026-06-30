use adw::prelude::*;

pub(super) fn bottom_navigation(stack: &gtk::Stack) -> gtk::Box {
    let navigation = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .homogeneous(true)
        .css_classes(["bottom-navigation"])
        .build();
    for (name, label, icon) in [
        ("overview", "Home", "audio-headphones-symbolic"),
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
