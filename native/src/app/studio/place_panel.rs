//! Place panel: symbols, parts, and distributor catalog.

use crate::app::{App, PartRow};
use crate::editor::CanvasTool;
use crate::symbols_draw::CompKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PlaceScope {
    All,
    Symbols,
    #[default]
    Parts,
    Catalog,
    Power,
}

impl PlaceScope {
    const LABELS: &'static [(&'static str, PlaceScope)] = &[
        ("All", PlaceScope::All),
        ("Symbols", PlaceScope::Symbols),
        ("Parts", PlaceScope::Parts),
        ("LCSC", PlaceScope::Catalog),
        ("Power", PlaceScope::Power),
    ];
}

impl App {
    pub(crate) fn render_studio_place_panel(&mut self, ui: &mut egui::Ui) {
        let tokens = self.ui_tokens;
        let ty = crate::ui::TypeRamp::default();
        let chrome = crate::app::studio::chrome::TabChrome::begin(ui, &tokens);
        chrome.header(ui, "Place", None);

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
            for (label, scope) in PlaceScope::LABELS {
                if crate::ui::layout::filter_chip(
                    ui,
                    chrome.tokens,
                    label,
                    self.place_scope == *scope,
                ) {
                    self.place_scope = *scope;
                    if !self.place_query.trim().is_empty() {
                        self.run_place_search();
                    }
                }
            }
        });
        ui.add_space(8.0);

        let mut do_search = crate::ui::layout::search_field(
            ui,
            &mut self.place_query,
            "MPN, LM358, Device:R, GND…",
        );
        ui.horizontal(|ui| {
            if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Search").clicked() {
                do_search = true;
            }
            if ui
                .button(egui::RichText::new("Clear").small().weak())
                .clicked()
            {
                self.place_query.clear();
                self.parts_hits.clear();
            }
        });
        if do_search {
            self.run_place_search();
        }
        if let Some(lib) = self.base_symbols.as_ref() {
            ui.label(
                egui::RichText::new(format!("{} symbols", lib.symbol_count()))
                    .small()
                    .color(chrome.tokens.text_muted),
            );
        }
        ui.add_space(6.0);
        egui::CollapsingHeader::new("Import symbol library")
            .default_open(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.symbol_import_path)
                            .hint_text("Folder with .tokito_sym / .kicad_sym…")
                            .desired_width((ui.available_width() - 88.0).max(96.0)),
                    );
                    if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Import").clicked() {
                        self.import_symbol_library_folder();
                    }
                });
            });
        ui.add_space(6.0);

        egui::ScrollArea::vertical()
            .id_salt("place_panel_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| match self.place_scope {
                PlaceScope::Power => self.render_place_power(ui, chrome.tokens),
                PlaceScope::Symbols => self.render_place_symbols(ui, chrome.tokens, &ty),
                PlaceScope::Parts => self.render_place_parts(ui, chrome.tokens),
                PlaceScope::Catalog => self.render_place_catalog(ui, chrome.tokens),
                PlaceScope::All => self.render_place_all(ui, chrome.tokens, &ty),
            });
    }

    fn run_place_search(&mut self) {
        match self.place_scope {
            PlaceScope::Catalog => self.search_catalog(),
            PlaceScope::Parts | PlaceScope::All => {
                self.search_parts_catalog();
                if matches!(self.place_scope, PlaceScope::All) {
                    self.search_catalog();
                }
            }
            _ => {}
        }
    }

    fn render_place_catalog(&mut self, ui: &mut egui::Ui, tokens: &crate::ui::tokens::UiTokens) {
        let ty = crate::ui::TypeRamp::default();
        let nexar_on = self.state.nexar.is_some();
        let status = if nexar_on {
            "LCSC + Nexar catalog search"
        } else {
            "LCSC catalog search"
        };
        ui.label(ty.small_weak(status).color(tokens.text_muted));
        ui.add_space(6.0);
        if self.place_query.trim().is_empty() {
            ui.label(
                ty.small_weak("Type MPN or keyword above (search runs automatically).")
                    .color(tokens.text_muted),
            );
            return;
        }
        if self.catalog_hits.is_empty() {
            ui.label(
                ty.small_weak("No catalog hits — check spelling or try Search again.")
                    .color(tokens.text_muted),
            );
            if crate::ui::widgets::secondary_button(ui, tokens, "Search again").clicked() {
                self.search_catalog();
            }
            return;
        }
        ui.label(
            ty.small_weak("Click Place or a row, then click the schematic.")
                .color(tokens.text_muted),
        );
        ui.add_space(4.0);
        let hits = self.catalog_hits.clone();
        for h in hits {
            self.render_catalog_row(ui, tokens, &h);
        }
    }

    fn render_catalog_row(
        &mut self,
        ui: &mut egui::Ui,
        tokens: &crate::ui::tokens::UiTokens,
        h: &crate::app::CatalogHit,
    ) {
        let hit = h.clone();
        let placed = place_list_row(ui, tokens, |ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(&hit.mpn).monospace().strong());
                ui.label(
                    egui::RichText::new(format!("{} · {}", hit.distributor, hit.sku))
                        .small()
                        .weak()
                        .color(tokens.text_muted),
                );
                if let Some(pkg) = &hit.package_name {
                    ui.label(
                        egui::RichText::new(format!("Package: {pkg}"))
                            .small()
                            .color(tokens.text_secondary),
                    );
                }
                if let Some(fp) = &hit.footprint_hint {
                    ui.label(
                        egui::RichText::new(format!("Footprint: {fp}"))
                            .small()
                            .weak(),
                    );
                }
            });
        });
        if placed {
            self.place_catalog_hit(&hit);
        }
        ui.add_space(4.0);
    }

    fn symbol_hits(&self) -> Vec<String> {
        let q = self.place_query.trim();
        self.base_symbols
            .as_ref()
            .map(|lib| {
                if q.is_empty() {
                    lib.search("", 40)
                } else {
                    lib.search(q, 60)
                }
            })
            .unwrap_or_default()
    }

    fn render_place_all(
        &mut self,
        ui: &mut egui::Ui,
        tokens: &crate::ui::tokens::UiTokens,
        ty: &crate::ui::TypeRamp,
    ) {
        let q = self.place_query.trim();
        if q.is_empty() {
            ui.label(
                ty.small_weak("Search above, or use quick primitives and power nets below.")
                    .color(tokens.text_muted),
            );
            ui.add_space(10.0);
            self.render_place_primitives(ui, tokens);
            ui.add_space(12.0);
            self.render_place_power(ui, tokens);
            return;
        }

        let sym_hits = self.symbol_hits();
        let sym_count = sym_hits.len();
        if sym_count > 0 {
            crate::ui::layout::list_section_label(ui, tokens, "Symbols", sym_count);
            for name in sym_hits {
                self.render_symbol_row(ui, tokens, &name);
            }
        }

        if !self.catalog_hits.is_empty() {
            let hits = self.catalog_hits.clone();
            crate::ui::layout::list_section_label(ui, tokens, "Catalog", hits.len());
            for h in hits {
                self.render_catalog_row(ui, tokens, &h);
            }
        }

        if self.parts_hits.is_empty() {
            ui.add_space(6.0);
            ui.label(
                ty.small_weak("Press Search to query local parts + LCSC catalog.")
                    .color(tokens.text_muted),
            );
        } else {
            let parts = self.parts_hits.clone();
            crate::ui::layout::list_section_label(ui, tokens, "Parts", parts.len());
            for p in parts {
                self.render_part_row(ui, tokens, &p);
            }
        }

        if sym_count == 0 && self.parts_hits.is_empty() && self.catalog_hits.is_empty() {
            ui.label(
                ty.small_weak("No matches — try another keyword or scope.")
                    .color(tokens.text_muted),
            );
        }
    }

    fn render_place_symbols(
        &mut self,
        ui: &mut egui::Ui,
        tokens: &crate::ui::tokens::UiTokens,
        ty: &crate::ui::TypeRamp,
    ) {
        let hits = self.symbol_hits();
        if self.base_symbols.is_none() {
            ui.label(
                ty.small_weak("Symbol library failed to load — restart the app.")
                    .color(tokens.text_muted),
            );
        } else if hits.is_empty() {
            ui.label(
                ty.small_weak("No symbols match. Try Device:R, LM358, GND…")
                    .color(tokens.text_muted),
            );
        } else {
            ui.label(
                ty.small_weak("Click Place or a row, then click the schematic.")
                    .color(tokens.text_muted),
            );
            ui.add_space(4.0);
            for name in hits {
                self.render_symbol_row(ui, tokens, &name);
            }
        }
        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("Primitives")
                .small()
                .strong()
                .color(tokens.text_secondary),
        );
        ui.add_space(6.0);
        self.render_place_primitives(ui, tokens);
    }

    fn render_place_parts(&mut self, ui: &mut egui::Ui, tokens: &crate::ui::tokens::UiTokens) {
        let ty = crate::ui::TypeRamp::default();
        if self.place_query.trim().is_empty() {
            ui.label(
                ty.small_weak(
                    "Type MPN or description in the search box above (search runs automatically).",
                )
                .color(tokens.text_muted),
            );
            return;
        }
        if self.parts_hits.is_empty() {
            ui.label(
                ty.small_weak("No parts found — try another keyword or check the database.")
                    .color(tokens.text_muted),
            );
            if crate::ui::widgets::secondary_button(ui, tokens, "Search again").clicked() {
                self.search_parts_catalog();
            }
            return;
        }
        ui.label(
            ty.small_weak("Click Place or a row, then click the schematic.")
                .color(tokens.text_muted),
        );
        ui.add_space(4.0);
        let parts = self.parts_hits.clone();
        for p in parts {
            self.render_part_row(ui, tokens, &p);
        }
    }

    fn render_place_power(&mut self, ui: &mut egui::Ui, tokens: &crate::ui::tokens::UiTokens) {
        ui.label(
            egui::RichText::new("Power nets")
                .small()
                .strong()
                .color(tokens.text_secondary),
        );
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
            for name in ["VCC", "GND", "+3V3", "+5V", "+12V", "-5V", "VEE", "VDD"] {
                if crate::ui::layout::filter_chip(ui, tokens, name, false) {
                    self.editor.new_wire_net = name.to_string();
                    self.editor.tool = CanvasTool::Power;
                    self.log_console(format!("Power tool: {name}"));
                }
            }
        });
    }

    fn render_place_primitives(&mut self, ui: &mut egui::Ui, tokens: &crate::ui::tokens::UiTokens) {
        let primitives = [
            (CompKind::Resistor, "R", "Resistor"),
            (CompKind::Capacitor, "C", "Capacitor"),
            (CompKind::Inductor, "L", "Inductor"),
            (CompKind::Diode, "D", "Diode"),
            (CompKind::Transistor, "Q", "Transistor"),
            (CompKind::IC, "U", "IC"),
            (CompKind::IC, "J", "Connector"),
        ];
        for (kind, prefix, label) in primitives {
            ui.horizontal(|ui| {
                ui.set_min_height(40.0);
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(52.0, 36.0), egui::Sense::hover());
                paint_preview_primitive(ui, &tokens, self.base_symbols.as_ref(), rect, kind);
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(label).size(12.0));
                    ui.label(
                        egui::RichText::new(prefix)
                            .small()
                            .weak()
                            .color(tokens.text_muted),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if crate::ui::widgets::secondary_button(ui, tokens, "Place").clicked() {
                        self.place_generic_symbol(prefix);
                        self.log_console(format!("Click canvas to place {label}."));
                    }
                });
            });
            ui.add_space(4.0);
        }
    }

    fn render_symbol_row(
        &mut self,
        ui: &mut egui::Ui,
        tokens: &crate::ui::tokens::UiTokens,
        name: &str,
    ) {
        let short = name.rsplit(':').next().unwrap_or(name);
        let name_owned = name.to_string();
        let placed = place_list_row(ui, tokens, |ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(52.0, 36.0), egui::Sense::hover());
            paint_preview_symbol(ui, tokens, self.base_symbols.as_ref(), rect, Some(name));
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(short).monospace().strong());
                if name.contains(':') {
                    ui.label(
                        egui::RichText::new(name)
                            .small()
                            .weak()
                            .color(tokens.text_muted),
                    );
                }
            });
        });
        if placed {
            self.place_symbol_from_library(&name_owned);
        }
        ui.add_space(4.0);
    }

    fn render_part_row(
        &mut self,
        ui: &mut egui::Ui,
        tokens: &crate::ui::tokens::UiTokens,
        p: &PartRow,
    ) {
        let part = p.clone();
        let placed = place_list_row(ui, tokens, |ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(52.0, 36.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 3.0, tokens.preview_bg);
            ui.painter()
                .rect_stroke(rect.shrink(0.5), 3.0, tokens.stroke_subtle);
            let prefix = crate::util::guess_prefix(&part.mpn);
            let kind = crate::symbols_draw::kind_from_refdes(prefix);
            crate::symbols_draw::paint_symbol_body(
                ui.painter(),
                rect.center(),
                rect.width() * 0.28,
                rect.height() * 0.28,
                0.0,
                kind,
                tokens.sym_ink,
                1.35,
            );
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(&part.mpn).monospace().strong());
                if let Some(d) = &part.description {
                    ui.label(
                        egui::RichText::new(crate::util::truncate_ui_chars(d, 72))
                            .small()
                            .weak()
                            .color(tokens.text_muted),
                    );
                }
            });
        });
        if placed {
            self.drop_part_as_symbol(&part);
            self.log_console(format!("Placed {}", part.mpn));
        }
        ui.add_space(4.0);
    }

    fn place_symbol_from_library(&mut self, name: &str) {
        let label = name.to_string();
        let pre = crate::base_symbols::BaseSymbolLibrary::refdes_prefix_for_library_id(name);
        let pin_layout = self
            .base_symbols
            .as_ref()
            .map(|lib| lib.pin_layout_for(name))
            .unwrap_or_default();
        let default_value = self
            .base_symbols
            .as_ref()
            .map(|lib| lib.default_value_for(name))
            .unwrap_or_else(|| crate::component_value::default_value_for_library_id(name));
        self.editor.place_request = Some(crate::editor::PlaceSymbolRequest {
            prefix: pre.to_string(),
            part_id: None,
            symbol_id: Some(label),
            pin_layout,
            default_value,
        });
        self.editor.tool = CanvasTool::PlaceSymbol;
        self.log_console(format!("Place {name} on canvas."));
    }
}

/// Row with a real **Place** button; click the row or the button to arm placement.
fn place_list_row(
    ui: &mut egui::Ui,
    tokens: &crate::ui::tokens::UiTokens,
    body: impl FnOnce(&mut egui::Ui),
) -> bool {
    let row_id = ui.auto_id_with("place_list_row");
    let mut place = false;
    let frame = egui::Frame::none()
        .fill(tokens.bg_elevated)
        .stroke(tokens.stroke_subtle)
        .rounding(tokens.radius_sm)
        .inner_margin(egui::Margin::symmetric(8.0, 6.0));
    let inner = frame.show(ui, |ui| {
        ui.set_min_height(44.0);
        if ui.available_width() < 260.0 {
            ui.vertical(|ui| {
                body(ui);
                ui.add_space(6.0);
                if crate::ui::widgets::secondary_button(ui, tokens, "Place").clicked() {
                    place = true;
                }
            });
        } else {
            ui.horizontal(|ui| {
                body(ui);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if crate::ui::widgets::secondary_button(ui, tokens, "Place").clicked() {
                        place = true;
                    }
                });
            });
        }
    });
    let row_rect = inner.response.rect;
    let row_resp = ui.interact(row_rect, row_id, egui::Sense::click());
    if row_resp.clicked() || row_resp.double_clicked() {
        place = true;
    }
    if row_resp.hovered() || inner.response.hovered() {
        ui.painter().rect_stroke(
            row_rect,
            tokens.radius_sm,
            egui::Stroke::new(1.0, tokens.accent),
        );
        row_resp.on_hover_text("Place on canvas (click row or Place, then click schematic)");
    }
    place
}

fn paint_preview_symbol(
    ui: &egui::Ui,
    tokens: &crate::ui::tokens::UiTokens,
    lib: Option<&crate::base_symbols::BaseSymbolLibrary>,
    rect: egui::Rect,
    library_id_or_kind: Option<&str>,
) {
    ui.painter().rect_filled(rect, 3.0, tokens.preview_bg);
    ui.painter()
        .rect_stroke(rect.shrink(0.5), 3.0, tokens.stroke_subtle);
    let ink = tokens.sym_ink;
    let spec = |kind: CompKind| {
        crate::base_symbols::SymbolPaintSpec::new(
            rect.center(),
            rect.width() * 0.35,
            rect.height() * 0.35,
            0.0,
            kind,
            ink,
            1.35,
            tokens.sym_outline,
        )
    };
    if let (Some(lib), Some(id)) = (lib, library_id_or_kind) {
        if lib.contains(id) {
            let kind = crate::symbols_draw::kind_from_refdes(
                &crate::base_symbols::BaseSymbolLibrary::refdes_prefix_for_library_id(id),
            );
            lib.paint_named_or_fallback(ui.painter(), spec(kind), id);
            return;
        }
        let short = id.rsplit(':').next().unwrap_or(id);
        let kind = crate::symbols_draw::kind_from_refdes(short);
        lib.paint_kind_or_fallback(ui.painter(), spec(kind));
    } else if let Some(id) = library_id_or_kind {
        let short = id.rsplit(':').next().unwrap_or(id);
        let kind = crate::symbols_draw::kind_from_refdes(short);
        crate::symbols_draw::paint_symbol_body(
            ui.painter(),
            rect.center(),
            rect.width() * 0.35,
            rect.height() * 0.35,
            0.0,
            kind,
            ink,
            1.2,
        );
    }
}

fn paint_preview_primitive(
    ui: &egui::Ui,
    tokens: &crate::ui::tokens::UiTokens,
    lib: Option<&crate::base_symbols::BaseSymbolLibrary>,
    rect: egui::Rect,
    kind: CompKind,
) {
    ui.painter().rect_filled(rect, 3.0, tokens.preview_bg);
    ui.painter()
        .rect_stroke(rect.shrink(0.5), 3.0, tokens.stroke_subtle);
    let spec = crate::base_symbols::SymbolPaintSpec::new(
        rect.center(),
        rect.width() * 0.35,
        rect.height() * 0.35,
        0.0,
        kind,
        tokens.sym_ink,
        1.35,
        tokens.sym_outline,
    );
    if let Some(lib) = lib {
        lib.paint_kind_or_fallback(ui.painter(), spec);
    } else {
        crate::symbols_draw::paint_symbol_body(
            ui.painter(),
            rect.center(),
            rect.width() * 0.35,
            rect.height() * 0.35,
            0.0,
            kind,
            tokens.sym_ink,
            1.2,
        );
    }
}
