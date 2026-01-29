fn main() {
    // cxx_build::bridge zaten kendi header yollarını derleyiciye ekler.
    // .include("..") ise bizim 'core' klasörünü görmemizi sağlar.
    cxx_build::bridge("src/ffi.rs")
        .file("../core/engine.cpp")
        .include("..")
        .flag_if_supported("-std=c++17")
        .compile("tracker-core");

    // SQLite kütüphanesini sisteme bağla
    println!("cargo:rustc-link-lib=sqlite3");

    // GUI bağımlılıkları varsa Zig GUI'yi derle
    if std::env::var("CARGO_FEATURE_NO_GUI").is_ok() {
        println!("GUI devre dışı bırakıldı (no-gui feature)");
        return;
    }

    // Zig GUI'yi derle
    println!("cargo:rerun-if-changed=../gui/main.zig");
    eprintln!("=== Zig GUI Derleniyor ===");
    
    // Zig derleme komutu - libc ve header yolları ile
    let zig_output = std::process::Command::new("zig")
        .args(&[
            "build-lib", 
            "../gui/main.zig", 
            "-femit-bin=../target/release/libgui.a", 
            "-fPIC",
            "-I/usr/include",
            "-I/usr/local/include",
            "-target", "native"
        ])
        .output();

    match zig_output {
        Ok(output) => {
            if !output.status.success() {
                eprintln!("Zig GUI derleme hatası:");
                eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
                eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
                eprintln!("GUI olmadan devam ediliyor...");
                return;
            }
            
            // Derlenen dosyayı kontrol et
            if !std::path::Path::new("../target/release/libgui.a").exists() {
                eprintln!("Zig GUI kütüphanesi oluşturulamadı: libgui.a bulunamadı");
                eprintln!("GUI olmadan devam ediliyor...");
                return;
            }
            
            // SQLite ve GUI kütüphanelerini sisteme bağla
            println!("cargo:rustc-link-lib=sqlite3");
            println!("cargo:rustc-link-lib=imgui");
            println!("cargo:rustc-link-lib=glfw3");
            println!("cargo:rustc-link-lib=GL");
            println!("cargo:rustc-link-lib=GLEW");
            
            // Derlenmiş Zig kütüphanesini bağla
            println!("cargo:rustc-link-search=native=../target/release");
            println!("cargo:rustc-link-lib=static=gui");
            
            // GUI kütüphanesi için ekstra linker flag'leri
            println!("cargo:rustc-link-arg=-Wl,--whole-archive");
            println!("cargo:rustc-link-arg=../target/release/libgui.a");
        }
        Err(e) => {
            eprintln!("Zig çalıştırılamıyor: {}", e);
            eprintln!("GUI olmadan devam ediliyor...");
        }
    }

    // Dosya değişikliklerini izle
    println!("cargo:rerun-if-changed=src/ffi.rs");
    println!("cargo:rerun-if-changed=../core/engine.hpp");
    println!("cargo:rerun-if-changed=../core/engine.cpp");
    println!("cargo:rerun-if-changed=../gui/main.zig");
}
