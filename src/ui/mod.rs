mod item_page;
mod network;
mod search_page;
mod settings_page;
mod episodes_page;
mod new_dropsel;
mod image;
mod movie_page;
mod home_page;
use gtk::gdk::Display;
use gtk::{prelude::*, CssProvider};
use gtk::{Application, ApplicationWindow, HeaderBar, Stack, StackSwitcher};

pub fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Tsukimi")
        .default_width(1200)
        .default_height(900)
        .build();

    let header = HeaderBar::new();
    header.set_show_title_buttons(true);

    let stack = Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::SlideLeftRight);

    let searchstack = Stack::new(); 
    let searchstackclone = searchstack.clone();
    let backbutton = gtk::Button::new();
    
    header.pack_start(&backbutton);

    backbutton.set_icon_name("go-previous-symbolic");
    backbutton.set_visible(false);
    let backbuttonclone = backbutton.clone();
    backbutton.connect_clicked(move |_| {
        backbuttonclone.set_visible(false);
        searchstackclone.set_visible_child_name("page1");
    });

    let homepage = home_page::create_page();
    stack.add_titled(&homepage, Some("page0"), "Home");

    let labelsearch = search_page::create_page1(searchstack,backbutton);   
    stack.add_titled(&labelsearch, Some("page1"), "Search");

    let labelsettings = settings_page::create_page2();
    stack.add_titled(&labelsettings, Some("page2"), "Server Settings");

    let stack_switcher = StackSwitcher::new();
    stack_switcher.set_stack(Some(&stack));
    header.set_title_widget(Some(&stack_switcher));
    window.set_titlebar(Some(&header));
    window.set_child(Some(&stack));

    window.show();
}

pub fn load_css(){
    let provider = CssProvider::new();
    provider.load_from_string(include_str!("style.css"));

    // Add the provider to the default screen
    gtk::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}