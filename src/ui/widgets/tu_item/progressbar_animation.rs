use crate::utils::spawn;
use adw::prelude::*;
use gtk::glib;

pub const PROGRESSBAR_ANIMATION_DURATION: u32 = 2000;

pub trait TuItemProgressbarAnimationPrelude {
    fn overlay(&self) -> gtk::Overlay;
}

pub trait TuItemProgressbarAnimation {
    fn set_progress(&self, progress: f64);
}

impl<T> TuItemProgressbarAnimation for T
where
    T: TuItemProgressbarAnimationPrelude,
{
    fn set_progress(&self, percentage: f64) {
        let progress = gtk::ProgressBar::builder()
            .fraction(0.)
            .margin_end(3)
            .margin_start(3)
            .valign(gtk::Align::End)
            .build();

        progress.add_css_class("pgb");

        self.overlay().add_overlay(&progress);

        spawn(glib::clone!(
            #[weak]
            progress,
            async move {
                let target = adw::CallbackAnimationTarget::new(glib::clone!(
                    #[weak]
                    progress,
                    move |process| progress.set_fraction(process)
                ));

                let animation = adw::TimedAnimation::builder()
                    .duration(PROGRESSBAR_ANIMATION_DURATION)
                    .widget(&progress)
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
