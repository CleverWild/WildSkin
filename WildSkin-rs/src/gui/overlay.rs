//! Ports `Hooks.cpp`'s `init_imgui` (style/font) and `wndProc` render/hotkey
//! dispatch into `hudhook`'s `ImguiRenderLoop`.
//!
//! `initialize()` does ONLY style/font setup: it runs on the first frame, but
//! `state::get()` is valid only after the startup sequence (scan, config load,
//! `state::init()`) completes.
use hudhook::imgui::{Context, FontConfig, FontSource, Ui};
use hudhook::windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use hudhook::{ImguiRenderLoop, RenderContext};

pub struct Overlay;

/// Undoes hudhook's DX11 double-DPI-scale bug (see call site) by resetting
/// `io.display_framebuffer_scale` to `[1.0, 1.0]`.
///
/// Takes `&Ui` as a witness that an imgui context is live, tying `igGetIO()`'s
/// validity to the type system.
fn reset_display_framebuffer_scale(_ui: &Ui) {
    // SAFETY: `igGetIO()` returns imgui's global IO struct, valid for the
    // context's lifetime, which `_ui` proves is live.
    let io_raw = unsafe { hudhook::imgui::sys::igGetIO() };
    // SAFETY: `io_raw` was just resolved above and is not stored past this call.
    unsafe {
        (*io_raw.cast::<hudhook::imgui::Io>()).display_framebuffer_scale = [1.0, 1.0];
    }
}

fn apply_style(ctx: &mut Context) {
    let style = ctx.style_mut();
    style.window_padding = [6.0, 6.0];
    style.frame_padding = [6.0, 4.0];
    style.item_spacing = [6.0, 4.0];
    style.window_title_align = [0.5, 0.5];
    style.scrollbar_size = 12.0;
    style.window_border_size = 0.5;
    style.child_border_size = 0.5;
    style.popup_border_size = 0.5;
    style.frame_border_size = 0.0;
    style.window_rounding = 0.0;
    style.child_rounding = 0.0;
    style.frame_rounding = 0.0;
    style.scrollbar_rounding = 0.0;
    style.grab_rounding = 0.0;
    style.tab_rounding = 0.0;
    style.popup_rounding = 0.0;
    style.anti_aliased_fill = true;
    style.anti_aliased_lines = true;

    #[allow(
        clippy::enum_glob_use,
        reason = "one-off bulk color-table assignment; naming all ~45 variants is less readable than the glob"
    )]
    #[allow(
        clippy::items_after_statements,
        reason = "the glob import only makes sense scoped right before the color-table assignments it feeds"
    )]
    use hudhook::imgui::StyleColor::*;
    let c = &mut style.colors;
    c[Text as usize] = [1.00, 1.00, 1.00, 1.00];
    c[TextDisabled as usize] = [0.44, 0.44, 0.44, 1.00];
    c[WindowBg as usize] = [0.06, 0.06, 0.06, 1.00];
    c[ChildBg as usize] = [0.00, 0.00, 0.00, 0.00];
    c[PopupBg as usize] = [0.08, 0.08, 0.08, 0.94];
    c[Border as usize] = [0.51, 0.36, 0.15, 1.00];
    c[BorderShadow as usize] = [0.00, 0.00, 0.00, 0.00];
    c[FrameBg as usize] = [0.11, 0.11, 0.11, 1.00];
    c[FrameBgHovered as usize] = [0.51, 0.36, 0.15, 1.00];
    c[FrameBgActive as usize] = [0.78, 0.55, 0.21, 1.00];
    c[TitleBg as usize] = [0.51, 0.36, 0.15, 1.00];
    c[TitleBgActive as usize] = [0.91, 0.64, 0.13, 1.00];
    c[TitleBgCollapsed as usize] = [0.00, 0.00, 0.00, 0.51];
    c[MenuBarBg as usize] = [0.11, 0.11, 0.11, 1.00];
    c[ScrollbarBg as usize] = [0.06, 0.06, 0.06, 0.53];
    c[ScrollbarGrab as usize] = [0.21, 0.21, 0.21, 1.00];
    c[ScrollbarGrabHovered as usize] = [0.47, 0.47, 0.47, 1.00];
    c[ScrollbarGrabActive as usize] = [0.81, 0.83, 0.81, 1.00];
    c[CheckMark as usize] = [0.78, 0.55, 0.21, 1.00];
    c[SliderGrab as usize] = [0.91, 0.64, 0.13, 1.00];
    c[SliderGrabActive as usize] = [0.91, 0.64, 0.13, 1.00];
    c[Button as usize] = [0.51, 0.36, 0.15, 1.00];
    c[ButtonHovered as usize] = [0.91, 0.64, 0.13, 1.00];
    c[ButtonActive as usize] = [0.78, 0.55, 0.21, 1.00];
    c[Header as usize] = [0.51, 0.36, 0.15, 1.00];
    c[HeaderHovered as usize] = [0.91, 0.64, 0.13, 1.00];
    c[HeaderActive as usize] = [0.93, 0.65, 0.14, 1.00];
    c[Separator as usize] = [0.21, 0.21, 0.21, 1.00];
    c[SeparatorHovered as usize] = [0.91, 0.64, 0.13, 1.00];
    c[SeparatorActive as usize] = [0.78, 0.55, 0.21, 1.00];
    c[ResizeGrip as usize] = [0.21, 0.21, 0.21, 1.00];
    c[ResizeGripHovered as usize] = [0.91, 0.64, 0.13, 1.00];
    c[ResizeGripActive as usize] = [0.78, 0.55, 0.21, 1.00];
    c[Tab as usize] = [0.51, 0.36, 0.15, 1.00];
    c[TabHovered as usize] = [0.91, 0.64, 0.13, 1.00];
    c[TabActive as usize] = [0.78, 0.55, 0.21, 1.00];
    c[PlotLines as usize] = [0.61, 0.61, 0.61, 1.00];
    c[PlotLinesHovered as usize] = [1.00, 0.43, 0.35, 1.00];
    c[PlotHistogram as usize] = [0.90, 0.70, 0.00, 1.00];
    c[PlotHistogramHovered as usize] = [1.00, 0.60, 0.00, 1.00];
    c[TextSelectedBg as usize] = [0.26, 0.59, 0.98, 0.35];
    c[DragDropTarget as usize] = [1.00, 1.00, 0.00, 0.90];
    c[NavHighlight as usize] = [0.26, 0.59, 0.98, 1.00];
    c[NavWindowingHighlight as usize] = [1.00, 1.00, 1.00, 0.70];
    c[NavWindowingDimBg as usize] = [0.80, 0.80, 0.80, 0.20];
    c[ModalWindowDimBg as usize] = [0.80, 0.80, 0.80, 0.35];

    let io = ctx.io_mut();
    io.config_flags |= hudhook::imgui::ConfigFlags::NO_MOUSE_CURSOR_CHANGE;
}

// Glyph ranges from Hooks.cpp's tahomaRanges: [start, end] inclusive pairs,
// ImGui ImWchar format.
const TAHOMA_RANGES: &[u32] = &[
    0x0020, 0x00FF, 0x0100, 0x024F, 0x0250, 0x02FF, 0x0300, 0x03FF, 0x0400, 0x052F, 0x0530, 0x06FF,
    0x0E00, 0x0E7F, 0x1E00, 0x1FFF, 0x2000, 0x20CF, 0x2100, 0x218F, 0,
];

fn load_fonts(ctx: &mut Context) {
    // `dirs::font_dir()` is `None` on Windows, so use `%windir%\Fonts` (env var
    // always set).
    let Some(fonts_dir) =
        std::env::var_os("windir").map(|windir| std::path::PathBuf::from(windir).join("Fonts"))
    else {
        return;
    };

    let tahoma = fonts_dir.join("tahoma.ttf");
    let malgun = fonts_dir.join("malgun.ttf");
    let msyh = fonts_dir.join("msyh.ttc");

    let Ok(tahoma_bytes) = std::fs::read(&tahoma) else {
        return;
    };
    let malgun_bytes = std::fs::read(&malgun).ok();
    let msyh_bytes = std::fs::read(&msyh).ok();

    // `add_font` merges every source after the first automatically (imgui-rs
    // has no `merge_mode` field; merging is positional), so tahoma/malgun/msyh
    // must go in one call.
    let mut sources = vec![FontSource::TtfData {
        data: &tahoma_bytes,
        size_pixels: 15.0,
        config: Some(FontConfig {
            glyph_ranges: hudhook::imgui::FontGlyphRanges::from_slice(TAHOMA_RANGES),
            ..FontConfig::default()
        }),
    }];
    if let Some(malgun_bytes) = malgun_bytes.as_deref() {
        sources.push(FontSource::TtfData {
            data: malgun_bytes,
            size_pixels: 15.0,
            config: Some(FontConfig {
                glyph_ranges: hudhook::imgui::FontGlyphRanges::korean(),
                ..FontConfig::default()
            }),
        });
    }
    if let Some(msyh_bytes) = msyh_bytes.as_deref() {
        sources.push(FontSource::TtfData {
            data: msyh_bytes,
            size_pixels: 15.0,
            config: Some(FontConfig {
                glyph_ranges: hudhook::imgui::FontGlyphRanges::chinese_full(),
                ..FontConfig::default()
            }),
        });
    }
    ctx.fonts().add_font(&sources);
}

impl ImguiRenderLoop for Overlay {
    fn initialize<'a>(&'a mut self, ctx: &mut Context, _render_context: &'a mut dyn RenderContext) {
        apply_style(ctx);
        load_fonts(ctx);
    }

    fn render(&mut self, ui: &mut Ui) {
        let state = crate::state::get();
        // SAFETY: reads a running-state flag from resolved offsets; no game
        // memory dereferenced here.
        if !unsafe { state.offsets.is_running() } {
            return;
        }
        // SAFETY: game confirmed `Running` above, so the lists/pointers
        // `apply_frame` walks are live.
        unsafe {
            crate::skin_logic::apply_frame();
        }

        // hudhook 0.9.1 double-counts DPI on DX11: sets `io.display_size` to
        // true pixels AND `display_framebuffer_scale` from `GetDpiForWindow`,
        // then multiplies the two, breaking 125%/150% scaling. Must run every
        // frame: hudhook recomputes on each resize/Present-recreate.
        reset_display_framebuffer_scale(ui);

        if state.is_menu_open() {
            crate::gui::render(ui);
        }
    }

    fn after_wnd_proc(&self, _hwnd: HWND, umsg: u32, wparam: WPARAM, _lparam: LPARAM) {
        const WM_KEYDOWN: u32 = 0x0100;
        if umsg == WM_KEYDOWN {
            // SAFETY: `state::get()` is initialized before any WndProc hook is
            // reachable.
            unsafe {
                crate::skin_logic::handle_keydown(wparam.0 as i32);
            }
        }
    }
}
