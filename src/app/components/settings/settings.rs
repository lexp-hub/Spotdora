use crate::app::components::EventListener;
use crate::app::AppEvent;
use crate::settings::SpotSettings;

use gettextrs::gettext;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use libadwaita::prelude::*;

use super::SettingsModel;

const SETTINGS: &str = "dev.lex.Spotdora";

mod imp {

    use super::*;
    use libadwaita::subclass::prelude::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/dev/lex/Spotdora/components/settings.ui")]
    pub struct SettingsDialog {
        #[template_child]
        pub player_bitrate: TemplateChild<libadwaita::ComboRow>,

        #[template_child]
        pub alsa_device: TemplateChild<gtk::Entry>,

        #[template_child]
        pub alsa_device_row: TemplateChild<libadwaita::ActionRow>,

        #[template_child]
        pub audio_backend: TemplateChild<libadwaita::ComboRow>,

        #[template_child]
        pub gapless_playback: TemplateChild<libadwaita::ActionRow>,

        #[template_child]
        pub ap_port: TemplateChild<gtk::Entry>,

        #[template_child]
        pub theme: TemplateChild<libadwaita::ComboRow>,

        #[template_child]
        pub updates_page: TemplateChild<libadwaita::PreferencesPage>,

        #[template_child]
        pub current_version_row: TemplateChild<libadwaita::ActionRow>,

        #[template_child]
        pub check_updates_btn: TemplateChild<gtk::Button>,

        #[template_child]
        pub update_status_row: TemplateChild<libadwaita::ActionRow>,

        #[template_child]
        pub update_spinner: TemplateChild<gtk::Spinner>,

        #[template_child]
        pub download_rpm_btn: TemplateChild<gtk::Button>,

        #[template_child]
        pub view_release_btn: TemplateChild<gtk::Button>,

        #[template_child]
        pub release_notes_row: TemplateChild<libadwaita::ExpanderRow>,

        #[template_child]
        pub release_notes_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SettingsDialog {
        const NAME: &'static str = "SettingsWindow";
        type Type = super::SettingsDialog;
        type ParentType = libadwaita::PreferencesDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SettingsDialog {}
    impl WidgetImpl for SettingsDialog {}
    impl AdwDialogImpl for SettingsDialog {}
    impl PreferencesDialogImpl for SettingsDialog {}
}

glib::wrapper! {
    pub struct SettingsDialog(ObjectSubclass<imp::SettingsDialog>) @extends gtk::Widget, libadwaita::Dialog, libadwaita::PreferencesDialog;
}

impl Default for SettingsDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsDialog {
    pub fn new() -> Self {
        let dialog: Self = glib::Object::new();

        dialog.bind_backend_and_device();
        dialog.bind_settings();
        dialog.connect_theme_select();
        dialog.setup_updates();
        dialog
    }

    fn setup_updates(&self) {
        let widget = self.imp();
        widget.current_version_row.set_subtitle(&format!(
            "Spotdora {} (Fedora Workstation)",
            crate::config::VERSION
        ));

        let self_clone = self.clone();
        widget.check_updates_btn.connect_clicked(move |_| {
            self_clone.trigger_update_check();
        });
    }

    pub fn select_updates_page(&self) {
        let widget = self.imp();
        self.set_visible_page(&*widget.updates_page);
    }

    pub fn trigger_update_check(&self) {
        let widget = self.imp();
        let check_btn = widget.check_updates_btn.get();
        let spinner = widget.update_spinner.get();
        let status_row = widget.update_status_row.get();
        let download_btn = widget.download_rpm_btn.get();
        let release_btn = widget.view_release_btn.get();
        let notes_row = widget.release_notes_row.get();
        let notes_label = widget.release_notes_label.get();

        check_btn.set_sensitive(false);
        spinner.set_visible(true);
        spinner.start();
        status_row.set_title(&gettext("Checking for updates..."));
        status_row.set_subtitle(&gettext("Connecting to GitHub to verify the latest release."));
        download_btn.set_visible(false);
        release_btn.set_visible(false);
        notes_row.set_visible(false);

        let current_version = crate::config::VERSION.to_string();

        glib::spawn_future_local(clone!(
            #[weak]
            check_btn,
            #[weak]
            spinner,
            #[weak]
            status_row,
            #[weak]
            download_btn,
            #[weak]
            release_btn,
            #[weak]
            notes_row,
            #[weak]
            notes_label,
            async move {
                let status = crate::app::updater::check_latest_release(&current_version).await;

                spinner.stop();
                spinner.set_visible(false);
                check_btn.set_sensitive(true);

                match status {
                    crate::app::updater::UpdateStatus::UpToDate(version) => {
                        status_row.set_title(&gettext("Spotdora is up to date"));
                        status_row.set_subtitle(&format!(
                            "Version {version} is the latest release available for Fedora Workstation."
                        ));
                    }
                    crate::app::updater::UpdateStatus::NewVersionAvailable {
                        version,
                        rpm_url,
                        html_url,
                        notes,
                    } => {
                        status_row.set_title(&gettext("New version available!"));
                        status_row.set_subtitle(&format!(
                            "Spotdora {version} is ready to download and install."
                        ));

                        if let Some(rpm) = rpm_url {
                            download_btn.set_visible(true);
                            download_btn.connect_clicked(move |_| {
                                let _ = gio::AppInfo::launch_default_for_uri(
                                    &rpm,
                                    None::<&gio::AppLaunchContext>,
                                );
                            });
                        }

                        release_btn.set_visible(true);
                        release_btn.connect_clicked(move |_| {
                            let _ = gio::AppInfo::launch_default_for_uri(
                                &html_url,
                                None::<&gio::AppLaunchContext>,
                            );
                        });

                        notes_label.set_label(&notes);
                        notes_row.set_visible(true);
                    }
                    crate::app::updater::UpdateStatus::Error(err) => {
                        status_row.set_title(&gettext("Check failed"));
                        status_row.set_subtitle(&err);
                    }
                }
            }
        ));
    }

    fn bind_backend_and_device(&self) {
        let widget = self.imp();

        let audio_backend = widget
            .audio_backend
            .downcast_ref::<libadwaita::ComboRow>()
            .unwrap();
        let alsa_device_row = widget
            .alsa_device_row
            .downcast_ref::<libadwaita::ActionRow>()
            .unwrap();

        audio_backend
            .bind_property("selected", alsa_device_row, "visible")
            .transform_to(|_, value: u32| Some(value == 1))
            .build();

        if audio_backend.selected() == 0 {
            alsa_device_row.set_visible(false);
        }
    }

    fn bind_settings(&self) {
        let widget = self.imp();
        let settings = gio::Settings::new(SETTINGS);

        let player_bitrate = widget
            .player_bitrate
            .downcast_ref::<libadwaita::ComboRow>()
            .unwrap();
        settings
            .bind("player-bitrate", player_bitrate, "selected")
            .mapping(|variant, _| {
                variant.str().map(|s| {
                    match s {
                        "96" => 0,
                        "160" => 1,
                        "320" => 2,
                        _ => unreachable!(),
                    }
                    .to_value()
                })
            })
            .set_mapping(|value, _| {
                value.get::<u32>().ok().map(|u| {
                    match u {
                        0 => "96",
                        1 => "160",
                        2 => "320",
                        _ => unreachable!(),
                    }
                    .to_variant()
                })
            })
            .build();

        let alsa_device = widget.alsa_device.downcast_ref::<gtk::Entry>().unwrap();
        settings.bind("alsa-device", alsa_device, "text").build();

        let audio_backend = widget
            .audio_backend
            .downcast_ref::<libadwaita::ComboRow>()
            .unwrap();
        settings
            .bind("audio-backend", audio_backend, "selected")
            .mapping(|variant, _| {
                variant.str().map(|s| {
                    match s {
                        "pulseaudio" => 0,
                        "alsa" => 1,
                        "gstreamer" => 2,
                        _ => unreachable!(),
                    }
                    .to_value()
                })
            })
            .set_mapping(|value, _| {
                value.get::<u32>().ok().map(|u| {
                    match u {
                        0 => "pulseaudio",
                        1 => "alsa",
                        2 => "gstreamer",
                        _ => unreachable!(),
                    }
                    .to_variant()
                })
            })
            .build();

        let gapless_playback = widget
            .gapless_playback
            .downcast_ref::<libadwaita::ActionRow>()
            .unwrap();
        settings
            .bind(
                "gapless-playback",
                &gapless_playback.activatable_widget().unwrap(),
                "active",
            )
            .build();

        let ap_port = widget.ap_port.downcast_ref::<gtk::Entry>().unwrap();
        settings
            .bind("ap-port", ap_port, "text")
            .mapping(|variant, _| variant.get::<u32>().map(|s| s.to_value()))
            .set_mapping(|value, _| value.get::<u32>().ok().map(|u| u.to_variant()))
            .build();

        let theme = widget.theme.downcast_ref::<libadwaita::ComboRow>().unwrap();
        settings
            .bind("theme-preference", theme, "selected")
            .mapping(|variant, _| {
                variant.str().map(|s| {
                    match s {
                        "light" => 0,
                        "dark" => 1,
                        "system" => 2,
                        _ => unreachable!(),
                    }
                    .to_value()
                })
            })
            .set_mapping(|value, _| {
                value.get::<u32>().ok().map(|u| {
                    match u {
                        0 => "light",
                        1 => "dark",
                        2 => "system",
                        _ => unreachable!(),
                    }
                    .to_variant()
                })
            })
            .build();
    }

    fn connect_theme_select(&self) {
        let widget = self.imp();
        let theme = widget.theme.downcast_ref::<libadwaita::ComboRow>().unwrap();
        theme.connect_selected_notify(|theme| {
            debug!("Theme switched! --> value: {}", theme.selected());
            let manager = libadwaita::StyleManager::default();

            let pref = match theme.selected() {
                0 => libadwaita::ColorScheme::ForceLight,
                1 => libadwaita::ColorScheme::ForceDark,
                _ => libadwaita::ColorScheme::Default,
            };

            manager.set_color_scheme(pref);
        });
    }

    fn connect_close<F>(&self, on_close: F)
    where
        F: Fn() + 'static,
    {
        let dialog = self.upcast_ref::<libadwaita::Dialog>();
        dialog.connect_close_attempt(move |_| {
            on_close();
        });
    }
}

#[derive(Clone)]
pub struct Settings {
    parent: gtk::Window,
    settings_dialog: SettingsDialog,
}

impl Settings {
    pub fn new(parent: gtk::Window, model: SettingsModel) -> Self {
        let settings_dialog = SettingsDialog::new();

        settings_dialog.connect_close(move || {
            let new_settings = SpotSettings::new_from_gsettings().unwrap_or_default();
            if model.settings().player_settings != new_settings.player_settings {
                model.stop_player();
            }
            model.set_settings();
        });

        Self {
            parent,
            settings_dialog,
        }
    }

    fn dialog(&self) -> &libadwaita::Dialog {
        self.settings_dialog.upcast_ref::<libadwaita::Dialog>()
    }

    pub fn show_self(&self) {
        self.dialog().present(Some(&self.parent));
    }

    pub fn show_updates(&self) {
        self.settings_dialog.select_updates_page();
        self.dialog().present(Some(&self.parent));
        self.settings_dialog.trigger_update_check();
    }
}

impl EventListener for Settings {
    fn on_event(&mut self, _: &AppEvent) {}
}
