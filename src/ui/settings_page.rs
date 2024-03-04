use gtk::glib::home_dir;
use gtk::prelude::*;
use gtk::{Box, Button, Entry, Label, Orientation};
use serde::Serialize;
use serde_yaml::to_string;
use std::fs::write;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use crate::ui::network;

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

    let path = home_dir().join(".config/fpv.yaml");

    let file = File::open(&path).expect("Unable to open file");
    let reader = BufReader::new(file);

    // 使用 serde_yaml::from_reader 函数来解析文件
    let config: Config = serde_yaml::from_reader(reader).expect("Unable to parse YAML file");

    // 将设置填入到对应的 Entry 中
    servername_entry.set_text(&config.domain);
    port_entry.set_text(&config.port);
    username_entry.set_text(&config.username);
    password_entry.set_text(&config.password);

    let hbox = Box::new(Orientation::Horizontal, 20);

    let servername_entry_clone = servername_entry.clone();
    let port_entry_clone = port_entry.clone();
    let username_entry_clone = username_entry.clone();
    let password_entry_clone = password_entry.clone();

    let save_button = Button::with_label("保存");
    save_button.set_size_request(200, -1);
    save_button.connect_clicked(move |_| {
        let domain = servername_entry_clone.text().to_string();
        let username = username_entry_clone.text().to_string();
        let password = password_entry_clone.text().to_string();
        let port = port_entry_clone.text().to_string();
        let config = Config {
            domain,
            username,
            password,
            port,
            user_id: "".to_string(),
            access_token: "".to_string(),
        };
        let yaml = to_string(&config).unwrap();
        let mut path = home_dir();
        path.push(".config");
        path.push("fpv.yaml");
        write(path, yaml).unwrap();
    });
    hbox.append(&save_button);

    let login_button = Button::with_label("登录");
    login_button.set_size_request(200, -1);

    login_button.connect_clicked(move |_| {
        let servername = servername_entry.text().to_string();
        let port = port_entry.text().to_string();
        let username = username_entry.text().to_string();
        let password = password_entry.text().to_string();
        tokio::spawn(async {
            match network::login(servername, username, password, port).await {
                Ok(_) => println!("Login successful"),
                Err(e) => eprintln!("Error: {}", e),
            }
        });
    });
    hbox.append(&login_button);

    hbox.set_halign(gtk::Align::Center);
    vbox.append(&hbox);

    vbox
}
