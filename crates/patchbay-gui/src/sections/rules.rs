use std::rc::Rc;

use gpui_kit::component::{
    ActiveTheme, Icon, IconName, Sizable, WindowExt as _,
    button::{Button, ButtonVariant, ButtonVariants},
    h_flex,
    input::{Input, InputState},
    menu::{DropdownMenu as _, PopupMenuItem},
    radio::Radio,
    switch::Switch,
    tag::Tag,
    v_flex,
};
use gpui_kit::prelude::*;
use gpui_kit::*;

use patchbay_core::domains::{self, DomainRule, RuleKind};

use crate::app::AppShell;
use crate::state::AppState;
use crate::widgets::{empty_state, scroll_list, separated_rows};

impl AppShell {
    pub(crate) fn rules_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement + 'static {
        let rules = self.state.read(cx).config.domains.clone();
        let muted = cx.theme().muted_foreground;

        let add_rule = {
            let state = self.state.clone();
            move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                open_rule_dialog(state.clone(), None, window, cx)
            }
        };

        let actions = h_flex().gap_2().child(
            Button::new("add-rule")
                .primary()
                .small()
                .icon(IconName::Plus)
                .label("Add rule")
                .on_click(add_rule.clone()),
        );

        let body = if rules.is_empty() {
            empty_state(
                IconName::Globe,
                "No rules yet",
                "Add a rule to route traffic through the tunnel.",
                Some(
                    Button::new("add-rule-empty")
                        .outline()
                        .label("Add rule")
                        .on_click(add_rule)
                        .into_any_element(),
                ),
            )
        } else {
            let rows = separated_rows(
                cx,
                rules.into_iter().enumerate().map(|(ix, rule)| {
                    let rule = Rc::new(rule);
                    let enabled = rule.enabled;
                    let toggle = rule.clone();
                    let edit = rule.clone();
                    let delete = rule.clone();
                    h_flex()
                        .id(ElementId::named_usize("rule", ix))
                        .w_full()
                        .px_6()
                        .py_2()
                        .gap_3()
                        .items_center()
                        .child(
                            Switch::new(ElementId::named_usize("rule-enabled", ix))
                                .checked(enabled)
                                .on_change(cx.listener(move |this, _, _, cx| {
                                    let id = toggle.id.clone();
                                    this.state
                                        .update(cx, |state, cx| state.toggle_rule(&id, cx));
                                })),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_sm()
                                .when(!enabled, |this| this.text_color(muted))
                                .child(rule.value.clone()),
                        )
                        .child(kind_tag(rule.kind))
                        .child(
                            Button::new(ElementId::named_usize("rule-menu", ix))
                                .ghost()
                                .small()
                                .icon(IconName::EllipsisVertical)
                                .tooltip("More actions")
                                .dropdown_menu({
                                    let state = self.state.clone();
                                    move |menu, _, _| {
                                        menu.item(PopupMenuItem::new("Edit").on_click({
                                            let state = state.clone();
                                            let rule = edit.clone();
                                            move |_, window, cx| {
                                                open_rule_dialog(
                                                    state.clone(),
                                                    Some((*rule).clone()),
                                                    window,
                                                    cx,
                                                )
                                            }
                                        }))
                                        .separator()
                                        .item(
                                            PopupMenuItem::new("Delete")
                                                .icon(IconName::Delete)
                                                .on_click({
                                                    let state = state.clone();
                                                    let rule = delete.clone();
                                                    move |_, window, cx| {
                                                        confirm_delete(
                                                            state.clone(),
                                                            (*rule).clone(),
                                                            window,
                                                            cx,
                                                        )
                                                    }
                                                }),
                                        )
                                    }
                                }),
                        )
                        .into_any_element()
                }),
            );

            scroll_list("rules-list", &self.rules_scroll, rows)
        };

        self.page(
            "Rules",
            "Choose what goes through the tunnel",
            actions,
            body,
            cx,
        )
    }
}

fn kind_tag(kind: RuleKind) -> AnyElement {
    match kind {
        RuleKind::Domain => Tag::secondary().small().child("domain").into_any_element(),
        RuleKind::Regex => Tag::info().small().child("regex").into_any_element(),
    }
}

struct RuleDraft {
    input: Entity<InputState>,
    kind: RuleKind,
    error: Option<SharedString>,
    editing: Option<String>,
}

impl RuleDraft {
    fn new(editing: Option<&DomainRule>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (value, kind, id) = match editing {
            Some(rule) => (rule.value.clone(), rule.kind, Some(rule.id.clone())),
            None => (String::new(), RuleKind::Domain, None),
        };
        let placeholder = match kind {
            RuleKind::Domain => "example.com",
            RuleKind::Regex => r"^ads\.",
        };
        Self {
            input: cx.new(|cx| {
                InputState::new(window, cx)
                    .default_value(value)
                    .placeholder(placeholder)
            }),
            kind,
            error: None,
            editing: id,
        }
    }
}

fn rule_validation_error(input: &str, kind: RuleKind) -> Option<SharedString> {
    match kind {
        RuleKind::Domain => domains::normalize_domain(input)
            .is_none()
            .then(|| "Enter a valid domain like example.com".into()),
        RuleKind::Regex => domains::regex_error(input.trim()).map(SharedString::from),
    }
}

fn open_rule_dialog(
    state: Entity<AppState>,
    editing: Option<DomainRule>,
    window: &mut Window,
    cx: &mut App,
) {
    let is_editing = editing.is_some();
    let draft = cx.new(|cx| RuleDraft::new(editing.as_ref(), window, cx));
    let focused = std::cell::Cell::new(false);

    window.open_alert_dialog(cx, {
        let draft = draft.clone();
        let state = state.clone();
        move |alert, window, cx| {
            if !focused.replace(true) {
                let handle = draft.read(cx).input.focus_handle(cx);
                handle.focus(window, cx);
            }

            let (kind, error) = {
                let draft = draft.read(cx);
                (draft.kind, draft.error.clone())
            };
            let muted = cx.theme().muted_foreground;
            let danger = cx.theme().danger;

            alert
                .confirm()
                .title(if is_editing { "Edit rule" } else { "Add rule" })
                .ok_text(if is_editing { "Save" } else { "Add" })
                .child(
                    v_flex()
                        .w_full()
                        .gap_3()
                        .child(
                            div()
                                .text_xs()
                                .text_color(muted)
                                .child("Value routed through the tunnel"),
                        )
                        .child(Input::new(&draft.read(cx).input).cleanable(true))
                        .child(
                            h_flex().gap_4().children(
                                [
                                    (RuleKind::Domain, "Domain", "rule-kind-domain"),
                                    (RuleKind::Regex, "Regex", "rule-kind-regex"),
                                ]
                                .into_iter()
                                .map(|(rule_kind, label, id)| {
                                    Radio::new(id)
                                        .label(label)
                                        .checked(kind == rule_kind)
                                        .on_click({
                                            let draft = draft.clone();
                                            move |checked, _, cx| {
                                                if *checked {
                                                    draft.update(cx, |draft, cx| {
                                                        draft.kind = rule_kind;
                                                        draft.error = None;
                                                        cx.notify();
                                                    });
                                                }
                                            }
                                        })
                                }),
                            ),
                        )
                        .when_some(error, |this, error| {
                            this.child(div().text_xs().text_color(danger).child(error))
                        }),
                )
                .on_ok({
                    let draft = draft.clone();
                    let state = state.clone();
                    move |_, _, cx| {
                        let (text, kind, editing) = {
                            let draft = draft.read(cx);
                            (
                                draft.input.read(cx).value().to_string(),
                                draft.kind,
                                draft.editing.clone(),
                            )
                        };
                        if let Some(message) = rule_validation_error(&text, kind) {
                            draft.update(cx, |draft, cx| {
                                draft.error = Some(message);
                                cx.notify();
                            });
                            return false;
                        }
                        match editing {
                            Some(id) => state
                                .update(cx, |state, cx| state.update_rule(&id, &text, kind, cx)),
                            None => state.update(cx, |state, cx| state.add_rule(&text, kind, cx)),
                        }
                        true
                    }
                })
        }
    });
}

fn confirm_delete(state: Entity<AppState>, rule: DomainRule, window: &mut Window, cx: &mut App) {
    window.open_alert_dialog(cx, {
        let value = rule.value.clone();
        move |alert, _, cx| {
            alert
                .confirm()
                .icon(
                    Icon::new(IconName::TriangleAlert)
                        .large()
                        .text_color(cx.theme().danger),
                )
                .title("Delete rule?")
                .description(format!(
                    "“{value}” will no longer be routed through the tunnel."
                ))
                .ok_text("Delete")
                .ok_variant(ButtonVariant::Danger)
                .on_ok({
                    let state = state.clone();
                    let id = rule.id.clone();
                    move |_, _, cx| {
                        state.update(cx, |state, cx| state.remove_rule(&id, cx));
                        true
                    }
                })
        }
    });
}
