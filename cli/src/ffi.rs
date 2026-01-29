#[cxx::bridge]
pub mod bridge {
    struct DailyStat {
        pub date: String,
        pub block_name: String,
        pub total_minutes: i32,
        pub target_minutes: i32,
    }

    struct BlockInfo {
        pub name: String,
        pub target_minutes: i32,
        pub total_minutes: i32,
        pub has_data: bool,
    }

    unsafe extern "C++" {
        include!("core/engine.hpp");

        type TrackerEngine;

        fn init_engine(db_path: &str) -> UniquePtr<TrackerEngine>;

        // Eski metodlar
        fn create_block(self: Pin<&mut TrackerEngine>, name: &str) -> bool;
        fn add_entry(
            self: Pin<&mut TrackerEngine>,
            block_name: &str,
            minutes: i32,
            date: &str,
        ) -> bool;
        fn get_report(self: &TrackerEngine, days_back: i32) -> Vec<DailyStat>;

        // Yeni Proje/Tablo metodları
        fn create_project(self: Pin<&mut TrackerEngine>, name: &str, refresh: &str) -> bool;
        fn add_block_to_project(
            self: Pin<&mut TrackerEngine>,
            project_name: &str,
            block_name: &str,
            target: i32,
        ) -> bool;
        fn add_entry_v2(
            self: Pin<&mut TrackerEngine>,
            project_name: &str,
            block_name: &str,
            minutes: i32,
            date: &str,
        ) -> bool;
        fn get_project_report(self: &TrackerEngine, project_name: &str) -> Vec<DailyStat>;
        fn cleanup_project(self: Pin<&mut TrackerEngine>, project_name: &str) -> bool;
        
        // Yeni: Projenin tüm bloklarını getir (veri olsun olmasın)
        fn get_project_blocks(self: &TrackerEngine, project_name: &str) -> Vec<BlockInfo>;
    }
}

pub use bridge::*;
