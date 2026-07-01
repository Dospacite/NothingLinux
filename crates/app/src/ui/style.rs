use gtk::gdk;

pub(super) fn install_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(
        "
.nxl-logo { color: #e61a1f; font-size: 22px; font-weight: 800; letter-spacing: 3px; }
.bottom-navigation { padding: 6px; background: alpha(@window_fg_color, .055); }
.bottom-navigation button { min-height: 48px; }
.tonal-card, .hero-card { padding: 20px; border-radius: 22px; background: alpha(@window_fg_color, .065); }
.tonal-card button { min-height: 44px; border-radius: 14px; }
.hero-card { background: alpha(@window_fg_color, .04); }
.status-pill { padding: 7px 12px; border-radius: 999px; background: alpha(@window_fg_color, .08); }
.battery-value { font-size: 16px; font-weight: 700; }
.dot-title { letter-spacing: 2px; }
.control-selected {
  color: #e61a1f;
  background: alpha(#e61a1f, .16);
  border-color: alpha(#e61a1f, .55);
}

.eq-tab-row { margin-top: 2px; border-bottom: 1px solid alpha(@window_fg_color, .16); }
.eq-tab-button {
  min-height: 48px;
  border-radius: 0;
  background: transparent;
  border: 0;
  box-shadow: none;
  font-weight: 700;
}
.eq-tab-button:hover { background: alpha(@window_fg_color, .045); }
.eq-tab-active {
  color: #e61a1f;
  border-bottom: 3px solid #e61a1f;
}
.eq-panel { padding-top: 10px; }
.eq-graph {
  border-radius: 999px;
  background: alpha(@window_fg_color, .055);
  color: @window_fg_color;
}
.eq-waveform {
  border-radius: 14px;
  background: alpha(@window_fg_color, .045);
}
.eq-control-group {
  padding: 14px;
  border-radius: 16px;
  background: alpha(@window_fg_color, .045);
}
.eq-preset-grid button, .eq-preset-button {
  min-height: 56px;
  border-radius: 14px;
  font-weight: 700;
}
.eq-preset-selected {
  color: #e61a1f;
  background: alpha(#e61a1f, .16);
  border-color: alpha(#e61a1f, .55);
}
.eq-profile-row dropdown { min-height: 44px; }
.eq-icon-button {
  min-width: 44px;
  min-height: 44px;
  border-radius: 14px;
}
.eq-gain-area {
  padding: 12px 4px 4px;
  border-radius: 16px;
  background: alpha(@window_fg_color, .035);
}
.eq-gain-column { min-width: 32px; }
.eq-gain-value { font-weight: 700; }
.eq-gain-scale slider { min-width: 18px; min-height: 18px; }
.eq-selected-band slider { background: #e61a1f; }
.eq-action-button { min-height: 52px; border-radius: 14px; }

.dual-device-header { margin-top: 4px; }
.dual-device-list {
  border-radius: 18px;
  background: alpha(@window_fg_color, .045);
}
.dual-device-row {
  min-height: 58px;
  padding: 14px 16px;
}
.dual-device-row-separated {
  border-top: 1px solid alpha(@window_fg_color, .1);
}
.dual-device-row checkbutton {
  min-width: 34px;
  min-height: 34px;
}

button:focus-visible, dropdown:focus-visible, scale:focus-visible { outline: 3px solid @accent_color; outline-offset: 2px; }
",
    );
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
