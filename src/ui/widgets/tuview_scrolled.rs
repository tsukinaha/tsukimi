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
    tu_item::{
        CardOptions,
        CardShape,
    },
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

    use std::cell::Cell;

    use glib::subclass::InitializingObject;
    use gtk::glib::Properties;

    use super::*;
    use crate::ui::provider::tu_object::TuObject;

    pub struct NoSelectionWrap(pub gtk::NoSelection);

    impl Default for NoSelectionWrap {
        fn default() -> Self {
            Self(gtk::NoSelection::new(Some(
                gio::ListStore::new::<TuObject>(),
            )))
        }
    }

    impl std::ops::Deref for NoSelectionWrap {
        type Target = gtk::NoSelection;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    #[derive(CompositeTemplate, Default, Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/tuview_scrolled.ui")]
    #[properties(wrapper_type = super::TuViewScrolled)]
    pub struct TuViewScrolled {
        #[template_child]
        pub scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub grid: TemplateChild<gtk::GridView>,
        #[template_child]
        pub list: TemplateChild<gtk::ListView>,
        #[template_child]
        pub spinner_revealer: TemplateChild<gtk::Revealer>,

        pub selection: NoSelectionWrap,
        pub lock: Arc<AtomicBool>,

        #[property(get, set, builder(CardShape::default()))]
        pub card_shape: Cell<CardShape>,
        #[property(get, set, default = false)]
        pub prefer_thumb: Cell<bool>,
        #[property(get, set, default = false)]
        pub prefer_banner: Cell<bool>,
        #[property(get, set, default = false)]
        pub prefer_parent_poster: Cell<bool>,
        #[property(get, set, default = false)]
        pub is_resume: Cell<bool>,
        pub resolved_card_shape: Cell<CardShape>,
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

    #[glib::derived_properties]
    impl ObjectImpl for TuViewScrolled {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().set_view_type(ViewType::GridView);
        }
    }

    impl WidgetImpl for TuViewScrolled {}
    impl BinImpl for TuViewScrolled {}
}

glib::wrapper! {
    pub struct TuViewScrolled(ObjectSubclass<imp::TuViewScrolled>)
        @extends gtk::Widget, adw::Bin, @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
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

    pub fn set_store<const C: bool>(&self, items: Vec<SimpleListItem>) {
        let imp = self.imp();
        let Some(store) = imp.selection.model().and_downcast::<gio::ListStore>() else {
            return;
        };

        if C {
            imp.resolved_card_shape.set(CardShape::Auto.resolve(&items));
            self.set_grid_factory();
        }

        let is_resume = self.is_resume();

        let items = items
            .into_iter()
            .map(|item| {
                let tu_item = TuItem::from_simple(item);
                tu_item.set_is_resume(is_resume);
                TuObject::new(tu_item)
            })
            .collect::<Vec<_>>();

        if C {
            store.splice(0, store.n_items(), &items);
        } else {
            store.extend_from_slice(&items);
        }
    }

    pub fn set_view_type(&self, view_type: ViewType) {
        let imp = self.imp();
        let factory = SignalListItemFactory::new();
        match view_type {
            ViewType::GridView => {
                imp.scrolled_window.set_child(Some(&imp.grid.get()));
                imp.grid
                    .set_factory(Some(factory.tu_item(self.card_options())));
                imp.grid.set_model(Some(&imp.selection.0));
            }
            ViewType::ListView => {
                imp.scrolled_window.set_child(Some(&imp.list.get()));
                imp.list.set_factory(Some(
                    factory.tu_overview_item(ViewGroup::ListView, self.card_options()),
                ));
                imp.list.set_model(Some(&imp.selection.0));
            }
        }
    }

    fn set_grid_factory(&self) {
        let factory = SignalListItemFactory::new();
        self.imp()
            .grid
            .set_factory(Some(factory.tu_item(self.card_options())));
    }

    fn effective_card_shape(&self) -> CardShape {
        match self.card_shape() {
            CardShape::Auto => self.imp().resolved_card_shape.get(),
            card_shape => card_shape,
        }
    }

    fn card_options(&self) -> CardOptions {
        CardOptions {
            shape: self.effective_card_shape(),
            prefer_thumb: self.prefer_thumb(),
            prefer_banner: self.prefer_banner(),
            prefer_parent_poster: self.prefer_parent_poster(),
        }
    }

    pub fn apply_card_shape(&self, card_shape: CardShape) {
        self.set_card_shape(card_shape);
        self.set_grid_factory();
    }

    pub fn apply_image_options(
        &self, card_shape: CardShape, prefer_thumb: bool, prefer_banner: bool,
    ) {
        self.set_card_shape(card_shape);
        self.set_prefer_thumb(prefer_thumb);
        self.set_prefer_banner(prefer_banner);
        self.set_grid_factory();
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
