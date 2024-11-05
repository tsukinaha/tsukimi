use derive_builder::Builder;
use gtk::{
    glib,
    prelude::*,
};

#[derive(Builder, Default, Clone, PartialEq)]
#[builder(default)]
pub struct DropdownList {
    pub line1: Option<String>,
    pub line2: Option<String>,
    pub index: Option<u64>,
    pub id: Option<String>,
    pub direct_url: Option<String>,
    pub transcoding_url: Option<String>,
    pub is_external: Option<bool>,
}

pub fn factory<const UPBIND: bool>() -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_bind(move |_, item| {
        let list_item = item.downcast_ref::<gtk::ListItem>().expect("Needs to be ListItem");

        if list_item.child().is_some() && !UPBIND {
            return;
        }

        if let Some(entry) = item
            .downcast_ref::<gtk::ListItem>()
            .expect("Needs to be ListItem")
            .item()
            .and_downcast::<glib::BoxedAnyObject>()
        {
            let dl: std::cell::Ref<DropdownList> = entry.borrow();

            let list_dropdown = crate::ui::widgets::list_dropdown::ListDropdown::new();

            list_dropdown.set_label1(&dl.line1);
            list_dropdown.set_tooltip_text(dl.line1.as_deref());

            if !UPBIND {
                list_dropdown.set_label2(&dl.line2);
            }

            list_item.set_child(Some(&list_dropdown));
        }
    });
    factory
}
