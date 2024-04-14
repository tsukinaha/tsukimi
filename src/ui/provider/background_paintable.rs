use gtk::{gdk, gio::File, glib, graphene, prelude::*, subclass::prelude::*};

mod imp {
    use std::{cell::RefCell, rc::Rc};

    use crate::ui::models::SETTINGS;

    use super::*;

    #[derive(Default)]
    pub struct BackgroundPaintable {
        pub pic: RefCell<Option<File>>,
        texture: Rc<RefCell<Option<gdk::Texture>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BackgroundPaintable {
        const NAME: &'static str = "BackgroundPaintable";
        type Type = super::BackgroundPaintable;
        type Interfaces = (gdk::Paintable,);
    }

    impl ObjectImpl for BackgroundPaintable {}
    impl PaintableImpl for BackgroundPaintable {
        fn snapshot(&self, snapshot: &gdk::Snapshot, width: f64, height: f64) {
            if let Some(file) = self.pic.borrow().as_ref() {
                let texture = self.texture.borrow();
                let texture = if texture.is_none() || self.pic.borrow().as_ref() != Some(file) {
                    drop(texture);
                    let new_texture =
                        gdk::Texture::from_file(file).expect("Failed to create texture from file");
                    *self.texture.borrow_mut() = Some(new_texture.clone());
                    new_texture
                } else {
                    texture.as_ref().unwrap().clone()
                };
                let texture_width = texture.width() as f64;
                let texture_height = texture.height() as f64;

                let scale_x = width / texture_width;
                let scale_y = height / texture_height;

                let scale = scale_x.max(scale_y);

                let new_width = texture_width * scale;
                let new_height = texture_height * scale;

                let dx = (width - new_width) / 2.0;
                let dy = (height - new_height) / 2.0;

                let rect =
                    graphene::Rect::new(dx as f32, dy as f32, new_width as f32, new_height as f32);
                snapshot.push_blur(SETTINGS.pic_blur() as f64);
                snapshot.append_texture(&texture, &rect);
                snapshot.pop();
            }
        }
    }
}

glib::wrapper! {
    pub struct BackgroundPaintable(ObjectSubclass<imp::BackgroundPaintable>)
        @implements gdk::Paintable;
}

impl BackgroundPaintable {
    pub fn set_pic(&self, pic: File) {
        self.imp().pic.replace(Some(pic));
        self.invalidate_contents();
    }
}

impl Default for BackgroundPaintable {
    fn default() -> Self {
        glib::Object::new()
    }
}
