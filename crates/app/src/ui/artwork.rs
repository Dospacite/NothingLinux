pub(super) fn logo_label() -> gtk::Label {
    gtk::Label::builder()
        .label("NXL")
        .css_classes(["nxl-logo"])
        .accessible_role(gtk::AccessibleRole::Heading)
        .build()
}
