mod ffi;

use chrono::Local;
use clap::{Parser, Subcommand};
use ffi::DailyStat;

// Zig'den gelen fonksiyon (sadece GUI derlenmişse)
#[cfg(not(feature = "no-gui"))]
extern "C" {
    fn zig_gui_main() -> i32;
}

#[derive(Parser)]
#[command(
    name = "tracker",
    about = "Çoklu Tablo Destekli Zaman Takip Sistemi",
    long_about = "Tablo bazlı (Günlük, Haftalık, Proje) çalışma bloklarını yöneten gelişmiş CLI aracı."
)]
struct Cli {
    /// GUI arayüzünü başlatır
    #[arg(short, long)]
    gui: bool,
    
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Yeni bir ana tablo/proje oluşturur (Örn: tracker new-table yazilim --refresh 30d)
    #[command(name = "new-table")]
    NewTable {
        name: String,
        /// Yenilenme süresi (örn: 1d, 7d, 30d)
        #[arg(short, long, default_value = "7d")]
        refresh: String,
    },

    /// Mevcut bir tabloya yeni bir çalışma bloğu ekler (Örn: tracker add-block yazilim rust)
    #[command(name = "add-block")]
    AddBlock {
        table: String,
        block: String,
        /// Dakika cinsinden hedef
        #[arg(short, long, default_value_t = 60)]
        target: i32,
    },

    /// Bir tablo içindeki bloğa süre kaydeder (Örn: tracker log yazilim rust 45)
    Log {
        table: String,
        block: String,
        duration: i32,
    },

    /// Belirli bir tablonun raporunu gösterir (Örn: tracker report yazilim)
    Report { table: String },

    /// Tablodaki refresh süresinden eski kayıtları siler (Örn: tracker cleanup yazilim)
    Cleanup { table: String },
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    
    // GUI modu kontrolü
    if args.gui {
        // GUI kütüphanesini kontrol et
        if std::env::var("CARGO_FEATURE_NO_GUI").is_ok() {
            anyhow::bail!("GUI derlenmedi. GUI bağımlılıklarını kurun: ./kurulum.sh deps");
        }
        
        // GUI'yi doğrudan çağır (libloading olmadan)
        #[cfg(not(feature = "no-gui"))]
        {
            let result = unsafe { zig_gui_main() };
            if result != 0 {
                anyhow::bail!("GUI çalıştırma hatası: {}", result);
            }
        }
        
        #[cfg(feature = "no-gui")]
        {
            anyhow::bail!("GUI derlenmedi. GUI bağımlılıklarını kurun: ./kurulum.sh deps");
        }
        
        return Ok(());
    }
    
    // CLI modu
    // Binary'nin bulunduğu dizini bul
    let exe_path = std::env::current_exe().unwrap_or_else(|_| "tracker".into());
    let exe_dir = exe_path.parent().unwrap_or_else(|| std::path::Path::new("."));
    
    // Binary'nin olduğu dizine tables klasörü oluştur
    let tables_dir = exe_dir.join("tables");
    let db_path = tables_dir.join("tracker.db");
    
    // Tables klasörü yoksa oluştur
    if !tables_dir.exists() {
        std::fs::create_dir_all(&tables_dir).unwrap_or_else(|e| {
            eprintln!("Tables klasörü oluşturulamadı: {}", e);
        });
    }
    
    let mut engine = ffi::init_engine(&db_path.to_string_lossy());
    let today = Local::now().format("%Y-%m-%d").to_string();

    match args.command {
        Some(Commands::NewTable { name, refresh }) => {
            if engine.pin_mut().create_project(&name, &refresh) {
                println!(
                    "✨ '{}' tablosu {} yenilenme süresiyle oluşturuldu.",
                    name, refresh
                );
            } else {
                eprintln!("❌ Hata: Tablo oluşturulamadı.");
            }
        }

        Some(Commands::AddBlock {
            table,
            block,
            target,
        }) => {
            if engine
                .pin_mut()
                .add_block_to_project(&table, &block, target)
            {
                println!(
                    "📦 '{}' tablosuna '{}' bloğu eklendi (Hedef: {} dk).",
                    table, block, target
                );
            } else {
                eprintln!("❌ Hata: Blok eklenemedi. Tablonun var olduğundan emin olun.");
            }
        }

        Some(Commands::Log {
            table,
            block,
            duration,
        }) => {
            // C++ tarafındaki add_entry_v2 fonksiyonunu çağırıyoruz
            let success = engine
                .pin_mut()
                .add_entry_v2(&table, &block, duration, &today);
            if success {
                println!("✅ {} dk kaydedildi: {} > {}", duration, table, block);
            } else {
                eprintln!("❌ Hata: Kayıt yapılamadı. Tablo veya blok ismini kontrol edin.");
            }
        }

        Some(Commands::Report { table }) => {
            println!("\n📊 '{}' Tablosu Raporu", table);
            println!("{}", "=".repeat(50));
            
            // Önce tüm blokları getir (veri olsun olmasın)
            let blocks = engine.get_project_blocks(&table);
            
            if blocks.is_empty() {
                println!("ℹ️  Bu tablo oluşturulmuş ancak henüz blok eklenmemiş.");
                println!("💡 Blok eklemek için: ./tracker add-block {} <blok_adı> --target <hedef_dk>", table);
            } else {
                println!("📋 Bloklar:");
                for block in &blocks {
                    let progress = if block.target_minutes > 0 {
                        (block.total_minutes as f32 / block.target_minutes as f32) * 100.0
                    } else {
                        0.0
                    };
                    
                    let status = if block.has_data {
                        "✅ Veri var"
                    } else {
                        "⭕ Veri yok"
                    };
                    
                    let progress_bar = draw_progress_bar(progress);
                    
                    println!("  � {} | Hedef: {} dk | Toplam: {} dk | {} | {}", 
                             block.name, block.target_minutes, block.total_minutes, status, progress_bar);
                }
                
                // Veri olan blokların detaylı raporu
                let stats = engine.get_project_report(&table);
                if !stats.is_empty() {
                    println!("\n📈 Detaylı Girişler:");
                    print_stylish_table(&table, stats);
                } else {
                    println!("\n💡 Veri eklemek için: ./tracker log {} <blok_adı> <süre_dk>", table);
                }
            }
            
            println!("{}", "=".repeat(50));
        }

        Some(Commands::Cleanup { table }) => {
            if engine.pin_mut().cleanup_project(&table) {
                println!("🧹 '{}' tablosundaki eski veriler temizlendi.", table);
            } else {
                eprintln!("❌ Hata: Temizlik işlemi başarısız.");
            }
        }
        
        None => {
            // Hiçbir komut verilmemişse help göster
            println!("Tracker - Zaman Takip Aracı");
            println!("Kullanım:");
            println!("  tracker --gui           # GUI arayüzünü başlatır (GUI bağımlılıkları gerekli)");
            println!("  tracker new-table ...   # Yeni tablo oluşturur");
            println!("  tracker add-block ...   # Blok ekler");
            println!("  tracker log ...         # Çalışma kaydı");
            println!("  tracker report ...      # Rapor gösterir");
            println!("  tracker cleanup ...     # Temizlik yapar");
            println!("  tracker --help          # Yardım gösterir");
        }
    }
    
    Ok(())
}

fn print_stylish_table(table_name: &str, stats: Vec<DailyStat>) {
    use comfy_table::{presets::UTF8_FULL, Attribute, Cell, Color, Table};

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);

    println!("\n📊 TABLO: {}", table_name.to_uppercase());

    table.set_header(vec![
        Cell::new("Tarih")
            .fg(Color::Blue)
            .add_attribute(Attribute::Bold),
        Cell::new("Blok").fg(Color::Cyan),
        Cell::new("Süre").fg(Color::Green),
        Cell::new("Hedef").fg(Color::Yellow),
        Cell::new("İlerleme / Durum"),
    ]);

    for s in stats {
        let progress_val = (s.total_minutes as f32 / s.target_minutes as f32) * 100.0;
        let bar = draw_progress_bar(progress_val);

        let status_color = if progress_val >= 100.0 {
            Color::Green
        } else {
            Color::White
        };

        table.add_row(vec![
            Cell::new(&s.date),
            Cell::new(&s.block_name),
            Cell::new(format!("{} dk", s.total_minutes)),
            Cell::new(format!("{} dk", s.target_minutes)),
            Cell::new(bar).fg(status_color),
        ]);
    }

    println!("{table}");
}

fn draw_progress_bar(percent: f32) -> String {
    let width = 10;
    let filled = ((percent / 100.0) * width as f32).round() as usize;
    let filled = std::cmp::min(filled, width);
    let empty = width - filled;

    format!(
        "[{}{}] {:.1}%",
        "█".repeat(filled),
        "░".repeat(empty),
        percent
    )
}
