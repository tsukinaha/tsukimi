use super::tu_list_item::imp::PosterType;
use crate::client::structs::SimpleListItem;
use adw::prelude::*;
use glib::Object;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
mod imp {

    use std::cell::RefCell;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    use crate::ui::widgets::tu_list_item::imp::PosterType;
    use crate::ui::widgets::tuview_scrolled::TuViewScrolled;

    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/single_grid.ui")]
    #[properties(wrapper_type = super::SingleGrid)]
    pub struct SingleGrid {
        #[template_child]
        pub count: TemplateChild<gtk::Label>,
        #[template_child]
        pub postmenu: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub dropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub adbutton: TemplateChild<gtk::Box>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub scrolled: TemplateChild<TuViewScrolled>,

        #[property(get, set, nullable)]
        pub listtype: RefCell<Option<String>>,

        pub popovermenu: RefCell<Option<gtk::PopoverMenu>>,
        pub sortorder: RefCell<String>,
        pub sortby: RefCell<String>,
        pub lock: Arc<AtomicBool>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for SingleGrid {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "SingleGrid";
        type Type = super::SingleGrid;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            TuViewScrolled::ensure_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
            klass.install_action_async("poster", None, |window, _action, _parameter| async move {
                window.poster(PosterType::Poster).await;
            });
            klass.install_action_async(
                "backdrop",
                None,
                |window, _action, _parameter| async move {
                    window.poster(PosterType::Backdrop).await;
                },
            );
            klass.install_action_async("banner", None, |window, _action, _parameter| async move {
                window.poster(PosterType::Banner).await;
            });
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for SingleGrid {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_up();
        }
    }

    impl WidgetImpl for SingleGrid {}

    impl WindowImpl for SingleGrid {}

    impl ApplicationWindowImpl for SingleGrid {}

    impl adw::subclass::navigation_page::NavigationPageImpl for SingleGrid {}
}

glib::wrapper! {
    pub struct SingleGrid(ObjectSubclass<imp::SingleGrid>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Default for SingleGrid {
    fn default() -> Self {
        Self::new()
    }
}

#[gtk::template_callbacks]
impl SingleGrid {
    pub fn new() -> Self {
        Object::new()
    }

    fn set_up(&self) {
        self.imp().sortorder.replace("Descending".to_string());
        self.imp().sortby.replace("SortName".to_string());
        self.handle_type();
    }

    #[template_callback]
    async fn sort_order_ascending_cb(&self, _btn: &gtk::ToggleButton) {
        self.imp().sortorder.replace("Ascending".to_string());
    }

    #[template_callback]
    async fn sort_order_descending_cb(&self, _btn: &gtk::ToggleButton) {
        self.imp().sortorder.replace("Descending".to_string());
    }

    #[template_callback]
    fn filter_panel_cb(&self, _btn: &gtk::Button) {
        let dialog = adw::Dialog::builder()
            .title("Filter")
            .presentation_mode(adw::DialogPresentationMode::BottomSheet)
            .build();
        dialog.present(Some(self));
    }

    fn handle_type(&self) {
        let imp = self.imp();
        let listtype = self.listtype().unwrap_or("".to_string());
        match listtype.as_str() {
            "all" => {}
            "resume" => {
                imp.postmenu.set_visible(false);
                imp.dropdown.set_visible(false);
                imp.adbutton.set_visible(false);
            }
            "boxset" => {
                imp.postmenu.set_visible(false);
            }
            "tags" => {
                imp.postmenu.set_visible(false);
                imp.dropdown.set_visible(false);
                imp.adbutton.set_visible(false);
            }
            "genres" => {
                imp.postmenu.set_visible(false);
                imp.dropdown.set_visible(false);
                imp.adbutton.set_visible(false);
            }
            "liked" => {
                imp.postmenu.set_visible(false);
            }
            _ => {
                imp.postmenu.set_visible(false);
            }
        }
    }

    pub fn update_sortby(&self, selected: u32) {
        let sortby = match selected {
            0 => "SortName",
            1 => "TotalBitrate,SortName",
            2 => "DateCreated,SortName",
            3 => "CommunityRating,SortName",
            4 => "CriticRating,SortName",
            5 => "ProductionYear,PremiereDate,SortName",
            6 => "OfficialRating,SortName",
            7 => "ProductionYear,SortName",
            8 => "DatePlayed,SortName",
            9 => "Runtime,SortName",
            _ => "SortName",
        };
        self.imp().sortby.replace(sortby.to_string());
    }

    pub async fn poster(&self, _poster_type: PosterType) {}

    pub fn add_items<const C: bool>(&self, items: Vec<SimpleListItem>) {
        let imp = self.imp();
        let scrolled = imp.scrolled.get();
        scrolled.set_grid::<C>(items);
        if scrolled.n_items() == 0 {
            imp.stack.set_visible_child_name("fallback");
        }
    }

    pub fn set_item_number(&self, n: u32) {
        self.imp().count.set_text(&format!("{} Items", n));
    }
}
