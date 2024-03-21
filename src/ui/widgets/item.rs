use gtk::{gio, glib};
use glib::Object;
mod imp{
    use std::path::PathBuf;
    use std::cell::OnceCell;
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{glib, CompositeTemplate};
    use gtk::prelude::*;

    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/item.ui")]
    #[properties(wrapper_type = super::ItemPage)]
    pub struct ItemPage {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,

        #[template_child]
        pub backdrop: TemplateChild<gtk::Picture>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for ItemPage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "ItemPage";
        type Type = super::ItemPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    #[glib::derived_properties]
    impl ObjectImpl for ItemPage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            let id = obj.id();
            let path = format!("{}/.local/share/tsukimi/b{}.png",dirs::home_dir().expect("msg").display(), id);
            let pathbuf = PathBuf::from(&path);
            let backdrop = self.backdrop.get();
            let (sender, receiver) = async_channel::bounded::<String>(1);
            let idclone = id.clone();
            if pathbuf.exists() {
                backdrop.set_file(Some(&gtk::gio::File::for_path(&path)));
            } else {
                crate::ui::network::runtime().spawn(async move {
                    let id = crate::ui::network::get_backdropimage(idclone).await.expect("msg");
                    sender.send(id.clone()).await.expect("The channel needs to be open.");
                });
            }
        
            glib::spawn_future_local(async move {
                while let Ok(_) = receiver.recv().await {
                    let path = format!("{}/.local/share/tsukimi/b{}.png",dirs::home_dir().expect("msg").display(), id);
                    let file = gtk::gio::File::for_path(&path);
                    backdrop.set_file(Some(&file));
                }
            });
        }

    }

    // Trait shared by all widgets
    impl WidgetImpl for ItemPage {}

    // Trait shared by all windows
    impl WindowImpl for ItemPage {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for ItemPage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for ItemPage {}
}

glib::wrapper! {
    pub struct ItemPage(ObjectSubclass<imp::ItemPage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl ItemPage {
    pub fn new(id:String) -> Self {
        Object::builder().property("id", id).build()
    }
}