#include "engine.hpp"
#include <iostream>
#include <sqlite3.h>
#include <string>
#include <utility>
#include <vector>

#include "rust/cxx.h"
#include "tracker-cli/src/ffi.rs.h" // DailyStat tanımı buradan gelir

// Yardımcı string dönüşüm fonksiyonu
static std::string to_string(rust::Str s) {
    return std::string(s.data(), s.size());
}

// Constructor
TrackerEngine::TrackerEngine(const std::string& db_path) {
    if (sqlite3_open(db_path.c_str(), &db) != SQLITE_OK) {
        std::cerr << "Veritabanı açılamadı!" << std::endl;
        db = nullptr;
    } else {
        init_db();
    }
}

// Destructor
TrackerEngine::~TrackerEngine() {
    if (db) sqlite3_close(db);
}

void TrackerEngine::init_db() {
    const char* sql = R"(
        CREATE TABLE IF NOT EXISTS projects (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT UNIQUE NOT NULL,
            refresh_interval TEXT DEFAULT '7d'
        );
        CREATE TABLE IF NOT EXISTS blocks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            target_minutes INTEGER DEFAULT 60,
            UNIQUE(project_id, name),
            FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            block_id INTEGER NOT NULL,
            minutes INTEGER NOT NULL,
            date TEXT NOT NULL,
            FOREIGN KEY(block_id) REFERENCES blocks(id) ON DELETE CASCADE
        );
    )";

    char* err = nullptr;
    if (sqlite3_exec(db, sql, nullptr, nullptr, &err) != SQLITE_OK) {
        if (err) {
            std::cerr << "Şema hatası: " << err << std::endl;
            sqlite3_free(err);
        }
    }
}

// --- Bridge Fonksiyonları ---

bool TrackerEngine::create_project(rust::Str name, rust::Str refresh) {
    std::string sql = "INSERT OR IGNORE INTO projects (name, refresh_interval) VALUES ('"
                      + to_string(name) + "', '" + to_string(refresh) + "');";
    return sqlite3_exec(db, sql.c_str(), nullptr, nullptr, nullptr) == SQLITE_OK;
}

bool TrackerEngine::add_block_to_project(rust::Str project_name, rust::Str block_name, int32_t target) {
    std::string sql = "INSERT OR IGNORE INTO blocks (project_id, name, target_minutes) "
                      "SELECT id, '" + to_string(block_name) + "', " + std::to_string(target) +
                      " FROM projects WHERE name = '" + to_string(project_name) + "';";
    return sqlite3_exec(db, sql.c_str(), nullptr, nullptr, nullptr) == SQLITE_OK;
}

bool TrackerEngine::add_entry_v2(rust::Str project_name, rust::Str block_name, int32_t minutes, rust::Str date) {
    std::string sql = "INSERT INTO entries (block_id, minutes, date) "
                      "SELECT b.id, " + std::to_string(minutes) + ", '" + to_string(date) + "' "
                      "FROM blocks b JOIN projects p ON b.project_id = p.id "
                      "WHERE p.name = '" + to_string(project_name) + "' AND b.name = '" + to_string(block_name) + "';";
    return sqlite3_exec(db, sql.c_str(), nullptr, nullptr, nullptr) == SQLITE_OK;
}

rust::Vec<DailyStat> TrackerEngine::get_project_report(rust::Str project_name) const {
    rust::Vec<DailyStat> stats;
    std::string query = R"(
        SELECT e.date, b.name, SUM(e.minutes), b.target_minutes
        FROM entries e
        JOIN blocks b ON e.block_id = b.id
        JOIN projects p ON b.project_id = p.id
        WHERE p.name = ')" + to_string(project_name) + R"('
        GROUP BY e.date, b.name
        ORDER BY e.date DESC;
    )";

    sqlite3_stmt* stmt = nullptr;
    if (sqlite3_prepare_v2(db, query.c_str(), -1, &stmt, nullptr) == SQLITE_OK) {
        while (sqlite3_step(stmt) == SQLITE_ROW) {
            DailyStat row;
            row.date = (const char*)sqlite3_column_text(stmt, 0);
            row.block_name = (const char*)sqlite3_column_text(stmt, 1);
            row.total_minutes = sqlite3_column_int(stmt, 2);
            row.target_minutes = sqlite3_column_int(stmt, 3);
            stats.push_back(std::move(row));
        }
    }
    sqlite3_finalize(stmt);
    return stats;
}

bool TrackerEngine::cleanup_project(rust::Str project_name) {
    std::string sql = "DELETE FROM entries WHERE block_id IN ( "
                      "SELECT b.id FROM blocks b JOIN projects p ON b.project_id = p.id "
                      "WHERE p.name = '" + to_string(project_name) + "');";
    return sqlite3_exec(db, sql.c_str(), nullptr, nullptr, nullptr) == SQLITE_OK;
}

rust::Vec<BlockInfo> TrackerEngine::get_project_blocks(rust::Str project_name) const {
    rust::Vec<BlockInfo> blocks;
    sqlite3_stmt* stmt;
    
    std::string query = "SELECT b.name, b.target_minutes, COALESCE(SUM(e.minutes), 0) as total_minutes, "
                       "CASE WHEN COUNT(e.id) > 0 THEN 1 ELSE 0 END as has_data "
                       "FROM blocks b "
                       "LEFT JOIN entries e ON b.id = e.block_id "
                       "JOIN projects p ON b.project_id = p.id "
                       "WHERE p.name = '" + to_string(project_name) + "' "
                       "GROUP BY b.id, b.name, b.target_minutes "
                       "ORDER BY b.name;";
    
    if (sqlite3_prepare_v2(db, query.c_str(), -1, &stmt, nullptr) == SQLITE_OK) {
        while (sqlite3_step(stmt) == SQLITE_ROW) {
            BlockInfo block;
            block.name = (const char*)sqlite3_column_text(stmt, 0);
            block.target_minutes = sqlite3_column_int(stmt, 1);
            block.total_minutes = sqlite3_column_int(stmt, 2);
            block.has_data = sqlite3_column_int(stmt, 3) > 0;
            blocks.push_back(std::move(block));
        }
    }
    sqlite3_finalize(stmt);
    return blocks;
}

// Geriye dönük uyumluluk için eski metodların stub'ları
bool TrackerEngine::create_block(rust::Str name) { return create_project("default", "7d") && add_block_to_project("default", name, 60); }
bool TrackerEngine::add_entry(rust::Str b, int32_t m, rust::Str d) { return add_entry_v2("default", b, m, d); }
rust::Vec<DailyStat> TrackerEngine::get_report(int32_t d) const { (void)d; return get_project_report("default"); }

// Fabrika fonksiyonu
std::unique_ptr<TrackerEngine> init_engine(rust::Str db_path) {
    return std::make_unique<TrackerEngine>(to_string(db_path));
}
