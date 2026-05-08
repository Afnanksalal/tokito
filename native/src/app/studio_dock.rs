//! Studio docking: `egui_dock` tabs + [`TabViewer`] that delegates to [`App`](super::App) methods.
//!
//! # Safety
//! The viewer holds a raw `*mut App` and is only used for a single `DockArea::show_inside` call
//! on the UI thread—no re-entrancy into the same `DockState` from within tab `ui`.

use egui::{Id, Ui, WidgetText};
use egui_dock::TabViewer;
use uuid::Uuid;

use super::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StudioTab {
    Canvas,
    Copilot,
    Inspector,
    Bom,
    Parts,
    Console,
}

impl StudioTab {
    pub const ALL: [StudioTab; 6] = [
        StudioTab::Canvas,
        StudioTab::Copilot,
        StudioTab::Inspector,
        StudioTab::Bom,
        StudioTab::Parts,
        StudioTab::Console,
    ];
}

pub struct AppDockViewer {
    pub app: *mut App,
    pub design_id: Uuid,
}

impl TabViewer for AppDockViewer {
    type Tab = StudioTab;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        match tab {
            StudioTab::Canvas => "Canvas".into(),
            StudioTab::Copilot => "Copilot".into(),
            StudioTab::Inspector => "Inspector".into(),
            StudioTab::Bom => "BOM".into(),
            StudioTab::Parts => "Parts".into(),
            StudioTab::Console => "Console".into(),
        }
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        let app = unsafe { &mut *self.app };
        match tab {
            StudioTab::Canvas => app.render_studio_canvas_tab(ui, self.design_id),
            StudioTab::Copilot => app.render_studio_copilot_tab(ui, self.design_id),
            StudioTab::Inspector => app.render_studio_inspector_tab(ui),
            StudioTab::Bom => app.render_studio_bom_tab(ui, self.design_id),
            StudioTab::Parts => app.render_studio_parts_tab(ui),
            StudioTab::Console => app.render_studio_console_tab(ui),
        }
    }

    fn id(&mut self, tab: &mut Self::Tab) -> Id {
        Id::new(("tokito_studio_tab", format!("{tab:?}")))
    }

    fn closeable(&mut self, tab: &mut Self::Tab) -> bool {
        !matches!(tab, StudioTab::Canvas)
    }

    fn scroll_bars(&self, tab: &Self::Tab) -> [bool; 2] {
        match tab {
            StudioTab::Canvas => [false, false],
            _ => [true, true],
        }
    }
}
