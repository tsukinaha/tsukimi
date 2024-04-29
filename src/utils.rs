use gtk::glib;
use gtk::prelude::*;
use std::env;

use crate::client::{network::RUNTIME, structs::Latest};
use crate::config::get_cache_dir;
use crate::ui::widgets::tu_list_item::tu_list_item_register;

pub fn _spawn_tokio_blocking<F>(fut: F) -> F::Output
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    let (sender, receiver) = tokio::sync::oneshot::channel();

    RUNTIME.spawn(async {
        let response = fut.await;
        sender.send(response)
    });
    receiver.blocking_recv().unwrap()
}

pub async fn spawn_tokio<F>(fut: F) -> F::Output
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    let (sender, receiver) = tokio::sync::oneshot::channel();

    RUNTIME.spawn(async {
        let response = fut.await;
        sender.send(response)
    });
    receiver.await.unwrap()
}

pub fn spawn<F>(fut: F)
where
    F: std::future::Future + 'static,
{
    let ctx = gtk::glib::MainContext::default();
    ctx.spawn_local(async move {
        fut.await;
    });
}

pub fn spawn_g_timeout<F>(future: F)
where
    F: std::future::Future<Output = ()> + 'static,
{
    gtk::glib::spawn_future_local(async move {
        // Give the GLib event loop a whole 250ms to animate the NavigtionPage
        gtk::glib::timeout_future(std::time::Duration::from_millis(250)).await;
        future.await;
    });
}

pub async fn get_data_with_cache<T, F>(
    id: String,
    item_type: &str,
    future: F,
) -> Result<T, reqwest::Error>
where
    T: for<'de> serde::Deserialize<'de> + Send + serde::Serialize + 'static,
    F: std::future::Future<Output = Result<T, reqwest::Error>> + 'static + Send,
{
    let mut path = get_path();
    path.push(format!("{}_{}.json", item_type, &id));

    if path.exists() {
        let data = std::fs::read_to_string(&path).expect("Unable to read file");
        let data: T = serde_json::from_str(&data).expect("JSON was not well-formatted");
        RUNTIME.spawn(async move {
            let v = future.await.unwrap();
            let s_data = serde_json::to_string(&v).expect("JSON was not well-formatted");
            std::fs::write(&path, s_data).expect("Unable to write file");
        });
        Ok(data)
    } else {
        let v = spawn_tokio(future).await?;
        let s_data = serde_json::to_string(&v).expect("JSON was not well-formatted");
        std::fs::write(&path, s_data).expect("Unable to write file");
        Ok(v)
    }
}

pub async fn _get_data<T, F>(id: String, item_type: &str, future: F) -> Result<T, reqwest::Error>
where
    T: for<'de> serde::Deserialize<'de> + Send + serde::Serialize + 'static,
    F: std::future::Future<Output = Result<T, reqwest::Error>> + 'static + Send,
{
    let mut path = get_path();
    path.push(format!("{}_{}.json", item_type, &id));
    let v = spawn_tokio(future).await?;
    let s_data = serde_json::to_string(&v).expect("JSON was not well-formatted");
    std::fs::write(&path, s_data).expect("Unable to write file");
    Ok(v)
}

pub async fn get_image_with_cache(
    id: &str,
    img_type: &str,
    tag: Option<u8>,
) -> Result<String, reqwest::Error> {
    let mut path = get_path();
    match img_type {
        "Pirmary" => path.push(format!("{}.png", id)),
        "Backdrop" => path.push(format!("b{}_{}.png", id, tag.unwrap())),
        "Thumb" => path.push(format!("t{}.png", id)),
        "Logo" => path.push(format!("l{}.png", id)),
        _ => path.push(format!("{}.png", id)),
    }
    let id = id.to_string();
    let img_type = img_type.to_string();
    if !path.exists() {
        spawn_tokio(async move { crate::client::network::get_image(id, &img_type, tag).await })
            .await?;
    }
    Ok(path.to_string_lossy().to_string())
}

async fn _s_path() {
    let pathbuf = get_path();
    std::fs::DirBuilder::new()
        .recursive(true)
        .create(pathbuf)
        .unwrap();
}

fn get_path() -> std::path::PathBuf {
    get_cache_dir(env::var("EMBY_NAME").unwrap()).expect("Failed to get cache dir!")
}

pub fn tu_list_item_factory() -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_bind(move |_, item| {
        let list_item = item
            .downcast_ref::<gtk::ListItem>()
            .expect("Needs to be ListItem");
        let entry = item
            .downcast_ref::<gtk::ListItem>()
            .expect("Needs to be ListItem")
            .item()
            .and_downcast::<glib::BoxedAnyObject>()
            .expect("Needs to be BoxedAnyObject");
        let latest: std::cell::Ref<Latest> = entry.borrow();
        if list_item.child().is_none() {
            tu_list_item_register(&latest, list_item, &latest.latest_type)
        }
    });
    factory
}
use adw::prelude::NavigationPageExt;
use gtk::subclass::prelude::ObjectSubclassIsExt;
pub fn tu_list_view_connect_activate(window: crate::ui::widgets::window::Window, result: &Latest) {
    let view = match window.current_view_name().as_str() {
        "homepage" => {
            window.set_title(&result.name);
            std::env::set_var("HOME_TITLE", &result.name);
            &window.imp().homeview
        }
        "searchpage" => {
            window.set_title(&result.name);
            std::env::set_var("SEARCH_TITLE", &result.name);
            &window.imp().searchview
        }
        "historypage" => {
            window.set_title(&result.name);
            std::env::set_var("HISTORY_TITLE", &result.name);
            &window.imp().historyview
        }
        _ => &window.imp().searchview,
    };
    match result.latest_type.as_str() {
        "Movie" => {
            window.set_title(&result.name);
            if view.find_page(result.name.as_str()).is_some() {
                view.pop_to_tag(result.name.as_str());
            } else {
                let item_page = crate::ui::widgets::movie::MoviePage::new(
                    result.id.clone(),
                    result.name.clone(),
                );
                item_page.set_tag(Some(&result.name));
                view.push(&item_page);
                window.set_pop_visibility(true)
            }
        }
        "Series" => {
            window.set_title(&result.name);
            if view.find_page(result.name.as_str()).is_some() {
                view.pop_to_tag(result.name.as_str());
            } else {
                let item_page =
                    crate::ui::widgets::item::ItemPage::new(result.id.clone(), result.id.clone());
                item_page.set_tag(Some(&result.name));
                view.push(&item_page);
                window.set_pop_visibility(true)
            }
        }
        "Episode" => {
            window.set_title(&result.name);
            if view.find_page(result.name.as_str()).is_some() {
                view.pop_to_tag(result.name.as_str());
            } else {
                let item_page = crate::ui::widgets::item::ItemPage::new(
                    result.series_id.as_ref().unwrap().clone(),
                    result.id.clone(),
                );
                item_page.set_tag(Some(&result.name));
                view.push(&item_page);
                window.set_pop_visibility(true)
            }
        }
        "People" => {
            window.set_title(&result.name);
            if view.find_page(result.name.as_str()).is_some() {
                view.pop_to_tag(result.name.as_str());
            } else {
                let item_page = crate::ui::widgets::actor::ActorPage::new(&result.id);
                item_page.set_tag(Some(&result.name));
                view.push(&item_page);
                window.set_pop_visibility(true)
            }
        }
        "BoxSet" => {
            window.toast("BoxSet not supported yet");
        }
        _ => {}
    }
}
