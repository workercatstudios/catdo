use std::{
    rc::Rc,
    sync::{Arc, LazyLock},
};

use chrono::Local;
use gpui_kit::component::{
    ActiveTheme, Collapsible, Icon, IconName, Selectable, Sizable, StyledExt,
    button::{Button, ButtonVariants},
    input::Input,
    select::Select,
    sidebar::{Sidebar, SidebarGroup, SidebarItem},
};
use gpui_kit::{prelude::*, *};

use crate::app::{CatDo, CreateKind, View};
use crate::theme::{Accent, accent, project_color_index};

static BRAND: LazyLock<Arc<Image>> = LazyLock::new(|| {
    Arc::new(Image::from_bytes(
        ImageFormat::Png,
        include_bytes!("../../../assets/com.workercat.catdo.png").to_vec(),
    ))
});

/// The CatDo artwork, shared by the sidebar brand and empty states.
pub fn brand() -> Arc<Image> {
    BRAND.clone()
}

type NavigationClick = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

// Kit Sidebar requires cloneable entries. Render each entry as a Kit Button so
// navigation keeps keyboard focus and native accessibility click actions.
#[derive(Clone)]
struct NavigationItem {
    label: SharedString,
    icon: IconName,
    tint: Option<Hsla>,
    active: bool,
    count: usize,
    on_click: NavigationClick,
}
impl Collapsible for NavigationItem {
    fn is_collapsed(&self) -> bool {
        false
    }
    fn collapsed(self, _: bool) -> Self {
        self
    }
}
impl SidebarItem for NavigationItem {
    fn render(self, id: impl Into<ElementId>, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = cx.theme().color_tokens();
        let active = self.active;
        div()
            .h_flex()
            .items_center()
            .w_full()
            .child(
                div()
                    .w(px(3.))
                    .h(px(18.))
                    .mr(px(6.))
                    .rounded_r_sm()
                    .bg(if active {
                        p.primary
                    } else {
                        transparent_black()
                    }),
            )
            .child(
                Button::new(id)
                    .ghost()
                    .selected(active)
                    .flex_1()
                    .h(px(36.))
                    .text_sm()
                    .when(active, |b| b.font_semibold())
                    .accessibility_label(self.label.clone())
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .w_full()
                            .gap_3()
                            .child(
                                Icon::new(self.icon)
                                    .size_4()
                                    .text_color(self.tint.unwrap_or(if active {
                                        p.primary
                                    } else {
                                        p.muted_foreground
                                    })),
                            )
                            .child(div().flex_1().text_left().truncate().child(self.label))
                            .when(self.count > 0, |el| {
                                el.child(
                                    div()
                                        .min_w(px(20.))
                                        .px_1p5()
                                        .py_0p5()
                                        .rounded_full()
                                        .text_xs()
                                        .text_center()
                                        .font_medium()
                                        .bg(if active { p.primary } else { p.muted })
                                        .text_color(if active {
                                            p.primary_foreground
                                        } else {
                                            p.muted_foreground
                                        })
                                        .child(self.count.to_string()),
                                )
                            }),
                    )
                    .on_click(move |event, window, cx| (self.on_click)(event, window, cx)),
            )
    }
}

#[derive(Clone)]
struct NavigationSection {
    label: Option<&'static str>,
    items: Vec<NavigationItem>,
}
impl Collapsible for NavigationSection {
    fn is_collapsed(&self) -> bool {
        false
    }
    fn collapsed(self, _: bool) -> Self {
        self
    }
}
impl SidebarItem for NavigationSection {
    fn render(
        self,
        id: impl Into<ElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let id = id.into();
        if let Some(label) = self.label {
            SidebarGroup::new(label)
                .children(self.items)
                .render(id, window, cx)
                .into_any_element()
        } else {
            div()
                .v_flex()
                .gap_1()
                .children(self.items.into_iter().enumerate().map(|(i, item)| {
                    item.render(SharedString::from(format!("{id}-{i}")), window, cx)
                        .into_any_element()
                }))
                .into_any_element()
        }
    }
}

impl CatDo {
    fn nav_item(
        &self,
        label: impl Into<SharedString>,
        icon: IconName,
        tint: Option<Hsla>,
        view: View,
        count: usize,
        cx: &Context<Self>,
    ) -> NavigationItem {
        NavigationItem {
            label: label.into(),
            icon,
            tint,
            active: self.view == view,
            count,
            on_click: Rc::new(
                cx.listener(move |this, _, window, cx| this.navigate(view, window, cx)),
            ),
        }
    }

    pub fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let today = Local::now().date_naive();
        let tasks = self
            .data
            .tasks
            .iter()
            .filter(|t| {
                t.workspace_id == self.workspace_id && t.active() && self.data.visible_task(t)
            })
            .collect::<Vec<_>>();
        let inbox = tasks
            .iter()
            .filter(|t| t.project_id.is_none() && t.parent_id.is_none())
            .count();
        let today_count = tasks
            .iter()
            .filter(|t| {
                t.scheduled.is_some_and(|d| d <= today) || t.due.is_some_and(|d| d <= today)
            })
            .count();
        let mut projects = self
            .data
            .projects
            .iter()
            .filter(|p| p.workspace_id == self.workspace_id && !p.archived)
            .map(|project| {
                self.nav_item(
                    project.name.clone(),
                    IconName::Folder,
                    Some(accent(Accent::Project(project_color_index(project.id)), cx)),
                    View::Project(project.id),
                    tasks
                        .iter()
                        .filter(|t| t.project_id == Some(project.id) && t.parent_id.is_none())
                        .count(),
                    cx,
                )
            })
            .collect::<Vec<_>>();
        projects.push(NavigationItem {
            label: "New project".into(),
            icon: IconName::Plus,
            tint: None,
            active: false,
            count: 0,
            on_click: Rc::new(
                cx.listener(|this, _, window, cx| {
                    this.start_named(CreateKind::Project, window, cx)
                }),
            ),
        });
        Sidebar::new("navigation")
            .collapsible(false)
            .w(px(240.))
            .header(
                div()
                    .v_flex()
                    .w_full()
                    .gap_4()
                    .pt_2()
                    .pb_2()
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap_3()
                            .px_2()
                            .pb_2()
                            .child(img(BRAND.clone()).size(px(26.)))
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("CatDo"),
                            ),
                    )
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap_1()
                            .child(Select::new(&self.workspace_select).flex_1().min_w_0())
                            .child(
                                Button::new("new-workspace")
                                    .ghost()
                                    .icon(IconName::Plus)
                                    .tooltip("New workspace")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.start_named(CreateKind::Workspace, window, cx)
                                    })),
                            ),
                    )
                    .child(
                        Button::new("sidebar-new-task")
                            .primary()
                            .w_full()
                            .h(px(38.))
                            .accessibility_label("New task")
                            .tooltip("Ctrl+N")
                            .child(
                                div()
                                    .h_flex()
                                    .items_center()
                                    .w_full()
                                    .gap_2()
                                    .child(Icon::new(IconName::Plus).size_4())
                                    .child(div().flex_1().text_left().child("New task"))
                                    .child(div().text_xs().opacity(0.7).child("Ctrl N")),
                            )
                            .on_click(cx.listener(|this, _, window, cx| this.new_task(window, cx))),
                    )
                    .child(
                        Input::new(&self.search)
                            .appearance(false)
                            .small()
                            .prefix(Icon::new(IconName::Search).size_4())
                            .cleanable(true),
                    ),
            )
            .child(NavigationSection {
                label: None,
                items: vec![
                    self.nav_item(
                        "Inbox",
                        IconName::Inbox,
                        Some(accent(Accent::Inbox, cx)),
                        View::Inbox,
                        inbox,
                        cx,
                    ),
                    self.nav_item(
                        "Today",
                        IconName::Sun,
                        Some(accent(Accent::Today, cx)),
                        View::Today,
                        today_count,
                        cx,
                    ),
                    self.nav_item(
                        "Upcoming",
                        IconName::Calendar,
                        Some(accent(Accent::Upcoming, cx)),
                        View::Upcoming,
                        0,
                        cx,
                    ),
                    self.nav_item(
                        "Calendar",
                        IconName::LayoutDashboard,
                        Some(accent(Accent::Calendar, cx)),
                        View::Calendar,
                        0,
                        cx,
                    ),
                ],
            })
            .child(NavigationSection {
                label: Some("Projects"),
                items: projects,
            })
            .footer(
                div()
                    .v_flex()
                    .w_full()
                    .gap_1()
                    .child(
                        Button::new("completed")
                            .accessibility_label("Completed")
                            .ghost()
                            .selected(self.view == View::Completed)
                            .w_full()
                            .child(
                                div()
                                    .h_flex()
                                    .items_center()
                                    .w_full()
                                    .gap_3()
                                    .child(
                                        Icon::new(IconName::CircleCheck)
                                            .size_4()
                                            .text_color(accent(Accent::Done, cx)),
                                    )
                                    .child("Completed"),
                            )
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.navigate(View::Completed, window, cx)
                            })),
                    )
                    .child(
                        Button::new("settings")
                            .accessibility_label("Settings")
                            .ghost()
                            .selected(self.view == View::Manage)
                            .w_full()
                            .icon(IconName::Settings)
                            .child(div().h_flex().w_full().child("Settings"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.navigate(View::Manage, window, cx)
                            })),
                    )
                    .when(!self.sync_status.is_empty(), |el| {
                        el.child(
                            Button::new("sync-status")
                                .ghost()
                                .w_full()
                                .accessibility_label(format!(
                                    "Sync status: {}. Open Settings",
                                    self.sync_status
                                ))
                                .tooltip(self.sync_status.clone())
                                .child(
                                    div()
                                        .w_full()
                                        .truncate()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(self.sync_status.clone()),
                                )
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.navigate(View::Manage, window, cx)
                                })),
                        )
                    }),
            )
    }
}
