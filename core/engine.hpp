#pragma once

#include <memory>
#include <sqlite3.h>
#include <rust/cxx.h>
#include <string>
#include <vector>

// DailyStat ve BlockInfo Rust tarafından generate edilecek
struct DailyStat;
struct BlockInfo;

class TrackerEngine {
private:
    sqlite3* db;
    void init_db();

public:
    explicit TrackerEngine(const std::string& db_path);
    ~TrackerEngine();

    // Eski versiyon metodlar (Bridge uyumluluğu için)
    bool create_block(rust::Str name);
    bool add_entry(rust::Str block_name, int32_t minutes, rust::Str date);
    rust::Vec<DailyStat> get_report(int32_t days_back) const;

    // Yeni Proje/Tablo bazlı metodlar
    bool create_project(rust::Str name, rust::Str refresh);
    bool add_block_to_project(rust::Str project_name, rust::Str block_name, int32_t target);
    bool add_entry_v2(rust::Str project_name, rust::Str block_name, int32_t minutes, rust::Str date);
    rust::Vec<DailyStat> get_project_report(rust::Str project_name) const;
    bool cleanup_project(rust::Str project_name);
    
    // Yeni: Projenin tüm bloklarını getir (veri olsun olmasın)
    rust::Vec<BlockInfo> get_project_blocks(rust::Str project_name) const;
};

// Fabrika fonksiyonu sınıfın dışındadır
std::unique_ptr<TrackerEngine> init_engine(rust::Str db_path);
