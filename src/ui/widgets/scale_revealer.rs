use adw::{prelude::*, subclass::prelude::*};
use gtk::{gdk, glib, glib::closure_local, graphene};
use tracing::warn;

const ANIMATION_DURATION: u32 = 250;

mod imp {
    use std::cell::{Cell, OnceCell, RefCell};

    use glib::{clone, subclass::Signal};
    use once_cell::sync::Lazy;

    use super::*;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::ScaleRevealer)]
    pub struct ScaleRevealer {
        /// Whether the child is revealed.
        #[property(get, set = Self::set_reveal_child, explicit_notify)]
        pub reveal_child: Cell<bool>,
        /// The source widget this revealer is transitioning from.
        #[property(get, set = Self::set_source_widget, explicit_notify, nullable)]
        pub source_widget: glib::WeakRef<gtk::Widget>,
        pub source_widget_texture: RefCell<Option<gdk::Texture>>,
        pub animation: OnceCell<adw::TimedAnimation>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ScaleRevealer {
        const NAME: &'static str = "ScaleRevealer";
        type Type = super::ScaleRevealer;
        type ParentType = adw::Bin;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ScaleRevealer {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("transition-done").build()]);
            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            let target = adw::CallbackAnimationTarget::new(clone!(
                @weak
                obj =>
                move |_| {
                    obj.queue_draw();
                }
            ));
            let animation = adw::TimedAnimation::new(&*obj, 0.0, 1.0, ANIMATION_DURATION, target);

            animation.set_easing(adw::Easing::EaseOutQuart);
            animation.connect_done(clone!(
                @weak
                obj =>
                move |_| {
                    let imp = obj.imp();

                    if !imp.reveal_child.get() {
                        if let Some(source_widget) = imp.source_widget.upgrade() {
                            // Show the original source widget now that the
                            // transition is over.
                            source_widget.set_opacity(1.0);
                        }
                        obj.set_visible(false);
                    }

                    obj.emit_by_name::<()>("transition-done", &[]);
                }
            ));

            self.animation.set(animation).unwrap();
            obj.set_visible(false);
        }
    }

    impl WidgetImpl for ScaleRevealer {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let obj = self.obj();
            let Some(child) = obj.child() else {
                return;
            };

            let progress = self.animation.get().unwrap().value();
            if progress == 1.0 {
                // The transition progress is at 100%, so just show the child
                obj.snapshot_child(&child, snapshot);
                return;
            }

            let source_bounds = self
                    .source_widget
                    .upgrade()
                    .and_then(|s| s.compute_bounds(&*obj))
                    .unwrap_or_else(|| {
                        warn!(
                            "The source widget bounds could not be calculated, using default bounds as fallback"
                        );
                        graphene::Rect::new(0.0, 0.0, 100.0, 100.0)
                    });
            let rev_progress = (1.0 - progress).abs();

            let x_scale = source_bounds.width() / obj.width() as f32;
            let y_scale = source_bounds.height() / obj.height() as f32;

            let x_scale = 1.0 + (x_scale - 1.0) * rev_progress as f32;
            let y_scale = 1.0 + (y_scale - 1.0) * rev_progress as f32;

            let x = source_bounds.x() * rev_progress as f32;
            let y = source_bounds.y() * rev_progress as f32;

            snapshot.translate(&graphene::Point::new(x, y));
            snapshot.scale(x_scale, y_scale);

            let source_widget_texture_ref = self.source_widget_texture.borrow();

            if let Some(source_widget_texture) = source_widget_texture_ref.as_ref() {
                if progress > 0.0 {
                    // We're in the middle of the cross fade transition, so...
                    // do the cross fade transition.
                    snapshot.push_cross_fade(progress);

                    source_widget_texture.snapshot(
                        snapshot,
                        obj.width() as f64,
                        obj.height() as f64,
                    );
                    snapshot.pop();

                    obj.snapshot_child(&child, snapshot);
                    snapshot.pop();
                } else if progress <= 0.0 {
                    source_widget_texture.snapshot(
                        snapshot,
                        obj.width() as f64,
                        obj.height() as f64,
                    );
                }
            } else {
                warn!("The source widget texture is None, using child snapshot as fallback");
                obj.snapshot_child(&child, snapshot);
            }
        }
    }

    impl BinImpl for ScaleRevealer {}

    impl ScaleRevealer {
        /// Set whether the child should be revealed or not.
        ///
        /// This will start the scale animation.
        fn set_reveal_child(&self, reveal_child: bool) {
            if self.reveal_child.get() == reveal_child {
                return;
            }
            let obj = self.obj();

            let animation = self.animation.get().unwrap();
            animation.set_value_from(animation.value());

            if reveal_child {
                animation.set_value_to(1.0);
                obj.set_visible(true);

                if let Some(source_widget) = self.source_widget.upgrade() {
                    // Render the current state of the source widget to a texture.
                    // This will be needed for the transition.
                    let texture = render_widget_to_texture(&source_widget);
                    self.source_widget_texture.replace(texture);

                    // Hide the source widget.
                    // We use opacity here so that the widget will stay allocated.
                    source_widget.set_opacity(0.0);
                } else {
                    self.source_widget_texture.replace(None);
                }
            } else {
                animation.set_value_to(0.0);
            }

            self.reveal_child.set(reveal_child);

            animation.play();

            obj.notify_reveal_child();
        }

        /// Set the source widget this revealer should transition from to show
        /// the child.
        fn set_source_widget(&self, source_widget: Option<&gtk::Widget>) {
            if self.source_widget.upgrade().as_ref() == source_widget {
                return;
            }

            self.source_widget.set(source_widget);
            self.obj().notify_source_widget();
        }
    }
}

glib::wrapper! {
    /// A widget to reveal a child with a scaling animation.
    pub struct ScaleRevealer(ObjectSubclass<imp::ScaleRevealer>)
        @extends gtk::Widget, adw::Bin;
}

impl ScaleRevealer {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn connect_transition_done<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_closure(
            "transition-done",
            true,
            closure_local!(move |obj: Self| {
                f(&obj);
            }),
        )
    }
}

impl Default for ScaleRevealer {
    fn default() -> Self {
        Self::new()
    }
}

fn render_widget_to_texture(widget: &impl IsA<gtk::Widget>) -> Option<gdk::Texture> {
    let widget_paintable = gtk::WidgetPaintable::new(Some(widget.as_ref()));
    let snapshot = gtk::Snapshot::new();

    widget_paintable.snapshot(
        &snapshot,
        widget_paintable.intrinsic_width() as f64,
        widget_paintable.intrinsic_height() as f64,
    );

    let node = snapshot.to_node()?;
    let native = widget.native()?;

    Some(native.renderer()?.render_texture(node, None))
}
