use gpui_kit::component::{
    ActiveTheme, Icon, IconName, StyledExt as _,
    empty::{
        Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyMediaVariant,
        EmptyTitle,
    },
    h_flex,
    scroll::ScrollableElement as _,
    separator::Separator,
    v_flex,
};
use gpui_kit::prelude::*;
use gpui_kit::*;

pub fn field_row(
    cx: &App,
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    control: impl IntoElement + 'static,
) -> impl IntoElement + 'static {
    let muted = cx.theme().muted_foreground;
    h_flex()
        .w_full()
        .items_center()
        .gap_6()
        .px_4()
        .py_3()
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(div().text_sm().font_medium().child(title.into()))
                .child(div().text_xs().text_color(muted).child(description.into())),
        )
        .child(div().flex_shrink_0().child(control))
}

pub fn group_box(
    cx: &App,
    children: impl IntoIterator<Item = AnyElement>,
) -> impl IntoElement + 'static {
    v_flex()
        .w_full()
        .rounded(cx.theme().radius_lg)
        .border_1()
        .border_color(cx.theme().border)
        .overflow_hidden()
        .children(children)
}

pub fn group_title(cx: &App, title: impl Into<SharedString>) -> impl IntoElement + 'static {
    let muted = cx.theme().muted_foreground;
    div()
        .px_1()
        .pb_2()
        .text_xs()
        .font_medium()
        .text_color(muted)
        .child(title.into())
}

pub fn separated_rows(cx: &App, rows: impl IntoIterator<Item = AnyElement>) -> Vec<AnyElement> {
    let border = cx.theme().border;
    let mut out = Vec::new();
    for (ix, row) in rows.into_iter().enumerate() {
        if ix > 0 {
            out.push(Separator::horizontal().color(border).into_any_element());
        }
        out.push(row);
    }
    out
}

pub fn page_body(id: &'static str, children: impl IntoIterator<Item = AnyElement>) -> AnyElement {
    div()
        .id(id)
        .size_full()
        .overflow_y_scroll()
        .child(v_flex().gap_6().p_6().max_w(px(720.)).children(children))
        .into_any_element()
}

pub fn scroll_list(
    id: &'static str,
    handle: &ScrollHandle,
    rows: impl IntoIterator<Item = AnyElement>,
) -> AnyElement {
    v_flex()
        .id(id)
        .relative()
        .size_full()
        .overflow_y_scroll()
        .track_scroll(handle)
        .vertical_scrollbar(handle)
        .children(rows)
        .into_any_element()
}

pub fn empty_state(
    icon: IconName,
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    action: Option<AnyElement>,
) -> AnyElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .child(
            Empty::new()
                .header(
                    EmptyHeader::new()
                        .media(
                            EmptyMedia::new()
                                .with_variant(EmptyMediaVariant::Icon)
                                .child(Icon::new(icon)),
                        )
                        .title(EmptyTitle::new().child(title.into()))
                        .description(EmptyDescription::new().child(description.into())),
                )
                .when_some(action, |this, action| {
                    this.content(EmptyContent::new().child(action))
                }),
        )
        .into_any_element()
}
