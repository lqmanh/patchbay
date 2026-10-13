use std::rc::Rc;

use gpui_kit::component::{
    ActiveTheme, Icon, IconName, IndexPath, Sizable, StyledExt as _, TitleBar,
    VirtualListScrollHandle, h_flex,
    input::{InputEvent, InputState},
    select::{SelectEvent, SelectItem, SelectState},
    sidebar::{Sidebar, SidebarCollapsible, SidebarMenu, SidebarMenuItem, SidebarToggleButton},
    v_flex,
};
use gpui_kit::prelude::*;
use gpui_kit::*;

use patchbay_core::config::{self, DEFAULT_SOCKS_PORT};

use crate::frame::WindowFrame;
use crate::selects::{ModeItem, ServerItem, ThemeItem, mode_items, theme_items};
use crate::state::{AppState, ThemePref};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Section {
    #[default]
    Status,
    Servers,
    Rules,
    Log,
    Settings,
}

/// The window and shell state kept while the window is closed, to restore on reopen.
#[derive(Debug, Clone, Default)]
pub struct ShellLayout {
    pub section: Section,
    pub collapsed: bool,
    /// `None` opens the window centered.
    pub frame: Option<WindowFrame>,
}

pub struct AppShell {
    pub(crate) state: Entity<AppState>,
    pub(crate) section: Section,
    pub(crate) collapsed: bool,
    frame: Option<WindowFrame>,
    pub(crate) log_follow: bool,
    pub(crate) log_sizes: Rc<Vec<Size<Pixels>>>,
    pub(crate) log_scroll: VirtualListScrollHandle,
    pub(crate) servers_scroll: ScrollHandle,
    pub(crate) rules_scroll: ScrollHandle,
    pub(crate) port_input: Entity<InputState>,
    pub(crate) server_select: Entity<SelectState<Vec<ServerItem>>>,
    pub(crate) server_items: Vec<ServerItem>,
    pub(crate) mode_select: Entity<SelectState<Vec<ModeItem>>>,
    pub(crate) theme_select: Entity<SelectState<Vec<ThemeItem>>>,
    _port_input: Subscription,
    _server_select: Subscription,
    _mode_select: Subscription,
    _theme_select: Subscription,
    _state: Subscription,
    _appearance: Subscription,
    _bounds: Subscription,
    _display: Subscription,
}

impl AppShell {
    pub fn new(
        state: Entity<AppState>,
        layout: ShellLayout,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let port = state.read(cx).config.socks_port;
        let port_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(port.to_string())
                .placeholder(DEFAULT_SOCKS_PORT.to_string())
                .validate(|text, _| text.is_empty() || config::parse_port_text(text).is_some())
        });

        let port_input_subscription = cx.subscribe_in(
            &port_input,
            window,
            |this, input, event: &InputEvent, window, cx| {
                if !matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                    return;
                }
                match config::parse_port_text(&input.read(cx).value()) {
                    Some(port) => this
                        .state
                        .update(cx, |state, cx| state.set_socks_port(port, cx)),
                    None => {
                        let current = this.state.read(cx).config.socks_port;
                        input.update(cx, |input, cx| {
                            input.set_value(current.to_string(), window, cx)
                        });
                    }
                }
            },
        );

        let mode = state.read(cx).config.mode;
        let (mode_select, mode_subscription) = select(
            cx,
            window,
            mode_items(),
            Some(&mode),
            |this, mode, _, cx| {
                this.state.update(cx, |state, cx| state.set_mode(mode, cx));
            },
        );

        let theme_pref = state.read(cx).theme();
        let (theme_select, theme_subscription) = select(
            cx,
            window,
            theme_items(),
            Some(&theme_pref),
            |this, pref, window, cx| {
                this.state.update(cx, |state, cx| state.set_theme(pref, cx));
                pref.apply(Some(window), cx);
            },
        );

        let (server_select, server_subscription) = select(
            cx,
            window,
            Vec::<ServerItem>::new(),
            None,
            |this, file, _, cx| {
                this.state
                    .update(cx, |state, cx| state.select_server(&file, cx));
            },
        );

        let state_subscription = cx.observe(&state, |_, _, cx| cx.notify());

        let appearance = window.observe_window_appearance({
            let state = state.clone();
            move |window, cx| {
                if state.read(cx).theme() == ThemePref::System {
                    ThemePref::System.apply(Some(window), cx);
                }
            }
        });

        let bounds = cx.observe_window_bounds(window, Self::track_bounds);
        let display = cx.observe_window_display(window, Self::track_display);

        Self {
            state,
            section: layout.section,
            collapsed: layout.collapsed,
            frame: WindowFrame::of(window, cx).or(layout.frame),
            log_follow: true,
            log_sizes: Rc::new(Vec::new()),
            log_scroll: VirtualListScrollHandle::new(),
            servers_scroll: ScrollHandle::new(),
            rules_scroll: ScrollHandle::new(),
            port_input,
            server_select,
            server_items: Vec::new(),
            mode_select,
            theme_select,
            _port_input: port_input_subscription,
            _server_select: server_subscription,
            _mode_select: mode_subscription,
            _theme_select: theme_subscription,
            _state: state_subscription,
            _appearance: appearance,
            _bounds: bounds,
            _display: display,
        }
    }

    pub fn layout(&self) -> ShellLayout {
        ShellLayout {
            section: self.section,
            collapsed: self.collapsed,
            frame: self.frame.clone(),
        }
    }

    fn track_bounds(&mut self, window: &mut Window, _: &mut Context<Self>) {
        if let Some(frame) = &mut self.frame {
            frame.set_bounds(window.window_bounds());
        }
    }

    fn track_display(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(frame) = WindowFrame::of(window, cx) {
            self.frame = Some(frame);
        }
    }

    fn sidebar_item(
        &self,
        label: &'static str,
        icon: IconName,
        target: Section,
        cx: &mut Context<Self>,
    ) -> SidebarMenuItem {
        menu_item(label, icon)
            .active(self.section == target)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.section = target;
                cx.notify();
            }))
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement + 'static {
        let icon_collapsed = self.collapsed;
        let muted = cx.theme().muted_foreground;

        Sidebar::new("sidebar")
            .collapsible(SidebarCollapsible::Icon)
            .collapsed(self.collapsed)
            .w(px(220.))
            .header(
                h_flex()
                    .gap_2()
                    .items_center()
                    // The collapsed sidebar pads around its header 12px more
                    // (pt_2 + gap_2 + p_2 vs pt_3); a header 12px shorter
                    // keeps the menu items in place.
                    .when(icon_collapsed, |this| this.h_8().w_full().justify_center())
                    .when(!icon_collapsed, |this| this.h_11().px_1())
                    .child(
                        crate::widgets::logo()
                            .size_8()
                            .flex_shrink_0()
                            .when(icon_collapsed, |this| this.size_6()),
                    )
                    .when(!icon_collapsed, |this| {
                        this.child(
                            v_flex()
                                .flex_1()
                                .overflow_hidden()
                                .child(div().text_sm().font_semibold().child("Patchbay"))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(muted)
                                        .child("Rule-based VPN routing"),
                                ),
                        )
                    }),
            )
            .child(
                SidebarMenu::new()
                    .child(self.sidebar_item(
                        "Status",
                        IconName::LayoutDashboard,
                        Section::Status,
                        cx,
                    ))
                    .child(self.sidebar_item("Servers", IconName::Network, Section::Servers, cx))
                    .child(self.sidebar_item("Rules", IconName::Globe, Section::Rules, cx))
                    .child(self.sidebar_item("Log", IconName::SquareTerminal, Section::Log, cx))
                    .child(self.sidebar_item("Settings", IconName::Settings, Section::Settings, cx))
                    .child(
                        menu_item("About", IconName::Info)
                            .on_click(cx.listener(|_, _, _, cx| crate::show_about(cx))),
                    ),
            )
    }

    fn title_bar(&self, cx: &mut Context<Self>) -> impl IntoElement + 'static {
        let kind = self.state.read(cx).state_kind();
        let color = kind.color(cx.theme());
        let muted = cx.theme().muted_foreground;

        TitleBar::new().child(
            h_flex()
                .w_full()
                .items_center()
                .gap_2()
                .pr_3()
                .child(titlebar_control(
                    SidebarToggleButton::new()
                        .collapsed(self.collapsed)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.collapsed = !this.collapsed;
                            cx.notify();
                        })),
                ))
                .child(
                    h_flex()
                        .ml_auto()
                        .items_center()
                        .gap(px(6.))
                        .child(Icon::new(kind.icon()).small().text_color(color))
                        .child(div().text_xs().text_color(muted).child(kind.label())),
                ),
        )
    }

    pub(crate) fn page(
        &self,
        title: impl Into<SharedString>,
        description: impl Into<SharedString>,
        actions: impl IntoElement + 'static,
        body: impl IntoElement + 'static,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + 'static {
        v_flex()
            .size_full()
            .min_h_0()
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .px_6()
                    .py_4()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        v_flex()
                            .gap_0p5()
                            .child(div().text_base().font_semibold().child(title.into()))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(description.into()),
                            ),
                    )
                    .child(h_flex().flex_shrink_0().gap_2().child(actions)),
            )
            .child(div().flex_1().min_h_0().child(body))
    }
}

/// Wraps a title bar button so the title bar's double-click action
/// (zoom/minimize) and window drag don't fire for it: gpui dispatches a click
/// only to ancestors that saw the mouse-down.
fn titlebar_control(child: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_none()
        .items_center()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(child)
}

/// Expanded items are fixed at h_7 but collapsed ones size to their content;
/// pin both so items don't shift when the sidebar collapses.
fn menu_item(label: &'static str, icon: IconName) -> SidebarMenuItem {
    SidebarMenuItem::new(label).icon(icon).h_7()
}

impl Render for AppShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.section {
            Section::Status => self.status_section(window, cx).into_any_element(),
            Section::Servers => self.servers_section(cx).into_any_element(),
            Section::Rules => self.rules_section(cx).into_any_element(),
            Section::Log => self.log_section(cx).into_any_element(),
            Section::Settings => self.settings_section(cx).into_any_element(),
        };

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(self.title_bar(cx))
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .child(self.sidebar(cx))
                    .child(div().flex_1().min_w_0().h_full().child(content)),
            )
    }
}

fn select<T>(
    cx: &mut Context<AppShell>,
    window: &mut Window,
    items: Vec<T>,
    selected: Option<&T::Value>,
    on_confirm: impl Fn(&mut AppShell, T::Value, &mut Window, &mut Context<AppShell>) + 'static,
) -> (Entity<SelectState<Vec<T>>>, Subscription)
where
    T: SelectItem + 'static,
{
    let index = selected.and_then(|value| items.iter().position(|item| item.value() == value));
    let select = cx.new(|cx| SelectState::new(items, index.map(IndexPath::new), window, cx));
    let subscription = cx.subscribe_in(
        &select,
        window,
        move |this, _, event: &SelectEvent<Vec<T>>, window, cx| {
            if let SelectEvent::Confirm(Some(value)) = event {
                on_confirm(this, value.clone(), window, cx);
            }
        },
    );
    (select, subscription)
}
