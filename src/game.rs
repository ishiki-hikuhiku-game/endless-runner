// #[cfg(test)]
// mod test_browser;
// #[cfg(not(test))]
// use crate::browser;
// #[cfg(test)]
// use test_browser as browser;

use crate::{
    browser,
    engine::{
        self, Audio, Cell, Collider, Game, Image, KeyState, Point, Rect, Renderer, Sheet, Sound,
        SpriteSheet,
    },
    segment::{
        platform_and_platform, platform_and_platform_and2, stone_and_platform, stone_and_platform2,
        stone_and_platform3, stone_and_platform4,
    },
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::channel::mpsc::UnboundedReceiver;
use gloo_utils::format::JsValueSerdeExt;
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::rc::Rc;
use web_sys::{AudioBufferSourceNode, HtmlImageElement};

use self::red_hat_boy_states::{
    Falling, Idle, Jumping, KnockedOut, RedHatBoyContext, RedHatBoyState, Running, Sliding,
};

const CANVAS_SIZE: i16 = 600;

///
pub struct WalkTheDog {
    machine: Option<WalkTheDogStateMachine>,
}

enum WalkTheDogStateMachine {
    Ready(WalkTheDogState<Ready>),
    Walking(WalkTheDogState<Walking>),
    GameOver(WalkTheDogState<GameOver>),
}

struct WalkTheDogState<T> {
    _state: T,
    scene: Scene,
}
struct Ready;
struct Walking;
struct GameOver {
    new_game_event: UnboundedReceiver<()>,
}

impl WalkTheDogStateMachine {
    fn update(self, key_state: &KeyState) -> Self {
        match self {
            WalkTheDogStateMachine::Ready(state) => state.update(key_state).into(),
            WalkTheDogStateMachine::Walking(state) => state.update(key_state).into(),
            WalkTheDogStateMachine::GameOver(state) => state.update(key_state).into(),
        }
    }

    fn draw(&self, renderer: &Renderer) {
        match self {
            WalkTheDogStateMachine::Ready(state) => state.draw(renderer),
            WalkTheDogStateMachine::Walking(state) => state.draw(renderer),
            WalkTheDogStateMachine::GameOver(state) => state.draw(renderer),
        }
    }
}

impl From<WalkTheDogState<Ready>> for WalkTheDogStateMachine {
    fn from(state: WalkTheDogState<Ready>) -> Self {
        WalkTheDogStateMachine::Ready(state)
    }
}
impl From<WalkTheDogState<Walking>> for WalkTheDogStateMachine {
    fn from(state: WalkTheDogState<Walking>) -> Self {
        WalkTheDogStateMachine::Walking(state)
    }
}
impl From<WalkTheDogState<GameOver>> for WalkTheDogStateMachine {
    fn from(state: WalkTheDogState<GameOver>) -> Self {
        WalkTheDogStateMachine::GameOver(state)
    }
}

enum ReadyEndState {
    Complete(WalkTheDogState<Walking>),
    Continue(WalkTheDogState<Ready>),
}

impl From<ReadyEndState> for WalkTheDogStateMachine {
    fn from(state: ReadyEndState) -> Self {
        match state {
            ReadyEndState::Complete(state) => state.into(),
            ReadyEndState::Continue(state) => state.into(),
        }
    }
}

impl WalkTheDogState<Ready> {
    fn update(mut self, key_state: &KeyState) -> ReadyEndState {
        self.scene.rhb.update();
        if key_state.is_pressed("ArrowRight") {
            ReadyEndState::Complete(self.start_running())
        } else {
            ReadyEndState::Continue(self)
        }
    }
}

impl From<WalkingEndState> for WalkTheDogStateMachine {
    fn from(state: WalkingEndState) -> Self {
        match state {
            WalkingEndState::Complete(state) => state.into(),
            WalkingEndState::Continue(state) => state.into(),
        }
    }
}

impl WalkTheDogState<Ready> {
    fn start_running(self) -> WalkTheDogState<Walking> {
        let mut scene = self.scene;
        scene.rhb.run_right(
            scene.audio.clone(),
            scene.background_music(),
            &mut scene.sound_nodes,
        );
        WalkTheDogState {
            _state: Walking,
            scene,
        }
    }
}

#[allow(clippy::large_enum_variant)]
enum WalkingEndState {
    Complete(WalkTheDogState<GameOver>),
    Continue(WalkTheDogState<Walking>),
}

impl WalkTheDogState<Walking> {
    fn update(self, key_state: &KeyState) -> WalkingEndState {
        let mut scene = self.scene;
        let horizontal_velocity = scene.horizontal_velocity();
        if key_state.is_pressed("ArrowUp") {
            scene.rhb.jump(scene.audio.clone(), scene.jumping_sound());
        }
        if key_state.is_pressed("ArrowDown") {
            scene.rhb.slide();
        }
        scene.rhb.update();
        scene.obstacles.retain(|obstacle| obstacle.right() > 0);
        scene.obstacles.iter_mut().for_each(|obstacle| {
            obstacle.move_horisontally(horizontal_velocity);
            obstacle.check_intersection(&mut scene.rhb, scene.sound_nodes.clone());
        });
        let [background1, background2] = &mut scene.backgrounds;
        background1.move_horisontally(horizontal_velocity);
        background2.move_horisontally(horizontal_velocity);
        if background1.right() < 0 {
            background1.set_x(background2.right());
        }
        if background2.right() < 0 {
            background2.set_x(background1.right());
        }

        if scene.timeline < TIMELINE_MINIMUM {
            scene.generate_next_segment();
        } else {
            scene.timeline += horizontal_velocity;
        }
        if let RedHatBoyStateMachine::KnockedOut(_) = scene.rhb.state_machine {
            let receiver =
                browser::draw_ui("<button id=\"new-game\" type=\"button\">New Game</button>")
                    .and_then(|_unit| browser::find_html_elemebt_by_id("new-game"))
                    .map(engine::add_click_handler)
                    .unwrap();
            WalkingEndState::Complete(WalkTheDogState {
                _state: GameOver {
                    new_game_event: receiver,
                },
                scene,
            })
        } else {
            WalkingEndState::Continue(WalkTheDogState {
                _state: Walking,
                scene,
            })
        }
    }
}

#[allow(clippy::large_enum_variant)]
enum GameOverEndState {
    Complete(WalkTheDogState<Ready>),
    Continue(WalkTheDogState<GameOver>),
}

impl From<GameOverEndState> for WalkTheDogStateMachine {
    fn from(state: GameOverEndState) -> Self {
        match state {
            GameOverEndState::Complete(state) => WalkTheDogStateMachine::Ready(state),
            GameOverEndState::Continue(state) => WalkTheDogStateMachine::GameOver(state),
        }
    }
}

impl WalkTheDogState<GameOver> {
    fn update(mut self, key_state: &KeyState) -> GameOverEndState {
        if self._state.new_game_pressed() || key_state.is_pressed("Enter") {
            GameOverEndState::Complete(self.new_game())
        } else {
            GameOverEndState::Continue(self)
        }
    }

    fn new_game(self) -> WalkTheDogState<Ready> {
        browser::hide_ui().expect("Can not hide UI elements");
        WalkTheDogState {
            _state: Ready,
            scene: Scene::reset(self.scene),
        }
    }
}

impl GameOver {
    fn new_game_pressed(&mut self) -> bool {
        matches!(self.new_game_event.try_next(), Ok(Some(())))
    }
}

pub struct Scene {
    rhb: RedHatBoy,
    backgrounds: [Image; 2],
    obstacle_sheet: Rc<SpriteSheet>,
    obstacles: Vec<Box<dyn Obstacle<RedHatBoy>>>,
    timeline: i16,
    stone_element: HtmlImageElement,
    audio: Rc<Audio>,
    sound_collection: HashMap<String, Rc<Sound>>,
    sound_nodes: HashMap<String, Rc<AudioBufferSourceNode>>,
}

const JUMPING_SOUND_FILENAME: &str = "sounds/SFX_Jump_23.mp3";
const BACKGROUND_MUSIC_FILENAME: &str = "sounds/background_song.mp3";
const BACKGROUND_MUSIC_NODENAME: &str = "background_music";
impl Scene {
    /// シーンの水平方向への移動速度
    fn horizontal_velocity(&self) -> i16 {
        -self.rhb.walking_speed()
    }

    fn jumping_sound(&self) -> Rc<Sound> {
        self.sound_collection
            .get(JUMPING_SOUND_FILENAME)
            .unwrap()
            .clone()
    }

    fn background_music(&self) -> Rc<Sound> {
        self.sound_collection
            .get(BACKGROUND_MUSIC_FILENAME)
            .unwrap()
            .clone()
    }

    /// 障害物を生成して環境に追加する
    fn generate_next_segment(&mut self) {
        let mut rng = thread_rng();
        let next_segment = rng.gen_range(0..6);
        let mut next_obstacles = match next_segment {
            0 => stone_and_platform(
                self.stone_element.clone(),
                self.obstacle_sheet.clone(),
                self.timeline + OBSTACLE_BUFFER,
            ),
            1 => stone_and_platform2(
                self.stone_element.clone(),
                self.obstacle_sheet.clone(),
                self.timeline + OBSTACLE_BUFFER,
            ),
            2 => stone_and_platform3(
                self.stone_element.clone(),
                self.obstacle_sheet.clone(),
                self.timeline + OBSTACLE_BUFFER,
            ),
            3 => stone_and_platform4(
                self.stone_element.clone(),
                self.obstacle_sheet.clone(),
                self.timeline + OBSTACLE_BUFFER,
            ),
            4 => platform_and_platform(
                self.stone_element.clone(),
                self.obstacle_sheet.clone(),
                self.timeline + OBSTACLE_BUFFER,
            ),
            5 => platform_and_platform_and2(
                self.stone_element.clone(),
                self.obstacle_sheet.clone(),
                self.timeline + OBSTACLE_BUFFER,
            ),
            _ => vec![],
        };
        self.timeline = rightmost(&next_obstacles);
        self.obstacles.append(&mut next_obstacles);
    }

    fn draw(&self, renderer: &Renderer) {
        renderer.clear(&Rect::new_from_x_y(0, 0, CANVAS_SIZE, CANVAS_SIZE));
        self.backgrounds.iter().for_each(|background| {
            background.draw(renderer);
        });
        self.rhb.draw(renderer);
        self.obstacles.iter().for_each(|obstacle| {
            obstacle.draw(renderer);
        });
    }

    fn reset(scene: Self) -> Self {
        let starting_obstacles = stone_and_platform(
            scene.stone_element.clone(),
            scene.obstacle_sheet.clone(),
            CANVAS_SIZE,
        );
        let timeline = rightmost(&starting_obstacles);
        Scene {
            rhb: RedHatBoy::reset(scene.rhb),
            backgrounds: scene.backgrounds,
            obstacle_sheet: scene.obstacle_sheet.clone(),
            obstacles: starting_obstacles,
            timeline,
            stone_element: scene.stone_element,
            audio: scene.audio,
            sound_collection: scene.sound_collection,
            sound_nodes: scene.sound_nodes,
        }
    }
}
const TIMELINE_MINIMUM: i16 = 500;
const OBSTACLE_BUFFER: i16 = 20;
#[async_trait(?Send)]
impl Game for WalkTheDog {
    async fn intialize(&self) -> Result<Box<dyn Game>> {
        match self.machine {
            None => {
                let json = browser::fetch_json("rhb_trimmed.json").await?;
                // キャラクターの設定
                let rhb = RedHatBoy::new(
                    json.into_serde::<Sheet>()?,
                    engine::load_image("rhb_trimmed.png").await?,
                );

                // 背景の設定
                let background_element = engine::load_image("BG.png").await?;
                let background_width = background_element.width() as i16;
                let background1 = Image::new(background_element.clone(), Point { x: 0, y: 0 });
                let background2 = Image::new(
                    background_element,
                    Point {
                        x: background_width,
                        y: 0,
                    },
                );
                // 障害物画像の取得
                let stone_element = engine::load_image("Stone.png").await?;
                // 足場の設定
                let tiles = browser::fetch_json("tiles.json").await?;
                let sprite_sheet = Rc::new(SpriteSheet::new(
                    tiles.into_serde::<Sheet>()?,
                    engine::load_image("tiles.png").await?,
                ));
                let starting_obstacles =
                    stone_and_platform(stone_element.clone(), sprite_sheet.clone(), CANVAS_SIZE);

                // 音声設定
                let audio = Rc::new(Audio::new()?).clone();
                let jumping_sound = audio.load_sound(JUMPING_SOUND_FILENAME).await?;
                let background_music = audio.load_sound(BACKGROUND_MUSIC_FILENAME).await?;
                let mut sound_collection = HashMap::new();
                sound_collection
                    .insert(String::from(JUMPING_SOUND_FILENAME), Rc::new(jumping_sound));
                sound_collection.insert(
                    String::from(BACKGROUND_MUSIC_FILENAME),
                    Rc::new(background_music),
                );
                let sound_nodes = HashMap::new();

                // タイムラインの設定
                let timeline = rightmost(&starting_obstacles);
                let scene = Scene {
                    rhb,
                    backgrounds: [background1, background2],
                    obstacle_sheet: sprite_sheet,
                    obstacles: starting_obstacles,
                    timeline,
                    stone_element,
                    audio,
                    sound_collection,
                    sound_nodes,
                };
                Ok(Box::new(WalkTheDog {
                    machine: Some(WalkTheDogStateMachine::Ready(WalkTheDogState {
                        _state: Ready,
                        scene,
                    })),
                }))
            }
            Some(_) => Err(anyhow!("Error Game is already initialized!")),
        }
    }
    fn update(&mut self, key_state: &KeyState) {
        if let Some(machine) = self.machine.take() {
            self.machine.replace(machine.update(key_state));
        }
        assert!(self.machine.is_some());
    }
    fn draw(&self, renderer: &Renderer) {
        if let Some(machine) = &self.machine {
            machine.draw(renderer);
        }
    }
}

impl<T> WalkTheDogState<T> {
    fn draw(&self, renderer: &Renderer) {
        self.scene.draw(renderer);
    }
}

impl WalkTheDog {
    pub fn new() -> Self {
        WalkTheDog { machine: None }
    }
}

pub struct RedHatBoy {
    state_machine: RedHatBoyStateMachine,
    sprite_sheet: Sheet,
    image: HtmlImageElement,
}

impl RedHatBoy {
    fn new(sheet: Sheet, image: HtmlImageElement) -> Self {
        RedHatBoy {
            state_machine: RedHatBoyStateMachine::Idle(RedHatBoyState::new()),
            sprite_sheet: sheet,
            image,
        }
    }
    fn reset(boy: Self) -> Self {
        RedHatBoy::new(boy.sprite_sheet, boy.image)
    }
    fn draw(&self, renderer: &Renderer) {
        let cell = self.current_sprite().expect("Cell not found");

        renderer.draw_image(&self.image, &cell.rect(), &self.destination_box());
        if cfg!(debug_assertions) {
            renderer.draw_rect(&self.bounding_box(), (0, 0, 255));
        }
    }
    fn current_sprite(&self) -> Option<&Cell> {
        let frame_name = format!(
            "{} ({}).png",
            self.state_machine.frame_name(),
            (self.state_machine.context().frame / 3) + 1
        );
        self.sprite_sheet.frames.get(&frame_name)
    }
    fn destination_box(&self) -> Rect {
        let cell = self.current_sprite().expect("Cell not found");
        cell.rect_start_x_y_with_size(
            self.state_machine.context().position.x,
            self.state_machine.context().position.y,
        )
    }
    fn bounding_box(&self) -> Rect {
        const X_OFFSET: i16 = 18;
        const Y_OFFSET: i16 = 14;
        const WIDTH_OFFSET: i16 = 28;
        let destination_box = self.destination_box();
        Rect::new_from_x_y(
            destination_box.x() + X_OFFSET,
            destination_box.y() + Y_OFFSET,
            destination_box.width - WIDTH_OFFSET,
            destination_box.height - Y_OFFSET,
        )
    }
    fn pos_y(&self) -> i16 {
        self.state_machine.context().position.y
    }
    fn velocity_y(&self) -> i16 {
        self.state_machine.context().velocity.y
    }
    /// RedHatBoy以外のものを逆方向に水平に動かすために必要なRedHatBoyのx速度
    fn walking_speed(&self) -> i16 {
        self.state_machine.context().velocity.x
    }
    fn update(&mut self) {
        self.state_machine = self.state_machine.transition(Event::Update);
    }
    fn run_right(
        &mut self,
        audio: Rc<Audio>,
        music: Rc<Sound>,
        sound_nodes: &mut HashMap<String, Rc<AudioBufferSourceNode>>,
    ) {
        self.state_machine = self
            .state_machine
            .transition(Event::Run(audio, music, sound_nodes));
    }
    fn jump(&mut self, audio: Rc<Audio>, sound: Rc<Sound>) {
        self.state_machine = self.state_machine.transition(Event::Jump(audio, sound));
    }
    fn slide(&mut self) {
        self.state_machine = self.state_machine.transition(Event::Slide);
    }
    fn land_on(&mut self, position_y: i16) {
        self.state_machine = self.state_machine.transition(Event::Land(position_y));
    }
    fn knock_out(&mut self, sound_nodes: HashMap<String, Rc<AudioBufferSourceNode>>) {
        self.state_machine = self.state_machine.transition(Event::KnockOut(sound_nodes));
    }
}

#[derive(Copy, Clone)]
enum RedHatBoyStateMachine {
    Idle(RedHatBoyState<Idle>),
    Running(RedHatBoyState<Running>),
    Jumping(RedHatBoyState<Jumping>),
    Sliding(RedHatBoyState<Sliding>),
    Falling(RedHatBoyState<Falling>),
    KnockedOut(RedHatBoyState<KnockedOut>),
}

impl From<RedHatBoyState<Idle>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<Idle>) -> Self {
        RedHatBoyStateMachine::Idle(state)
    }
}

impl From<RedHatBoyState<Running>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<Running>) -> Self {
        RedHatBoyStateMachine::Running(state)
    }
}

impl From<RedHatBoyState<Jumping>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<Jumping>) -> Self {
        RedHatBoyStateMachine::Jumping(state)
    }
}

impl From<RedHatBoyState<Sliding>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<Sliding>) -> Self {
        RedHatBoyStateMachine::Sliding(state)
    }
}

impl From<RedHatBoyState<Falling>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<Falling>) -> Self {
        RedHatBoyStateMachine::Falling(state)
    }
}

impl From<RedHatBoyState<KnockedOut>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<KnockedOut>) -> Self {
        RedHatBoyStateMachine::KnockedOut(state)
    }
}

pub enum Event<'a> {
    Run(
        Rc<Audio>,
        Rc<Sound>,
        &'a mut HashMap<String, Rc<AudioBufferSourceNode>>,
    ),
    Jump(Rc<Audio>, Rc<Sound>),
    Slide,
    KnockOut(HashMap<String, Rc<AudioBufferSourceNode>>),
    Land(i16),
    Update,
}

impl RedHatBoyStateMachine {
    fn transition(self, event: Event) -> Self {
        match (self, event) {
            (RedHatBoyStateMachine::Idle(state), Event::Run(audio, music, sound_nodes)) => {
                state.run(audio, music, sound_nodes).into()
            }
            (RedHatBoyStateMachine::Running(state), Event::Jump(audio, sound)) => {
                state.jump(audio, sound).into()
            }
            (RedHatBoyStateMachine::Running(state), Event::Slide) => state.slide().into(),
            (RedHatBoyStateMachine::Idle(state), Event::Land(position_y)) => {
                state.land_on(position_y).into()
            }
            (RedHatBoyStateMachine::Jumping(state), Event::Land(position_y)) => {
                state.land_on(position_y).into()
            }
            (RedHatBoyStateMachine::Running(state), Event::Land(position_y)) => {
                state.land_on(position_y).into()
            }
            (RedHatBoyStateMachine::Sliding(state), Event::Land(position_y)) => {
                state.land_on(position_y).into()
            }
            (RedHatBoyStateMachine::Running(state), Event::KnockOut(sound_nodes)) => {
                state.knock_out(sound_nodes).into()
            }
            (RedHatBoyStateMachine::Jumping(state), Event::KnockOut(sound_nodes)) => {
                state.knock_out(sound_nodes).into()
            }
            (RedHatBoyStateMachine::Sliding(state), Event::KnockOut(sound_nodes)) => {
                state.knock_out(sound_nodes).into()
            }
            (RedHatBoyStateMachine::Idle(state), Event::Update) => state.update().into(),
            (RedHatBoyStateMachine::Running(state), Event::Update) => state.update().into(),
            (RedHatBoyStateMachine::Jumping(state), Event::Update) => state.update().into(),
            (RedHatBoyStateMachine::Sliding(state), Event::Update) => state.update().into(),
            (RedHatBoyStateMachine::Falling(state), Event::Update) => state.update().into(),
            _ => self,
        }
    }
}

impl RedHatBoyStateMachine {
    /**
     * あまり良い実装ではないがenumの仕組み上仕方ない
     */
    fn frame_name(&self) -> &str {
        match self {
            RedHatBoyStateMachine::Idle(state) => state.frame_name(),
            RedHatBoyStateMachine::Running(state) => state.frame_name(),
            RedHatBoyStateMachine::Jumping(state) => state.frame_name(),
            RedHatBoyStateMachine::Sliding(state) => state.frame_name(),
            RedHatBoyStateMachine::Falling(state) => state.frame_name(),
            RedHatBoyStateMachine::KnockedOut(state) => state.frame_name(),
        }
    }
    /**
     * あまり良い実装ではないがenumの仕組み上仕方ない
     */
    fn context(&self) -> &RedHatBoyContext {
        match self {
            RedHatBoyStateMachine::Idle(state) => state.context(),
            RedHatBoyStateMachine::Running(state) => state.context(),
            RedHatBoyStateMachine::Jumping(state) => state.context(),
            RedHatBoyStateMachine::Sliding(state) => state.context(),
            RedHatBoyStateMachine::Falling(state) => state.context(),
            RedHatBoyStateMachine::KnockedOut(state) => state.context(),
        }
    }
}

mod red_hat_boy_states {
    use std::{collections::HashMap, rc::Rc};

    use web_sys::AudioBufferSourceNode;

    use crate::{
        engine::{Audio, Point, Sound},
        sound::Looping,
    };

    use super::{RedHatBoyStateMachine, BACKGROUND_MUSIC_NODENAME, CANVAS_SIZE};

    const FLOOR: i16 = 479;
    const PLAYER_HEIGHT: i16 = CANVAS_SIZE - FLOOR;
    const STATING_POINT: i16 = -20;
    const IDLE_FRAME_NAME: &str = "Idle";
    const RUN_FRAME_NAME: &str = "Run";
    const JUMP_FRAME_NAME: &str = "Jump";
    const SLIDING_FRAME_NAME: &str = "Slide";
    const FALLING_FRAME_NAME: &str = "Dead";
    const IDLE_FRAME: u8 = 29; // 10 * 3 - 1
    const RUNNING_FRAME: u8 = 23; // 8 * 3 - 1
    const JUMPING_FRAME: u8 = 35; // 12 * 3 - 1
    const SLIDING_FRAME: u8 = 14; // 5 * 3 - 1
    const FALLING_FRAME: u8 = 29; // 10 * 3 - 1
    const RUNNING_SPEED: i16 = 3;
    const JUMPING_SPEED: i16 = -25;
    /// 重力加速度
    ///
    /// 1フレームにy速度がどれだけ加速するか
    const GRAVITY: i16 = 1;
    /// 落下スピードの終端速度
    ///
    /// 落下スピードが高くなっていくと、
    /// 1フレームでの移動がブロックの幅を超える。
    /// するとブロックに着地せずにすり抜けてしまう
    /// そうならないように、落下速度の最高速度を決める
    /// 実際の落下でも空気抵抗により落下速度の加速は最終的な速度までで止まる
    const TERMINAL_VELOCITY_Y: i16 = 20;

    #[derive(Copy, Clone)]
    pub struct RedHatBoyState<S> {
        context: RedHatBoyContext,
        _state: S,
    }

    #[derive(Copy, Clone)]
    pub struct Idle;

    #[derive(Copy, Clone)]
    pub struct Running;

    #[derive(Copy, Clone)]
    pub struct Jumping;

    #[derive(Copy, Clone)]
    pub struct Sliding;

    #[derive(Copy, Clone)]
    pub struct Falling;

    #[derive(Copy, Clone)]
    pub struct KnockedOut;

    impl RedHatBoyState<Idle> {
        pub fn new() -> Self {
            RedHatBoyState {
                context: RedHatBoyContext {
                    frame: 0,
                    position: Point {
                        x: STATING_POINT,
                        y: FLOOR,
                    },
                    velocity: Point { x: 0, y: 0 },
                },
                _state: Idle {},
            }
        }

        pub fn frame_name(&self) -> &str {
            IDLE_FRAME_NAME
        }

        pub fn update(mut self) -> Self {
            self.context = self.context.update(IDLE_FRAME);
            self
        }

        pub fn run(
            self,
            audio: Rc<Audio>,
            music: Rc<Sound>,
            sound_nodes: &mut HashMap<String, Rc<AudioBufferSourceNode>>,
        ) -> RedHatBoyState<Running> {
            let audio_node = audio.play_sound(&music, Looping::Yes).unwrap();
            sound_nodes.insert(String::from(BACKGROUND_MUSIC_NODENAME), Rc::new(audio_node));
            RedHatBoyState {
                context: self.context.reset_frame().run_right(),
                _state: Running {},
            }
        }

        pub fn land_on(self, position_y: i16) -> RedHatBoyState<Running> {
            RedHatBoyState {
                context: self.context.set_on(position_y),
                _state: Running {},
            }
        }
    }

    impl RedHatBoyState<Running> {
        pub fn frame_name(&self) -> &str {
            RUN_FRAME_NAME
        }

        pub fn update(mut self) -> Self {
            self.context = self.context.update(RUNNING_FRAME);
            self
        }

        pub fn jump(self, audio: Rc<Audio>, sound: Rc<Sound>) -> RedHatBoyState<Jumping> {
            audio.play_sound(&sound, Looping::No).unwrap();
            RedHatBoyState {
                context: self
                    .context
                    .set_vertical_velocity(JUMPING_SPEED)
                    .reset_frame(),
                _state: Jumping {},
            }
        }

        pub fn slide(self) -> RedHatBoyState<Sliding> {
            RedHatBoyState {
                context: self.context.reset_frame(),
                _state: Sliding {},
            }
        }

        pub fn land_on(self, position_y: i16) -> RedHatBoyState<Running> {
            RedHatBoyState {
                context: self.context.set_on(position_y),
                _state: Running {},
            }
        }
    }

    impl RedHatBoyState<Jumping> {
        pub fn frame_name(&self) -> &str {
            JUMP_FRAME_NAME
        }

        pub fn update(mut self) -> JumpingEndstate {
            self.context = self.context.update(JUMPING_FRAME);
            if self.context.position.y >= FLOOR {
                JumpingEndstate::Complete(self.land_on(CANVAS_SIZE))
            } else {
                JumpingEndstate::Jumping(self)
            }
        }

        pub fn land_on(self, position_y: i16) -> RedHatBoyState<Running> {
            RedHatBoyState {
                context: self.context.reset_frame().set_on(position_y),
                _state: Running {},
            }
        }
    }

    pub enum JumpingEndstate {
        Complete(RedHatBoyState<Running>),
        Jumping(RedHatBoyState<Jumping>),
    }

    impl From<JumpingEndstate> for RedHatBoyStateMachine {
        fn from(end_state: JumpingEndstate) -> Self {
            match end_state {
                JumpingEndstate::Complete(running_state) => running_state.into(),
                JumpingEndstate::Jumping(jumping_state) => jumping_state.into(),
            }
        }
    }

    impl RedHatBoyState<Sliding> {
        pub fn frame_name(&self) -> &str {
            SLIDING_FRAME_NAME
        }

        pub fn update(mut self) -> SlidingEndState {
            self.context = self.context.update(SLIDING_FRAME);
            if self.context.frame >= SLIDING_FRAME {
                SlidingEndState::Complete(self.stand())
            } else {
                SlidingEndState::Sliding(self)
            }
        }

        pub fn stand(self) -> RedHatBoyState<Running> {
            RedHatBoyState {
                context: self.context.reset_frame(),
                _state: Running {},
            }
        }

        pub fn land_on(self, position_y: i16) -> RedHatBoyState<Sliding> {
            RedHatBoyState {
                context: self.context.set_on(position_y),
                _state: Sliding {},
            }
        }
    }

    pub enum SlidingEndState {
        Complete(RedHatBoyState<Running>),
        Sliding(RedHatBoyState<Sliding>),
    }

    impl From<SlidingEndState> for RedHatBoyStateMachine {
        fn from(end_state: SlidingEndState) -> Self {
            match end_state {
                SlidingEndState::Complete(running_state) => running_state.into(),
                SlidingEndState::Sliding(sliding_state) => sliding_state.into(),
            }
        }
    }

    impl RedHatBoyState<Falling> {
        pub fn frame_name(&self) -> &str {
            FALLING_FRAME_NAME
        }
        pub fn update(mut self) -> FallingEndState {
            self.context = self.context.update(FALLING_FRAME);
            if self.context.frame >= FALLING_FRAME {
                FallingEndState::Complete(RedHatBoyState {
                    context: self.context,
                    _state: KnockedOut,
                })
            } else {
                FallingEndState::Falling(self)
            }
        }
    }

    pub enum FallingEndState {
        Complete(RedHatBoyState<KnockedOut>),
        Falling(RedHatBoyState<Falling>),
    }

    impl From<FallingEndState> for RedHatBoyStateMachine {
        fn from(end_state: FallingEndState) -> Self {
            match end_state {
                FallingEndState::Complete(knocked_out_state) => knocked_out_state.into(),
                FallingEndState::Falling(falling_state) => falling_state.into(),
            }
        }
    }

    impl RedHatBoyState<KnockedOut> {
        pub fn frame_name(&self) -> &str {
            FALLING_FRAME_NAME
        }
    }

    impl<S> RedHatBoyState<S> {
        pub fn context(&self) -> &RedHatBoyContext {
            &self.context
        }
        pub fn knock_out(
            &self,
            mut sound_nodes: HashMap<String, Rc<AudioBufferSourceNode>>,
        ) -> RedHatBoyState<Falling> {
            let key = String::from(BACKGROUND_MUSIC_NODENAME);
            if let Some(background_music_node) = sound_nodes.get(&key) {
                background_music_node
                    .stop()
                    .expect("Can not stop background music!");
                sound_nodes.remove(&key);
            } else {
                log!("cannot find background music node!");
                sound_nodes.iter().for_each(|(k, _)| {
                    log!("{}", k);
                });
            }
            RedHatBoyState {
                context: self.context.reset_frame().stop(),
                _state: Falling {},
            }
        }
    }

    #[derive(Copy, Clone)]
    pub struct RedHatBoyContext {
        pub frame: u8,
        pub position: Point,
        pub velocity: Point,
    }

    impl RedHatBoyContext {
        /// RedHatBoyの状態を更新する。
        ///
        /// [`self.frame`]をを一つ進めると共に、
        /// [`self.velocity.y`]を[`self.position.y`]に加算する。
        ///
        /// 重力加速度を落下速度に加算する。
        ///
        /// RedHatBoyは同じ場所を走り続け、背景やその他のオブジェクトが[`self.velocity.x`]に従って逆方向に動くことで動きが実現される。
        ///
        /// * `frame_count` - [`self::frame`]の折り返し。この数字に達したら[`self.frame`]は0にリセットされる。
        pub fn update(mut self, frame_count: u8) -> Self {
            if self.frame < frame_count {
                self.frame += 1;
            } else {
                self.frame = 0;
            }
            self.position.y += self.velocity.y;
            if self.position.y > FLOOR {
                self.position.y = FLOOR;
                self.velocity.y = 0;
            }
            self.velocity.y += GRAVITY;
            if self.velocity.y > TERMINAL_VELOCITY_Y {
                self.velocity.y = TERMINAL_VELOCITY_Y;
            }
            self
        }
        fn reset_frame(mut self) -> Self {
            self.frame = 0;
            self
        }
        fn stop(mut self) -> Self {
            self.velocity.x = 0;
            self.velocity.y = 0;
            self
        }
        fn run_right(mut self) -> Self {
            self.velocity.x += RUNNING_SPEED;
            self
        }
        fn set_vertical_velocity(mut self, y: i16) -> Self {
            self.velocity.y = y;
            self
        }
        /// 地面に接地する
        /// 高さを地面の高さに調節し
        /// y速度を0にする。
        fn set_on(mut self, position_y: i16) -> Self {
            self.position.y = position_y - PLAYER_HEIGHT;
            self.velocity.y = 0;
            self
        }
    }
}

pub trait Obstacle<T> {
    fn check_intersection(
        &self,
        rhb: &mut T,
        sound_nodes: HashMap<String, Rc<AudioBufferSourceNode>>,
    );
    fn draw(&self, renderer: &Renderer);
    fn move_horisontally(&mut self, distance: i16);
    fn right(&self) -> i16;
}

struct Sprite {
    cell: Cell,
    offset: Point,
}

pub struct Platform {
    sheet: Rc<SpriteSheet>,
    bounding_boxes: Vec<Rect>,
    sprites: Vec<Sprite>,
    position: Point,
}

impl Platform {
    pub fn new(
        sheet: Rc<SpriteSheet>,
        bounding_boxes: &[&Rect],
        sprite_names: &[&str],
        offsets: &[Point],
        position: Point,
    ) -> Self {
        let bounding_boxes = bounding_boxes
            .iter()
            .map(|bounding_box| {
                Rect::new_from_x_y(
                    bounding_box.x() + position.x,
                    bounding_box.y() + position.y,
                    bounding_box.width,
                    bounding_box.height,
                )
            })
            .collect();
        let sprites = sprite_names
            .iter()
            .filter_map(|sprite_name| sheet.cell(sprite_name).cloned())
            .zip(offsets.iter())
            .map(|(cell, offset)| Sprite {
                cell,
                offset: *offset,
            })
            .collect();
        Platform {
            sheet,
            bounding_boxes,
            sprites,
            position,
        }
    }
}

impl Obstacle<RedHatBoy> for Platform {
    fn draw(&self, renderer: &Renderer) {
        self.sprites.iter().for_each(|sprite| {
            self.sheet.draw(
                renderer,
                &sprite.cell.rect(),
                &sprite.cell.rect_start_x_y(
                    self.position.x + sprite.offset.x,
                    self.position.y + sprite.offset.y,
                ),
            );
        });
        if cfg!(debug_assertions) {
            self.bounding_boxes.iter().for_each(|bounding_box| {
                renderer.draw_rect(bounding_box, (255, 255, 255));
            });
        }
    }
    fn check_intersection(
        &self,
        rhb: &mut RedHatBoy,
        _: HashMap<String, Rc<AudioBufferSourceNode>>,
    ) {
        if let Some(box_to_land_on) = self
            .bounding_boxes
            .iter()
            .find(|&bounding_box| rhb.bounding_box().intersects(bounding_box))
        {
            if rhb.velocity_y() > 0 && rhb.pos_y() < self.position.y {
                rhb.land_on(box_to_land_on.y());
            }
        }
    }
    fn move_horisontally(&mut self, distance: i16) {
        self.position.x += distance;
        self.bounding_boxes.iter_mut().for_each(|bounding_box| {
            bounding_box.set_x(bounding_box.position.x + distance);
        })
    }

    fn right(&self) -> i16 {
        self.bounding_boxes
            .iter()
            .map(|bounding_box| bounding_box.right())
            .max()
            .unwrap_or(0)
    }
}

pub struct Barrier {
    collider: Collider,
}

impl Obstacle<RedHatBoy> for Barrier {
    fn check_intersection(
        &self,
        rhb: &mut RedHatBoy,
        sound_nodes: HashMap<String, Rc<AudioBufferSourceNode>>,
    ) {
        if rhb.bounding_box().intersects(self.collider.bounding_box()) {
            rhb.knock_out(sound_nodes);
        }
    }
    fn draw(&self, renderer: &Renderer) {
        self.collider.draw(renderer);
    }
    fn move_horisontally(&mut self, distance: i16) {
        self.collider.move_horisontally(distance);
    }
    fn right(&self) -> i16 {
        self.collider.bounding_box().right()
    }
}

impl From<Image> for Barrier {
    fn from(image: Image) -> Self {
        Barrier {
            collider: image.into(),
        }
    }
}

fn rightmost(obstacl_list: &[Box<dyn Obstacle<RedHatBoy>>]) -> i16 {
    obstacl_list
        .iter()
        .map(|obstacle| obstacle.right())
        .max_by(|x, y| x.cmp(y))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::channel::mpsc::unbounded;
    use std::collections::HashMap;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    /// 新しいゲームが始まった時にUIを消す。
    ///
    /// TODO 現在の設計の問題点
    /// 新しいScene構造体を作るのが大変
    /// gameモジュールは、web-sysやwasm-bindgenに強く依存しすぎている。
    #[wasm_bindgen_test]
    fn test_transition_from_game_over_to_new_game() {
        // 準備
        let (_, receiver) = unbounded();
        let image = HtmlImageElement::new().unwrap();
        let rhb = RedHatBoy::new(
            Sheet {
                frames: HashMap::new(),
            },
            image.clone(),
        );
        let sprite_sheet = SpriteSheet::new(
            Sheet {
                frames: HashMap::new(),
            },
            image.clone(),
        );
        let audio = Audio::new().unwrap();
        let scene = Scene {
            rhb,
            backgrounds: [
                Image::new(image.clone(), Point { x: 0, y: 0 }),
                Image::new(image.clone(), Point { x: 0, y: 0 }),
            ],
            obstacles: vec![],
            obstacle_sheet: Rc::new(sprite_sheet),
            audio: Rc::new(audio),
            sound_collection: HashMap::new(),
            sound_nodes: HashMap::new(),
            stone_element: image.clone(),
            timeline: 0,
        };
        let document = browser::document().unwrap();
        let body = document.body().unwrap();
        body.insert_adjacent_html("afterbegin", "<canvas id=\"canvas\" tabindex=\"0\" height=\"600\" width=\"600\">Your browser does not support the canvas.</canvas>")
            .unwrap();
        body.insert_adjacent_html("afterbegin", "<div id=\"ui\"></div>")
            .unwrap();
        browser::draw_ui("<p>This is the UI</p>").unwrap();
        let ui = browser::find_html_elemebt_by_id("ui").unwrap();
        assert_eq!(ui.child_element_count(), 1);
        let state = WalkTheDogState {
            _state: GameOver {
                new_game_event: receiver,
            },
            scene,
        };

        // 実行
        state.new_game();

        // 確認
        let ui = browser::find_html_elemebt_by_id("ui").unwrap();
        assert_eq!(ui.child_element_count(), 0);
    }
}
