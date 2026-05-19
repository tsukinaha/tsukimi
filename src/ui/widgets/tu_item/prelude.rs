use crate::ui::provider::tu_item::TuItem;

pub trait TuItemBasic {
    fn item(&self) -> TuItem;
}
