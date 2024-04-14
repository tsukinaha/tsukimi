use gtk::pango;
use gtk::prelude::*;

pub fn factory() -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(move |_, item| {
        let list_item = item
            .downcast_ref::<gtk::ListItem>()
            .expect("Needs to be ListItem");
        let label = gtk::Label::builder()
            .ellipsize(pango::EllipsizeMode::End)
            .halign(gtk::Align::Start)
            .build();
        list_item.set_child(Some(&label));
    });
    factory.connect_bind(move |_, item| {
        let label = item
            .downcast_ref::<gtk::ListItem>()
            .expect("Needs to be ListItem")
            .child()
            .and_downcast::<gtk::Label>()
            .expect("need to be label");
        let string = item
            .downcast_ref::<gtk::ListItem>()
            .expect("Needs to be ListItem")
            .item()
            .and_downcast::<gtk::StringObject>()
            .expect("need to be string")
            .string();
        label.set_label(&string);
    });
    factory
}
