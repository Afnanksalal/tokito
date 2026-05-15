//! Studio dock tabs (`egui_dock` TabViewer).

use egui::{Id, Ui, WidgetText};
use egui_dock::{DockState, NodeIndex, TabViewer};
use uuid::Uuid;

use super::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StudioTab {
    Canvas,
    Build,
    DesignManager,
    Bom,
    Messages,
    Research,
    Viewer3d,
    Console,
}

impl StudioTab {
    pub const DOCK_TABS: [StudioTab; 8] = [
        StudioTab::Canvas,
        StudioTab::Build,
        StudioTab::DesignManager,
        StudioTab::Bom,
        StudioTab::Messages,
        StudioTab::Research,
        StudioTab::Viewer3d,
        StudioTab::Console,
    ];
}

/// Default layout: canvas center, Build/BOM right, messages/console bottom.
pub fn default_studio_dock() -> DockState<StudioTab> {
    let mut dock = DockState::new(vec![StudioTab::Canvas]);
    let root = NodeIndex::root();
    let [canvas, _bottom] = dock.main_surface_mut().split_below(
        root,
        0.82,
        vec![StudioTab::Messages, StudioTab::Console],
    );
    let [_canvas, _right] = dock.main_surface_mut().split_right(
        canvas,
        0.68,
        vec![StudioTab::Build, StudioTab::Bom, StudioTab::DesignManager],
    );
    dock
}

pub struct AppDockViewer {
    pub app: *mut App,
    pub design_id: Uuid,
}

impl TabViewer for AppDockViewer {
    type Tab = StudioTab;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        match tab {
            StudioTab::Canvas => "Schematic".into(),
            StudioTab::Build => "Build".into(),
            StudioTab::DesignManager => "Design".into(),
            StudioTab::Bom => "BOM".into(),
            StudioTab::Messages => "Messages".into(),
            StudioTab::Research => "Research".into(),
            StudioTab::Viewer3d => "3D".into(),
            StudioTab::Console => "Console".into(),
        }
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
        let ctx = ui.ctx().clone();
        // SAFETY: `app` points at the `App` in `eframe::App::update`, which is mutably borrowed
        // for the duration of `DockArea::show_inside` and not moved.
        let app = unsafe { &mut *self.app };
        match tab {
            StudioTab::Canvas => app.render_studio_canvas_tab(ui, self.design_id),
            StudioTab::Build => app.render_studio_build_tab(ui, self.design_id),
            StudioTab::DesignManager => app.render_studio_design_manager_tab(ui),
            StudioTab::Bom => app.render_studio_bom_tab(ui, self.design_id),
            StudioTab::Messages => app.render_studio_messages_tab(ui),
            StudioTab::Research => app.render_studio_research_tab(ui, self.design_id),
            StudioTab::Viewer3d => app.render_studio_viewer3d_tab(ui, &ctx),
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
