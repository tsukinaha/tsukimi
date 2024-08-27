use crate::client::client::EMBY_CLIENT;
use crate::client::network::RUNTIME;
use crate::ui::models::emby_cache_path;

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

pub async fn req_cache<T, F>(tag: &str, future: F) -> Result<T, reqwest::Error>
where
    T: for<'de> serde::Deserialize<'de> + Send + serde::Serialize + 'static,
    F: std::future::Future<Output = Result<T, reqwest::Error>> + 'static + Send,
{
    let mut path = emby_cache_path();
    path.push(format!("{}.json", tag));

    if path.exists() {
        let data = std::fs::read_to_string(&path).expect("Unable to read file");
        let data: T = serde_json::from_str(&data).expect("JSON was not well-formatted");
        RUNTIME.spawn(async move {
            let v = match future.await {
                Ok(v) => v,
                Err(_) => return,
            };
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
    path.push(format!("{}-{}-{}", id, img_type, tag.unwrap_or(0)));
    let id = id.to_string();
    let img_type = img_type.to_string();
    if !path.exists() {
        spawn_tokio(async move { EMBY_CLIENT.get_image(&id, &img_type, tag).await }).await?;
    }
    Ok(path.to_string_lossy().to_string())
}

pub async fn req_cache_single<T, F>(
    tag: &str,
    future: F,
    enable_cache: bool,
) -> Result<Option<T>, reqwest::Error>
where
    T: for<'de> serde::Deserialize<'de> + Send + serde::Serialize + 'static,
    F: std::future::Future<Output = Result<T, reqwest::Error>> + 'static + Send,
{
    let mut path = emby_cache_path();
    path.push(format!("{}.json", tag));

    if enable_cache {
        if path.exists() {
            let data = std::fs::read_to_string(&path).expect("Unable to read file");
            let data: T = serde_json::from_str(&data).expect("JSON was not well-formatted");
            Ok(Some(data))
        } else {
            Ok(None)
        }
    } else {
        let v = spawn_tokio(future).await?;
        let s_data = serde_json::to_string(&v).expect("JSON was not well-formatted");
        std::fs::write(&path, s_data).expect("Unable to write file");
        Ok(Some(v))
    }
}
