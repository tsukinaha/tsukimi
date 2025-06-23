use crate::{
    alert_dialog,
    client::{
        error::UserFacingError,
        jellyfin_client::JELLYFIN_CLIENT,
    },
    ui::{
        provider::IS_ADMIN,
        widgets::{
            missing_episodes_dialog::MissingEpisodesDialog,
            window::Window,
        },
    },
    utils::{
        spawn,
        spawn_tokio,
    },
};

use crate::ui::GlobalToast;

use super::prelude::TuItemMenuPrelude;
use adw::prelude::AlertDialogExt;
use gettextrs::gettext;
use gtk::{
    Builder,
    PopoverMenu,
    gdk::Rectangle,
    gio::{
        self,
        MenuModel,
    },
    glib,
    prelude::*,
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

    fn gesture(&self);

    fn set_action(&self) -> Option<gio::SimpleActionGroup>;

    fn set_item_action(
        &self, is_playable: bool, is_editable: bool, is_favouritable: bool,
    ) -> Option<gio::SimpleActionGroup>;

    async fn delete_item(&self);

    async fn view_missing_episodes(&self);

    async fn remove_identification(&self);
}

impl<T> TuItemAction for T
where
    T: TuItemBasic + TuItemMenuPrelude + IsA<gtk::Widget> + glib::clone::Downgrade,
    <T as glib::clone::Downgrade>::Weak: glib::clone::Upgrade<Strong = T>,
{
    async fn perform_action_inner(id: &str, action: &Action) -> Result<()> {
        match action {
            Action::Like => JELLYFIN_CLIENT.like(id).await,
            Action::Unlike => JELLYFIN_CLIENT.unlike(id).await,
            Action::Played => JELLYFIN_CLIENT.set_as_played(id).await,
            Action::Unplayed => JELLYFIN_CLIENT.set_as_unplayed(id).await,
            Action::Remove => JELLYFIN_CLIENT.hide_from_resume(id).await,
        }
    }

    async fn perform_action(&self, action: Action) {
        let id = self.item().id().to_owned();
        self.update_state(&action);
        let result = spawn_tokio(async move { Self::perform_action_inner(&id, &action).await });

        match result.await {
            Ok(_) => self.toast(gettext("Success")),
            Err(e) => {
                self.toast(e.to_user_facing());
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
            | "Director" | "Writer" | "Producer" | "GuestStar" | "TvChannel" | "Folder"
            | "Season" => self.set_item_action(false, true, true),
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
                                match spawn_tokio(async move { JELLYFIN_CLIENT.scan(&id).await })
                                    .await
                                {
                                    Ok(_) => {
                                        obj.toast(gettext("Scanning..."));
                                    }
                                    Err(e) => {
                                        obj.toast(e.to_user_facing());
                                    }
                                }
                            }
                        ))
                    }
                ))
                .build()]);

            if is_playable && is_editable && !self.item().is_resume() {
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
                                    let dialog = crate::ui::widgets::identify::IdentifyDialog::new(
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

                action_group.add_action_entries([gio::ActionEntry::builder("delete")
                    .activate(glib::clone!(
                        #[weak(rename_to = obj)]
                        self,
                        move |_, _, _| {
                            spawn(glib::clone!(
                                #[weak]
                                obj,
                                async move {
                                    obj.delete_item().await;
                                }
                            ))
                        }
                    ))
                    .build()]);

                action_group.add_action_entries([gio::ActionEntry::builder(
                    "remove-identification",
                )
                .activate(glib::clone!(
                    #[weak(rename_to = obj)]
                    self,
                    move |_, _, _| {
                        spawn(glib::clone!(
                            #[weak]
                            obj,
                            async move {
                                obj.remove_identification().await;
                            }
                        ))
                    }
                ))
                .build()]);
            }

            if self.item().item_type() == "Series" {
                action_group.add_action_entries([gio::ActionEntry::builder("view-missing")
                    .activate(glib::clone!(
                        #[weak(rename_to = obj)]
                        self,
                        move |_, _, _| {
                            spawn(glib::clone!(
                                #[weak]
                                obj,
                                async move {
                                    obj.view_missing_episodes().await;
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

    async fn delete_item(&self) {
        let id = self.item().id();
        let id_clone = id.to_owned();

        let delete_info =
            match spawn_tokio(async move { JELLYFIN_CLIENT.delete_info(&id).await }).await {
                Ok(info) => info,
                Err(e) => {
                    self.toast(e.to_user_facing());
                    return;
                }
            };

        let alert_dialog = adw::AlertDialog::builder()
            .heading(gettext("Delete Item"))
            .title("Delete Item")
            .body(format!(
                "{}\n{}\n{}",
                gettext("Deleting this item will delete it from both the file system and your media library.\nThe following files and folders will be deleted:"),
                delete_info.paths.join("\n"),
                gettext("Are you sure you wish to continue?")
            ))
            .build();

        alert_dialog.add_response("close", &gettext("Cancel"));
        alert_dialog.add_response("delete", &gettext("Delete"));
        alert_dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

        alert_dialog.connect_response(
            Some("delete"),
            glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |_, _| {
                    let id_clone = id_clone.to_owned();

                    spawn(glib::clone!(
                        #[weak]
                        obj,
                        async move {
                            match spawn_tokio(
                                async move { JELLYFIN_CLIENT.delete(&id_clone).await },
                            )
                            .await
                            .and_then(|r| r.error_for_status().map_err(|e| e.into()))
                            {
                                Ok(_) => {
                                    obj.toast(gettext("Item deleted"));
                                }
                                Err(e) => {
                                    obj.toast(e.to_user_facing());
                                }
                            }
                        }
                    ));
                }
            ),
        );

        alert_dialog!(self, alert_dialog);
    }

    async fn view_missing_episodes(&self) {
        let binding = self.root();
        let Some(window) = binding.and_downcast_ref::<Window>() else {
            return;
        };

        let id = self.item().id();

        use adw::prelude::*;

        let dialog = MissingEpisodesDialog::new(&id);
        dialog.present(Some(window));
    }

    async fn remove_identification(&self) {
        let id = self.item().id();
        let alert_dialog = adw::AlertDialog::builder()
            .heading(gettext("Remove Identification"))
            .title(gettext("Remove Identification"))
            .body(gettext("Are you sure you wish to reset all metadata?"))
            .build();

        alert_dialog.add_response("close", &gettext("Cancel"));
        alert_dialog.add_response("remove", &gettext("Remove Identification"));
        alert_dialog.set_response_appearance("remove", adw::ResponseAppearance::Destructive);

        alert_dialog.connect_response(
            Some("remove"),
            glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |_, _| {
                    let id = id.to_owned();

                    spawn(glib::clone!(
                        #[weak]
                        obj,
                        async move {
                            match spawn_tokio(
                                async move { JELLYFIN_CLIENT.reset_metadata(&id).await },
                            )
                            .await
                            .and_then(|r| r.error_for_status().map_err(|e| e.into()))
                            {
                                Ok(_) => {
                                    obj.toast(gettext("Item deleted"));
                                }
                                Err(e) => {
                                    obj.toast(e.to_user_facing());
                                }
                            }
                        }
                    ));
                }
            ),
        );

        alert_dialog!(self, alert_dialog);
    }
}
