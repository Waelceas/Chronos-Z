const std = @import("std");
const c = @cImport({
    @cInclude("imgui.h");
    @cInclude("imgui_impl_glfw.h");
    @cInclude("imgui_impl_opengl3.h");
    @cInclude("GLFW/glfw3.h");
    @cInclude("../core/engine.hpp");
});

extern fn init_engine([*c]const u8) ?*c.TrackerEngine;
extern fn create_project(engine: *c.TrackerEngine, name: [*c]const u8, refresh: [*c]const u8) bool;
extern fn add_block_to_project(engine: *c.TrackerEngine, project_name: [*c]const u8, block_name: [*c]const u8, target: i32) bool;
extern fn add_entry_v2(engine: *c.TrackerEngine, project_name: [*c]const u8, block_name: [*c]const u8, minutes: i32, date: [*c]const u8) bool;

// Rust'tan çağrılacak ana fonksiyon
export fn zig_gui_main() i32 {
    gui_main_internal() catch |err| {
        std.log.err("GUI hatası: {}", .{err});
        return 1;
    };
    return 0;
}

fn gui_main_internal() !void {
    // GLFW başlat
    if (c.glfwInit() != c.GLFW_TRUE) {
        return error.GlfwInitFailed;
    }
    defer c.glfwTerminate();

    // OpenGL ayarları
    c.glfwWindowHint(c.GLFW_CONTEXT_VERSION_MAJOR, 3);
    c.glfwWindowHint(c.GLFW_CONTEXT_VERSION_MINOR, 3);
    c.glfwWindowHint(c.GLFW_OPENGL_PROFILE, c.GLFW_OPENGL_CORE_PROFILE);

    // Pencere oluştur
    const window = c.glfwCreateWindow(800, 600, "Timer Tracker", null, null) orelse {
        return error.WindowCreationFailed;
    };
    defer c.glfwDestroyWindow(window);

    c.glfwMakeContextCurrent(window);
    c.glfwSwapInterval(1);

    // ImGui başlat
    _ = c.igCreateContext(null);
    const io = c.igGetIO();
    io.*.ConfigFlags |= c.ImGuiConfigFlags_NavEnableKeyboard;

    _ = c.igGlfwInitForOpenGL(window, true);
    _ = c.igOpenGL3Init("#version 330");

    // Engine'i başlat
    const engine = init_engine("tracker.db") orelse {
        return error.EngineInitFailed;
    };

    // GUI state
    var project_name: [128:0]u8 = std.mem.zeroes([128:0]u8);
    var block_name: [128:0]u8 = std.mem.zeroes([128:0]u8);
    var minutes: i32 = 25;
    var timer_running: bool = false;
    var timer_start: std.time.Instant = undefined;
    var current_seconds: i32 = 0;

    // Ana döngü
    while (c.glfwWindowShouldClose(window) != c.GLFW_TRUE) {
        c.glfwPollEvents();

        // Yeni frame
        c.igOpenGL3NewFrame();
        c.igGlfwNewFrame();
        c.igNewFrame();

        // Ana pencere
        {
            _ = c.igBegin("Timer Tracker", null, 0);

            // Timer durumunu güncelle
            if (timer_running) {
                const now = std.time.Instant.now() catch unreachable;
                const elapsed = now.since(timer_start);
                current_seconds = @intCast(elapsed / std.time.ns_per_s);

                // Hedef süreyi kontrol et
                if (current_seconds >= minutes * 60) {
                    timer_running = false;
                    // Veritabanına kaydet
                    if (std.mem.len(project_name[0..]) > 0 and std.mem.len(block_name[0..]) > 0) {
                        const project_str = std.mem.sliceTo(&project_name, 0);
                        const block_str = std.mem.sliceTo(&block_name, 0);
                        _ = add_entry_v2(engine, project_str, block_str, minutes, "today");
                    }
                    current_seconds = 0;
                }
            }

            // Input alanları
            _ = c.igInputText("Proje Adı", &project_name, project_name.len, 0, null, null);
            _ = c.igInputText("Blok Adı", &block_name, block_name.len, 0, null, null);
            _ = c.igInputInt("Süre (dakika)", &minutes, 1, 5, 0);

            // Timer kontrol butonları
            if (!timer_running) {
                if (c.igButton("Başlat", .{ .x = 0, .y = 0 })) {
                    if (std.mem.len(project_name[0..]) > 0 and std.mem.len(block_name[0..]) > 0) {
                        // Proje ve blok oluştur
                        const project_str = std.mem.sliceTo(&project_name, 0);
                        const block_str = std.mem.sliceTo(&block_name, 0);
                        
                        _ = create_project(engine, project_str, "7d");
                        _ = add_block_to_project(engine, project_str, block_str, minutes);

                        timer_running = true;
                        timer_start = std.time.Instant.now() catch unreachable;
                        current_seconds = 0;
                    }
                }
            } else {
                if (c.igButton("Durdur", .{ .x = 0, .y = 0 })) {
                    timer_running = false;
                    // Veritabanına kaydet
                    if (std.mem.len(project_name[0..]) > 0 and std.mem.len(block_name[0..]) > 0) {
                        const project_str = std.mem.sliceTo(&project_name, 0);
                        const block_str = std.mem.sliceTo(&block_name, 0);
                        const elapsed_minutes = @divTrunc(current_seconds, 60);
                        if (elapsed_minutes > 0) {
                            _ = add_entry_v2(engine, project_str, block_str, elapsed_minutes, "today");
                        }
                    }
                    current_seconds = 0;
                }
            }

            // Timer göstergesi
            if (timer_running) {
                _ = c.igSeparator();
                const remaining = (minutes * 60) - current_seconds;
                const display_min = @divTrunc(remaining, 60);
                const display_sec = @rem(remaining, 60);
                
                var buffer: [32:0]u8 = undefined;
                const text = std.fmt.bufPrintZ(&buffer, "Kalan süre: {d:0>2}:{d:0>2}", .{ display_min, display_sec }) catch unreachable;
                c.igText(text);

                // Progress bar
                const progress = @as(f32, @floatFromInt(current_seconds)) / @as(f32, @floatFromInt(minutes * 60));
                _ = c.igProgressBar(progress, .{ .x = 200, .y = 0 }, null, null);
            }

            // Raporlama
            _ = c.igSeparator();
            c.igText("Bugünkü çalışma:");
            c.igText("- Henüz kayıt yok (Rust bridge gerekli)");

            c.igEnd();
        }

        // Render
        c.igRender();
        var display_w: i32 = undefined;
        var display_h: i32 = undefined;
        c.glfwGetFramebufferSize(window, &display_w, &display_h);
        c.glViewport(0, 0, display_w, display_h);
        c.glClearColor(0.1, 0.1, 0.1, 1.0);
        c.glClear(c.GL_COLOR_BUFFER_BIT);
        c.igOpenGL3RenderDrawData(c.igGetDrawData());

        c.glfwSwapBuffers(window);
    }

    // Temizlik
    c.igOpenGL3Shutdown();
    c.igGlfwShutdown();
    c.igDestroyContext();
}
