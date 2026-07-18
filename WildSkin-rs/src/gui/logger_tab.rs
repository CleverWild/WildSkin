//! The "Logger" tab: dumps the shared logger's buffered lines.

use hudhook::imgui::Ui;

/// Prints every buffered log line, oldest first, with no scrollback limit or
/// filtering.
pub(super) fn render_logger_tab(ui: &Ui) {
    let state = crate::state::get();
    let logger = state.logger.lock().unwrap();
    for line in logger.lines() {
        ui.text(line);
    }
}
