//! ButtonGroup and DropdownButton: GPUI Component's, ported to Kirakira's [`Button`].
//!
//! GPUI Component keeps both in its `button` module, so they are re-exported from
//! [`crate::button`]. Same builders, same layout, same joins: the buttons of a group and the two
//! halves of a split button share their inner edges and square their inner corners. Each button
//! keeps its own press and pop.

use std::{cell::Cell, rc::Rc};

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::{Disableable, Selectable, Sizable, Size};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    Anchor, App, Axis, Context, Corners, Edges, ElementId, InteractiveElement as _, IntoElement,
    ParentElement as _, RenderOnce, StatefulInteractiveElement as _, StyleRefinement, Styled,
    Window, div,
};

use crate::button::{Button, ButtonVariant, ButtonVariants};
use crate::menu::{DropdownMenu as _, PopupMenu};

/// A ButtonGroup element, to wrap multiple buttons in a group.
#[derive(IntoElement)]
pub struct ButtonGroup {
    id: ElementId,
    style: StyleRefinement,
    children: Vec<Button>,
    multiple: bool,
    disabled: bool,
    layout: Axis,

    // The button props
    compact: bool,
    outline: bool,
    variant: Option<ButtonVariant>,
    size: Option<Size>,

    on_click: Option<Box<dyn Fn(&Vec<usize>, &mut Window, &mut App) + 'static>>,
}

impl Disableable for ButtonGroup {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl ButtonGroup {
    /// Creates a new ButtonGroup.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            children: Vec::new(),
            variant: None,
            size: None,
            compact: false,
            outline: false,
            multiple: false,
            disabled: false,
            layout: Axis::Horizontal,
            on_click: None,
        }
    }

    /// Adds a button as a child to the ButtonGroup.
    pub fn child(mut self, child: Button) -> Self {
        self.children.push(child.disabled(self.disabled));
        self
    }

    /// Adds multiple buttons as children to the ButtonGroup.
    pub fn children(mut self, children: impl IntoIterator<Item = Button>) -> Self {
        self.children.extend(children);
        self
    }

    /// With the multiple selection mode, default is false (single selection).
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// Set the layout of the button group. Default is `Axis::Horizontal`.
    pub fn layout(mut self, layout: Axis) -> Self {
        self.layout = layout;
        self
    }

    /// With the compact mode for the ButtonGroup.
    ///
    /// See also: [`Button::compact()`]
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// With the outline mode for the ButtonGroup.
    ///
    /// See also: [`Button::outline()`]
    pub fn outline(mut self) -> Self {
        self.outline = true;
        self
    }

    /// Sets the on_click handler for the ButtonGroup.
    ///
    /// The `&Vec<usize>` is the indices of the clicked (selected in `multiple` mode) buttons.
    /// For example: `[0, 2, 3]` means the first, third and fourth buttons are clicked.
    pub fn on_click(
        mut self,
        handler: impl Fn(&Vec<usize>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl Sizable for ButtonGroup {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = Some(size.into());
        self
    }
}

impl Styled for ButtonGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ButtonVariants for ButtonGroup {
    fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = Some(variant);
        self
    }
}

/// The corners and edges a button keeps at `index` of `len` in a group, as GPUI Component's
/// group joins them.
fn group_join(index: usize, len: usize, vertical: bool) -> Option<(Corners<bool>, Edges<bool>)> {
    if len == 1 {
        None
    } else if index == 0 {
        Some((
            Corners {
                top_left: true,
                top_right: vertical,
                bottom_left: !vertical,
                bottom_right: false,
            },
            Edges::all(true),
        ))
    } else if index == len - 1 {
        Some((
            Corners {
                top_left: false,
                top_right: !vertical,
                bottom_left: vertical,
                bottom_right: true,
            },
            Edges {
                left: vertical,
                top: !vertical,
                right: true,
                bottom: true,
            },
        ))
    } else {
        Some((
            Corners::default(),
            Edges {
                left: vertical,
                top: !vertical,
                right: true,
                bottom: true,
            },
        ))
    }
}

impl RenderOnce for ButtonGroup {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let children_len = self.children.len();
        let state = Rc::new(Cell::new(None));
        let selected_ixs: Vec<usize> = self
            .children
            .iter()
            .enumerate()
            .filter(|(_, child)| child.is_selected())
            .map(|(ix, _)| ix)
            .collect();

        let vertical = self.layout == Axis::Vertical;
        let multiple = self.multiple;
        let has_on_click = self.on_click.is_some();

        div()
            .id(self.id)
            .flex()
            .when(vertical, |this| this.flex_col().justify_center())
            .when(!vertical, |this| this.items_center())
            .refine_style(&self.style)
            .children(
                self.children
                    .into_iter()
                    .enumerate()
                    .map(|(child_index, child)| {
                        let state = Rc::clone(&state);
                        // The group as a whole is a toggle control, so every child advertises its
                        // pressed state.
                        let selected = child.is_selected();
                        let child = child.toggled(selected);
                        match group_join(child_index, children_len, vertical) {
                            Some((corners, edges)) => child.join(corners, edges),
                            None => child,
                        }
                        .when_some(self.size, |this, size| this.with_size(size))
                        .when_some(self.variant, |this, variant| this.with_variant(variant))
                        .when(self.compact, |this| this.compact())
                        .when(self.outline, |this| this.outline())
                        .when(has_on_click, |this| {
                            this.on_click(move |_, _, _| {
                                state.set(Some(child_index));
                            })
                        })
                    }),
            )
            .when_some(
                self.on_click.filter(|_| !self.disabled),
                move |this, on_click| {
                    this.on_click(move |_, window, cx| {
                        let mut selected_ixs = selected_ixs.clone();
                        if let Some(ix) = state.get() {
                            if multiple {
                                if let Some(pos) = selected_ixs.iter().position(|&i| i == ix) {
                                    selected_ixs.remove(pos);
                                } else {
                                    selected_ixs.push(ix);
                                }
                            } else {
                                selected_ixs.clear();
                                selected_ixs.push(ix);
                            }
                        }

                        on_click(&selected_ixs, window, cx);
                    })
                },
            )
    }
}

/// Group name shared by both halves, so hovering one can style the other.
const HALVES_GROUP: &str = "dropdown-button";

type MenuBuilder = Box<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu>;

/// A split button: an action button with an attached menu trigger.
///
/// The two halves stay visually joined. A `ghost` split is transparent at rest; hovering either
/// half surfaces the whole control with the hovered half emphasized, and it stays surfaced while
/// the menu is open, so the pair reads as one control rather than two buttons.
#[derive(IntoElement)]
pub struct DropdownButton {
    id: ElementId,
    style: StyleRefinement,
    button: Option<Button>,
    menu: Option<MenuBuilder>,
    selected: bool,
    disabled: bool,
    // The button props, applied to both halves. Unset means the inner [`Button`] keeps whatever
    // it was given.
    outline: bool,
    variant: Option<ButtonVariant>,
    size: Option<Size>,
    anchor: Anchor,
}

impl DropdownButton {
    /// Create a new DropdownButton.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            button: None,
            menu: None,
            selected: false,
            disabled: false,
            outline: false,
            variant: None,
            size: None,
            anchor: Anchor::TopRight,
        }
    }

    fn effective_variant(&self) -> ButtonVariant {
        self.variant
            .or_else(|| self.button.as_ref().map(Button::variant))
            .unwrap_or_default()
    }

    fn effective_size(&self) -> Size {
        self.size
            .or_else(|| self.button.as_ref().map(Button::button_size))
            .unwrap_or_default()
    }

    /// Set the left button of the dropdown button.
    ///
    /// The button keeps its own label, icon, tooltip and click handler. A variant or size set on
    /// the [`DropdownButton`] applies to both halves and overrides the one set here. When either
    /// outer value is unset, this button's value becomes the shared value for both halves.
    pub fn button(mut self, button: Button) -> Self {
        self.button = Some(button);
        self
    }

    /// Set the dropdown menu of the button.
    pub fn dropdown_menu(
        mut self,
        menu: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Self {
        self.menu = Some(Box::new(menu));
        self
    }

    /// Set the dropdown menu of the button with anchor corner.
    pub fn dropdown_menu_with_anchor(
        mut self,
        anchor: impl Into<Anchor>,
        menu: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Self {
        self.menu = Some(Box::new(menu));
        self.anchor = anchor.into();
        self
    }

    /// Set the button to outline style.
    ///
    /// See also: [`Button::outline`]
    pub fn outline(mut self) -> Self {
        self.outline = true;
        self
    }
}

impl Disableable for DropdownButton {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Styled for DropdownButton {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for DropdownButton {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = Some(size.into());
        self
    }
}

impl ButtonVariants for DropdownButton {
    fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = Some(variant);
        self
    }
}

impl Selectable for DropdownButton {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl RenderOnce for DropdownButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        debug_assert!(
            self.button.is_some() || self.menu.is_some(),
            "a DropdownButton needs a `button`, a `dropdown_menu`, or both"
        );

        let variant = self.effective_variant();
        let size = self.effective_size();
        let selected = self.selected || self.button.as_ref().is_some_and(Selectable::is_selected);
        // Only a ghost split has no surface at rest, so only it needs hovering one half to reveal
        // the other, and the action half to stay revealed while the menu holds the trigger
        // pressed.
        let is_ghost = variant.is_ghost();
        let menu_open = window.use_keyed_state(self.id.clone(), cx, |_, _| false);
        let is_menu_open = *menu_open.read(cx);

        let button = self.button.map(|button| {
            let disabled = self.disabled || button.is_disabled();
            button
                .join(
                    Corners {
                        top_left: true,
                        top_right: false,
                        bottom_left: true,
                        bottom_right: false,
                    },
                    Edges::all(true),
                )
                .selected(selected)
                .disabled(disabled)
                .when(self.outline, |this| this.outline())
                .with_size(size)
                .with_variant(variant)
                .when(is_ghost, |this| {
                    this.ghost_hover_group(HALVES_GROUP, is_menu_open, cx)
                })
        });

        let trigger = self.menu.map(|menu| {
            Button::new("popup")
                .dropdown_caret(true)
                .join(
                    Corners {
                        top_left: false,
                        top_right: true,
                        bottom_left: false,
                        bottom_right: true,
                    },
                    Edges {
                        left: false,
                        top: true,
                        right: true,
                        bottom: true,
                    },
                )
                .selected(selected)
                .disabled(self.disabled)
                .when(self.outline, |this| this.outline())
                .with_size(size)
                .with_variant(variant)
                .when(is_ghost, |this| {
                    this.ghost_hover_group(HALVES_GROUP, false, cx)
                })
                .dropdown_menu_with_anchor(self.anchor, menu)
                .on_open_change(move |open, _, cx| {
                    menu_open.update(cx, |state, cx| {
                        *state = *open;
                        cx.notify();
                    })
                })
        });

        div()
            .id(self.id)
            .when(is_ghost, |this| this.group(HALVES_GROUP))
            .h_flex()
            .refine_style(&self.style)
            .children(button)
            .children(trigger)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::component::IconName;

    #[test]
    fn builders_take_kirakira_buttons() {
        let group = ButtonGroup::new("group")
            .child(Button::new("a").label("A").selected(true))
            .children([Button::new("b").label("B"), Button::new("c").label("C")])
            .primary()
            .large()
            .outline()
            .compact()
            .multiple(true)
            .layout(Axis::Vertical)
            .on_click(|_, _, _| {});
        assert_eq!(group.children.len(), 3);
        assert_eq!(group.variant, Some(ButtonVariant::Primary));
        assert_eq!(group.size, Some(Size::Large));
        assert!(group.outline && group.compact && group.multiple);
        assert_eq!(group.layout, Axis::Vertical);

        let split = DropdownButton::new("split")
            .button(Button::new("inner").label("Action").icon(IconName::Plus))
            .primary()
            .outline()
            .large()
            .dropdown_menu_with_anchor(Anchor::BottomLeft, |menu, _, _| menu);
        assert!(split.button.is_some() && split.menu.is_some());
        assert_eq!(split.anchor, Anchor::BottomLeft);
    }

    #[test]
    fn inner_button_sets_the_split_variant_and_size() {
        let split = DropdownButton::new("split")
            .button(Button::new("inner").label("Action").ghost().small())
            .dropdown_menu(|menu, _, _| menu);
        assert_eq!(split.variant, None);
        assert_eq!(split.effective_variant(), ButtonVariant::Ghost);
        assert_eq!(split.effective_size(), Size::Small);
    }

    #[test]
    fn group_joins_match_gpui_component() {
        assert_eq!(group_join(0, 1, false), None);
        let (corners, edges) = group_join(0, 3, false).unwrap();
        assert!(corners.top_left && corners.bottom_left);
        assert!(!corners.top_right && !corners.bottom_right);
        assert_eq!(edges, Edges::all(true));
        let (corners, edges) = group_join(1, 3, false).unwrap();
        assert_eq!(corners, Corners::default());
        assert!(!edges.left && edges.top && edges.right && edges.bottom);
        let (corners, edges) = group_join(2, 3, false).unwrap();
        assert!(corners.top_right && corners.bottom_right);
        assert!(!corners.top_left && !corners.bottom_left);
        assert!(!edges.left);
        let (corners, edges) = group_join(2, 3, true).unwrap();
        assert!(corners.bottom_left && corners.bottom_right && !corners.top_left);
        assert!(edges.left && !edges.top);
    }
}
