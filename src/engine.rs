use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::channel::mpsc::{unbounded, UnboundedReceiver};
use futures::channel::oneshot::channel;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;
use web_sys::{
    AudioBuffer, AudioBufferSourceNode, AudioContext, CanvasRenderingContext2d, HtmlElement,
    HtmlImageElement,
};

use crate::browser::{self, LoopClosure};
use crate::sound::{self, Looping};

pub async fn load_image(source: &str) -> Result<HtmlImageElement> {
    let image = browser::new_image()?;

    let (complete_tx, complete_rx) = channel::<Result<()>>();
    let success_tx = Rc::new(Mutex::new(Some(complete_tx)));
    let error_tx = Rc::clone(&success_tx);
    // lock()によりmutex guardがResultに入って取り出される
    // ok()によりそれがOptionに変わる
    // and_thenはそのSomeの中の値にclosureを適用してOption値を返す。
    // takeによって、optがSome(X)ならばSome(X)が返されNoneならばNoneで、optはNoneに変わる。
    // つまり、ok()の結果がSome(Some(X))ならば、Some(X)が返される。
    // ok()の結果がSome(None)ならばNoneが返される。
    // というようにSomeが一つ剥がされる。
    // optの中身(あれば）にsend(Ok())がなされる
    let success_callback = Closure::once(Box::new(move || {
        if let Some(success_tx) = success_tx.lock().ok().and_then(|mut opt| opt.take()) {
            success_tx
                .send(Ok(()))
                .expect("can not send success message");
        }
    }));
    // let success_callback = browser::closure_once(move || {
    //     if let Some(success_tx) = success_tx.lock().ok().and_then(|mut opt| opt.take()) {
    //         success_tx.send(Ok(()));
    //     }
    // });
    let error_callback: Closure<dyn FnMut(JsValue)> = Closure::once(Box::new(move |err| {
        if let Some(error_tx) = error_tx.lock().ok().and_then(|mut opt| opt.take()) {
            error_tx
                .send(Err(anyhow!("Error loading Image: {:#?}", err)))
                .expect("can not send error message");
        }
    }));
    // let error_callback: Closure<dyn FnMut(JsValue)> = browser::closure_once(move |err| {
    //     if let Some(error_tx) = error_tx.lock().ok().and_then(|mut opt| opt.take()) {
    //         error_tx.send(Err(anyhow!("Error loading Image: {:#?}", err)));
    //     }
    // });
    image.set_onload(Some(success_callback.as_ref().unchecked_ref()));
    image.set_onerror(Some(error_callback.as_ref().unchecked_ref()));
    image.set_src(source);

    complete_rx.await??;

    Ok(image)
}

#[async_trait(?Send)]
pub trait Game {
    async fn intialize(&self) -> Result<Box<dyn Game>>;
    fn update(&mut self, key_state: &KeyState);
    fn draw(&self, renderer: &Renderer);
}

const FRAME_SIZE: f32 = 1.0 / 60.0 * 1000.0;

pub struct GameLoop {
    last_frame: f64,
    /**
     * 性能を良くするために、f32で行う。
     * JavaScriptはf64のみだが、ここでは問題ない。
     */
    accumulated_delta: f32,
}
type SharedLoopClosure = Rc<RefCell<Option<LoopClosure>>>;

impl GameLoop {
    pub async fn start(game: impl Game + 'static) -> Result<()> {
        let mut keyevent_receiver = prepare_input()?;
        let mut game = game.intialize().await?;
        let mut game_loop = GameLoop {
            last_frame: browser::now()?,
            accumulated_delta: 0.0,
        };
        let renderer = Renderer {
            context: browser::context()?,
        };
        let f: SharedLoopClosure = Rc::new(RefCell::new(None));
        let g = f.clone();

        let mut key_state = KeyState::new();
        *g.borrow_mut() = Some(browser::create_raf_closure(move |perf: f64| {
            process_input(&mut key_state, &mut keyevent_receiver);

            let frame_time = perf - game_loop.last_frame;
            game_loop.accumulated_delta += frame_time as f32;
            while game_loop.accumulated_delta > FRAME_SIZE {
                game.update(&key_state);
                game_loop.accumulated_delta -= FRAME_SIZE;
            }
            game_loop.last_frame = perf;
            game.draw(&renderer);

            if cfg!(debug_assertions) {
                unsafe {
                    draw_frame_rate(&renderer, frame_time);
                }
            }

            // ここでエラーが起きても何もできないのでunwrapでpanicさせる。
            browser::request_animation_frame(f.borrow().as_ref().unwrap()).unwrap();
        }));

        browser::request_animation_frame(
            g.borrow()
                .as_ref()
                .ok_or_else(|| anyhow!("GameLoop: Loop is None"))?,
        )?;
        Ok(())
    }
}

pub struct Renderer {
    context: CanvasRenderingContext2d,
}

pub struct Rect {
    pub position: Point,
    pub width: i16,
    pub height: i16,
}

impl Rect {
    pub const fn new(position: Point, width: i16, height: i16) -> Self {
        Rect {
            position,
            width,
            height,
        }
    }

    pub const fn new_from_x_y(x: i16, y: i16, width: i16, height: i16) -> Self {
        Rect::new(Point { x, y }, width, height)
    }

    pub fn x(&self) -> i16 {
        self.position.x
    }

    pub fn set_x(&mut self, x: i16) {
        self.position.x = x;
    }

    pub fn y(&self) -> i16 {
        self.position.y
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x() < (other.x() + other.width)
            && other.x() < self.x() + self.width
            && self.y() < (other.y() + other.height)
            && other.y() < self.y() + self.height
    }

    pub fn right(&self) -> i16 {
        self.x() + self.width
    }
}

impl Renderer {
    pub fn clear(&self, rect: &Rect) {
        self.context.clear_rect(
            rect.x().into(),
            rect.y().into(),
            rect.width.into(),
            rect.height.into(),
        )
    }

    pub fn draw_image(&self, image: &HtmlImageElement, frame: &Rect, destination: &Rect) {
        self.context
            .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                image,
                frame.x().into(),
                frame.y().into(),
                frame.width.into(),
                frame.height.into(),
                destination.x().into(),
                destination.y().into(),
                destination.width.into(),
                destination.height.into(),
            )
            .expect("Drawing is throwing exceptions! Unrecoverable error.");
    }

    pub fn draw_entire_image(&self, image: &HtmlImageElement, position: &Point) {
        self.context
            .draw_image_with_html_image_element(image, position.x.into(), position.y.into())
            .expect("Drawing is throwing exceptions! Unrecoverable error.");
    }

    /// デバッグ時に衝突範囲のbouding_boxを描画する
    #[allow(dead_code)]
    pub fn draw_rect(&self, rect: &Rect, color: (u8, u8, u8)) {
        let color_str = format!("rgb({}, {}, {})", color.0, color.1, color.2);
        self.context
            .set_stroke_style(&JsValue::from_str(&color_str));
        self.context.begin_path();
        self.context.move_to(rect.x().into(), rect.y().into());
        self.context
            .line_to(rect.x().into(), (rect.y() + rect.height).into());
        self.context.line_to(
            (rect.x() + rect.width).into(),
            (rect.y() + rect.height).into(),
        );
        self.context
            .line_to((rect.x() + rect.width).into(), rect.y().into());
        self.context.close_path();
        self.context.stroke();
    }

    #[allow(dead_code)]
    pub fn draw_text(&self, test: &str, location: &Point) -> Result<()> {
        self.context.set_font("16pt serif");
        self.context
            .fill_text(test, location.x.into(), location.y.into())
            .map_err(|err| anyhow!("Error filling text {:#?}", err))?;
        Ok(())
    }
}

/// フレームレートを表示するデバッグ用の関数
///
/// この関数はマルチスレッドでは安全ではないのでunsafe
/// ブラウザ環境ではこの関数がマルチスレッドで呼ばれることはない。
/// それよりも速さが大切なので、unsafeで良い。
unsafe fn draw_frame_rate(renderer: &Renderer, frame_time: f64) {
    static mut FRAMES_COUNTED: i32 = 0;
    static mut TOTAL_FRAME_TIME_MILLISECONDS: f64 = 0.0;
    static mut FRAME_RATE: i32 = 0;

    FRAMES_COUNTED += 1;
    TOTAL_FRAME_TIME_MILLISECONDS += frame_time;
    // 1秒が過ぎたら計測結果を更新
    if TOTAL_FRAME_TIME_MILLISECONDS > 1000.0 {
        FRAME_RATE = FRAMES_COUNTED;
        TOTAL_FRAME_TIME_MILLISECONDS = 0.0;
        FRAMES_COUNTED = 0;
    }
    if let Err(err) = renderer.draw_text(
        &format!("Frame Rate {}", FRAME_RATE),
        &Point { x: 400, y: 100 },
    ) {
        error!("Could not draw text {:#?}", err);
    }
}

enum KeyPress {
    KeyUp(web_sys::KeyboardEvent),
    KeyDown(web_sys::KeyboardEvent),
}

fn prepare_input() -> Result<UnboundedReceiver<KeyPress>> {
    let (keydown_sender, keyevent_receiver) = unbounded();
    let keydown_sender = Rc::new(RefCell::new(keydown_sender));
    let keyup_sender = Rc::clone(&keydown_sender);
    let onkeydown = browser::closure_wrap(Box::new(move |keycode: web_sys::KeyboardEvent| {
        keydown_sender
            .borrow_mut()
            .start_send(KeyPress::KeyDown(keycode))
            .expect("can not send keydown message");
    }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
    let onkeyup = browser::closure_wrap(Box::new(move |keycode: web_sys::KeyboardEvent| {
        keyup_sender
            .borrow_mut()
            .start_send(KeyPress::KeyUp(keycode))
            .expect("can not send keyup message");
    }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
    browser::canvas()
        .unwrap()
        .set_onkeydown(Some(onkeydown.as_ref().unchecked_ref()));
    browser::canvas()
        .unwrap()
        .set_onkeyup(Some(onkeyup.as_ref().unchecked_ref()));
    onkeydown.forget();
    onkeyup.forget();
    Ok(keyevent_receiver)
}

pub struct KeyState {
    pressed_keys: HashMap<String, web_sys::KeyboardEvent>,
}

impl KeyState {
    fn new() -> Self {
        KeyState {
            pressed_keys: HashMap::new(),
        }
    }
    pub fn is_pressed(&self, code: &str) -> bool {
        self.pressed_keys.contains_key(code)
    }
    fn set_pressed(&mut self, code: &str, event: web_sys::KeyboardEvent) {
        self.pressed_keys.insert(code.into(), event);
    }
    fn set_released(&mut self, code: &str) {
        self.pressed_keys.remove(code);
    }
}

fn process_input(state: &mut KeyState, keyevent_receiver: &mut UnboundedReceiver<KeyPress>) {
    loop {
        match keyevent_receiver.try_next() {
            Ok(None) => break,
            Err(_err) => break,
            Ok(Some(evt)) => match evt {
                KeyPress::KeyUp(evt) => state.set_released(&evt.code()),
                KeyPress::KeyDown(evt) => state.set_pressed(&evt.code(), evt),
            },
        }
    }
}

#[derive(Clone, Copy)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

pub struct Image {
    element: HtmlImageElement,
    position: Point,
}

impl Image {
    pub fn new(element: HtmlImageElement, position: Point) -> Self {
        Self { element, position }
    }
    pub fn draw(&self, renderer: &Renderer) {
        renderer.draw_entire_image(&self.element, &self.position)
    }
    /// 水平方向に移動させる。
    pub fn move_horisontally(&mut self, distance: i16) {
        self.set_x(self.position.x + distance);
    }
    pub fn set_x(&mut self, x: i16) {
        self.position.x = x;
    }
    pub fn right(&self) -> i16 {
        self.position.x + self.element.width() as i16
    }
}

pub struct Collider {
    image: Image,
    bounding_box: Rect,
}

impl From<Image> for Collider {
    fn from(image: Image) -> Self {
        Self::new(image)
    }
}

impl Collider {
    pub fn new(image: Image) -> Self {
        let bounding_box = Rect::new_from_x_y(
            image.position.x,
            image.position.y,
            image.element.width() as i16,
            image.element.height() as i16,
        );
        Self {
            image,
            bounding_box,
        }
    }

    pub fn draw(&self, renderer: &Renderer) {
        self.image.draw(renderer);
        if cfg!(debug_assertions) {
            renderer.draw_rect(&self.bounding_box, (255, 0, 0));
        }
    }

    pub fn bounding_box(&self) -> &Rect {
        &self.bounding_box
    }

    // 水平方向に移動させる。
    pub fn move_horisontally(&mut self, distance: i16) {
        self.bounding_box.set_x(self.bounding_box.x() + distance);
        self.image.move_horisontally(distance);
    }
}

#[derive(Deserialize, Clone)]
pub struct Sheet {
    pub frames: HashMap<String, Cell>,
}

#[derive(Deserialize, Clone)]
struct SheetRect {
    x: i16,
    y: i16,
    w: i16,
    h: i16,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Cell {
    frame: SheetRect,
    sprite_source_size: SheetRect,
}

impl Cell {
    pub fn rect(&self) -> Rect {
        Rect::new_from_x_y(self.frame.x, self.frame.y, self.frame.w, self.frame.h)
    }

    pub fn rect_start_x_y(&self, x: i16, y: i16) -> Rect {
        Rect::new_from_x_y(x, y, self.frame.w, self.frame.h)
    }

    pub fn rect_start_x_y_with_size(&self, x: i16, y: i16) -> Rect {
        Rect::new_from_x_y(
            x + self.sprite_source_size.x,
            y + self.sprite_source_size.y,
            self.frame.w,
            self.frame.h,
        )
    }
}

pub struct SpriteSheet {
    sheet: Sheet,
    image: HtmlImageElement,
}

impl SpriteSheet {
    pub fn new(sheet: Sheet, image: HtmlImageElement) -> Self {
        SpriteSheet { sheet, image }
    }
    pub fn cell(&self, name: &str) -> Option<&Cell> {
        self.sheet.frames.get(name)
    }
    pub fn draw(&self, renderer: &Renderer, source: &Rect, destination: &Rect) {
        renderer.draw_image(&self.image, source, destination);
    }
}

#[derive(Clone)]
pub struct Audio {
    context: AudioContext,
}

#[derive(Clone)]
pub struct Sound {
    buffer: AudioBuffer,
}

impl Audio {
    pub fn new() -> Result<Self> {
        Ok(Audio {
            context: sound::creat_audio_context()?,
        })
    }

    pub async fn load_sound(&self, filename: &str) -> Result<Sound> {
        let array_buffer = browser::fetch_array_buffer(filename).await?;
        let audio_buffer = sound::decode_auto_data(&self.context, &array_buffer).await?;
        Ok(Sound {
            buffer: audio_buffer,
        })
    }

    pub fn play_sound(&self, sound: &Sound, looping: Looping) -> Result<AudioBufferSourceNode> {
        sound::play_sound(&self.context, &sound.buffer, looping)
    }
}

pub fn add_click_handler(elem: HtmlElement) -> UnboundedReceiver<()> {
    let (mut click_sender, click_receiver) = unbounded();
    let on_click = browser::closure_wrap(Box::new(move || {
        click_sender.start_send(()).expect("can not send message");
    }) as Box<dyn FnMut()>);
    elem.set_onclick(Some(on_click.as_ref().unchecked_ref()));
    on_click.forget();
    click_receiver
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn two_rects_that_intersect_on_the_left() {
        let rect1 = Rect {
            position: Point { x: 10, y: 10 },
            height: 100,
            width: 100,
        };
        let rect2 = Rect {
            position: Point { x: 0, y: 10 },
            height: 100,
            width: 100,
        };

        assert_eq!(rect2.intersects(&rect1), true);
    }

    #[test]
    fn two_rects_that_intersect_on_the_top() {
        let rect1 = Rect {
            position: Point { x: 10, y: 10 },
            height: 100,
            width: 100,
        };
        let rect2 = Rect {
            position: Point { x: 10, y: 0 },
            height: 100,
            width: 100,
        };

        assert_eq!(rect2.intersects(&rect1), true);
    }

    #[test]
    fn two_rects_that_not_intersect() {
        let rect1 = Rect {
            position: Point { x: 10, y: 10 },
            height: 100,
            width: 100,
        };
        let rect2 = Rect {
            position: Point { x: 10, y: 110 },
            height: 100,
            width: 100,
        };

        assert_eq!(rect2.intersects(&rect1), false);
    }
}
