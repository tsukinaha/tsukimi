use adw::prelude::*;
use gtk::{
    glib,
    subclass::prelude::*,
};

use super::player::MutsumiVideoPlayer;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct MutsumiVideoLayout;

    #[glib::object_subclass]
    impl ObjectSubclass for MutsumiVideoLayout {
        const NAME: &'static str = "MutsumiVideoLayout";
        type Type = super::MutsumiVideoLayout;
        type ParentType = gtk::LayoutManager;
    }

    impl ObjectImpl for MutsumiVideoLayout {}

    impl MutsumiVideoLayout {
        fn player(&self) -> Option<MutsumiVideoPlayer> {
            self.obj().widget()?.downcast().ok()
        }
    }

    impl LayoutManagerImpl for MutsumiVideoLayout {
        fn measure(
            &self, _widget: &gtk::Widget, orientation: gtk::Orientation, for_size: i32,
        ) -> (i32, i32, i32, i32) {
            self.player()
                .and_then(|player| player.child())
                .map_or((0, 0, -1, -1), |child| child.measure(orientation, for_size))
        }

        fn allocate(&self, _widget: &gtk::Widget, width: i32, height: i32, baseline: i32) {
            let Some(player) = self.player() else {
                return;
            };

            if let Some(child) = player.child() {
                child.allocate(width, height, baseline, None);
            }
            player.update_viewport(width, height);
        }
    }
}

glib::wrapper! {
    pub struct MutsumiVideoLayout(ObjectSubclass<imp::MutsumiVideoLayout>)
        @extends gtk::LayoutManager;
}
