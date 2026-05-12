use crate::utils::spawn;
use adw::prelude::*;
use gtk::glib;

pub const PROGRESSBAR_ANIMATION_DURATION: u32 = 2000;

pub trait TuItemProgressbarAnimationPrelude {
    fn progress_bar(&self) -> gtk::ProgressBar;
}

pub trait TuItemProgressbarAnimation {
    fn set_progress(&self, progress: f64);
}

impl<T> TuItemProgressbarAnimation for T
where
    T: TuItemProgressbarAnimationPrelude,
{
    fn set_progress(&self, percentage: f64) {
        let bar = self.progress_bar();
        bar.set_visible(true);

        spawn(glib::clone!(
            #[weak]
            bar,
            async move {
                let target = adw::CallbackAnimationTarget::new(glib::clone!(
                    #[weak]
                    bar,
                    move |v| bar.set_fraction(v)
                ));

                let animation = adw::TimedAnimation::builder()
                    .duration(PROGRESSBAR_ANIMATION_DURATION)
                    .widget(&bar)
                    .target(&target)
                    .easing(adw::Easing::EaseOutQuart)
                    .value_from(0.)
                    .value_to(percentage / 100.0)
                    .build();

                glib::timeout_future_seconds(1).await;
                animation.play();
            }
        ));
    }
}
