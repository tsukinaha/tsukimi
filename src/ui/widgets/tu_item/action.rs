use crate::{
    client::{
        emby_client::EMBY_CLIENT,
        error::UserFacingError,
    },
    toast,
    ui::provider::IS_ADMIN,
    utils::{
        spawn,
        spawn_tokio,
    },
};

use super::prelude::TuItemMenuPrelude;
use gettextrs::gettext;
use gtk::{
    gdk::Rectangle,
    gio::{
        self,
        MenuModel,
    },
    glib,
    prelude::*,
    Builder,
    PopoverMenu,
};

use super::TuItemBasic;
use anyhow::Result;

pub enum Action {
    Like,
    Unlike,
    Played,
    Unplayed,
    Remove,
}

pub trait TuItemAction {
    async fn perform_action_inner(id: &str, action: &Action) -> Result<()>;

    async fn perform_action(&self, action: Action);

    fn update_state(&self, action: &Action);

    async fn process_item(&self, action: fn(&String) -> Result<(), Box<dyn std::error::Error>>);

    fn gesture(&self);

    fn set_action(&self) -> Option<gio::SimpleActionGroup>;

    fn set_item_action(
        &self, is_playable: bool, is_editable: bool, is_favouritable: bool,
    ) -> Option<gio::SimpleActionGroup>;
}

impl<T> TuItemAction for T
where
    T: TuItemBasic + TuItemMenuPrelude + IsA<gtk::Widget> + glib::clone::Downgrade,
    <T as glib::clone::Downgrade>::Weak: glib::clone::Upgrade<Strong = T>,
{
    async fn perform_action_inner(id: &str, action: &Action) -> Result<()> {
        match action {
            Action::Like => EMBY_CLIENT.like(id).await,
            Action::Unlike => EMBY_CLIENT.unlike(id).await,
            Action::Played => EMBY_CLIENT.set_as_played(id).await,
            Action::Unplayed => EMBY_CLIENT.set_as_unplayed(id).await,
            Action::Remove => EMBY_CLIENT.hide_from_resume(id).await,
        }
    }

    async fn perform_action(&self, action: Action) {
        let id = self.item().id().clone();
        self.update_state(&action);
        let result = spawn_tokio(async move { Self::perform_action_inner(&id, &action).await });

        match result.await {
            Ok(_) => {
                toast!(self, gettext("Success"))
            }
            Err(e) => {
                toast!(self, e.to_user_facing());
            }
        }

        self.insert_action_group("item", self.set_action().as_ref());
    }

    fn update_state(&self, action: &Action) {
        match action {
            Action::Like => self.item().set_is_favorite(true),
            Action::Unlike => self.item().set_is_favorite(false),
            Action::Played => self.item().set_played(true),
            Action::Unplayed => self.item().set_played(false),
            Action::Remove => {
                spawn(glib::clone!(
                    #[weak(rename_to = obj)]
                    self,
                    async move {
                        let Some(parent) = obj.parent().and_then(|p| p.parent()) else {
                            return;
                        };
                        if let Some(list_view) = parent.downcast_ref::<gtk::ListView>() {
                            list_view
                                .model()
                                .and_downcast::<gtk::SingleSelection>()
                                .map(|sel| {
                                    sel.model()
                                        .and_downcast::<gio::ListStore>()
                                        .map(|store| store.remove(sel.selected()))
                                });
                        } else if let Some(grid_view) = parent.downcast_ref::<gtk::GridView>() {
                            grid_view
                                .model()
                                .and_downcast::<gtk::SingleSelection>()
                                .map(|sel| {
                                    sel.model()
                                        .and_downcast::<gio::ListStore>()
                                        .map(|store| store.remove(sel.selected()))
                                });
                        }
                    }
                ));
            }
        }
        self.gesture();
    }

    async fn process_item(&self, action: fn(&String) -> Result<(), Box<dyn std::error::Error>>) {
        let id = self.item().id();
        spawn_tokio(async move {
            action(&id).unwrap();
        })
        .await;
        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                toast!(obj, gettext("Success"));
            }
        ));
    }

    fn gesture(&self) {
        let builder = Builder::from_resource("/moe/tsuna/tsukimi/ui/pop-menu.ui");
        let menu = builder.object::<MenuModel>("rightmenu");
        match menu {
            Some(popover) => {
                let new_popover = PopoverMenu::builder()
                    .menu_model(&popover)
                    .halign(gtk::Align::Start)
                    .has_arrow(false)
                    .build();
                if let Some(popover) = self.popover().borrow_mut().take() {
                    popover.unparent();
                }
                new_popover.set_parent(self);
                self.popover().replace(Some(new_popover));
            }
            None => eprintln!("Failed to load popover"),
        }
        let gesture = gtk::GestureClick::new();
        gesture.set_button(3);
        gesture.connect_released(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |gesture, _n, x, y| {
                gesture.set_state(gtk::EventSequenceState::Claimed);
                obj.insert_action_group("item", obj.set_action().as_ref());
                if let Some(popover) = obj.popover().borrow().as_ref() {
                    popover.set_pointing_to(Some(&Rectangle::new(x as i32, y as i32, 0, 0)));
                    popover.popup();
                };
            }
        ));

        self.add_controller(gesture);
    }

    fn set_action(&self) -> Option<gio::SimpleActionGroup> {
        let item_type = self.item().item_type();
        match item_type.as_str() {
            "Movie" | "Series" | "Episode" | "MusicVideo" | "AdultVideo" | "Audio" => {
                self.set_item_action(true, true, true)
            }
            "MusicAlbum" | "BoxSet" | "Tag" | "Genre" | "Views" | "Person" | "Actor"
            | "Director" | "Writer" | "Producer" | "TvChannel" | "Folder" | "Season" => {
                self.set_item_action(false, true, true)
            }
            "CollectionFolder" | "UserView" => self.set_item_action(false, false, false),
            _ => None,
        }
    }

    fn set_item_action(
        &self, is_playable: bool, is_editable: bool, is_favouritable: bool,
    ) -> Option<gio::SimpleActionGroup> {
        let action_group = gio::SimpleActionGroup::new();

        if is_editable {
            action_group.add_action_entries([gio::ActionEntry::builder("editm")
                .activate(glib::clone!(
                    #[weak(rename_to = obj)]
                    self,
                    move |_, _, _| {
                        spawn(glib::clone!(
                            #[weak]
                            obj,
                            async move {
                                let id = obj.item().id();
                                let dialog =
                                    crate::ui::widgets::metadata_dialog::MetadataDialog::new(&id);
                                crate::insert_editm_dialog!(obj, dialog);
                            }
                        ))
                    }
                ))
                .build()]);

            action_group.add_action_entries([gio::ActionEntry::builder("editi")
                .activate(glib::clone!(
                    #[weak(rename_to = obj)]
                    self,
                    move |_, _, _| {
                        spawn(glib::clone!(
                            #[weak]
                            obj,
                            async move {
                                let id = obj.item().id();
                                let dialog =
                                    crate::ui::widgets::image_dialog::ImageDialog::new(&id);
                                crate::insert_editm_dialog!(obj, dialog);
                            }
                        ))
                    }
                ))
                .build()]);
        }

        if IS_ADMIN.load(std::sync::atomic::Ordering::Relaxed) {
            action_group.add_action_entries([gio::ActionEntry::builder("scan")
                .activate(glib::clone!(
                    #[weak(rename_to = obj)]
                    self,
                    move |_, _, _| {
                        spawn(glib::clone!(
                            #[weak]
                            obj,
                            async move {
                                let id = obj.item().id();
                                match spawn_tokio(async move { EMBY_CLIENT.scan(&id).await }).await
                                {
                                    Ok(_) => {
                                        toast!(obj, gettext("Scanning..."));
                                    }
                                    Err(e) => {
                                        toast!(obj, e.to_user_facing());
                                    }
                                }
                            }
                        ))
                    }
                ))
                .build()]);

            if is_editable && !self.item().is_resume() {
                action_group.add_action_entries([gio::ActionEntry::builder("identify")
                    .activate(glib::clone!(
                        #[weak(rename_to = obj)]
                        self,
                        move |_, _, _| {
                            spawn(glib::clone!(
                                #[weak]
                                obj,
                                async move {
                                    let id = obj.item().id();
                                    let type_ = obj.item().item_type();
                                    let dialog =
                                        crate::ui::widgets::identify_dialog::IdentifyDialog::new(
                                            &id, &type_,
                                        );
                                    crate::insert_editm_dialog!(obj, dialog);
                                }
                            ))
                        }
                    ))
                    .build()]);

                action_group.add_action_entries([gio::ActionEntry::builder("refresh")
                    .activate(glib::clone!(
                        #[weak(rename_to = obj)]
                        self,
                        move |_, _, _| {
                            spawn(glib::clone!(
                                #[weak]
                                obj,
                                async move {
                                    let id = obj.item().id();
                                    let dialog =
                                        crate::ui::widgets::refresh_dialog::RefreshDialog::new(&id);
                                    crate::insert_editm_dialog!(obj, dialog);
                                }
                            ))
                        }
                    ))
                    .build()]);
            }
        }

        if is_favouritable {
            match self.item().is_favorite() {
                true => action_group.add_action_entries([gio::ActionEntry::builder("unlike")
                    .activate(glib::clone!(
                        #[weak(rename_to = obj)]
                        self,
                        move |_, _, _| {
                            spawn(glib::clone!(
                                #[weak]
                                obj,
                                async move {
                                    obj.perform_action(Action::Unlike).await;
                                }
                            ))
                        }
                    ))
                    .build()]),
                false => action_group.add_action_entries([gio::ActionEntry::builder("like")
                    .activate(glib::clone!(
                        #[weak(rename_to = obj)]
                        self,
                        move |_, _, _| {
                            spawn(glib::clone!(
                                #[weak]
                                obj,
                                async move {
                                    obj.perform_action(Action::Like).await;
                                }
                            ))
                        }
                    ))
                    .build()]),
            }
        }

        if is_playable {
            match self.item().played() {
                true => action_group.add_action_entries([gio::ActionEntry::builder("unplayed")
                    .activate(glib::clone!(
                        #[weak(rename_to = obj)]
                        self,
                        move |_, _, _| {
                            spawn(glib::clone!(
                                #[weak]
                                obj,
                                async move {
                                    obj.perform_action(Action::Unplayed).await;
                                }
                            ))
                        }
                    ))
                    .build()]),
                false => action_group.add_action_entries([gio::ActionEntry::builder("played")
                    .activate(glib::clone!(
                        #[weak(rename_to = obj)]
                        self,
                        move |_, _, _| {
                            spawn(glib::clone!(
                                #[weak]
                                obj,
                                async move {
                                    obj.perform_action(Action::Played).await;
                                }
                            ))
                        }
                    ))
                    .build()]),
            }
        }

        if self.item().is_resume() {
            action_group.add_action_entries([gio::ActionEntry::builder("remove")
                .activate(glib::clone!(
                    #[weak(rename_to = obj)]
                    self,
                    move |_, _, _| {
                        spawn(glib::clone!(
                            #[weak]
                            obj,
                            async move {
                                obj.perform_action(Action::Remove).await;
                            }
                        ))
                    }
                ))
                .build()]);
        }
        Some(action_group)
    }
}
