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
use gtk::glib::{self, clone};

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
    let homestack = Stack::new();
    homestack.set_transition_type(gtk::StackTransitionType::Crossfade);
    let backbutton = gtk::Button::new();
    
    header.pack_start(&backbutton);

    backbutton.set_icon_name("go-previous-symbolic");
    backbutton.set_visible(false);
    let backbuttonclone = backbutton.clone();
    backbutton.connect_clicked(clone!(@weak searchstack,@weak homestack => move |_| {
        backbuttonclone.set_visible(false);
        searchstack.set_visible_child_name("page1");
        let episodes = format!("episodes_page");
        let item = format!("item_page");
        if searchstack.child_by_name(&episodes).is_some() {
            searchstack.remove(&searchstack.child_by_name(&episodes).unwrap());
        }
        if searchstack.child_by_name(&item).is_some() {
            searchstack.remove(&searchstack.child_by_name(&item).unwrap());
        }
        homestack.set_visible_child_name("page0");
        if homestack.child_by_name(&episodes).is_some() {
            homestack.remove(&homestack.child_by_name(&episodes).unwrap());
        }
        if homestack.child_by_name(&item).is_some() {
            homestack.remove(&homestack.child_by_name(&item).unwrap());
        }
    }));

    let viewmorebutton = gtk::Button::new();
    viewmorebutton.set_icon_name("view-more-symbolic");
    viewmorebutton.connect_clicked(move |_| {
        let window = gtk::Window::new();
        window.set_title(Some("Settings"));
        window.set_default_size(700, 400);
        window.set_child(Some(&settings_page::create_page2()));
        window.set_visible(true);
    });
    header.pack_end(&viewmorebutton);

    let homepage = home_page::create_page(homestack,backbutton.clone());
    stack.add_titled(&homepage, Some("page0"), "Home");

    let labelsearch = search_page::create_page1(searchstack,backbutton);   
    stack.add_titled(&labelsearch, Some("page1"), "Search");

    let stack_switcher = StackSwitcher::new();
    stack_switcher.set_stack(Some(&stack));
    header.set_title_widget(Some(&stack_switcher));
    window.set_titlebar(Some(&header));
    window.set_child(Some(&stack));

    window.set_visible(true);
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