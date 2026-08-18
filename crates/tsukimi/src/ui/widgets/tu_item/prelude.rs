use std::cell::RefCell;

use crate::ui::provider::tu_item::TuItem;

pub trait TuItemBasic {
    fn item(&self) -> TuItem;
}

pub trait TuItemMenuPrelude: TuItemBasic {
    fn popover(&self) -> &RefCell<Option<gtk::PopoverMenu>>;
}
