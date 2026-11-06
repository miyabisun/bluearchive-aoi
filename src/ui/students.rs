//! The student tab: the students read, ordered by the column chosen, each markable for
//! training.

use gpui::{
    AnyElement, Context, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _, Role,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
    prelude::FluentBuilder as _, px,
};

use super::{
    Aoi, icons,
    theme::{FS_CAPTION, FS_SM, RADIUS_SM, SP_1, SP_2, Theme, tabular},
    widgets::{text, tooltip},
};
use crate::{plan::Target, view};

/// The columns after the name, each ordering the table by its key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Level,
    Star,
    Skills,
    Equipment,
    Gear,
    Ability,
    Bond,
}

impl Column {
    pub const ALL: [(Self, &'static str, bool); 7] = [
        (Self::Level, "Lv", true),
        (Self::Star, "星", false),
        (Self::Skills, "スキル", false),
        (Self::Equipment, "装備", false),
        (Self::Gear, "愛用品", false),
        (Self::Ability, "能力解放", false),
        (Self::Bond, "絆", true),
    ];

    pub fn key(self) -> view::StudentColumn {
        match self {
            Self::Level => view::StudentColumn::Level,
            Self::Star => view::StudentColumn::Star,
            Self::Skills => view::StudentColumn::Skills,
            Self::Equipment => view::StudentColumn::Equipment,
            Self::Gear => view::StudentColumn::Gear,
            Self::Ability => view::StudentColumn::Ability,
            Self::Bond => view::StudentColumn::Bond,
        }
    }
}

/// What the student tab tells before any student is read.
pub const EMPTY: &str = "ブルーアーカイブの生徒詳細画面（基本情報）を開いて「＞」で送ると、見えた生徒の育成状況がここに並びます。";

/// The stars the game draws for `star` (★1–★5, then 固有1–固有4 on the weapon's), 1em a star,
/// overlapping as the game's: their left edges 0.833 of a star apart. Its name to read it by.
pub fn stars(key: &str, star: u8, size: gpui::Pixels) -> impl IntoElement {
    let marks = view::star_marks(star);
    let box_size = size * (32. / 28.);
    let step = size * 0.833;
    div()
        .id(SharedString::from(format!("stars-{key}")))
        .role(Role::Image)
        .aria_label(marks.label.clone())
        .debug_selector(move || format!("stars:{star}"))
        .flex()
        .items_center()
        .children(marks.places.map(|i| {
            icons::star_image(marks.kind == view::StarKind::Weapon, box_size)
                .when(i > 0, |s| s.ml(step - box_size))
        }))
}

impl Aoi {
    pub(super) fn students_tab(&mut self, theme: &Theme, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if self.shown.students.is_empty() {
            return super::widgets::empty(theme, "students", EMPTY).into_any_element();
        }
        let query = self.query.read(cx).text().to_owned();
        let found = view::filter_rows(&self.shown.students, &query);
        let shown = view::sort_students(found, self.sort_by.key());
        let search = super::search(theme, &self.query);
        if shown.is_empty() {
            return div()
                .flex()
                .flex_col()
                .child(search)
                .child(super::widgets::empty(theme, "no-match", "一致する項目がありません"))
                .into_any_element();
        }
        let targeted: std::collections::HashSet<i64> = self.shown.targets.iter().map(|t| t.id).collect();
        let elephs: std::collections::HashMap<i64, crate::quantity::Count> = self
            .shown
            .items
            .iter()
            .filter(|r| r.eleph)
            .map(|r| (r.id, r.count))
            .collect();
        let rule = theme.border;
        let cell = |grow: bool| {
            let c = div().px(SP_2).py(SP_2).whitespace_nowrap();
            if grow { c.flex_1().min_w(px(8. * 16.)).whitespace_normal() } else { c.flex_none() }
        };
        // The column widths, as the table lays them out: the mark, then each column's.
        let widths = [px(48.), px(52.), px(96.), px(72.), px(96.), px(64.), px(84.), px(40.)];
        let sort_by = self.sort_by;
        let head = div()
            .flex()
            .items_center()
            .border_b_1()
            .border_color(rule)
            .bg(theme.surface)
            .text_size(FS_SM)
            .text_color(theme.muted)
            .child(cell(false).w(widths[0]).child(text("育成")))
            .child(cell(true).child(text("生徒")))
            .children(Column::ALL.into_iter().enumerate().map(|(i, (column, label, num))| {
                let sorted = sort_by == column;
                div()
                    .id(SharedString::from(format!("sort-{label}")))
                    .role(Role::ColumnHeader)
                    .aria_label(label)
                    .when(sorted, |h| h.aria_description("降順"))
                    .tab_stop(true)
                    .w(widths[i + 1])
                    .flex_none()
                    .flex()
                    .when(num, |h| h.justify_end())
                    .px(SP_2)
                    .py(SP_2)
                    .cursor_pointer()
                    .when(sorted, |h| h.text_color(theme.on_surface).font_weight(FontWeight::SEMIBOLD))
                    .child(text(label))
                    .when(sorted, |h| h.child(div().ml(px(2.)).text_size(FS_CAPTION).child("▼")))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.sort_by = column;
                        cx.notify();
                    }))
            }));
        let rows = shown.iter().map(|row| {
            let cells = view::student_cells(&row.training);
            let marked = targeted.contains(&row.id);
            let id = row.id;
            let check = div()
                .id(SharedString::from(format!("mark-{id}")))
                .role(Role::CheckBox)
                .aria_label(format!("{}を育成対象にする", row.name))
                .aria_toggled(if marked { gpui::Toggled::True } else { gpui::Toggled::False })
                .tab_stop(true)
                .size(px(16.))
                .flex()
                .items_center()
                .justify_center()
                .border_1()
                .border_color(if marked { theme.accent } else { theme.muted })
                .rounded(px(3.))
                .when(marked, |c| c.bg(theme.accent).text_color(theme.surface).text_size(FS_CAPTION).child("✓"))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    let _ = this.core.set_target(id, (!marked).then(Target::default));
                    cx.notify();
                }));
            let star_title = view::star_title(&cells.weapon, elephs.get(&row.id).copied());
            let read = |key: &str, value: String| {
                super::widgets::read(format!("student-{id}-{key}"), value)
            };
            div()
                .flex()
                .items_center()
                .border_b_1()
                .border_color(rule)
                .child(cell(false).w(widths[0]).flex().justify_center().child(check))
                .child(cell(true).child(read("name", row.name.clone())))
                .child(cell(false).w(widths[1]).flex().justify_end().font_features(tabular()).child(read("level", cells.level.clone())))
                .child(
                    div()
                        .id(SharedString::from(format!("student-{id}-star")))
                        .w(widths[2])
                        .flex_none()
                        .px(SP_2)
                        .py(SP_2)
                        .tooltip(tooltip(star_title))
                        .child(match cells.star {
                            Some(star) => stars(&format!("student-{id}"), star, px(16.)).into_any_element(),
                            None => text("?").into_any_element(),
                        }),
                )
                .child(cell(false).w(widths[3]).font_features(tabular()).child(read("skills", cells.skills.clone())))
                .child(cell(false).w(widths[4]).font_features(tabular()).child(read("equipment", cells.equipment.clone())))
                .child(cell(false).w(widths[5]).child(read("gear", cells.gear.clone())))
                .child(cell(false).w(widths[6]).font_features(tabular()).child(read("ability", cells.ability.clone())))
                .child(cell(false).w(widths[7]).flex().justify_end().font_features(tabular()).child(read("bond", cells.bond.clone())))
        });
        let _ = RADIUS_SM;
        let _ = SP_1;
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(search)
            .child(
                super::scroll("students-scroll", &self.content)
                    .child(div().min_w(px(640.)).child(head).children(rows)),
            )
            .into_any_element()
    }
}
