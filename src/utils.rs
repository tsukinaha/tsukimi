use gtk::glib;
use gtk::prelude::*;

use crate::client::{network::RUNTIME, structs::SimpleListItem};
use crate::ui::models::emby_cache_path;
use crate::ui::provider::tu_item::TuItem;
use crate::ui::widgets::singlelist::SingleListPage;
use crate::ui::widgets::tu_list_item::tu_list_item_register;

pub fn spawn_tokio_blocking<F>(fut: F) -> F::Output
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
    let mut path = emby_cache_path();
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

pub async fn get_data_with_cache_else<T, F>(
    id: String,
    item_type: &str,
    future: F,
) -> Result<T, reqwest::Error>
where
    T: for<'de> serde::Deserialize<'de> + Send + serde::Serialize + 'static,
    F: std::future::Future<Output = Result<T, reqwest::Error>> + 'static + Send,
{
    let mut path = emby_cache_path();
    path.push(format!("{}_{}.json", item_type, &id));

    if path.exists() {
        let data = std::fs::read_to_string(&path).expect("Unable to read file");
        let data: T = serde_json::from_str(&data).expect("JSON was not well-formatted");
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
    let mut path = emby_cache_path();
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
    let mut path = emby_cache_path();
    match img_type {
        "Primary" => path.push(id),
        "Backdrop" => path.push(format!("b{}_{}", id, tag.unwrap())),
        "Thumb" => path.push(format!("t{}", id)),
        "Logo" => path.push(format!("l{}", id)),
        _ => path.push(id),
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
    let pathbuf = emby_cache_path();
    std::fs::DirBuilder::new()
        .recursive(true)
        .create(pathbuf)
        .unwrap();
}

pub fn tu_list_item_factory(listtype: String) -> gtk::SignalListItemFactory {
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
        let latest: std::cell::Ref<SimpleListItem> = entry.borrow();
        if list_item.child().is_none() {
            tu_list_item_register(&latest, list_item, &listtype)
        }
    });
    factory
}
use adw::prelude::NavigationPageExt;
use gtk::subclass::prelude::ObjectSubclassIsExt;
pub fn tu_list_view_connect_activate(
    window: crate::ui::widgets::window::Window,
    result: &SimpleListItem,
    parentid: Option<String>,
) {
    let (view, title_var) = match window.current_view_name().as_str() {
        "homepage" => (&window.imp().homeview, "HOME_TITLE"),
        "searchpage" => (&window.imp().searchview, "SEARCH_TITLE"),
        "historypage" => (&window.imp().historyview, "HISTORY_TITLE"),
        _ => (&window.imp().searchview, "SEARCH_TITLE"),
    };
    window.set_title(&result.name);
    std::env::set_var(title_var, &result.name);

    match result.latest_type.as_str() {
        "Movie" => push_page(
            view,
            &window,
            &result.name,
            crate::ui::widgets::movie::MoviePage::new(result.id.clone(), result.name.clone()),
        ),
        "Series" => push_page(
            view,
            &window,
            &result.name,
            crate::ui::widgets::item::ItemPage::new(
                result.id.clone(),
                result.id.clone(),
                result.name.clone(),
            ),
        ),
        "Episode" => push_page(
            view,
            &window,
            &result.name,
            crate::ui::widgets::item::ItemPage::new(
                result.series_id.as_ref().unwrap().clone(),
                result.id.clone(),
                result.series_name.clone().unwrap_or("".to_string()),
            ),
        ),
        "Actor" | "Person" => push_page(
            view,
            &window,
            &result.name,
            crate::ui::widgets::actor::ActorPage::new(&result.id),
        ),
        "BoxSet" => push_page(
            view,
            &window,
            &result.name,
            crate::ui::widgets::boxset::BoxSetPage::new(&result.id),
        ),
        "MusicAlbum" => {
            let item = TuItem::from_simple(result, None);
            push_page(
                view,
                &window,
                &result.name,
                crate::ui::widgets::music_album::AlbumPage::new(item),
            )
        }
        _ => push_page(
            view,
            &window,
            &result.name,
            SingleListPage::new(
                result.id.clone(),
                "".to_string(),
                &result.latest_type,
                parentid,
            ),
        ),
    }
}

fn push_page<T: 'static + Clone + gtk::prelude::IsA<adw::NavigationPage>>(
    view: &adw::NavigationView,
    window: &crate::ui::widgets::window::Window,
    tag: &str,
    page: T,
) {
    if view.find_page(tag).is_some() {
        view.pop_to_tag(tag);
    } else {
        let item_page = page;
        item_page.set_tag(Some(tag));
        view.push(&item_page);
        window.set_pop_visibility(true);
    }
}
