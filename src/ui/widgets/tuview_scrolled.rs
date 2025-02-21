use std::sync::{
    Arc,
    atomic::{
        AtomicBool,
        Ordering,
    },
};

use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gtk::{
    CompositeTemplate,
    SignalListItemFactory,
    gio,
    glib::{
        self,
        clone,
    },
    template_callbacks,
};

use super::{
    single_grid::imp::ViewType,
    tu_list_item::imp::PosterType,
    tu_overview_item::imp::ViewGroup,
    utils::TuItemBuildExt,
};
use crate::{
    client::structs::SimpleListItem,
    ui::provider::{
        tu_item::TuItem,
        tu_object::TuObject,
    },
};

pub(crate) mod imp {

    use std::sync::{
        Arc,
        atomic::AtomicBool,
    };

    use glib::subclass::InitializingObject;

    use super::*;
    use crate::ui::provider::tu_object::TuObject;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/tuview_scrolled.ui")]
    pub struct TuViewScrolled {
        #[template_child]
        pub scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub grid: TemplateChild<gtk::GridView>,
        #[template_child]
        pub list: TemplateChild<gtk::ListView>,
        #[template_child]
        pub spinner_revealer: TemplateChild<gtk::Revealer>,

        pub selection: gtk::SingleSelection,
        pub lock: Arc<AtomicBool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TuViewScrolled {
        const NAME: &'static str = "TuViewScrolled";
        type Type = super::TuViewScrolled;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TuViewScrolled {
        fn constructed(&self) {
            self.parent_constructed();
            let store = gio::ListStore::new::<TuObject>();
            self.selection.set_model(Some(&store));
            self.obj().set_view_type(ViewType::GridView);
        }
    }

    impl WidgetImpl for TuViewScrolled {}
    impl BinImpl for TuViewScrolled {}
}

glib::wrapper! {
    pub struct TuViewScrolled(ObjectSubclass<imp::TuViewScrolled>)
        @extends gtk::Widget, @implements gtk::Accessible;
}

impl Default for TuViewScrolled {
    fn default() -> Self {
        Self::new()
    }
}

#[template_callbacks]
impl TuViewScrolled {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_store<const C: bool>(&self, items: Vec<SimpleListItem>, is_resume: bool) {
        let imp = self.imp();
        let Some(store) = imp.selection.model().and_downcast::<gio::ListStore>() else {
            return;
        };

        if C {
            store.remove_all();
        }

        for item in items {
            let tu_item = TuItem::from_simple(&item, None);
            tu_item.set_is_resume(is_resume);
            let tu_item = TuObject::new(&tu_item);
            store.append(&tu_item);
        }
    }

    pub fn set_view_type(&self, view_type: ViewType) {
        let imp = self.imp();
        let factory = SignalListItemFactory::new();
        match view_type {
            ViewType::GridView => {
                imp.scrolled_window.set_child(Some(&imp.grid.get()));
                imp.grid
                    .set_factory(Some(factory.tu_item(PosterType::default())));
                imp.grid.set_model(Some(&imp.selection));
            }
            ViewType::ListView => {
                imp.scrolled_window.set_child(Some(&imp.list.get()));
                imp.list
                    .set_factory(Some(factory.tu_overview_item(ViewGroup::ListView)));
                imp.list.set_model(Some(&imp.selection));
            }
        }
    }

    #[template_callback]
    fn on_gridview_item_activated(&self, position: u32, view: &gtk::GridView) {
        let Some(model) = view.model() else {
            return;
        };
        let Some(tu_obj) = model.item(position).and_downcast::<TuObject>() else {
            return;
        };
        tu_obj.activate(view);
    }

    #[template_callback]
    fn on_listview_item_activated(&self, position: u32, view: &gtk::ListView) {
        let Some(model) = view.model() else {
            return;
        };
        let Some(tu_obj) = model.item(position).and_downcast::<TuObject>() else {
            return;
        };
        tu_obj.activate(view);
    }

    pub fn connect_end_edge_reached<F>(&self, cb: F)
    where
        F: Fn(&Self, Arc<AtomicBool>) + 'static,
    {
        self.imp().scrolled_window.connect_edge_overshot(clone!(
            #[weak(rename_to = obj)]
            self,
            move |_scrolled, pos| if pos == gtk::PositionType::Bottom {
                let is_running = Arc::clone(&obj.imp().lock);

                if is_running
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_err()
                {
                    return;
                }

                cb(&obj, is_running);
            }
        ));
    }

    pub fn n_items(&self) -> u32 {
        let imp = self.imp();
        let Some(store) = imp.selection.model().and_downcast::<gio::ListStore>() else {
            return 0;
        };
        store.n_items()
    }

    pub fn reveal_spinner(&self, reveal: bool) {
        self.imp().spinner_revealer.set_reveal_child(reveal);
    }
}
