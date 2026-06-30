use gtk::gdk;

pub(super) fn install_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string("\n.navigation { padding: 20px 12px; background: alpha(@window_fg_color, .035); }\n.nav-button { min-height: 44px; padding: 0 14px; }\n.mobile-navigation { padding: 6px; background: alpha(@window_fg_color, .055); }\n.mobile-navigation button { min-height: 48px; }\n.tonal-card, .hero-card { padding: 20px; border-radius: 22px; background: alpha(@window_fg_color, .065); }\n.tonal-card button { min-height: 44px; border-radius: 14px; }\n.hero-card { background: alpha(@window_fg_color, .04); }\n.status-pill { padding: 7px 12px; border-radius: 999px; background: alpha(@window_fg_color, .08); }\n.battery-value { font-size: 16px; font-weight: 700; }\n.dot-title { letter-spacing: 2px; }\nbutton:focus-visible, dropdown:focus-visible, scale:focus-visible { outline: 3px solid @accent_color; outline-offset: 2px; }\n");
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
