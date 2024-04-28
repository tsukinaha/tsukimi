use glib::Object;
use gtk::gdk::Rectangle;
use gtk::gio::MenuModel;
use gtk::glib::subclass::types::ObjectSubclassIsExt;
use gtk::prelude::*;
use gtk::Builder;
use gtk::PopoverMenu;
use gtk::{gio, glib};

use crate::ui::image::setbackdropimage;
use crate::ui::image::setimage;
use crate::ui::provider::tu_item::TuItem;

mod imp {
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{glib, CompositeTemplate};
    use gtk::{prelude::*, PopoverMenu};
    use std::cell::{OnceCell, RefCell};

    use crate::ui::provider::tu_item::TuItem;

    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/listitem.ui")]
    #[properties(wrapper_type = super::TuListItem)]
    pub struct TuListItem {
        #[property(get, set, construct_only)]
        pub item: OnceCell<TuItem>,
        #[property(get, set, construct_only)]
        pub itemtype: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub isresume: OnceCell<bool>,
        pub popover: RefCell<Option<PopoverMenu>>,
        #[template_child]
        pub listlabel: TemplateChild<gtk::Label>,
        #[template_child]
        pub overlay: TemplateChild<gtk::Overlay>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for TuListItem {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "TuListItem";
        type Type = super::TuListItem;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    #[glib::derived_properties]
    impl ObjectImpl for TuListItem {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_up();
            obj.gesture();
        }

        fn dispose(&self) {
            if let Some(popover) = self.popover.borrow().as_ref() {
                popover.unparent();
            };
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for TuListItem {}

    // Trait shared by all windows
    impl WindowImpl for TuListItem {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for TuListItem {}

    impl BinImpl for TuListItem {}
}

glib::wrapper! {
    pub struct TuListItem(ObjectSubclass<imp::TuListItem>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl TuListItem {
    pub fn new(item: TuItem, item_type: &str, isresume: bool) -> Self {
        Object::builder()
            .property("itemtype", item_type)
            .property("item", item)
            .property("isresume", isresume)
            .build()
    }

    pub fn set_up(&self) {
        let imp = self.imp();
        let item = imp.item.get().unwrap();
        let item_type = imp.itemtype.get().unwrap();
        match item_type.as_str() {
            "Movie" => {
                let year = if item.production_year() != 0 { 
                    item.production_year().to_string()
                } else { 
                    String::from("") 
                };
                imp.listlabel.set_text(format!("{}\n{}", item.name(), year).as_str());
                self.set_picture();
                self.set_played();
            }
            "Series" => {
                imp.listlabel.set_text(format!("{}\n{}", item.name(), item.production_year()).as_str());
                self.set_picture();
                self.set_played();
                self.set_count();
            }
            "BoxSet" => {
                imp.listlabel.set_text(format!("{}", item.name()).as_str());
                self.set_picture();
            }
            "Tag" | "Genre" => {
                imp.overlay.set_size_request(190,190);
                imp.listlabel.set_text(format!("{}", item.name()).as_str());
                self.set_picture();
            }
            _ => {}
        }
    }

    pub fn set_picture(&self) {
        let imp = self.imp();
        let item = imp.item.get().unwrap();
        let id = item.id();
        let image = if let Some(true) = imp.isresume.get() {
            imp.overlay.set_size_request(250, 141);
            setbackdropimage(id, 0)
        } else {
            setimage(id)
        };
        imp.overlay.set_child(Some(&image)); 
    }

    pub fn set_played(&self) {
        let imp = self.imp();
        let item = imp.item.get().unwrap();
        if item.played() {
            let mark = gtk::Image::from_icon_name("object-select-symbolic");
            mark.set_halign(gtk::Align::End);
            mark.set_valign(gtk::Align::Start);
            mark.set_height_request(40);
            mark.set_width_request(40);
            imp.overlay.add_overlay(&mark);
        }
    }

    pub fn set_count(&self) {
        let imp = self.imp();
        let item = imp.item.get().unwrap();
        let count = item.unplayed_item_count();
        if count > 0 {
            let mark = gtk::Label::new(Some(&count.to_string()));
            mark.set_halign(gtk::Align::End);
            mark.set_valign(gtk::Align::Start);
            mark.set_height_request(40);
            mark.set_width_request(40);
            imp.overlay.add_overlay(&mark);
        }
    }

    pub fn gesture(&self) {
        let imp = self.imp();
        let builder = Builder::from_resource("/moe/tsukimi/pop-menu.ui");
        let menu = builder.object::<MenuModel>("rightmenu");
        match menu {
            Some(popover) => {
                let popover = PopoverMenu::builder().menu_model(&popover).has_arrow(false).build();
                popover.set_parent(self);
                let _ = imp.popover.replace(Some(popover));
            }
            None => eprintln!("Failed to load popover"),
        }
        let gesture = gtk::GestureClick::new();
        gesture.set_button(3);
        gesture.connect_released(glib::clone!(@weak imp => move | gesture, _n, x, y| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            if let Some(popover) = imp.popover.borrow().as_ref() {
                popover.set_pointing_to(Some(&Rectangle::new(x as i32, y as i32, 1, 1)));
                popover.popup();
            };
        }));
        self.add_controller(gesture);
    }
}
