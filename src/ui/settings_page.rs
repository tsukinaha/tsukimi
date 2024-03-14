use gtk::glib;
use dirs::home_dir;
use gtk::prelude::*;
use gtk::{Box, Button, Entry, Label, Orientation};
use serde::Serialize;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use crate::ui::network;

use super::network::runtime;

#[derive(Serialize,Debug, Deserialize)]
pub struct Config {
    pub domain: String,
    pub username: String,
    pub password: String,
    pub port: String,
    pub user_id: String,
    pub access_token: String,
}

pub fn create_page2() -> Box {
    let vbox = Box::new(Orientation::Vertical, 10);

    let spacer = Label::new(None);
    spacer.set_size_request(-1, 15);
    vbox.append(&spacer);

    let hbox = Box::new(Orientation::Horizontal, 10);
    let severname_label = Label::new(Some("服务器："));
    hbox.append(&severname_label);

    let servername_entry = Entry::new();
    servername_entry.set_size_request(400, -1);
    servername_entry.set_placeholder_text(Some("http(s)://example.com"));
    hbox.append(&servername_entry);
    hbox.set_halign(gtk::Align::Center);
    vbox.append(&hbox);

    let hbox = Box::new(Orientation::Horizontal, 30);
    let port_label = Label::new(Some("端口："));
    hbox.append(&port_label);

    let port_entry = Entry::new();
    port_entry.set_size_request(400, -1);
    port_entry.set_placeholder_text(Some("8096"));
    hbox.append(&port_entry);
    hbox.set_halign(gtk::Align::Center);
    vbox.append(&hbox);

    let hbox = Box::new(Orientation::Horizontal, 10);
    let username_label = Label::new(Some("用户名："));
    hbox.append(&username_label);

    let username_entry = Entry::new();
    username_entry.set_size_request(400, -1);
    hbox.append(&username_entry);
    hbox.set_halign(gtk::Align::Center);
    vbox.append(&hbox);

    let hbox = Box::new(Orientation::Horizontal, 30);
    let password_label = Label::new(Some("密码："));
    hbox.append(&password_label);

    let password_entry = Entry::new();
    password_entry.set_size_request(400, -1);
    hbox.append(&password_entry);
    hbox.set_halign(gtk::Align::Center);
    vbox.append(&hbox);
    let path = home_dir().unwrap()
                                    .join(".config/tsukimi.yaml");
    if path.exists() {
        let file = File::open(&path).expect("Unable to open file");
        let reader = BufReader::new(file);
        let config: Config = serde_yaml::from_reader(reader).expect("Unable to parse YAML file");
        servername_entry.set_text(&config.domain);
        port_entry.set_text(&config.port);
        username_entry.set_text(&config.username);
        password_entry.set_text(&config.password);
    } 
    
    let hbox = Box::new(Orientation::Horizontal, 20);

    let login_button = Button::with_label("登录");
    login_button.set_size_request(200, -1);

    login_button.connect_clicked(move |login_button| {
        let servername = servername_entry.text().to_string();
        let port = port_entry.text().to_string();
        let username = username_entry.text().to_string();
        let password = password_entry.text().to_string();
        login_button.set_label("登录中");
        let login_button = login_button.clone();

        let (sender, receiver) = async_channel::bounded::<String>(1);

        runtime().spawn(async move {
            match network::login(servername, username, password, port).await {
                Ok(_) => {
                    sender.send("1".to_string()).await.expect("The channel needs to be open.");
                },
                Err(e) => eprintln!("Error: {}", e),
            }
        });
        glib::MainContext::default().spawn_local(async move {
            while let Ok(_) = receiver.recv().await {
                login_button.set_label("登录成功");
                println!("Login successful");
            }
        });
    });
    hbox.append(&login_button);

    hbox.set_halign(gtk::Align::Center);
    vbox.append(&hbox);

    vbox
}
