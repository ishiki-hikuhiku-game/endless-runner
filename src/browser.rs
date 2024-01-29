use anyhow::{anyhow, Result};
use futures::Future;
use wasm_bindgen::closure::WasmClosure;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::js_sys::ArrayBuffer;
use web_sys::{
    CanvasRenderingContext2d, Document, Element, HtmlCanvasElement, HtmlElement, HtmlImageElement,
    Response, Window,
};

macro_rules! log {
    ( $($t:tt)* ) => {
        web_sys::console::log_1(&format!( $($t)* ).into());
    }
}

macro_rules! error {
    ( $($t:tt)* ) => {
        web_sys::console::error_1(&format!( $($t)* ).into());
    }
}

pub fn window() -> Result<Window> {
    web_sys::window().ok_or_else(|| anyhow!("No Window Found"))
}

pub fn document() -> Result<Document> {
    window()?
        .document()
        .ok_or_else(|| anyhow!("No Document Found"))
}

pub fn canvas() -> Result<HtmlCanvasElement> {
    document()?
        .get_element_by_id("canvas")
        .ok_or_else(|| anyhow!("No Canvas Element found with ID 'canvas'"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|element| anyhow!("Error converting {:#?} to HtmlCanvasElement", element))
}

pub fn context() -> Result<CanvasRenderingContext2d> {
    canvas()?
        .get_context("2d")
        .map_err(|js_value| anyhow!("Error getting 2d context {:#?}", js_value))?
        .ok_or_else(|| anyhow!("No 2d context found"))?
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .map_err(|element| {
            anyhow!(
                "Error converting {:#?} to CanvasRenderingContext2d",
                element
            )
        })
}

pub fn spawn_local<F>(future: F)
where
    F: Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future)
}

/**
 * fetch apiを呼び出す。
 */
pub async fn fetch_with_str(resource: &str) -> Result<JsValue> {
    JsFuture::from(window()?.fetch_with_str(resource))
        .await
        .map_err(|err| anyhow!("error fetching {:#?}", err))
}

/**
 * fetch apiでリソースを読み込む
 */
pub async fn fetch_response(resource: &str) -> Result<Response> {
    let resp_velue = fetch_with_str(resource).await?;
    let resp: Response = resp_velue
        .dyn_into()
        .map_err(|element| anyhow!("Error converting {:#?} to Response", element))?;
    Ok(resp)
}

/**
 * jsonをfetch apiで読み込む
 */
pub async fn fetch_json(json_path: &str) -> Result<JsValue> {
    let resp = fetch_response(json_path).await?;
    JsFuture::from(
        resp.json()
            .map_err(|err| anyhow!("Could not get JSON from response {:#?}", err))?,
    )
    .await
    .map_err(|err| anyhow!("error fetching JSON {:#?}", err))
}

/**
 * バイトデータをfetch apiで読み込む
 */
pub async fn fetch_array_buffer(resource: &str) -> Result<ArrayBuffer> {
    let array_buffer = fetch_response(resource)
        .await?
        .array_buffer()
        .map_err(|err| anyhow!("Error loading array buffer {:#?}", err))?;

    JsFuture::from(array_buffer)
        .await
        .map_err(|err| anyhow!("Error converting array buffer into a future {:#?}", err))?
        .dyn_into()
        .map_err(|err| anyhow!("Error converting raw JSValue to ArrayBuffer {:#?}", err))
}

pub fn new_image() -> Result<HtmlImageElement> {
    HtmlImageElement::new().map_err(|err| anyhow!("Could not create HtmlImageElement: {:#?}", err))
}

// この関数はなぜか使えない
// pub fn closure_once<F, A, R>(fn_once: F) -> Closure<F::FnMut>
// where
//     F: 'static + WasmClosureFnOnce<A, F>,
// {
//     Closure::once(fn_once)
// }

pub type LoopClosure = Closure<dyn FnMut(f64)>;
pub fn request_animation_frame(callback: &LoopClosure) -> Result<i32> {
    window()?
        .request_animation_frame(callback.as_ref().unchecked_ref())
        .map_err(|err| anyhow!("Cannot request animation frame {:#?}", err))
}

pub fn closure_wrap<T: WasmClosure + ?Sized>(data: Box<T>) -> Closure<T> {
    Closure::wrap(data)
}

pub fn create_raf_closure(f: impl FnMut(f64) + 'static) -> LoopClosure {
    closure_wrap(Box::new(f))
}

pub fn now() -> Result<f64> {
    Ok(window()?
        .performance()
        .ok_or_else(|| anyhow!("Performance object not found"))?
        .now())
}

fn find_ui() -> Result<Element> {
    document().and_then(|doc| {
        doc.get_element_by_id("ui")
            .ok_or_else(|| anyhow!("UI element not found"))
    })
}

pub fn find_html_elemebt_by_id(id: &str) -> Result<HtmlElement> {
    document()
        .and_then(|doc| {
            doc.get_element_by_id(id)
                .ok_or_else(|| anyhow!("Elemebt with id {} not found", id))
        })
        .and_then(|element| {
            element
                .dyn_into::<HtmlElement>()
                .map_err(|err| anyhow!("cOuld not cast into HtmlElement {:#?}", err))
        })
}

/// UIを追加する。
///
/// html: HTMLの断片
///
/// Example:
///
///     let element = draw_ui("<button>New Game</button>");
pub fn draw_ui(html: &str) -> Result<Element> {
    let ui = find_ui()?;

    let _ = ui
        .insert_adjacent_html("afterbegin", html)
        .map(|_| ())
        .map_err(|err| anyhow!("Could not insert html {:#?}", err));
    Ok(ui)
}

/**
 * UIを隠す。追加されたUIはた高々一つと仮定している。
 */
pub fn hide_ui() -> Result<()> {
    let ui = find_ui()?;

    if let Some(child) = ui.first_child() {
        ui.remove_child(&child)
            .map(|_removed_child| ())
            .map_err(|err| anyhow!("Failed to remove child {:#?}", err))
            .and_then(|_unit| {
                canvas()?
                    .focus()
                    .map_err(|err| anyhow!("Could not set focus to canvas! {:#?}", err))
            })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_error_loading_json() {
        let json = fetch_json("not_therer.json").await;

        assert_eq!(json.is_err(), true);
    }
}
