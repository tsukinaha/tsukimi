mod image_adw_dialog;
mod image_drop_row;
mod image_edit_dialog_page;
mod image_infocard;
mod search_page;

use gtk::prelude::*;
pub use image_adw_dialog::ImagesDialog as ImageDialog;
pub use image_drop_row::ImageDropRow;
pub use image_edit_dialog_page::ImageDialogEditPage;
pub use image_infocard::ImageInfoCard;
pub use search_page::ImageDialogSearchPage;

pub trait ImageDialogNavigtion {
    fn image_dialog(&self) -> Option<ImageDialog>;
}

impl<T> ImageDialogNavigtion for T
where
    T: IsA<gtk::Widget>,
{
    fn image_dialog(&self) -> Option<ImageDialog> {
        self.ancestor(ImageDialog::static_type())
            .and_then(|dialog| dialog.downcast::<ImageDialog>().ok())
    }
}
