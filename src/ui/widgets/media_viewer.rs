use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gettextrs::gettext;
use gtk::{
    CompositeTemplate,
    gdk,
    glib,
    glib::clone,
    graphene,
};

use crate::toast;

const ANIMATION_DURATION: u32 = 250;
const CANCEL_SWIPE_ANIMATION_DURATION: u32 = 400;

mod imp {
    use std::{
        cell::{
            Cell,
            OnceCell,
            RefCell,
        },
        collections::HashMap,
    };

    use glib::subclass::InitializingObject;

    use super::*;
    use crate::ui::widgets::{
        content_viewer::MediaContentViewer,
        scale_revealer::ScaleRevealer,
    };

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/media_viewer.ui")]
    #[properties(wrapper_type = super::MediaViewer)]
    pub struct MediaViewer {
        /// Whether the viewer is fullscreened.
        #[property(get, set = Self::set_fullscreened, explicit_notify)]
        pub fullscreened: Cell<bool>,
        /// The body of the media event.
        #[property(get)]
        pub body: RefCell<Option<String>>,
        pub animation: OnceCell<adw::TimedAnimation>,
        pub swipe_tracker: OnceCell<adw::SwipeTracker>,
        pub swipe_progress: Cell<f64>,
        #[template_child]
        pub toolbar_view: TemplateChild<adw::ToolbarView>,
        #[template_child]
        pub header_bar: TemplateChild<gtk::HeaderBar>,
        #[template_child]
        pub menu: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub revealer: TemplateChild<ScaleRevealer>,
        #[template_child]
        pub media: TemplateChild<MediaContentViewer>,
        pub actions_expression_watches: RefCell<HashMap<&'static str, gtk::ExpressionWatch>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MediaViewer {
        const NAME: &'static str = "MediaViewer";
        type Type = super::MediaViewer;
        type ParentType = gtk::Widget;
        type Interfaces = (adw::Swipeable,);

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            Self::Type::bind_template_callbacks(klass);

            klass.set_css_name("media-viewer");

            klass.install_action("media-viewer.close", None, |obj, _, _| {
                obj.close();
            });
            klass.add_binding_action(
                gdk::Key::Escape,
                gdk::ModifierType::empty(),
                "media-viewer.close",
            );

            // Menu actions
            klass.install_action("media-viewer.copy-image", None, |obj, _, _| {
                obj.copy_image();
            });
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for MediaViewer {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            let target = adw::CallbackAnimationTarget::new(clone!(
                #[weak]
                obj,
                move |value| {
                    // This is needed to fade the header bar content
                    obj.imp().header_bar.set_opacity(value);

                    obj.queue_draw();
                }
            ));
            let animation = adw::TimedAnimation::new(&*obj, 0.0, 1.0, ANIMATION_DURATION, target);
            self.animation.set(animation).unwrap();

            let swipe_tracker = adw::SwipeTracker::new(&*obj);
            swipe_tracker.set_orientation(gtk::Orientation::Vertical);
            swipe_tracker.connect_update_swipe(clone!(
                #[weak]
                obj,
                move |_, progress| {
                    obj.imp().header_bar.set_opacity(0.0);
                    obj.imp().swipe_progress.set(progress);
                    obj.queue_allocate();
                    obj.queue_draw();
                }
            ));
            swipe_tracker.connect_end_swipe(clone!(
                #[weak]
                obj,
                move |_, _, to| {
                    if to == 0.0 {
                        let target = adw::CallbackAnimationTarget::new(clone!(
                            #[weak]
                            obj,
                            move |value| {
                                obj.imp().swipe_progress.set(value);
                                obj.queue_allocate();
                                obj.queue_draw();
                            }
                        ));
                        let swipe_progress = obj.imp().swipe_progress.get();
                        let animation = adw::TimedAnimation::new(
                            &obj,
                            swipe_progress,
                            0.0,
                            CANCEL_SWIPE_ANIMATION_DURATION,
                            target,
                        );
                        animation.set_easing(adw::Easing::EaseOutCubic);
                        animation.connect_done(clone!(
                            #[weak]
                            obj,
                            move |_| {
                                obj.imp().header_bar.set_opacity(1.0);
                            }
                        ));
                        animation.play();
                    } else {
                        obj.close();
                        obj.imp().header_bar.set_opacity(1.0);
                    }
                }
            ));
            self.swipe_tracker.set(swipe_tracker).unwrap();

            // Bind `fullscreened` to the window property of the same name.
            obj.connect_root_notify(|obj| {
                if let Some(window) = obj.root().and_downcast::<gtk::Window>() {
                    window
                        .bind_property("fullscreened", obj, "fullscreened")
                        .sync_create()
                        .build();
                }
            });

            self.revealer.connect_transition_done(clone!(
                #[weak]
                obj,
                move |revealer| {
                    if !revealer.reveal_child() {
                        obj.set_visible(false);
                    }
                }
            ));
        }

        fn dispose(&self) {
            self.toolbar_view.unparent();

            for expr_watch in self.actions_expression_watches.take().values() {
                expr_watch.unwatch();
            }
        }
    }

    impl WidgetImpl for MediaViewer {
        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            let swipe_y_offset = -height as f64 * self.swipe_progress.get();
            let allocation = gtk::Allocation::new(0, swipe_y_offset as i32, width, height);
            self.toolbar_view.size_allocate(&allocation, baseline);
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let obj = self.obj();
            let progress = {
                let swipe_progress = 1.0 - self.swipe_progress.get().abs();
                let animation_progress = self.animation.get().unwrap().value();
                swipe_progress.min(animation_progress)
            };

            if progress > 0.0 {
                let background_color = gdk::RGBA::new(0.0, 0.0, 0.0, 1.0 * progress as f32);
                let bounds = graphene::Rect::new(0.0, 0.0, obj.width() as f32, obj.height() as f32);
                snapshot.append_color(&background_color, &bounds);
            }

            obj.snapshot_child(&*self.toolbar_view, snapshot);
        }
    }

    impl SwipeableImpl for MediaViewer {
        fn cancel_progress(&self) -> f64 {
            0.0
        }

        fn distance(&self) -> f64 {
            self.obj().height() as f64
        }

        fn progress(&self) -> f64 {
            self.swipe_progress.get()
        }

        fn snap_points(&self) -> Vec<f64> {
            vec![-1.0, 0.0, 1.0]
        }

        fn swipe_area(&self, _: adw::NavigationDirection, _: bool) -> gdk::Rectangle {
            gdk::Rectangle::new(0, 0, self.obj().width(), self.obj().height())
        }
    }

    impl MediaViewer {
        /// Set whether the viewer is fullscreened.
        fn set_fullscreened(&self, fullscreened: bool) {
            if fullscreened == self.fullscreened.get() {
                return;
            }

            self.fullscreened.set(fullscreened);

            if fullscreened {
                // Upscale the media on fullscreen
                self.media.set_halign(gtk::Align::Fill);
                self.toolbar_view
                    .set_top_bar_style(adw::ToolbarStyle::Raised);
            } else {
                self.media.set_halign(gtk::Align::Center);
                self.toolbar_view.set_top_bar_style(adw::ToolbarStyle::Flat);
            }

            self.obj().notify_fullscreened();
        }
    }
}

glib::wrapper! {
    /// A widget allowing to view a media file.
    pub struct MediaViewer(ObjectSubclass<imp::MediaViewer>)
        @extends gtk::Widget, @implements gtk::Accessible, adw::Swipeable;
}

impl Default for MediaViewer {
    fn default() -> Self {
        Self::new()
    }
}

#[gtk::template_callbacks]
impl MediaViewer {
    pub fn new() -> Self {
        glib::Object::new()
    }

    /// Reveal this widget by transitioning from `source_widget`.
    pub fn reveal(&self, source_widget: &impl IsA<gtk::Widget>) {
        let imp = self.imp();

        self.set_visible(true);
        imp.menu.grab_focus();

        imp.swipe_progress.set(0.0);
        imp.revealer
            .set_source_widget(Some(source_widget.upcast_ref()));
        imp.revealer.set_reveal_child(true);

        let animation = imp.animation.get().unwrap();
        animation.set_value_from(animation.value());
        animation.set_value_to(1.0);
        animation.play();
    }

    fn close(&self) {
        if self.fullscreened() {
            self.activate_action("win.toggle-fullscreen", None).unwrap();
        }

        self.imp().media.stop_playback();
        self.imp().revealer.set_reveal_child(false);

        let animation = self.imp().animation.get().unwrap();

        animation.set_value_from(animation.value());
        animation.set_value_to(0.0);
        animation.play();
    }

    pub fn view_image(&self, image: gtk::gdk::Paintable) {
        self.imp().media.view_image(&image);
    }

    fn reveal_headerbar(&self, reveal: bool) {
        if self.fullscreened() {
            self.imp().toolbar_view.set_reveal_top_bars(reveal);
        }
    }

    #[template_callback]
    fn handle_motion(&self, _x: f64, y: f64) {
        if y <= 50.0 {
            self.reveal_headerbar(true);
        }
    }

    #[template_callback]
    fn handle_click(&self, n_pressed: i32) {
        if n_pressed == 1 {
            self.close();
        } else if n_pressed == 2 {
            self.activate_action("win.toggle-fullscreen", None).unwrap();
        }
    }

    /// Copy the current image to the clipboard.
    fn copy_image(&self) {
        let Some(texture) = self.imp().media.texture() else {
            return;
        };
        self.clipboard().set_texture(&texture);
        toast!(self, gettext("Image copied to clipboard"));
    }
}
