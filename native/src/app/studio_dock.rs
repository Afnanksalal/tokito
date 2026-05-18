//! Studio dock tabs (`egui_dock` TabViewer).

use egui::{Id, RichText, Ui, WidgetText};
use egui_dock::{DockState, NodeIndex, SurfaceIndex, TabViewer};
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
    Settings,
    Agent,
}

impl StudioTab {
    pub const DOCK_TABS: [StudioTab; 10] = [
        StudioTab::Canvas,
        StudioTab::Build,
        StudioTab::DesignManager,
        StudioTab::Bom,
        StudioTab::Messages,
        StudioTab::Research,
        StudioTab::Viewer3d,
        StudioTab::Console,
        StudioTab::Settings,
        StudioTab::Agent,
    ];

    /// Tabs the user can add from the dock “+” menu (Schematic stays singleton via non-closable root).
    pub const ADDABLE_TABS: [StudioTab; 9] = [
        StudioTab::Build,
        StudioTab::DesignManager,
        StudioTab::Bom,
        StudioTab::Messages,
        StudioTab::Research,
        StudioTab::Viewer3d,
        StudioTab::Console,
        StudioTab::Settings,
        StudioTab::Agent,
    ];

    pub const fn panel_label(self) -> &'static str {
        match self {
            StudioTab::Canvas => "Schematic",
            StudioTab::Build => "Build",
            StudioTab::DesignManager => "Design",
            StudioTab::Bom => "BOM",
            StudioTab::Messages => "Messages",
            StudioTab::Research => "Research",
            StudioTab::Viewer3d => "Preview",
            StudioTab::Console => "Console",
            StudioTab::Settings => "Settings",
            StudioTab::Agent => "Agent",
        }
    }
}

/// Default layout: schematic center, build/BOM/design/preview on the right, messages/research/console below.
pub fn default_studio_dock() -> DockState<StudioTab> {
    let mut dock = DockState::new(vec![StudioTab::Canvas]);
    let root = NodeIndex::root();
    let [canvas, _bottom] = dock.main_surface_mut().split_below(
        root,
        0.78,
        vec![StudioTab::Messages, StudioTab::Research, StudioTab::Console],
    );
    let [_canvas, _right] = dock.main_surface_mut().split_right(
        canvas,
        0.66,
        vec![
            StudioTab::Build,
            StudioTab::Bom,
            StudioTab::DesignManager,
            StudioTab::Viewer3d,
        ],
    );
    dock
}

/// Focus an existing tab or insert it into the currently focused dock leaf if the user closed it.
pub fn ensure_tab_visible(dock: &mut DockState<StudioTab>, tab: StudioTab) {
    if let Some((surface, node, tab_idx)) = dock.find_tab(&tab) {
        dock.set_active_tab((surface, node, tab_idx));
        dock.set_focused_node_and_surface((surface, node));
    } else if dock.main_surface().num_tabs() == 0 {
        *dock = default_studio_dock();
        ensure_tab_visible(dock, tab);
    } else {
        dock.push_to_focused_leaf(tab);
    }
}

pub struct AppDockViewer {
    pub app: *mut App,
    pub design_id: Uuid,
}

impl TabViewer for AppDockViewer {
    type Tab = StudioTab;

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        tab.panel_label().into()
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
            StudioTab::DesignManager => app.render_studio_design_manager_tab(ui, self.design_id),
            StudioTab::Bom => app.render_studio_bom_tab(ui, self.design_id),
            StudioTab::Messages => app.render_studio_messages_tab(ui),
            StudioTab::Research => app.render_studio_research_tab(ui, self.design_id),
            StudioTab::Viewer3d => app.render_studio_viewer3d_tab(ui, &ctx),
            StudioTab::Console => app.render_studio_console_tab(ui),
            StudioTab::Settings => app.render_studio_settings_tab(ui, &ctx),
            StudioTab::Agent => app.render_studio_agent_tab(ui, self.design_id),
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

    fn add_popup(&mut self, ui: &mut Ui, surface: SurfaceIndex, node: NodeIndex) {
        ui.label(RichText::new("Add panel to this tab group").small().weak());
        ui.separator();
        let app = unsafe { &mut *self.app };
        egui::ScrollArea::vertical()
            .max_height(260.0)
            .show(ui, |ui| {
                for kind in StudioTab::ADDABLE_TABS {
                    let label = kind.panel_label();
                    let already = app.dock_state.find_tab(&kind).is_some();
                    if already {
                        ui.add_enabled(false, egui::Button::new(format!("{label} (already open)")));
                    } else if ui.button(label).clicked() {
                        app.dock_state[surface][node].append_tab(kind);
                    }
                }
            });
    }
}
