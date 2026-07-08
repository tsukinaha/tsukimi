use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gtk::{
    CompositeTemplate,
    gio,
    glib,
};

use crate::{
    client::structs::{
        SGTitem,
        Urls,
    },
    ui::widgets::fix::scroll_widget_to_row_center,
    utils::spawn,
};

mod imp {
    use std::cell::{
        Cell,
        RefCell,
    };

    use glib::subclass::InitializingObject;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/horbu_scrolled.ui")]
    #[properties(wrapper_type = super::HorbuScrolled)]
    pub struct HorbuScrolled {
        #[template_child]
        pub label: TemplateChild<gtk::Label>,
        #[template_child]
        pub wrapbox: TemplateChild<adw::WrapBox>,
        #[template_child]
        pub revealer: TemplateChild<gtk::Revealer>,

        #[property(get, set)]
        pub title: RefCell<String>,

        pub selection: gtk::SingleSelection,
        pub selected_index: Cell<Option<usize>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HorbuScrolled {
        const NAME: &'static str = "HorbuScrolled";
        type Type = super::HorbuScrolled;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for HorbuScrolled {}

    impl WidgetImpl for HorbuScrolled {}

    impl BinImpl for HorbuScrolled {}
}

glib::wrapper! {
    /// A scrolled list of items.
    pub struct HorbuScrolled(ObjectSubclass<imp::HorbuScrolled>)
        @extends gtk::Widget, adw::Bin, @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for HorbuScrolled {
    fn default() -> Self {
        Self::new()
    }
}

impl HorbuScrolled {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_items(&self, items: &[SGTitem], type_: &str) {
        if items.is_empty() {
            return;
        }

        self.set_visible(true);

        let items = items.to_owned();

        let imp = self.imp();

        imp.revealer.set_reveal_child(true);

        let wrapbox = imp.wrapbox.get();
        let type_ = type_.to_string();

        spawn(glib::clone!(
            #[weak]
            wrapbox,
            #[weak(rename_to = obj)]
            self,
            async move {
                for result in items {
                    let buttoncontent = adw::ButtonContent::builder().label(&result.name).build();

                    match type_.as_str() {
                        "Studios" => {
                            buttoncontent.set_icon_name("sound-symbolic");
                        }
                        "Genres" => {
                            buttoncontent.set_icon_name("music-note-single-outline-symbolic");
                        }
                        _ => {
                            buttoncontent.set_icon_name("tag-outline-symbolic");
                        }
                    }

                    let button = gtk::Button::builder().child(&buttoncontent).build();

                    let type_ = type_.to_string();
                    button.connect_clicked(glib::clone!(
                        #[weak]
                        obj,
                        move |_| {
                            result.activate(&obj, type_.to_string());
                        }
                    ));

                    wrapbox.append(&button);
                }
            }
        ));
    }

    pub fn set_links(&self, items: &[Urls]) {
        if items.is_empty() {
            return;
        }

        self.set_visible(true);

        let items = items.to_owned();

        let imp = self.imp();

        imp.revealer.set_reveal_child(true);

        let wrapbox = imp.wrapbox.get();

        while let Some(child) = wrapbox.last_child() {
            wrapbox.remove(&child);
        }

        spawn(glib::clone!(
            #[weak]
            wrapbox,
            async move {
                for result in items {
                    let buttoncontent = adw::ButtonContent::builder()
                        .label(&result.name)
                        .icon_name("external-link-symbolic")
                        .build();

                    let button = gtk::Button::builder().child(&buttoncontent).build();

                    button.connect_clicked(move |_| {
                        let _ = gio::AppInfo::launch_default_for_uri(
                            &result.url,
                            Option::<&gio::AppLaunchContext>::None,
                        );
                    });

                    wrapbox.append(&button);
                }
            }
        ));
    }

    fn wrapbox_buttons(&self) -> Vec<gtk::Button> {
        let wrapbox = self.imp().wrapbox.get();
        let mut buttons = Vec::new();
        let mut child = wrapbox.first_child();
        while let Some(widget) = child {
            if let Ok(button) = widget.clone().downcast::<gtk::Button>() {
                buttons.push(button);
            }
            child = widget.next_sibling();
        }
        buttons
    }

    pub fn button_count(&self) -> usize {
        self.wrapbox_buttons().len()
    }

    pub fn ensure_selection(&self) {
        if self.button_count() == 0 {
            return;
        }
        if self.imp().selected_index.get().is_none() {
            self.set_selection_index(0);
        }
    }

    pub fn clear_selection(&self) {
        let buttons = self.wrapbox_buttons();
        if let Some(index) = self.imp().selected_index.get()
            && let Some(button) = buttons.get(index)
        {
            crate::tv::set_tv_focused(button, false);
        }
        self.imp().selected_index.set(None);
    }

    pub fn move_selection(&self, delta: i32) {
        let count = self.button_count();
        if count == 0 {
            return;
        }
        let current = self.imp().selected_index.get().unwrap_or(0);
        let next = (current as i32 + delta).clamp(0, count as i32 - 1) as usize;
        self.set_selection_index(next);
    }

    fn set_selection_index(&self, index: usize) {
        let buttons = self.wrapbox_buttons();
        if buttons.is_empty() {
            return;
        }
        let index = index.min(buttons.len() - 1);
        let prev = self.imp().selected_index.get();
        if prev == Some(index) {
            return;
        }
        if let Some(prev_index) = prev
            && let Some(button) = buttons.get(prev_index)
        {
            crate::tv::set_tv_focused(button, false);
        }
        if let Some(button) = buttons.get(index) {
            crate::tv::set_tv_focused(button, true);
        }
        self.imp().selected_index.set(Some(index));
    }

    pub fn activate_selected(&self) {
        let buttons = self.wrapbox_buttons();
        if let Some(index) = self.imp().selected_index.get()
            && let Some(button) = buttons.get(index)
        {
            button.emit_clicked();
        }
    }

    pub fn scroll_into_parent_viewport(&self) {
        scroll_widget_to_row_center(self);
    }
}
