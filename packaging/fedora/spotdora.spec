Name:           spotdora
Version:        0.5.1
Release:        1%{?dist}
Summary:        Native Spotify client heavily optimized for Fedora Workstation
Summary(it):    Client Spotify nativo fortemente ottimizzato per Fedora Workstation

License:        MIT
URL:            https://github.com/lexp-hub/Spotdora
Source0:        %{url}/archive/v%{version}/%{name}-%{version}.tar.gz
Source1:        spotdora.service

BuildRequires:  gcc
BuildRequires:  meson >= 0.59.0
BuildRequires:  ninja-build
BuildRequires:  cargo
BuildRequires:  rust >= 1.70.0
BuildRequires:  pkgconfig(gtk4) >= 4.12
BuildRequires:  pkgconfig(libadwaita-1) >= 1.5
BuildRequires:  blueprint-compiler >= 0.8.0
BuildRequires:  pkgconfig(openssl)
BuildRequires:  pkgconfig(alsa)
BuildRequires:  pkgconfig(libpulse)
BuildRequires:  desktop-file-utils
BuildRequires:  appstream
BuildRequires:  systemd-rpm-macros

Requires:       hicolor-icon-theme
Requires:       pipewire-pulseaudio
Recommends:     pipewire-gstreamer
Recommends:     gstreamer1-plugins-good
Recommends:     gstreamer1-plugins-bad-free

Provides:       spot = %{version}-%{release}
Obsoletes:      spot < 0.5.1

%description
Spotdora is a native Gtk4/Libadwaita Spotify client crafted specifically
for Fedora Workstation, featuring native PipeWire/WirePlumber audio routing,
GNOME Shell MPRIS and Quick Actions integration, and Wayland desktop optimizations.

Requires a Spotify Premium account.

%description -l it
Spotdora è un client nativo per Spotify sviluppato in GTK4 e Libadwaita,
progettato specificamente per Fedora Workstation. Offre integrazione nativa
con PipeWire/WirePlumber, controlli multimediali MPRIS e azioni rapide per GNOME Shell,
nonché ottimizzazioni per sessioni Wayland.

Richiede un account Spotify Premium.

%prep
%autosetup -p1 -n spotdora-%{version}

%build
export PATH="$HOME/.cargo/bin:$PATH"
%meson -Doffline=false
%meson_build

%install
%meson_install

# Install systemd user service
install -Dpm 0644 %{SOURCE1} %{buildroot}%{_userunitdir}/spotdora.service

%find_lang spotdora

%check
desktop-file-validate %{buildroot}%{_datadir}/applications/dev.lex.Spotdora.desktop
if command -v appstreamcli &>/dev/null; then
    appstreamcli validate --no-net %{buildroot}%{_datadir}/metainfo/dev.lex.Spotdora.appdata.xml || true
elif command -v appstream-util &>/dev/null; then
    appstream-util validate-relax --nonet %{buildroot}%{_datadir}/metainfo/dev.lex.Spotdora.appdata.xml
fi

%files -f spotdora.lang
%license LICENSE
%doc README.md AUTHORS
%{_bindir}/spotdora
%{_datadir}/spotdora/
%{_datadir}/applications/dev.lex.Spotdora.desktop
%{_datadir}/metainfo/dev.lex.Spotdora.appdata.xml
%{_datadir}/glib-2.0/schemas/dev.lex.Spotdora.gschema.xml
%{_datadir}/icons/hicolor/*/apps/dev.lex.Spotdora*
%{_userunitdir}/spotdora.service

%changelog
* Sun Sep 20 2026 Spotdora Contributors <https://github.com/lexp-hub/Spotdora> - 0.5.1-1
- Spotdora release optimized for Fedora Workstation
- Native PipeWire / WirePlumber audio stream property classification
- Enhanced GNOME Shell MPRIS (Quit, Stop, OpenURI) and Dash Quick Actions
- Compiler release profile optimizations (LTO, strip)
- Systemd user service unit integration

