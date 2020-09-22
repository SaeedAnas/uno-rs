use cursive::traits::*;
use cursive::views::{Button, Dialog, EditView, LinearLayout};
use cursive::Cursive;

#[derive(Debug)]
pub struct Config {
    user: String,
    network: Option<String>,
}

impl Config {
    fn default() -> Self {
        Self {
            user: String::new(),
            network: None,
        }
    }

    fn user(&mut self, user: &str) {
        self.user = user.to_string();
    }

    fn network(&mut self, network: Option<&str>) {
        match network {
            Some(s) => self.network = Some(s.to_string()),
            None => self.network = None,
        }
    }

    pub fn get_user(&self) -> &String {
        &self.user
    }

    pub fn get_network(&self) -> Option<&String> {
        self.network.as_ref()
    }
}

pub fn prompt() -> Config {
    let mut siv = cursive::default();
    siv.set_user_data(Config::default());

    let user = EditView::new()
        .on_submit(handle_user)
        .with_name("username")
        .fixed_width(20);

    let form = Dialog::around(user)
        .button("Ok", |s| {
            let name = s
                .call_on_name("username", |view: &mut EditView| view.get_content())
                .unwrap();

            handle_user(s, &name);
        })
        .title("Enter your name");

    siv.add_layer(form);

    siv.run();

    siv.take_user_data().unwrap()
}

fn handle_user(s: &mut Cursive, user: &str) {
    if user.is_empty() {
        // Try again
        s.add_layer(Dialog::info("Please enter a name"));
    } else {
        let config: &mut Config = s.user_data().unwrap();
        config.user(user);
        s.pop_layer();
        get_network(s);
    }
}

fn get_network(s: &mut Cursive) {
    let buttons = LinearLayout::vertical()
        .child(Button::new("Make a new network", new_network))
        .child(Button::new("Join a network", join_network));
    s.add_layer(Dialog::around(buttons));
}

fn new_network(s: &mut Cursive) {
    let config: &mut Config = s.user_data().unwrap();
    config.network(None);
    s.quit();
}

fn join_network(s: &mut Cursive) {
    let user = EditView::new()
        .on_submit(handle_network)
        .with_name("network")
        .fixed_width(20);

    let form = Dialog::around(user)
        .button("Ok", |s| {
            let network = s
                .call_on_name("network", |view: &mut EditView| view.get_content())
                .unwrap();

            handle_network(s, &network);
        })
        .title("Enter network addr");

    s.add_layer(form);
}

fn handle_network(s: &mut Cursive, network: &str) {
    if network.is_empty() {
        // Try again
        s.add_layer(Dialog::info("Please enter network addr"));
    } else {
        let config: &mut Config = s.user_data().unwrap();
        config.network(Some(network));
        s.quit();
    }
}
