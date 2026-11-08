//! The training tab: the students marked and their targets, what the targets take against what
//! is held, and how to make up what is short by hand (the user's of 2026-10-13: chips per
//! source, a click pays one, a right click takes one back, with Shift all or none; 精算; リセット).

use std::time::Duration;

use gpui::{
    AnyElement, AppContext as _, Bounds, Context, Focusable as _, FontWeight,
    InteractiveElement as _, IntoElement, MouseButton, ParentElement as _, Pixels, Role,
    SharedString, StatefulInteractiveElement as _, Styled as _, StyledImage as _, Window, anchored,
    deferred, div, point, prelude::FluentBuilder as _, px,
};

use super::{
    Aoi, StarMenu, icons,
    students::stars,
    text_input::{self, TextInput},
    theme::{FS_CAPTION, FS_MD, FS_SM, RADIUS_SM, SP_1, SP_2, SP_3, SP_4, SP_5, Theme, tabular},
    widgets::{outlined_button, read, text, tooltip},
};
use crate::{plan::Target, view};

/// What the training tab tells before any student is marked.
pub const EMPTY: &str =
    "生徒タブで「育成」に印を付けた生徒の目標と、そこまでに要る素材の過不足がここに並びます。";

/// Held down, a chip pays (or takes back) again and again, as a key held down types.
const REPEAT_AFTER: Duration = Duration::from_millis(400);
const REPEAT_EVERY: Duration = Duration::from_millis(60);

/// The columns of a target: where each reads and changes, and its highest value; the star is
/// chosen from a menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Level,
    Star,
    Skill(usize),
    Equipment(usize),
}

pub const COLUMNS: [(Part, &str, u8); 9] = [
    (Part::Level, "Lv", 90),
    (Part::Star, "星", 8),
    (Part::Skill(0), "EX", 5),
    (Part::Skill(1), "N", 10),
    (Part::Skill(2), "P", 10),
    (Part::Skill(3), "S", 10),
    (Part::Equipment(0), "装備1", 10),
    (Part::Equipment(1), "装備2", 10),
    (Part::Equipment(2), "装備3", 10),
];

/// A target's value of `part`.
fn target_value(target: &Target, part: Part) -> u8 {
    match part {
        Part::Level => target.level,
        Part::Star => target.star.unwrap_or(0),
        Part::Skill(i) => target.skills[i],
        Part::Equipment(i) => target.equipment[i],
    }
}

/// `target` with `part` set to `value`, kept within the column's range (1 to `max`).
pub fn retarget(target: &Target, part: Part, value: &str, max: u8) -> Target {
    let n = value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
        .map_or(1, |n| n.round().clamp(1., f64::from(max)) as u8);
    let mut target = *target;
    match part {
        Part::Level => target.level = n,
        Part::Star => target.star = Some(n),
        Part::Skill(i) => target.skills[i] = n,
        Part::Equipment(i) => target.equipment[i] = n,
    }
    target
}

/// A student's training read for `part` ("?" unread).
fn training_value(training: Option<&crate::student::Training>, part: Part) -> String {
    let Some(t) = training else {
        return "?".to_owned();
    };
    let shown = |v: Option<u8>| v.map_or_else(|| "?".to_owned(), |v| v.to_string());
    match part {
        Part::Level => shown(t.level),
        Part::Star => String::new(),
        Part::Skill(i) => shown(t.skills[i]),
        Part::Equipment(i) => shown(t.equipment[i].map(|e| e.tier)),
    }
}

impl Aoi {
    /// The field of a target's `part` for the student `id`, kept while it is typed in so the
    /// snapshots of every frame leave what is typed; it sets the target on Enter or as the focus
    /// leaves.
    fn target_field(
        &mut self,
        id: i64,
        column: usize,
        value: u8,
        label: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<TextInput> {
        if let Some(field) = self.target_fields.get(&(id, column)) {
            let field = field.clone();
            // Not being typed in: it shows the target as it is now.
            if !field.read(cx).focus_handle(cx).contains_focused(window, cx) {
                field.update(cx, |f, cx| f.set_text(&value.to_string(), cx));
            }
            return field;
        }
        let field = cx.new(|cx| TextInput::new(label, &value.to_string(), cx));
        let commit =
            move |this: &mut Self, field: &gpui::Entity<TextInput>, cx: &mut Context<Self>| {
                let Some(row) = this.shown.targets.iter().find(|t| t.id == id) else {
                    return;
                };
                let (part, _, max) = COLUMNS[column];
                let typed = field.read(cx).text().to_owned();
                let target = retarget(&row.target, part, &typed, max);
                if target != row.target {
                    let _ = this.core.set_target(id, Some(target));
                }
            };
        cx.subscribe(&field, move |this, field, _: &text_input::Submitted, cx| {
            commit(this, &field, cx);
        })
        .detach();
        let blurred = field.read(cx).focus_handle(cx);
        cx.on_blur(&blurred, window, {
            let field = field.clone();
            move |this, _, cx| commit(this, &field, cx)
        })
        .detach();
        self.target_fields.insert((id, column), field.clone());
        field
    }

    pub(super) fn plan_tab(
        &mut self,
        theme: &Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.shown.targets.is_empty() {
            return super::widgets::empty(theme, "plan", EMPTY).into_any_element();
        }
        let mut sections: Vec<AnyElement> = Vec::new();
        sections.push(
            div()
                .flex()
                .items_center()
                .gap(SP_3)
                .mb(SP_2)
                .child(section_title("既存の生徒一覧"))
                .child(
                    small_button(theme, "reload", "再読込").on_click(cx.listener(
                        |this, _, _, cx| {
                            this.core.reload_plan();
                            cx.notify();
                        },
                    )),
                )
                .into_any_element(),
        );
        if let Some(behind) = &self.shown.plan_behind {
            sections.push(
                div()
                    .id("behind")
                    .role(Role::Status)
                    .aria_label(view::behind_text(Some(behind)))
                    .mb(SP_2)
                    .text_size(FS_SM)
                    .text_color(theme.warn)
                    .child(text(view::behind_text(Some(behind))))
                    .into_any_element(),
            );
        }
        sections.push(self.targets_table(theme, window, cx).into_any_element());
        for line in view::unknown_text(&self.shown.plan_unknown) {
            sections.push(
                div()
                    .mb(SP_2)
                    .text_size(FS_SM)
                    .text_color(theme.muted)
                    .child(text(format!("{line} は数えられません")))
                    .into_any_element(),
            );
        }
        sections.push(
            section_title("必要素材")
                .mt(SP_5)
                .mb(SP_2)
                .into_any_element(),
        );
        if self.shown.plan.is_empty() && self.shown.blueprints.is_empty() {
            sections.push(
                super::widgets::empty(theme, "reached", "目標までに要る素材はありません")
                    .into_any_element(),
            );
        } else {
            sections.push(self.needs(theme).into_any_element());
            sections.push(
                section_title("不足分の調達")
                    .mt(SP_5)
                    .mb(SP_2)
                    .into_any_element(),
            );
            sections.extend(self.procure(theme, cx));
            let places: Vec<crate::work::PlaceRow> = (self.shown.work.bounties.iter())
                .map(crate::work::BountyRow::place)
                .chain(self.shown.work.scrimmages.iter().cloned())
                .collect();
            let candidates = view::bounty_candidates(&self.shown.plan, &places);
            if !candidates.is_empty() {
                sections.push(
                    section_title("日々の行動")
                        .mt(SP_5)
                        .mb(SP_2)
                        .into_any_element(),
                );
                sections.push(daily(theme, &candidates).into_any_element());
            }
        }
        super::scroll("plan-scroll", &self.content)
            .on_scroll(cx.listener(|this, _, _, cx| {
                // Scrolled, the star menu would stay where its button was: closed instead.
                if this.star_menu.take().is_some() {
                    cx.notify();
                }
            }))
            .children(sections)
            .into_any_element()
    }

    fn targets_table(
        &mut self,
        theme: &Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let rule = theme.border;
        let head = div()
            .flex()
            .items_center()
            .border_b_1()
            .border_color(rule)
            .text_size(FS_SM)
            .text_color(theme.muted)
            .child(div().w(px(32.)).flex_none())
            .child(
                div()
                    .flex_1()
                    .min_w(px(8. * 16.))
                    .px(SP_1)
                    .py(SP_2)
                    .child(text("生徒")),
            )
            .children(COLUMNS.iter().map(|(_, label, _)| {
                div()
                    .w(column_width(label))
                    .flex_none()
                    .px(SP_1)
                    .py(SP_2)
                    .child(text(*label))
            }));
        let targets = self.shown.targets.clone();
        let rows: Vec<AnyElement> = targets
            .iter()
            .map(|row| {
                let id = row.id;
                let name = if row.name.is_empty() {
                    self.shown
                        .students
                        .iter()
                        .find(|s| s.id == id)
                        .map(|s| s.name.clone())
                        .unwrap_or_default()
                } else {
                    row.name.clone()
                };
                let training = row.training.or_else(|| {
                    self.shown
                        .students
                        .iter()
                        .find(|s| s.id == id)
                        .map(|s| s.training)
                });
                let remove = div()
                    .id(SharedString::from(format!("remove-{id}")))
                    .role(Role::Button)
                    .aria_label(format!("{name}を育成対象から外す"))
                    .tab_stop(true)
                    .size(px(24.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(RADIUS_SM)
                    .text_color(theme.danger)
                    .cursor_pointer()
                    .hover({
                        let bg = theme.danger_subtle;
                        move |s| s.bg(bg)
                    })
                    .tooltip(tooltip("育成対象から外す"))
                    .child(icons::glyph(icons::Glyph::Close, px(14.), theme.danger))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let _ = this.core.set_target(id, None);
                        cx.notify();
                    }));
                let mut cells: Vec<AnyElement> = Vec::new();
                for (column, (part, label, _)) in COLUMNS.iter().enumerate() {
                    let cell = div().w(column_width(label)).flex_none().px(SP_1).py(SP_1);
                    if *part == Part::Star {
                        let now = training.as_ref().and_then(view::star_of);
                        let to = row.target.star.or(now);
                        let open = self.star_menu.is_some_and(|m| m.id == id);
                        let picker = div()
                            .id(SharedString::from(format!("star-pick-{id}")))
                            .debug_selector(move || format!("star-pick-{id}"))
                            .role(Role::Button)
                            .aria_label(format!("{name}の星の目標"))
                            .when(open, |b| b.aria_description("開いています"))
                            .tab_stop(true)
                            .flex()
                            .items_center()
                            .gap(SP_1)
                            .px(SP_1)
                            .py(px(2.))
                            .border_1()
                            .border_color(if open { theme.accent } else { theme.border })
                            .rounded(RADIUS_SM)
                            .bg(theme.surface)
                            .text_color(theme.muted)
                            .text_size(FS_SM)
                            .whitespace_nowrap()
                            .cursor_pointer()
                            .child(match to {
                                Some(star) => {
                                    stars(&format!("to-{id}"), star, px(14.)).into_any_element()
                                }
                                None => text("?").into_any_element(),
                            })
                            .child("▾")
                            .on_click(cx.listener(
                                move |this, event: &gpui::ClickEvent, window, cx| {
                                    if this.star_menu.is_some_and(|m| m.id == id) {
                                        this.star_menu = None;
                                    } else {
                                        let at =
                                            window.focused(cx).and(None).unwrap_or_else(|| {
                                                Bounds::new(
                                                    event.position(),
                                                    gpui::size(px(1.), px(24.)),
                                                )
                                            });
                                        this.star_menu = Some(StarMenu { id, at });
                                    }
                                    cx.notify();
                                },
                            ));
                        cells.push(
                            cell.child(
                                div().text_size(FS_CAPTION).text_color(theme.muted).child(
                                    match now {
                                        Some(star) => stars(&format!("now-{id}"), star, px(12.))
                                            .into_any_element(),
                                        None => text("?").into_any_element(),
                                    },
                                ),
                            )
                            .child(picker)
                            .into_any_element(),
                        );
                    } else {
                        let value = target_value(&row.target, *part);
                        let field = self.target_field(
                            id,
                            column,
                            value,
                            format!("{name}の{label}の目標"),
                            window,
                            cx,
                        );
                        let focused = field.read(cx).focus_handle(cx).is_focused(window);
                        cells.push(
                            cell.child(
                                div()
                                    .text_size(FS_CAPTION)
                                    .text_color(theme.muted)
                                    .font_features(tabular())
                                    .child(text(training_value(training.as_ref(), *part))),
                            )
                            .child(
                                div()
                                    .id(SharedString::from(format!("target-{id}-{column}")))
                                    .debug_selector(move || format!("target-{id}-{column}"))
                                    .role(Role::SpinButton)
                                    .aria_label(format!("{name}の{label}の目標"))
                                    .w(px(3. * 16.))
                                    .h(px(28.))
                                    .px(SP_1)
                                    .border_1()
                                    .border_color(if focused { theme.accent } else { theme.border })
                                    .rounded(RADIUS_SM)
                                    .bg(theme.surface)
                                    .font_features(tabular())
                                    .child(field),
                            )
                            .into_any_element(),
                        );
                    }
                }
                div()
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(rule)
                    .child(
                        div()
                            .w(px(32.))
                            .flex_none()
                            .flex()
                            .justify_center()
                            .child(remove),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(8. * 16.))
                            .px(SP_1)
                            .child(super::widgets::read(format!("target-name-{id}"), name)),
                    )
                    .children(cells)
                    .into_any_element()
            })
            .collect();
        div()
            .id("targets")
            .mb(SP_4)
            .overflow_x_scroll()
            .child(div().min_w(px(640.)).child(head).children(rows))
    }

    /// What the targets take against what is held: the other lines as a table, the kinds of four
    /// tiers on one line each, then each slot's blueprints.
    fn needs(&self, theme: &Theme) -> impl IntoElement + use<> {
        let plan = view::group_plan(&self.shown.plan);
        let rule = theme.border;
        let fit = |child: AnyElement| {
            div()
                .flex_none()
                .min_w(px(64.))
                .pl(SP_3)
                .flex()
                .justify_end()
                .font_features(tabular())
                .whitespace_nowrap()
                .child(child)
        };
        let tone = |enough: bool| {
            if enough {
                (theme.ok, FontWeight::NORMAL)
            } else {
                (theme.danger, FontWeight::SEMIBOLD)
            }
        };
        let head = div()
            .flex()
            .py(SP_2)
            .border_b_1()
            .border_color(rule)
            .text_size(FS_SM)
            .text_color(theme.muted)
            .child(div().flex_1().child(text("要るもの")))
            .child(fit(text("必要").into_any_element()))
            .child(fit(text("所持").into_any_element()))
            .child(fit(text("過不足").into_any_element()));
        let rest = plan.rest.iter().map(|line| {
            let cells = view::need_cells(line);
            let (color, weight) = tone(cells.enough);
            let at = format!("need-{:?}-{}", line.kind, line.id);
            let cell = |part: &str, words: &str| read(format!("{at}-{part}"), words.to_owned());
            div()
                .flex()
                .py(SP_2)
                .border_b_1()
                .border_color(rule)
                .child(
                    div()
                        .flex_1()
                        .min_w(px(8. * 16.))
                        .child(cell("name", &cells.name)),
                )
                .child(fit(cell("need", &cells.need).into_any_element()))
                .child(fit(cell("have", &cells.have).into_any_element()))
                .child(fit(div()
                    .text_color(color)
                    .font_weight(weight)
                    .child(cell("short", &cells.short))
                    .into_any_element()))
        });
        let families = (!plan.families.is_empty()).then(|| {
            div()
                .mt(SP_4)
                .flex()
                .flex_col()
                .gap(px(2.))
                .children(plan.families.iter().map(|family| {
                    div()
                        .flex()
                        .children(family.tiers.iter().enumerate().map(|(i, line)| {
                            let cell = div().flex_1().py(SP_1).px(px(10.)).bg(theme.tiers[i]);
                            match line {
                                Some(line) => {
                                    let tier = view::tier_cell(line.need, line.have);
                                    let (color, weight) = tone(tier.enough);
                                    cell.id(SharedString::from(format!("family-{}-{i}", family.id)))
                                        .tooltip(tooltip(line.name.clone()))
                                        .flex()
                                        .items_center()
                                        .gap(SP_2)
                                        .child(icons::icon(&line.icon, px(40.)))
                                        .child(
                                            div()
                                                .text_color(color)
                                                .font_weight(weight)
                                                .font_features(tabular())
                                                .child(text(tier.text)),
                                        )
                                        .into_any_element()
                                }
                                None => cell.into_any_element(),
                            }
                        }))
                }))
        });
        let prints = (!self.shown.blueprints.is_empty()).then(|| {
            div()
                .mt(SP_4)
                .flex()
                .flex_col()
                .children(self.shown.blueprints.iter().map(|row| {
                    let cells = view::blueprint_cells(row);
                    let colored = |tone: Option<view::Tone>| match tone {
                        Some(view::Tone::Enough) => (Some(theme.ok), FontWeight::NORMAL),
                        Some(view::Tone::Short) => (Some(theme.danger), FontWeight::SEMIBOLD),
                        _ => (None, FontWeight::NORMAL),
                    };
                    let (uc, uw) = colored(Some(cells.universal.tone));
                    div()
                        .flex()
                        .border_b_1()
                        .border_color(rule)
                        .py(SP_1)
                        .child(
                            div()
                                .id(SharedString::from(format!("slot-{}", cells.name)))
                                .w(px(56.))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .tooltip(tooltip(cells.name.clone()))
                                .child(icons::icon(&cells.icon, px(48.))),
                        )
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .child(div().flex().children(cells.tiers.iter().map(|tier| {
                                    div()
                                        .flex_1()
                                        .flex()
                                        .justify_center()
                                        .text_size(FS_CAPTION)
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(theme.muted)
                                        .child(text(tier.label.clone()))
                                })))
                                .child(div().flex().children(cells.tiers.iter().map(|tier| {
                                    let (color, weight) = colored(tier.tone);
                                    div()
                                        .id(SharedString::from(format!("print-need-{}", tier.key)))
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .items_center()
                                        .text_size(FS_SM)
                                        .font_features(tabular())
                                        .whitespace_nowrap()
                                        .tooltip(tooltip(tier.name.clone()))
                                        .when(!tier.text.is_empty(), |c| {
                                            c.child(icons::icon(&tier.icon, px(32.))).child(
                                                div()
                                                    .when_some(color, gpui::Styled::text_color)
                                                    .font_weight(weight)
                                                    .child(text(tier.text.clone())),
                                            )
                                        })
                                })))
                                .child(
                                    div()
                                        .flex()
                                        .justify_end()
                                        .items_center()
                                        .gap(SP_1)
                                        .text_size(FS_SM)
                                        .text_color(theme.muted)
                                        .font_features(tabular())
                                        .child(text("T2 - T9:"))
                                        .child(
                                            div()
                                                .id(SharedString::from(format!(
                                                    "universal-{}",
                                                    cells.name
                                                )))
                                                .tooltip(tooltip(cells.universal.name.clone()))
                                                .child(icons::icon(&cells.universal.icon, px(32.))),
                                        )
                                        .child(
                                            div()
                                                .when_some(uc, gpui::Styled::text_color)
                                                .font_weight(uw)
                                                .child(text(cells.universal.text.clone())),
                                        ),
                                ),
                        )
                }))
        });
        div()
            .flex()
            .flex_col()
            .child(head)
            .children(rest)
            .children(families)
            .children(prints)
    }

    /// Pays one more from the `i`th source of the `at`th line (all with Shift), or takes back.
    fn pay(&mut self, at: usize, i: usize, shift: bool, right: bool, cx: &mut Context<Self>) {
        let settled = view::settle(&self.shown.procure, &self.payments);
        let Some(line) = settled.lines.get(at) else {
            return;
        };
        let Some(chip) = line.chips.get(i) else {
            return;
        };
        let key = format!("{}|{}", line.key, chip.key);
        let n = view::step(
            &chip.source,
            chip.n,
            line.left,
            view::Click { right, shift },
        );
        self.payments.insert(key, n);
        cx.notify();
    }

    /// A chip pressed: pays one (takes one back with the right button), and held down, again and
    /// again.
    fn press(&mut self, at: usize, i: usize, shift: bool, right: bool, cx: &mut Context<Self>) {
        self.pay(at, i, shift, right, cx);
        self.repeating = None;
        if shift {
            return;
        }
        self.repeating = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(REPEAT_AFTER).await;
            loop {
                if this
                    .update(cx, |this, cx| this.pay(at, i, false, right, cx))
                    .is_err()
                {
                    break;
                }
                cx.background_executor().timer(REPEAT_EVERY).await;
            }
        }));
    }

    fn release(&mut self) {
        self.repeating = None;
    }

    /// Each line short with what could make it up as chips, リセット, then 精算: what was used,
    /// held, spent and left.
    fn procure(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let settled = view::settle(&self.shown.procure, &self.payments);
        let procure = view::procure_cells(&settled);
        if procure.lines.is_empty() {
            return vec![
                div()
                    .text_size(FS_SM)
                    .text_color(theme.muted)
                    .child(text("足りないものはありません"))
                    .into_any_element(),
            ];
        }
        let rule = theme.border;
        let fit = |child: AnyElement| {
            div()
                .flex_none()
                .min_w(px(64.))
                .pl(SP_3)
                .flex()
                .justify_end()
                .font_features(tabular())
                .whitespace_nowrap()
                .child(child)
        };
        let head = div()
            .flex()
            .py(SP_2)
            .border_b_1()
            .border_color(rule)
            .text_size(FS_SM)
            .text_color(theme.muted)
            .child(div().flex_1().child(text("足りないもの")))
            .child(fit(text("不足").into_any_element()))
            .child(fit(text("残り").into_any_element()));
        let mut parts: Vec<AnyElement> = vec![head.into_any_element()];
        for (at, line) in procure.lines.iter().enumerate() {
            let (color, weight) = if line.done {
                (theme.ok, FontWeight::NORMAL)
            } else {
                (theme.danger, FontWeight::SEMIBOLD)
            };
            let chips: Vec<AnyElement> = if line.chips.is_empty() {
                vec![
                    div()
                        .text_size(FS_CAPTION)
                        .text_color(theme.muted)
                        .child(text("手持ちのボックスや今月のショップでは補えません"))
                        .into_any_element(),
                ]
            } else {
                line.chips
                    .iter()
                    .enumerate()
                    .map(|(i, chip)| {
                        let paid = !chip.given.is_empty();
                        let border = if paid { theme.accent } else { theme.border };
                        let muted = theme.muted;
                        div()
                            .id(SharedString::from(format!(
                                "chip-{}-{}",
                                line.key, chip.key
                            )))
                            .debug_selector({
                                let selector = format!("chip-{}-{}", line.key, chip.key);
                                move || selector.clone()
                            })
                            .role(Role::Button)
                            .aria_label(format!("{} {}", chip.title, chip.text))
                            .tab_stop(true)
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .mr(SP_2)
                            .mb(SP_1)
                            .px(SP_2)
                            .py(px(2.))
                            .border_1()
                            .border_color(if chip.out { theme.border } else { border })
                            .rounded_full()
                            .text_size(FS_SM)
                            .font_features(tabular())
                            .when(chip.out, |c| c.text_color(theme.muted))
                            .cursor_pointer()
                            .focus(move |s| if paid { s } else { s.border_color(muted) })
                            .tooltip(tooltip(chip.title.clone()))
                            .child(
                                icons::icon(&chip.icon, px(22.))
                                    .when(chip.out, |i| i.grayscale(true).opacity(0.6)),
                            )
                            .child(
                                div()
                                    .when(chip.negative, |t| {
                                        t.text_color(theme.danger).font_weight(FontWeight::SEMIBOLD)
                                    })
                                    .child(text(chip.text.clone())),
                            )
                            .when(paid, |c| {
                                c.child(
                                    div()
                                        .text_color(theme.accent)
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(text(chip.given.clone())),
                                )
                            })
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                                    this.press(at, i, event.modifiers.shift, false, cx);
                                }),
                            )
                            .on_mouse_down(
                                MouseButton::Right,
                                cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                                    this.press(at, i, event.modifiers.shift, true, cx);
                                }),
                            )
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _, _, _| this.release()),
                            )
                            .on_mouse_up(
                                MouseButton::Right,
                                cx.listener(|this, _, _, _| this.release()),
                            )
                            .on_mouse_up_out(
                                MouseButton::Left,
                                cx.listener(|this, _, _, _| this.release()),
                            )
                            .on_hover(cx.listener(|this, hovered: &bool, _, _| {
                                if !hovered {
                                    this.release();
                                }
                            }))
                            .on_key_down(cx.listener(
                                move |this, event: &gpui::KeyDownEvent, _, cx| {
                                    let key = event.keystroke.key.as_str();
                                    let shift = event.keystroke.modifiers.shift;
                                    if key == "enter" || key == "space" {
                                        this.pay(at, i, shift, false, cx);
                                    } else if key == "delete" || key == "backspace" {
                                        this.pay(at, i, shift, true, cx);
                                    }
                                },
                            ))
                            .into_any_element()
                    })
                    .collect()
            };
            parts.push(
                div()
                    .border_b_1()
                    .border_color(rule)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .py(SP_2)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(8. * 16.))
                                    .flex()
                                    .items_center()
                                    .gap(SP_2)
                                    .when(!line.icon.is_empty(), |w| {
                                        w.child(icons::icon(&line.icon, px(28.)))
                                    })
                                    .child(read(
                                        format!("procure-{}-name", line.key),
                                        line.name.clone(),
                                    )),
                            )
                            .child(fit(read(
                                format!("procure-{}-short", line.key),
                                line.short.clone(),
                            )
                            .into_any_element()))
                            .child(fit(div()
                                .text_color(color)
                                .font_weight(weight)
                                .child(read(
                                    format!("procure-{}-left", line.key),
                                    line.left.clone(),
                                ))
                                .into_any_element())),
                    )
                    .child(div().flex().flex_wrap().px(SP_2).pb(SP_2).children(chips))
                    .into_any_element(),
            );
        }
        parts.push(
            div()
                .my(SP_3)
                .child(
                    small_button(theme, "reset", "リセット").on_click(cx.listener(
                        |this, _, _, cx| {
                            this.payments.clear();
                            cx.notify();
                        },
                    )),
                )
                .into_any_element(),
        );
        // Nothing paid, no settlement.
        if !procure.resources.is_empty() {
            parts.push(
                div()
                    .text_size(FS_MD)
                    .font_weight(FontWeight::SEMIBOLD)
                    .mb(SP_2)
                    .child(text("精算"))
                    .into_any_element(),
            );
            let head = div()
                .flex()
                .py(SP_1)
                .px(SP_2)
                .border_b_1()
                .border_color(rule)
                .text_size(FS_SM)
                .text_color(theme.muted)
                .child(div().flex_1().child(text("使ったもの")))
                .child(fit(text("所持").into_any_element()))
                .child(fit(text("消費").into_any_element()))
                .child(fit(text("残り").into_any_element()));
            parts.push(head.into_any_element());
            for resource in &procure.resources {
                let after = match resource.tone {
                    Some(view::Tone::Plus) => Some((theme.ok, FontWeight::NORMAL)),
                    Some(view::Tone::Minus) => Some((theme.danger, FontWeight::SEMIBOLD)),
                    _ => None,
                };
                parts.push(
                    div()
                        .flex()
                        .items_center()
                        .py(SP_1)
                        .px(SP_2)
                        .border_b_1()
                        .border_color(rule)
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .items_center()
                                .gap(SP_2)
                                .child(icons::icon(&resource.icon, px(22.)))
                                .child(text(resource.name.clone())),
                        )
                        .child(fit(text(resource.held.clone()).into_any_element()))
                        .child(fit(div()
                            .text_color(theme.danger)
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(text(resource.spent.clone()))
                            .into_any_element()))
                        .child(fit(div()
                            .when_some(after, |d, (c, w)| d.text_color(c).font_weight(w))
                            .child(text(resource.after.clone()))
                            .into_any_element()))
                        .into_any_element(),
                );
            }
        }
        parts
    }

    /// The star menu of a target, over the page by its button: every star drawn as the game's,
    /// none below the student's now, the target ticked.
    pub(super) fn star_menu_view(
        &mut self,
        menu: StarMenu,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let row = self.shown.targets.iter().find(|t| t.id == menu.id)?.clone();
        let training = row.training.or_else(|| {
            self.shown
                .students
                .iter()
                .find(|s| s.id == row.id)
                .map(|s| s.training)
        });
        let now = training.as_ref().and_then(view::star_of);
        let to = row.target.star.or(now);
        let name = if row.name.is_empty() {
            self.shown
                .students
                .iter()
                .find(|s| s.id == row.id)
                .map(|s| s.name.clone())
                .unwrap_or_default()
        } else {
            row.name.clone()
        };
        let id = row.id;
        let hover = theme.wash_base;
        Some(
            deferred(
                anchored()
                    .position(point(menu.at.left(), menu.at.bottom() + px(4.)))
                    .snap_to_window_with_margin(px(4.))
                    .child(
                        div()
                            .id("star-menu")
                            .role(Role::Menu)
                            .aria_label(format!("{name}の星の目標"))
                            .occlude()
                            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                                this.star_menu = None;
                                cx.notify();
                            }))
                            .flex()
                            .flex_col()
                            .py(SP_1)
                            .border_1()
                            .border_color(theme.border)
                            .rounded(RADIUS_SM)
                            .bg(theme.surface)
                            .children(view::star_choices(now).into_iter().map(|choice| {
                                let checked = to == Some(choice.star);
                                let star = choice.star;
                                let disabled = choice.disabled;
                                let target = row.target;
                                div()
                                    .id(SharedString::from(format!("star-{star}")))
                                    .debug_selector(move || format!("star-{star}"))
                                    .role(Role::MenuItemRadio)
                                    .aria_label(view::star_name(star))
                                    .aria_toggled(if checked {
                                        gpui::Toggled::True
                                    } else {
                                        gpui::Toggled::False
                                    })
                                    .when(disabled, |i| {
                                        i.aria_description("押せません").opacity(0.35)
                                    })
                                    .tab_stop(!disabled)
                                    .flex()
                                    .items_center()
                                    .h(px(30.))
                                    .pl(SP_2)
                                    .pr(SP_3)
                                    .text_size(FS_SM)
                                    .when(!disabled, |i| {
                                        i.cursor_pointer()
                                            .hover(move |s| s.bg(hover))
                                            .focus(move |s| s.bg(hover))
                                    })
                                    .child(
                                        div()
                                            .w(px(14. * 1.4))
                                            .flex_none()
                                            .when(!checked, gpui::Styled::invisible)
                                            .child("✓"),
                                    )
                                    .child(stars(&format!("menu-{star}"), star, px(14.)))
                                    .when(!disabled, |i| {
                                        i.on_click(cx.listener(move |this, _, _, cx| {
                                            let mut target = target;
                                            target.star = Some(star);
                                            let _ = this.core.set_target(id, Some(target));
                                            this.star_menu = None;
                                            cx.notify();
                                        }))
                                    })
                            })),
                    ),
            )
            .with_priority(2)
            .into_any_element(),
        )
    }
}

/// What falls short that the daily places (指名手配, 学園交流会) drop: a line each, with the places
/// that drop it, the one making up the most first (the user's of 2026-10-13 「毎日プレイ出来る「指名手配」の項目で…参加すべき
/// ステージの候補を表示する」).
fn daily(theme: &Theme, candidates: &[view::BountyCandidate]) -> impl IntoElement + use<> {
    let rule = theme.border;
    let head = div()
        .flex()
        .py(SP_2)
        .border_b_1()
        .border_color(rule)
        .text_size(FS_SM)
        .text_color(theme.muted)
        .child(div().flex_1().child(text("足りないもの")))
        .child(div().w(px(64.)).flex().justify_end().child(text("不足")))
        .child(div().flex_1().pl(SP_3).child(text("指名手配・学園交流会")));
    let rows = candidates.iter().map(|c| {
        let at = format!("daily-{}", c.id);
        div()
            .flex()
            .items_center()
            .py(SP_2)
            .border_b_1()
            .border_color(rule)
            .child(
                div()
                    .flex_1()
                    .min_w(px(8. * 16.))
                    .flex()
                    .items_center()
                    .gap(SP_2)
                    .child(icons::icon(&c.icon, px(28.)))
                    .child(read(format!("{at}-name"), c.name.clone())),
            )
            .child(
                div()
                    .w(px(64.))
                    .flex()
                    .justify_end()
                    .font_features(tabular())
                    .text_color(theme.danger)
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(read(format!("{at}-short"), c.short.clone())),
            )
            .child(
                div()
                    .flex_1()
                    .pl(SP_3)
                    .text_size(FS_SM)
                    .child(read(format!("{at}-places"), c.places.join("・"))),
            )
    });
    div().flex().flex_col().child(head).children(rows)
}

/// A target's column's width: the star's wider for its stars.
fn column_width(label: &str) -> Pixels {
    match label {
        "星" => px(96.),
        "Lv" => px(56.),
        _ => px(52.),
    }
}

fn section_title(title: &str) -> gpui::Div {
    div()
        .text_size(FS_MD)
        .font_weight(FontWeight::SEMIBOLD)
        .child(text(title.to_owned()))
}

/// 再読込 and リセット: the border's frame, body-sm.
fn small_button(theme: &Theme, id: &str, label: &str) -> gpui::Stateful<gpui::Div> {
    let hover = theme.muted;
    outlined_button(theme, id, label, false)
        .text_size(FS_SM)
        .hover(move |s| s.border_color(hover))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_typed_value_is_kept_within_the_column() {
        let target = Target::default();
        assert_eq!(
            retarget(&target, Part::Level, "120", 90).level,
            90,
            "at most the column's highest"
        );
        assert_eq!(
            retarget(&target, Part::Skill(0), "0", 5).skills[0],
            1,
            "at least 1"
        );
        assert_eq!(
            retarget(&target, Part::Skill(1), "7.6", 10).skills[1],
            8,
            "rounded"
        );
        assert_eq!(
            retarget(&target, Part::Equipment(2), "abc", 10).equipment[2],
            1,
            "unreadable is 1"
        );
    }
}
