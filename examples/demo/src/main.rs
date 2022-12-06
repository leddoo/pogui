use core::cell::Cell;
use std::rc::Rc;

use pogui::gui::*;
use pogui::native_gui::NativeGui;



fn mk_node<C: IntoIterator<Item=Node>>(kind: NodeKind, children: C, style: &[(&str, &str)], gui: &mut Gui) -> Node {
    let node = gui.create_node(kind);
    gui.set_children(node, children.into_iter());
    gui.set_style(node, style.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect());
    node
}

fn div<C: IntoIterator<Item=Node>>(children: C, style: &[(&str, &str)], gui: &mut Gui) -> Node {
    mk_node(NodeKind::Div, children, style, gui)
}

fn span<C: IntoIterator<Item=Node>>(children: C, style: &[(&str, &str)], gui: &mut Gui) -> Node {
    mk_node(NodeKind::Span, children, style, gui)
}

fn text(value: &str, gui: &mut Gui) -> Node {
    gui.create_text(value)
}

fn button<C: IntoIterator<Item=Node>, H: Fn(&mut Gui, &mut Event) + 'static>(children: C, style: &[(&str, &str)], on_click: H, gui: &mut Gui) -> Node {
    let node = mk_node(NodeKind::Button, children, style, gui);
    gui.set_on_click(node, on_click);
    node
}


pub fn main() {
    let mut ngui = NativeGui::new();

    ngui.with_gui(|g| {
        let the_list = div([], &[], g);
        let add_button = button([text("+", g)], &[], move |gui, _e| {
            let item = div([text("something ", gui)], &[], gui);
            let button = button([text("x", gui)], &[], move |gui, _e| {
                gui.destroy_node(item);
            }, gui);
            gui.append_child(item, button);
            gui.append_child(the_list, item);
        }, g);

        let active = Rc::new(Cell::new(None));

        fn mk_button_handler(active: &Rc<Cell<Option<Node>>>, hidden: &Rc<Cell<Node>>) -> impl EventHandler {
            let active = active.clone();
            let hidden = hidden.clone();
            move |gui: &mut Gui, e: &mut Event| {
                let this = e.target;
                if let Some(mut other) = active.get() {
                    if other == this {
                        other = hidden.get();
                        hidden.set(this);
                    }

                    gui.swap_nodes(this, other);
                    gui.set_children(other, []);
                    active.set(None);
                }
                else {
                    let x = text("x", gui);
                    gui.set_children(this, [x]);
                    active.set(Some(this));
                }
            }
        }
        fn mk_button(g: &mut Gui, color: &str, handler: impl EventHandler) -> Node {
            button([], &[("background_color", color), ("width", "30"), ("height", "30"), ("display", "block")], handler, g)
        }
        let bp = mk_button(g, "ff00ff", |_: &mut Gui, _: &mut Event| {});
        let hidden = Rc::new(Cell::new(bp));
        g.set_on_click(bp, mk_button_handler(&active, &hidden));
        let br = mk_button(g, "ff0000", mk_button_handler(&active, &hidden));
        let bg = mk_button(g, "00ff00", mk_button_handler(&active, &hidden));
        let bb = mk_button(g, "0000ff", mk_button_handler(&active, &hidden));
        let bw = mk_button(g, "ffffff", mk_button_handler(&active, &hidden));


        let state = Rc::new(Cell::new(1));

        let the_span = span([text(&state.get().to_string(), g)], &[], g);

        let nodes =
            [
                text("hello, ", g),
                text("weirdo!", g),
                br,
                div([
                    text("new line cause div", g),
                    div([
                        text("div in div with inherited text color.", g),
                        div([
                            text("ADivInADivInADiv", g),
                        ], &[], g),
                    ], &[
                        ("min_width", "190"),
                        ("max_width", "400"),
                        ("min_height", "70"),
                        ("max_height", "100"),
                        ("background_color", "d040a0"),
                    ], g),
                    div([], &[
                        ("width",  "50"),
                        ("height", "50"),
                        ("background_color", "807060"),
                    ], g),
                    div([
                        text("nested div with a ", g),
                        span([text("different", g)], &[
                            ("text_color", "40b040"),
                        ], g),
                        text(" text color.", g),
                    ], &[
                        ("text_color", "306080"),
                    ], g),
                    text("more of the outer div", g),
                ], &[
                    ("font_size", "69"),
                    ("text_color", "802020"),
                    ("background_color", "eeeeff"),
                    ("min_height", "250"),
                ], g),
                div([
                    text("count: ", g),
                    the_span,
                    text(" ", g),
                    button([text("increment", g)], &[
                        ("background_color", "ffffdd"),
                    ], { let state = state.clone(); move |gui, _e| {
                        state.set(state.get() + 1);
                        let new_text = text(&state.get().to_string(), gui);
                        gui.set_children(the_span, [new_text]);
                    }}, g),
                    text(" ", g),
                    div([
                        div([text("hi", g)], &[], g),
                        div([text("there", g)], &[], g),
                    ], &[
                        ("display", "inline"),
                        ("background_color", "ddaadd"),
                    ], g),
                ], &[
                    ("background_color", "ddddff"),
                ], g),
                the_list,
                add_button,
                bb, bg, bw,
                div([text("this bish has sum phat content. she wayy too thicc to fit. an dats whai da lines be scrollin. anyway, here's some more text: The high-order word indicates the distance the wheel is rotated, expressed in multiples or divisions of WHEEL_DELTA, which is 120. A positive value indicates that the wheel was rotated forward, away from the user; a negative value indicates that the wheel was rotated backward, toward the user.", g)],
                    &[("width", "300"), ("height", "100"), ("background_color", "20c0d0"), ("overflow_y", "auto")], g),
            ];
        let root = g.root();
        g.set_children(root, nodes);
    });

    ngui.run_message_loop();
}

