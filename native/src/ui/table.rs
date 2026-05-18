//! Simple sortable data table header helper.

use egui::{RichText, Ui};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum SortDir {
    #[default]
    None,
    Asc,
    Desc,
}

#[derive(Default)]
pub struct SortState {
    pub column: usize,
    pub dir: SortDir,
}

impl SortState {
    pub fn toggle(&mut self, col: usize) {
        if self.column == col {
            self.dir = match self.dir {
                SortDir::None => SortDir::Asc,
                SortDir::Asc => SortDir::Desc,
                SortDir::Desc => SortDir::None,
            };
        } else {
            self.column = col;
            self.dir = SortDir::Asc;
        }
    }

    pub fn sort_strings(&self, rows: &mut [Vec<String>]) {
        if self.dir == SortDir::None {
            return;
        }
        let col = self.column;
        rows.sort_by(|a, b| {
            let x = a.get(col).map(String::as_str).unwrap_or("");
            let y = b.get(col).map(String::as_str).unwrap_or("");
            match self.dir {
                SortDir::Asc => x.cmp(y),
                SortDir::Desc => y.cmp(x),
                SortDir::None => std::cmp::Ordering::Equal,
            }
        });
    }
}

/// Scrollable table with sortable column headers.
pub fn data_table<R, FRow>(
    ui: &mut Ui,
    id: &str,
    headers: &[&str],
    sort: &mut SortState,
    rows: &mut [R],
    mut render_row: FRow,
) where
    FRow: FnMut(&mut Ui, &R),
{
    ui.horizontal(|ui| {
        for (col, label) in headers.iter().enumerate() {
            sortable_header(ui, label, col, sort);
        }
    });
    ui.add_space(4.0);
    egui::ScrollArea::vertical()
        .id_salt(id)
        .show(ui, |ui| {
            for row in rows.iter() {
                ui.horizontal(|ui| {
                    render_row(ui, row);
                });
                ui.add_space(2.0);
            }
        });
}

pub fn sortable_header(ui: &mut Ui, label: &str, col: usize, sort: &mut SortState) -> bool {
    let arrow = if sort.column == col {
        match sort.dir {
            SortDir::Asc => " ▲",
            SortDir::Desc => " ▼",
            SortDir::None => "",
        }
    } else {
        ""
    };
    let clicked = ui
        .button(RichText::new(format!("{label}{arrow}")).small().strong())
        .clicked();
    if clicked {
        sort.toggle(col);
    }
    clicked
}
