#!/usr/bin/env bash
#
# Spotdora Fedora Workstation Build & Package Helper
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}=== Spotdora: Fedora Workstation Build Helper ===${NC}"

usage() {
    echo "Uso: $0 [opzioni]"
    echo ""
    echo "Opzioni:"
    echo "  --deps       Installa tutte le dipendenze di build con dnf (richiede sudo)"
    echo "  --build      Configura e compila Spotdora in modalità release ottimizzata"
    echo "  --install    Installa Spotdora localmente in ~/.local"
    echo "  --rpm        Genera il pacchetto RPM per Fedora Workstation"
    echo "  --all        Installa dipendenze, compila e installa localmente"
    echo "  --help       Mostra questo messaggio di aiuto"
    exit 1
}

FEDORA_DEPS=(
    gcc
    meson
    ninja-build
    cargo
    rust
    gtk4-devel
    libadwaita-devel
    blueprint-compiler
    openssl-devel
    alsa-lib-devel
    pulseaudio-libs-devel
    pipewire-pulseaudio
    pipewire-gstreamer
    desktop-file-utils
    libappstream-glib
)

install_deps() {
    echo -e "${BLUE}>> Installazione delle dipendenze per Fedora Workstation...${NC}"
    if command -v dnf &>/dev/null; then
        sudo dnf install -y "${FEDORA_DEPS[@]}"
    else
        echo -e "${RED}Errore: dnf non trovato. Assicurati di essere su Fedora Workstation.${NC}"
        exit 1
    fi
    echo -e "${GREEN}>> Dipendenze installate con successo!${NC}"
}

build_spotdora() {
    echo -e "${BLUE}>> Configurazione di Meson con profilo Release ottimizzato per Fedora...${NC}"
    cd "${ROOT_DIR}"
    
    # Enable Wayland preferred backend
    export GDK_BACKEND="wayland,x11"
    
    if [ ! -d "target" ]; then
        meson setup target \
            -Dbuildtype=release \
            -Doffline=false \
            --prefix="${HOME}/.local"
    else
        meson setup --reconfigure target \
            -Dbuildtype=release \
            -Doffline=false \
            --prefix="${HOME}/.local"
    fi

    echo -e "${BLUE}>> Compilazione con ninja...${NC}"
    ninja -C target
    echo -e "${GREEN}>> Compilazione completata!${NC}"
}

install_local() {
    echo -e "${BLUE}>> Installazione in ~/.local...${NC}"
    cd "${ROOT_DIR}"
    ninja install -C target

    mkdir -p "${HOME}/.local/bin"

    # Update desktop database and icon cache
    update-desktop-database "${HOME}/.local/share/applications" 2>/dev/null || true
    gtk-update-icon-cache -f -t "${HOME}/.local/share/icons/hicolor" 2>/dev/null || true

    echo -e "${GREEN}>> Spotdora installato in ~/.local/bin/spotdora !${NC}"
    echo -e "Assicurati che ~/.local/bin sia nel tuo PATH."
}

build_rpm() {
    echo -e "${BLUE}>> Creazione pacchetto RPM per Fedora Workstation...${NC}"
    mkdir -p "${HOME}/rpmbuild/"{BUILD,RPMS,SOURCES,SPECS,SRPMS}
    
    VERSION="0.5.1"
    ARCHIVE_NAME="spotdora-${VERSION}"
    
    echo -e "${BLUE}>> Preparazione archivio sorgenti...${NC}"
    git -C "${ROOT_DIR}" archive --format=tar.gz --prefix="${ARCHIVE_NAME}/" HEAD -o "${HOME}/rpmbuild/SOURCES/${ARCHIVE_NAME}.tar.gz"
    cp "${ROOT_DIR}/packaging/fedora/spotdora.service" "${HOME}/rpmbuild/SOURCES/"
    cp "${ROOT_DIR}/packaging/fedora/spotdora.spec" "${HOME}/rpmbuild/SPECS/"
    
    echo -e "${BLUE}>> Esecuzione rpmbuild...${NC}"
    export PATH="${HOME}/.cargo/bin:${PATH}"
    rpmbuild --nodeps -ba "${HOME}/rpmbuild/SPECS/spotdora.spec"
    
    cp "${HOME}/rpmbuild/RPMS/"*"/spotdora-"*.rpm "${ROOT_DIR}/" 2>/dev/null || true
    echo -e "${GREEN}>> RPM generato con successo e copiato in ${ROOT_DIR}/ !${NC}"
}

if [ $# -eq 0 ]; then
    usage
fi

while [ $# -gt 0 ]; do
    case "$1" in
        --deps)
            install_deps
            shift
            ;;
        --build)
            build_spotdora
            shift
            ;;
        --install)
            install_local
            shift
            ;;
        --rpm)
            build_rpm
            shift
            ;;
        --all)
            install_deps
            build_spotdora
            install_local
            shift
            ;;
        --help|-h)
            usage
            ;;
        *)
            echo -e "${RED}Opzione sconosciuta: $1${NC}"
            usage
            ;;
    esac
done

