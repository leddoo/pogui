use core::cell::RefCell;
use std::rc::Rc;

use pogui::gui::*;
use pogui::native_gui::NativeGui;


#[derive(Clone, Copy, PartialEq)]
enum Action {
    None,
    Add, Sub, Mul, Div,
}


struct Calc {
    state:   f64,
    action:  Action,
    input:   String,
    display: Node,
}

impl Calc {
    fn new(display: Node, g: &mut Gui) -> Calc {
        let this = Calc {
            state:  0.0,
            action: Action::None,
            input:  String::new(),
            display,
        };
        this.update_display(g);
        this
    }

    fn input(&mut self, c: char, g: &mut Gui) {
        self.input.push(c);
        self.update_display(g)
    }

    fn action(&mut self, a: Action, g: &mut Gui) {
        if self.input.len() > 0 {
            let input = self.input.parse::<f64>().unwrap();
            self.input.clear();

            self.state = match self.action {
                Action::None => input,
                Action::Add  => self.state + input,
                Action::Sub  => self.state - input,
                Action::Mul  => self.state * input,
                Action::Div  => self.state / input,
            }
        }
        self.action = a;
        self.update_display(g)
    }

    fn update_display(&self, g: &mut Gui) {
        let text =
            if self.input.len() == 0 {
                g.create_text(&self.state.to_string())
            }
            else {
                g.create_text(&self.input)
            };
        g.set_children(self.display, [text]);
    }
}

fn number_button(n: u32, calc: &Rc<RefCell<Calc>>, g: &mut Gui) -> Node {
    let button = g.create_node(NodeKind::Button);

    let text = g.create_text(&n.to_string());
    g.append_child(button, text);

    let input = char::from_u32('0' as u32 + n).unwrap();

    let calc = calc.clone();
    g.set_on_click(button, move |g: &mut Gui, _e: &mut Event| {
        let mut calc = calc.borrow_mut();
        calc.input(input, g);
    });

    button
}

fn action_button(a: Action, calc: &Rc<RefCell<Calc>>, g: &mut Gui) -> Node {
    let button = g.create_node(NodeKind::Button);

    let text = g.create_text(match a {
        Action::None => "=",
        Action::Add  => "+",
        Action::Sub  => "-",
        Action::Mul  => "*",
        Action::Div  => "/",
    });
    g.append_child(button, text);

    let calc = calc.clone();
    g.set_on_click(button, move |g: &mut Gui, _e: &mut Event| {
        let mut calc = calc.borrow_mut();
        calc.action(a, g);
    });

    button
}


fn main() {
    let mut ngui = NativeGui::new();

    ngui.with_gui(|g| {
        let wrapper = g.create_node(NodeKind::Div);

        let display = g.create_node(NodeKind::Div);
        g.append_child(wrapper, display);

        let calc = Rc::new(RefCell::new(Calc::new(display, g)));

        let buttons: [Node; 10] = core::array::from_fn(|i| {
            number_button(i as u32, &calc, g)
        });

        let row1 = g.create_node(NodeKind::Div);
        g.set_children(row1, [ buttons[7], buttons[8], buttons[9] ]);
        g.append_child(wrapper, row1);

        let row2 = g.create_node(NodeKind::Div);
        g.set_children(row2, [ buttons[4], buttons[5], buttons[6] ]);
        g.append_child(wrapper, row2);

        let row3 = g.create_node(NodeKind::Div);
        g.set_children(row3, [ buttons[1], buttons[2], buttons[3] ]);
        g.append_child(wrapper, row3);

        g.append_child(wrapper, buttons[0]);

        let actions = [
            action_button(Action::Add,  &calc, g),
            action_button(Action::Sub,  &calc, g),
            action_button(Action::Mul,  &calc, g),
            action_button(Action::Div,  &calc, g),
            action_button(Action::None, &calc, g),
        ];

        let row5 = g.create_node(NodeKind::Div);
        g.set_children(row5, actions);
        g.append_child(wrapper, row5);

        let root = g.root();
        g.append_child(root, wrapper);
    });

    ngui.run_message_loop();
}

