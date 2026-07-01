use std::{
    cell::RefCell,
    rc::Rc,
};

use adw::prelude::*;
use gtk::glib;

pub const PROGRESSBAR_ANIMATION_DURATION: u32 = 2000;

pub trait TuItemProgressbarAnimationPrelude {
    fn progress_bar(&self) -> gtk::ProgressBar;
}

pub trait TuItemProgressbarAnimation {
    fn set_progress(&self, progress: f64);
    fn clear_progress(&self);
}

impl<T> TuItemProgressbarAnimation for T
where
    T: TuItemProgressbarAnimationPrelude,
{
    fn set_progress(&self, percentage: f64) {
        let bar = self.progress_bar();
        if percentage <= 0.0 {
            self.clear_progress();
            return;
        }
        bar.set_visible(true);

        // If the progress bar is already mapped, we can play the animation immediately. Otherwise, we need to wait until it is mapped.
        // This is useful when the progress bar is not yet visible, for example when the app is first launched.
        if bar.is_mapped() {
            play_progress_animation(&bar, percentage);
        } else {
            bar.set_fraction(0.);
            play_progress_when_mapped(&bar, percentage);
        }
    }

    fn clear_progress(&self) {
        let bar = self.progress_bar();
        bar.set_fraction(0.);
        bar.set_visible(false);
    }
}

fn play_progress_when_mapped(bar: &gtk::ProgressBar, percentage: f64) {
    let handler_id = Rc::new(RefCell::new(None));
    let handler_id_clone = handler_id.clone();

    let id = bar.connect_map(move |bar| {
        if let Some(id) = handler_id_clone.borrow_mut().take() {
            bar.disconnect(id);
        }
        play_progress_animation(bar, percentage);
    });

    *handler_id.borrow_mut() = Some(id);
}

fn play_progress_animation(bar: &gtk::ProgressBar, percentage: f64) {
    let target = adw::CallbackAnimationTarget::new(glib::clone!(
        #[weak]
        bar,
        move |v| bar.set_fraction(v)
    ));

    let animation = adw::TimedAnimation::builder()
        .duration(PROGRESSBAR_ANIMATION_DURATION)
        .widget(bar)
        .target(&target)
        .easing(adw::Easing::EaseOutQuart)
        .value_from(bar.fraction())
        .value_to(percentage / 100.0)
        .build();

    animation.play();
}
