use std::cell::{
    Cell,
    OnceCell,
};

use adw::prelude::*;
use gtk::glib;

const SHOW_BUTTON_ANIMATION_DURATION: u32 = 500;
const SCROLL_STEP: f64 = 800.0;
const SCROLL_ANIMATION_DURATION_US: i64 = 1000 * 400;
const BUTTON_OPACITY: f64 = 0.7;

pub(crate) trait HorControlsExt:
    IsA<gtk::Widget> + glib::clone::Downgrade + 'static
where
    <Self as glib::clone::Downgrade>::Weak: glib::clone::Upgrade<Strong = Self>,
{
    fn scroll_widget(&self) -> gtk::ScrolledWindow;
    fn left_button(&self) -> gtk::Button;
    fn right_button(&self) -> gtk::Button;
    fn show_left_animation_cell(&self) -> &OnceCell<adw::TimedAnimation>;
    fn hide_left_animation_cell(&self) -> &OnceCell<adw::TimedAnimation>;
    fn show_right_animation_cell(&self) -> &OnceCell<adw::TimedAnimation>;
    fn hide_right_animation_cell(&self) -> &OnceCell<adw::TimedAnimation>;
    fn is_hovering(&self) -> &Cell<bool>;

    fn connect_scroll_controls(&self) {
        let adj = self.scroll_widget().hadjustment();
        adj.connect_value_changed(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |_| {
                obj.update_left_button(true);
                obj.update_right_button(true);
            }
        ));
        adj.connect_upper_notify(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |_| {
                obj.update_left_button(false);
                obj.update_right_button(false);
            }
        ));
        adj.connect_page_size_notify(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |_| {
                obj.update_left_button(false);
                obj.update_right_button(false);
            }
        ));
    }

    fn set_left_opacity(&self, opacity: f64) {
        let button = self.left_button();
        button.set_visible(opacity > 0.);
        button.set_opacity(opacity);
    }

    fn set_right_opacity(&self, opacity: f64) {
        let button = self.right_button();
        button.set_visible(opacity > 0.);
        button.set_opacity(opacity);
    }

    fn left_opacity(&self) -> f64 {
        self.left_button().opacity()
    }

    fn right_opacity(&self) -> f64 {
        self.right_button().opacity()
    }

    fn is_at_lower(&self) -> bool {
        let adj = self.scroll_widget().hadjustment();
        adj.value() <= adj.lower() + f64::EPSILON
    }

    fn is_at_upper(&self) -> bool {
        let adj = self.scroll_widget().hadjustment();
        let max_value = (adj.upper() - adj.page_size()).max(adj.lower());
        adj.value() >= max_value - f64::EPSILON
    }

    fn show_left_animation(&self) -> &adw::TimedAnimation {
        self.show_left_animation_cell().get_or_init(|| {
            let target = adw::CallbackAnimationTarget::new(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |opacity| obj.set_left_opacity(opacity)
            ));

            adw::TimedAnimation::builder()
                .duration(SHOW_BUTTON_ANIMATION_DURATION)
                .widget(&self.scroll_widget())
                .target(&target)
                .value_to(BUTTON_OPACITY)
                .build()
        })
    }

    fn hide_left_animation(&self) -> &adw::TimedAnimation {
        self.hide_left_animation_cell().get_or_init(|| {
            let target = adw::CallbackAnimationTarget::new(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |opacity| obj.set_left_opacity(opacity)
            ));

            adw::TimedAnimation::builder()
                .duration(SHOW_BUTTON_ANIMATION_DURATION)
                .widget(&self.scroll_widget())
                .target(&target)
                .value_to(0.)
                .build()
        })
    }

    fn show_right_animation(&self) -> &adw::TimedAnimation {
        self.show_right_animation_cell().get_or_init(|| {
            let target = adw::CallbackAnimationTarget::new(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |opacity| obj.set_right_opacity(opacity)
            ));

            adw::TimedAnimation::builder()
                .duration(SHOW_BUTTON_ANIMATION_DURATION)
                .widget(&self.scroll_widget())
                .target(&target)
                .value_to(BUTTON_OPACITY)
                .build()
        })
    }

    fn hide_right_animation(&self) -> &adw::TimedAnimation {
        self.hide_right_animation_cell().get_or_init(|| {
            let target = adw::CallbackAnimationTarget::new(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |opacity| obj.set_right_opacity(opacity)
            ));

            adw::TimedAnimation::builder()
                .duration(SHOW_BUTTON_ANIMATION_DURATION)
                .widget(&self.scroll_widget())
                .target(&target)
                .value_to(0.)
                .build()
        })
    }

    fn update_left_button(&self, animate: bool) {
        let should_show = self.is_hovering().get() && !self.is_at_lower();
        let current = self.left_opacity();
        if should_show && current < BUTTON_OPACITY {
            self.hide_left_animation().pause();
            self.show_left_animation().set_value_from(current);
            self.show_left_animation().play();
        } else if !should_show && current > 0. {
            if animate {
                self.show_left_animation().pause();
                self.hide_left_animation().set_value_from(current);
                self.hide_left_animation().play();
            } else {
                self.show_left_animation().pause();
                self.set_left_opacity(0.);
            }
        }
    }

    fn update_right_button(&self, animate: bool) {
        let should_show = self.is_hovering().get() && !self.is_at_upper();
        let current = self.right_opacity();
        if should_show && current < BUTTON_OPACITY {
            self.hide_right_animation().pause();
            self.show_right_animation().set_value_from(current);
            self.show_right_animation().play();
        } else if !should_show && current > 0. {
            if animate {
                self.show_right_animation().pause();
                self.hide_right_animation().set_value_from(current);
                self.hide_right_animation().play();
            } else {
                self.show_right_animation().pause();
                self.set_right_opacity(0.);
            }
        }
    }

    fn on_enter_scroll_controls(&self) {
        self.is_hovering().set(true);
        self.update_left_button(true);
        self.update_right_button(true);
    }

    fn on_leave_scroll_controls(&self) {
        self.is_hovering().set(false);
        self.update_left_button(true);
        self.update_right_button(true);
    }

    fn scroll_controls_anime<const R: bool>(&self) {
        let scrolled = self.scroll_widget();
        let adj = scrolled.hadjustment();

        let Some(clock) = scrolled.frame_clock() else {
            return;
        };

        let start = adj.value();
        let end = if R {
            start + SCROLL_STEP
        } else {
            start - SCROLL_STEP
        };

        let start_time = clock.frame_time();
        let end_time = start_time + SCROLL_ANIMATION_DURATION_US;

        scrolled.add_tick_callback(move |_view, clock| {
            let now = clock.frame_time();
            if now < end_time && adj.value() != end {
                let mut t = (now - start_time) as f64 / (end_time - start_time) as f64;
                t = Self::ease_in_out_cubic(t);
                adj.set_value(start + t * (end - start));
                glib::ControlFlow::Continue
            } else {
                adj.set_value(end);
                glib::ControlFlow::Break
            }
        });
    }

    fn ease_in_out_cubic(t: f64) -> f64 {
        if t < 0.5 {
            4.0 * t * t * t
        } else {
            let t = 2.0 * t - 2.0;
            0.5 * t * t * t + 1.0
        }
    }
}
