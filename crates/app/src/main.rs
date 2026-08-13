mod tray;
mod ui;
mod ui_state;

use adw::prelude::*;
use gtk::{gio, glib};
use nothing_core::{
    AppConfig, DeviceCommand, DeviceEvent, Paths, redact_sensitive, wait_for_vendor_device,
};
use std::{
    cell::{Cell, RefCell},
    fs::OpenOptions,
    io::{self, Write},
    rc::Rc,
    sync::{Arc, Mutex, mpsc},
};
use tracing_subscriber::EnvFilter;

const APP_ID: &str = "io.github.nothinglinux.nothinglinux";
#[cfg(target_os = "linux")]
const DEFAULT_GSK_RENDERER: &str = "cairo";
#[cfg(target_os = "linux")]
const RENDERER_REEXEC_MARKER: &str = "NOTHING_LINUX_GSK_RENDERER_DEFAULTED";

fn main() -> glib::ExitCode {
    ensure_stable_renderer();
    init_logging();
    let background = std::env::args().any(|argument| argument == "--background");
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
    app.add_main_option(
        "background",
        b'b'.into(),
        glib::OptionFlags::NONE,
        glib::OptionArg::None,
        "Start without presenting the main window",
        None,
    );
    app.connect_startup(|app| {
        gtk::Window::set_default_icon_name(APP_ID);
        let quit = gio::ActionEntry::builder("quit")
            .activate(|app: &adw::Application, _, _| app.quit())
            .build();
        app.add_action_entries([quit]);
        app.set_accels_for_action("app.quit", &["<primary>q"]);
        let _hold = app.hold();
    });
    let initial_background = Rc::new(Cell::new(background));
    app.connect_activate(move |app| {
        build_application(app, initial_background.replace(false));
    });
    app.connect_command_line(|app, command| {
        let _background = command
            .options_dict()
            .lookup::<bool>("background")
            .ok()
            .flatten()
            .unwrap_or(false);
        app.activate();
        0.into()
    });
    app.run()
}

#[cfg(target_os = "linux")]
fn ensure_stable_renderer() {
    if std::env::var_os("GSK_RENDERER").is_some()
        || std::env::var_os(RENDERER_REEXEC_MARKER).is_some()
    {
        return;
    }

    let Ok(executable) = std::env::current_exe() else {
        return;
    };

    // GTK 4's GL renderers can segfault during the first frame on some drivers.
    use std::os::unix::process::CommandExt;
    let error = std::process::Command::new(executable)
        .args(std::env::args_os().skip(1))
        .env("GSK_RENDERER", DEFAULT_GSK_RENDERER)
        .env(RENDERER_REEXEC_MARKER, "1")
        .exec();
    eprintln!("failed to restart with GSK_RENDERER={DEFAULT_GSK_RENDERER}: {error}");
}

#[cfg(not(target_os = "linux"))]
fn ensure_stable_renderer() {}

fn init_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("nothing_linux=info,nothing_core=info"));
    let file = Paths::discover().ok().and_then(|paths| {
        paths.ensure().ok()?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(paths.diagnostics_file())
            .ok()
    });
    if let Some(file) = file {
        let file = Arc::new(Mutex::new(file));
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(move || DiagnosticWriter(file.clone()))
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }
}

struct DiagnosticWriter(Arc<Mutex<std::fs::File>>);
impl Write for DiagnosticWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        io::stderr().write_all(buffer)?;
        let mut redacted = redact_sensitive(&String::from_utf8_lossy(buffer));
        if buffer.ends_with(b"\n") {
            redacted.push('\n');
        }
        if let Ok(mut file) = self.0.lock() {
            file.write_all(redacted.as_bytes())?;
        }
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        io::stderr().flush()?;
        if let Ok(mut file) = self.0.lock() {
            file.flush()?;
        }
        Ok(())
    }
}

fn build_application(app: &adw::Application, background: bool) {
    if let Some(window) = app.active_window() {
        if !background {
            window.present();
        }
        return;
    }
    let (event_tx, event_rx) = mpsc::channel();
    let (command_tx, command_rx) = mpsc::channel();
    let (tray_handle, tray_actions) = tray::start();
    start_backend(event_tx, command_rx);
    let paths = Paths::discover().ok();
    let config = Rc::new(RefCell::new(
        paths
            .as_ref()
            .and_then(|value| AppConfig::load(value).ok())
            .unwrap_or_default(),
    ));
    let shell = ui::build(app, command_tx.clone(), config.clone(), paths.clone());
    let event_shell = shell.clone();
    let weak_app = app.downgrade();
    glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
        for event in event_rx.try_iter() {
            if let (Some(handle), DeviceEvent::Battery(battery)) = (&tray_handle, &event) {
                handle.update(|tray| tray.set_battery(battery));
            }
            event_shell.apply_event(event);
        }
        for action in tray_actions.try_iter() {
            match action {
                tray::TrayAction::Open => event_shell.window().present(),
                tray::TrayAction::AncOff => {
                    let _ = command_tx.send(DeviceCommand::SetAnc {
                        mode: nothing_core::AncMode::Off,
                        level: nothing_core::AncLevel::High,
                    });
                }
                tray::TrayAction::AncTransparency => {
                    let _ = command_tx.send(DeviceCommand::SetAnc {
                        mode: nothing_core::AncMode::Transparency,
                        level: nothing_core::AncLevel::High,
                    });
                }
                tray::TrayAction::AncOn => {
                    let _ = command_tx.send(DeviceCommand::SetAnc {
                        mode: nothing_core::AncMode::NoiseCancellation,
                        level: nothing_core::AncLevel::High,
                    });
                }
                tray::TrayAction::Quit => {
                    if let Some(app) = weak_app.upgrade() {
                        app.quit();
                    }
                }
            }
        }
        glib::ControlFlow::Continue
    });
    install_close_behavior(shell.window(), config, paths);
    if !background {
        shell.window().present();
    }
}

fn start_backend(events: mpsc::Sender<DeviceEvent>, commands: mpsc::Receiver<DeviceCommand>) {
    std::thread::Builder::new().name("nothing-linux-worker".into()).spawn(move || {
        let runtime = match tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build() { Ok(runtime) => runtime, Err(error) => { tracing::error!(%error, "failed to create async runtime"); return; } };
        runtime.block_on(async move {
            let mut last_discovery_error = None;
            let session = loop {
                let _ = events.send(DeviceEvent::ConnectionChanged(
                    nothing_core::ConnectionState::Connecting,
                ));
                match bluer_session().await {
                    Ok(session) => break session,
                    Err(reason) => {
                        tracing::warn!(%reason, "Nothing Ear discovery failed");
                        if last_discovery_error.as_deref() != Some(reason.as_str()) {
                            let _ = events.send(DeviceEvent::CommandFailed {
                                sequence: None,
                                command: DeviceCommand::QueryBattery,
                                reason: reason.clone(),
                            });
                            last_discovery_error = Some(reason);
                        }
                        let _ = events.send(DeviceEvent::ConnectionChanged(
                            nothing_core::ConnectionState::Disconnected,
                        ));
                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    }
                }
            };
            let mut controller = nothing_core::Controller::spawn_managed(session.device);
            loop {
                tokio::select! {
                    event = controller.events.recv() => { match event { Some(event) => { if events.send(event).is_err() { break; } }, None => break } }
                    _ = tokio::time::sleep(std::time::Duration::from_millis(5)) => {
                        while let Ok(command) = commands.try_recv() { if controller.handle.send(command).await.is_err() { break; } }
                    }
                }
            }
        });
    }).ok();
}

struct SelectedDevice {
    device: bluer::Device,
}
async fn bluer_session() -> Result<SelectedDevice, String> {
    let session = bluer::Session::new()
        .await
        .map_err(|error| format!("BlueZ is unavailable: {error}"))?;
    let adapter = session
        .default_adapter()
        .await
        .map_err(|error| format!("No Bluetooth adapter: {error}"))?;
    let descriptor = wait_for_vendor_device(&adapter)
        .await
        .map_err(|error| error.to_string())?;
    let address = descriptor
        .address
        .parse()
        .map_err(|_| "BlueZ returned an invalid Bluetooth address".to_owned())?;
    let device = adapter
        .device(address)
        .map_err(|error| format!("Could not open the paired Bluetooth device: {error}"))?;
    Ok(SelectedDevice { device })
}

fn install_close_behavior(
    window: &adw::ApplicationWindow,
    config: Rc<RefCell<AppConfig>>,
    paths: Option<Paths>,
) {
    let application = window.application();
    window.connect_close_request(move |window| {
        if config.borrow().first_close_explained {
            window.set_visible(false);
            return glib::Propagation::Stop;
        }
        let dialog = adw::MessageDialog::builder()
            .transient_for(window)
            .modal(true)
            .heading("Keep Nothing Linux running?")
            .body("Background monitoring keeps battery and controls available. You can quit at any time with Ctrl+Q.")
            .build();
        dialog.add_responses(&[("keep", "Keep running"), ("quit", "Quit")]);
        dialog.set_default_response(Some("keep"));
        dialog.set_close_response("keep");
        if let Some(application) = &application {
            install_quit_shortcut(&dialog, application);
        }
        let weak_window = window.downgrade();
        let config = config.clone();
        let paths = paths.clone();
        let application = application.clone();
        dialog.connect_response(None, move |_dialog, response| {
            if response == "quit" {
                // MessageDialog does not reliably inherit the GtkApplication
                // through a transient parent on every Wayland compositor.
                // Use the parent application's handle captured above instead.
                if let Some(application) = &application {
                    application.quit();
                }
                return;
            }
            config.borrow_mut().first_close_explained = true;
            if let Some(paths) = &paths {
                let _ = config.borrow().save(paths);
            }
            if let Some(window) = weak_window.upgrade() {
                window.set_visible(false);
            }
        });
        dialog.present();
        glib::Propagation::Stop
    });
}

fn install_quit_shortcut(widget: &impl IsA<gtk::Widget>, application: &impl IsA<gtk::Application>) {
    let application = application.downgrade();
    let controller = gtk::ShortcutController::new();
    controller.set_scope(gtk::ShortcutScope::Global);
    controller.add_shortcut(gtk::Shortcut::new(
        Some(gtk::KeyvalTrigger::new(
            gtk::gdk::Key::q,
            gtk::gdk::ModifierType::CONTROL_MASK,
        )),
        Some(gtk::CallbackAction::new(move |_, _| {
            if let Some(application) = application.upgrade() {
                application.upcast_ref::<gtk::Application>().quit();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        })),
    ));
    widget.add_controller(controller);
}
