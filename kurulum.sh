#!/bin/bash

GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}=== Tracker Derleme Sistemi ===${NC}"

# Bağımlılık kontrolleri
check_deps() {
    echo -e "\n${BLUE}Bağımlılıklar kontrol ediliyor...${NC}"
    
    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}Hata: Rust/Cargo bulunamadı.${NC}"
        return 1
    fi

    if ! command -v g++ &> /dev/null && ! command -v clang++ &> /dev/null; then
        echo -e "${RED}Hata: C++ derleyicisi bulunamadı.${NC}"
        return 1
    fi

    # SQLite kontrolü
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        if ! ldconfig -p | grep libsqlite3 &> /dev/null; then
            echo -e "${RED}Hata: libsqlite3-dev bulunamadı.${NC}"
            return 1
        fi
    fi
    
    # GUI bağımlılıkları (opsiyonel)
    if ! ldconfig -p | grep libglfw &> /dev/null; then
        echo -e "${YELLOW}Uyarı: GUI bağımlılıkları eksik. GUI çalışmayabilir.${NC}"
        echo -e "${YELLOW}GUI için: ./kurulum.sh deps${NC}"
    fi
    
    return 0
}

# GUI bağımlılıkları kurma
install_gui_deps() {
    echo -e "\n${BLUE}=== GUI Bağımlılıkları Kuruluyor ===${NC}"
    
    # Ubuntu/Debian için
    if command -v apt-get &> /dev/null; then
        echo "Ubuntu/Debian tespit edildi..."
        sudo apt-get update
        sudo apt-get install -y \
            libglfw3-dev \
            libglew-dev \
            libgl1-mesa-dev \
            libglu1-mesa-dev \
            libimgui-dev \
            build-essential \
            pkg-config

    # Arch Linux için
    elif command -v pacman &> /dev/null; then
        echo "Arch Linux tespit edildi..."
        sudo pacman -S --needed \
            glfw \
            glew \
            mesa \
            base-devel \
            pkgconf

    # Fedora için
    elif command -v dnf &> /dev/null; then
        echo "Fedora tespit edildi..."
        sudo dnf install -y \
            glfw-devel \
            glew-devel \
            mesa-libGL-devel \
            mesa-libGLU-devel \
            imgui-devel \
            gcc-c++ \
            pkgconfig

    else
        echo -e "${RED}Desteklenmeyen dağıtım!${NC}"
        echo "Lütfen manuel kurun: libglfw3-dev, libglew-dev, libimgui-dev"
        return 1
    fi

    echo -e "${GREEN}GUI bağımlılıkları kuruldu!${NC}"
    return 0
}

# Derleme
build() {
    echo -e "\n${BLUE}=== Tracker Derleniyor ===${NC}"
    
    if ! check_deps; then
        return 1
    fi

    echo -e "${BLUE}Cargo ile derleniyor...${NC}"
    cd cli
    
    # GUI olmadan derle (no-gui feature)
    cargo build --release --target-dir ../target --features no-gui

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}Derleme başarılı!${NC}"
        cd ..
        mkdir -p bin
        
        # Binary'yi kopyala
        cp target/release/tracker ./bin/tracker
        
        echo -e "${GREEN}Binary hazır: ./bin/tracker${NC}"
        echo -e "${BLUE}Veritabanı: binary ile aynı dizinde tables/tracker.db (ilk kullanımda otomatik oluşturulur)${NC}"
        echo -e "${BLUE}CLI: ./bin/tracker${NC}"
        echo -e "${YELLOW}GUI: GUI devre dışı (Zig+ImGui uyumsuzluğu)${NC}"
        return 0
    else
        echo -e "${RED}Derleme hatası!${NC}"
        cd ..
        return 1
    fi
}

# Temizleme
clean() {
    echo -e "\n${BLUE}=== Temizleniyor ===${NC}"
    
    rm -rf bin/
    rm -rf target/
    rm -rf cli/target/
    rm -rf cli/Cargo.lock
    
    echo -e "${GREEN}Temizleme tamamlandı!${NC}"
}

# Menü fonksiyonu
show_menu() {
    echo -e "\n${BLUE}Ne yapmak istersiniz?${NC}"
    echo -e "${YELLOW}1)${NC} Tracker'ı derle"
    echo -e "${YELLOW}2)${NC} GUI bağımlılıklarını kur"
    echo -e "${YELLOW}3)${NC} Temizle"
    echo -e "${YELLOW}4)${NC} Çıkış"
    echo -n -e "${BLUE}Seçiminiz: ${NC}"
}

# Ana döngü
main() {
    while true; do
        show_menu
        read choice
        
        case $choice in
            1)
                build
                ;;
            2)
                install_gui_deps
                ;;
            3)
                clean
                ;;
            4)
                echo -e "${BLUE}Güle güle!${NC}"
                exit 0
                ;;
            *)
                echo -e "${RED}Geçersiz seçim!${NC}"
                ;;
        esac
        
        echo -n -e "\n${BLUE}Devam etmek için Enter'a basın...${NC}"
        read
    done
}

# Script parametresiz çalıştırılırsa menüyü göster
if [ $# -eq 0 ]; then
    main
else
    # Parametre ile çalıştırma
    case $1 in
        "deps")
            install_gui_deps
            ;;
        "clean")
            clean
            ;;
        *)
            echo "Kullanım: $0 [deps|clean]"
            echo "  deps  - GUI bağımlılıklarını kurar"
            echo "  clean - Derleme dosyalarını temizler"
            exit 1
            ;;
    esac
fi
